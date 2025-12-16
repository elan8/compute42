use crate::pipeline::sources::{Document, ProjectContext};
use crate::pipeline::parser::JuliaParser;
use crate::pipeline::storage::CacheManager;
use crate::pipeline::{
    WorkspacePipeline, PackagePipeline, JuliaPipeline,
    sources::WorkspaceSource,
    storage::Index,
    query::{SymbolQuery, CompletionQuery},
    PackagePipelineInput,
    Pipeline,
};
use crate::features::{HoverProvider, DefinitionProvider, ReferencesProvider, DiagnosticsProvider};
use crate::features::diagnostics::incremental::IncrementalDiagnostics;
use crate::types::{Position, LspError, CompletionList, Location, Diagnostic};
use std::collections::HashMap;
use std::path::{PathBuf, Path};

/// Configuration for the embedded LSP service
#[derive(Debug, Clone)]
pub struct LspConfig {
    /// Path to Julia executable (required for future Julia IPC features)
    pub julia_executable: PathBuf,
    /// Project root directory
    pub project_root: Option<PathBuf>,
    /// Additional Julia environment variables
    pub julia_env: HashMap<String, String>,
    /// Enable enhanced local hover (scope-aware + type hints)
    pub enhanced_hover: bool,
    /// Enable Julia augmentation for types (background)
    pub augment_with_julia: bool,
    /// Custom Julia depot path (Compute42 uses com.compute42.dev/depot)
    pub julia_depot_path: Option<PathBuf>,
}

impl LspConfig {
    pub fn new(julia_executable: PathBuf) -> Self {
        Self {
            julia_executable,
            project_root: None,
            julia_env: HashMap::new(),
            enhanced_hover: true,
            augment_with_julia: false,
            julia_depot_path: None,
        }
    }
    
    pub fn with_project_root(mut self, project_root: PathBuf) -> Self {
        self.project_root = Some(project_root);
        self
    }
    
    pub fn with_env_var(mut self, key: String, value: String) -> Self {
        self.julia_env.insert(key, value);
        self
    }

    pub fn with_enhanced_hover(mut self, enabled: bool) -> Self {
        self.enhanced_hover = enabled;
        self
    }

    pub fn with_augmented_julia(mut self, enabled: bool) -> Self {
        self.augment_with_julia = enabled;
        self
    }

    pub fn with_depot_path(mut self, depot_path: PathBuf) -> Self {
        self.julia_depot_path = Some(depot_path);
        self
    }
}

/// Embedded LSP service for use in internals actor system
pub struct EmbeddedLspService {
    config: LspConfig,
    documents: HashMap<PathBuf, Document>,
    parser: JuliaParser,
    
    // Unified index (replaces SymbolTable, ReferenceTable, TypeRegistry)
    index: Index,
    
    // Project infrastructure
    project_context: Option<ProjectContext>,
    cache_manager: CacheManager,
    instance_id: usize,
    
    // Note: Base and package documentation are now stored in the Index
    // (merged during project opening)
    
    // Incremental diagnostics tracker
    incremental_diagnostics: IncrementalDiagnostics,
}

impl EmbeddedLspService {
    pub fn new(config: LspConfig) -> Self {
        let instance_id = 0usize; // Will be properly initialized after allocation
        
        let mut service = Self {
            config: config.clone(),
            documents: HashMap::new(),
            parser: JuliaParser::new(),
            index: Index::new(),
            project_context: None,
            cache_manager: CacheManager::new(),
            instance_id,
            incremental_diagnostics: IncrementalDiagnostics::new(),
        };
        // Get the address as a unique id after allocation
        service.instance_id = &service as *const _ as usize;
        log::debug!("LSP Service: Initialized");
        service
    }

    /// Open a project and initialize workspace indexing
    pub fn open_project(&mut self, project_root: PathBuf) -> Result<(), LspError> {
        log::info!("EmbeddedLspService[{:x}]: Opening project at {:?}", self.instance_id, project_root);
        
        // Update config so downstream InitProject uses the correct root
        self.config.project_root = Some(project_root.clone());

        // Create project context with depot path if available
        let context = if let Some(ref depot_path) = self.config.julia_depot_path {
            ProjectContext::with_depot_path(project_root.clone(), Some(depot_path.clone()))?
        } else {
            ProjectContext::new(project_root.clone())?
        };
        log::debug!("LSP Service: Opening project at {:?}", project_root);
        
        // Step 1: Extract Base/stdlib metadata (signatures, types, exports)
        // Pipeline handles its own cache checking and rebuilding
        let mut base_index = Index::new();
        let julia_pipeline = JuliaPipeline::new();
        match julia_pipeline.run(self.config.julia_executable.clone()) {
            Ok(index) => {
                base_index = index;
                log::info!("EmbeddedLspService: Base/stdlib metadata loaded/processed");
            }
            Err(e) => {
                log::warn!("EmbeddedLspService: Base/stdlib metadata processing failed: {}. Continuing without Base/stdlib metadata.", e);
            }
        }
        
        // Step 2: Extract package metadata if depot path is configured
        // Pipeline handles its own cache checking and rebuilding
        let mut package_index = Index::new();
        if let Some(ref depot_path) = self.config.julia_depot_path {
            log::info!("EmbeddedLspService: Processing packages from depot: {:?}", depot_path);
            let package_pipeline = PackagePipeline::new();
            let input = PackagePipelineInput {
                depot_path: depot_path.clone(),
                project_context: context.clone(),
            };
            match package_pipeline.run(input) {
                Ok(index) => {
                    package_index = index;
                    let signature_count: usize = package_index.get_all_modules().iter()
                        .map(|m| package_index.get_module_functions(m).len())
                        .sum();
                    log::info!("EmbeddedLspService: Package metadata processing complete ({} function signatures)", 
                        signature_count);
                }
                Err(e) => {
                    log::warn!("EmbeddedLspService: Package processing failed: {}. Continuing without package metadata.", e);
                }
            }
        }
        
        // Step 3: Merge Base and package indexes into combined index
        let mut combined_index = Index::new();
        combined_index.merge(base_index);
        combined_index.merge(package_index);
        
        // Promote submodule functions to parent modules (e.g., Flux.Losses.crossentropy -> Flux.crossentropy)
        // This ensures functions from submodules are available at the top-level module
        combined_index.promote_submodule_functions();
        
        // Step 4: Index workspace using pipeline with combined Base/package index as input
        // This allows workspace type inference to use Base/package signatures
        let workspace_source = WorkspaceSource::new(project_root.clone());
        let source_items = workspace_source.discover()?;
        log::info!("EmbeddedLspService: Discovered {} Julia files", source_items.len());
        
        let workspace_pipeline = WorkspacePipeline::new();
        // Pass combined index so workspace pipeline can use Base/package signatures for type inference
        self.index = workspace_pipeline.run_with_index(source_items, Some(combined_index))?;
        
        log::info!(
            "EmbeddedLspService: Workspace indexed - {} symbols",
            self.index.get_all_symbols().len()
        );
        
        // Store project context
        self.project_context = Some(context);
        
        log::debug!("LSP Service: Project opened - {} documents, {} symbols", 
                   self.documents.len(), self.index.get_all_symbols().len());
        
        Ok(())
    }
    
    /// Invalidate cache for a specific file
    pub fn invalidate_cache(&mut self, uri: &std::path::Path) {
        let uri_str = uri.to_string_lossy();
        self.cache_manager.invalidate_file(&uri_str);
    }
    
    /// Get the Julia executable path
    pub fn julia_executable(&self) -> &Path {
        &self.config.julia_executable
    }
    
    /// Get the project root directory
    pub fn project_root(&self) -> Option<&PathBuf> {
        self.config.project_root.as_ref()
    }
    
    /// Get Julia environment variables
    pub fn julia_env(&self) -> &HashMap<String, String> {
        &self.config.julia_env
    }
    
    /// Open/update a document
    pub fn update_document(&mut self, uri: PathBuf, content: String) -> Result<(), LspError> {
        log::trace!("LSP Service: Updating document {:?} ({} chars)", uri, content.len());
        
        // Invalidate cache for this document
        self.invalidate_cache(&uri);
        
        // Process file using pipeline
        let source_item = crate::pipeline::types::SourceItem {
            path: uri.clone(),
            content: content.clone(),
            metadata: crate::pipeline::types::FileMetadata::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                content.len() as u64,
            ),
        };
        
        let workspace_pipeline = WorkspacePipeline::new();
        let analysis = workspace_pipeline.run_single_file(source_item)?;
        
        // Merge analysis result into main index (replaces data for this file)
        self.index.merge_file(&uri, analysis)?;
        
        // Store the document for quick access
        let mut doc = Document::new(uri.to_string_lossy().to_string(), content);
        let mut parser = self.parser.create_parser()?;
        doc.parse(&mut parser)?;
        self.documents.insert(uri.clone(), doc);
        
        // Record change for incremental diagnostics
        if let Some(doc) = self.documents.get(&uri) {
            self.incremental_diagnostics.record_change(doc);
            // Update document cache timestamp for this file
            self.cache_manager.document_cache.update(uri.to_string_lossy().to_string(), doc.last_modified());
        }
        
        Ok(())
    }
    
    /// Get hover information (async for Julia LSP integration)
    pub async fn hover(&self, uri: &PathBuf, line: u32, character: u32) -> Option<String> {
        log::trace!("LSP Service: Hover request at {}:{}", line, character);
        
        let doc = self.documents.get(uri)?;
        let position = Position { line, character };
        
        // Use query engine for symbol resolution
        let symbol_query = SymbolQuery::new(&self.index);
        
        // Extract symbol name using SymbolResolver
        let tree = doc.tree()?;
        let text = doc.text();
        let resolver = crate::pipeline::query::SymbolResolver::new(tree, &text);
        let node = resolver.node_at_position(position.line, position.character)?;
        
        let symbol_name = resolver.extract_symbol_name(node)?;
        
        // Resolve symbol with scope awareness
        let _symbol = symbol_query.resolve_symbol_at(&symbol_name, &uri.to_string_lossy(), position)
            .or_else(|| symbol_query.find_symbol(&symbol_name));
        
        // Use HoverProvider for content building
        // Note: Base and package docs are now in the Index, so we pass None for BaseDocsRegistry
        let result = HoverProvider::hover(&self.index, doc, position, None, None, None).await?;
        Some(result.contents)
    }
    
    /// Get completion suggestions (synchronous for embedded use)
    pub fn complete(&self, uri: &PathBuf, line: u32, character: u32) -> Option<CompletionList> {
        let doc = self.documents.get(uri)?;
        let position = Position { line, character };
        
        // Use query engine for completion
        let query = CompletionQuery::new(&self.index);
        let prefix = Self::extract_prefix_from_position(doc, position);
        let completions = query.complete_in_file(uri, &prefix);
        
        Some(CompletionList {
            is_incomplete: false,
            items: completions,
        })
    }
    
    fn extract_prefix_from_position(doc: &Document, position: Position) -> String {
        if let Some(line_text) = doc.get_line(position.line as usize) {
            let char_pos = position.character.min(line_text.len() as u32) as usize;
            let text_before = &line_text[..char_pos];
            
            // Extract word before cursor
            if let Some(last_space) = text_before.rfind(|c: char| !c.is_alphanumeric() && c != '_') {
                text_before[last_space + 1..].to_string()
            } else {
                text_before.to_string()
            }
        } else {
            String::new()
        }
    }
    
    /// Find definition of symbol at position
    pub fn find_definition(&self, uri: &PathBuf, line: u32, character: u32) -> Option<Vec<Location>> {
        let doc = self.documents.get(uri)?;
        let position = Position { line, character };
        
        // Use DefinitionProvider with Index
        DefinitionProvider::find_definition(&self.index, doc, position)
    }
    
    /// Find references to symbol at position
    pub fn find_references(&self, uri: &PathBuf, line: u32, character: u32, include_declaration: bool) -> Option<Vec<Location>> {
        let doc = self.documents.get(uri)?;
        let position = Position { line, character };
        
        // Use ReferencesProvider with Index
        ReferencesProvider::find_references(&self.index, doc, position, include_declaration)
    }
    
    /// Get diagnostics for a document
    pub fn get_diagnostics(&self, uri: &PathBuf) -> Vec<Diagnostic> {
        log::trace!("LSP Service: Computing diagnostics for {:?}", uri);
        
        let Some(doc) = self.documents.get(uri) else {
            return Vec::new();
        };
        
        // Check if we should recompute (incremental diagnostics with debouncing)
        if !self.incremental_diagnostics.should_recompute(doc) {
            // Check cache for existing diagnostics
            let uri_str = uri.to_string_lossy().to_string();
            let version = doc.version();
            if let Some(cached) = self.cache_manager.diagnostics_cache.get(&uri_str, version) {
                return cached;
            }
        }
        
        // Check cache first (even if we should recompute, cache might be valid)
        let uri_str = uri.to_string_lossy().to_string();
        let version = doc.version();
        if let Some(cached) = self.cache_manager.diagnostics_cache.get(&uri_str, version) {
            return cached;
        }
        
        // Compute diagnostics with context using Index
        let depot_path = self.config.julia_depot_path.as_deref();
        let manifest = self.project_context.as_ref().and_then(|ctx| ctx.manifest_toml.as_ref());
        let diagnostics = DiagnosticsProvider::compute_diagnostics_with_context(
            doc,
            Some(&self.index),
            depot_path,
            manifest,
        );
        
        // Cache the results
        self.cache_manager.diagnostics_cache.put(&uri_str, version, diagnostics.clone());
        
        log::trace!("LSP Service: Computed {} diagnostics", diagnostics.len());
        
        diagnostics
    }
    
    /// Get code actions for a diagnostic
    pub fn get_code_actions(&self, uri: &PathBuf, diagnostic: &Diagnostic) -> Vec<crate::types::CodeAction> {
        let Some(doc) = self.documents.get(uri) else {
            return Vec::new();
        };
        
        let Some(tree) = doc.tree() else {
            return Vec::new();
        };
        
        let text = doc.text();
        let uri_str = uri.to_string_lossy().to_string();
        let mut actions = crate::features::CodeActionsProvider::get_actions(diagnostic, tree, &text);
        
        // Fill in URI for all actions
        for action in &mut actions {
            if let Some(ref mut edit) = action.edit {
                for (file_uri, _) in &mut edit.changes {
                    if file_uri.is_empty() {
                        *file_uri = uri_str.clone();
                    }
                }
            }
        }
        
        actions
    }
    
    /// Get code actions for all diagnostics in a document
    pub fn get_code_actions_for_document(&self, uri: &PathBuf) -> Vec<crate::types::CodeAction> {
        let diagnostics = self.get_diagnostics(uri);
        let Some(doc) = self.documents.get(uri) else {
            return Vec::new();
        };
        
        let Some(tree) = doc.tree() else {
            return Vec::new();
        };
        
        let text = doc.text();
        let uri_str = uri.to_string_lossy().to_string();
        let mut actions = crate::features::CodeActionsProvider::get_actions_for_diagnostics(&diagnostics, tree, &text);
        
        // Fill in URI for all actions
        for action in &mut actions {
            if let Some(ref mut edit) = action.edit {
                for (file_uri, _) in &mut edit.changes {
                    if file_uri.is_empty() {
                        *file_uri = uri_str.clone();
                    }
                }
            }
        }
        
        actions
    }
    
    /// Get document count
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }
    
    /// Check if document exists
    pub fn has_document(&self, uri: &PathBuf) -> bool {
        self.documents.contains_key(uri)
    }
    
    /// Get configuration
    pub fn config(&self) -> &LspConfig {
        &self.config
    }

    /// Debug method to get symbol count
    pub fn debug_symbol_count(&self) -> usize {
        self.index.get_all_symbols().len()
    }
    

    /// Create a new parser instance
    pub fn create_parser() -> Result<JuliaParser, LspError> {
        Ok(JuliaParser::new())
    }
    
    /// Shutdown the service and clean up resources
    pub async fn shutdown(&mut self) -> Result<(), LspError> {
        log::debug!("LSP Service: Shutting down");
        // No cleanup needed - BaseDocsRegistry is just a HashMap
        Ok(())
    }
}
