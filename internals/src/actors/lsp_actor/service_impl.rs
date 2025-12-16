// LSP service implementation using EmbeddedLspService from languageserver crate

use log::{debug, warn};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::RwLock as TokioRwLock;

use languageserver::embedded::{EmbeddedLspService, LspConfig};
use crate::types::{
    LspCallHierarchyItem, LspCodeAction, LspCompletionItem, LspDiagnostic, LspDocumentSymbol,
    LspHover, LspInlayHint, LspLocation, LspMarkedString, LspPosition, LspRange, LspSemanticToken, LspSignatureHelp,
    LspTextEdit, LspWorkspaceEdit,
};

use super::type_conversions::*;
use super::utils::LspUtils;

/// LSP service implementation using EmbeddedLspService
#[derive(Clone)]
pub struct LspService {
    service: Arc<TokioRwLock<Option<EmbeddedLspService>>>,
    config: Arc<RwLock<LspConfig>>,
    is_running: Arc<RwLock<bool>>,
    current_project: Arc<RwLock<Option<String>>>,
    utils: LspUtils,
}

impl LspService {
    /// Create a new LSP service implementation
    /// Note: EmbeddedLspService is created lazily on first use (when start_lsp_server is called)
    /// This ensures base docs extraction is complete before loading the registry
    pub fn new(julia_executable: PathBuf, project_root: Option<PathBuf>, depot_path: Option<PathBuf>) -> Self {
        debug!("LspService: Creating with Julia executable: {:?}, project_root: {:?}, depot_path: {:?}", julia_executable, project_root, depot_path);
        
        let mut config = LspConfig::new(julia_executable)
            .with_project_root(project_root.unwrap_or_else(|| PathBuf::from(".")))
            .with_enhanced_hover(true);
        
        // Set depot path if provided
        if let Some(depot_path) = depot_path {
            config = config.with_depot_path(depot_path);
        }
        
        Self {
            service: Arc::new(TokioRwLock::new(None)),
            config: Arc::new(RwLock::new(config)),
            is_running: Arc::new(RwLock::new(false)),
            current_project: Arc::new(RwLock::new(None)),
            utils: LspUtils::new(),
        }
    }
    
    /// Update Julia executable path in config
    /// If EmbeddedLspService has already been created, it will be cleared and recreated on next use
    pub fn update_julia_executable(&self, julia_executable: PathBuf) {
        debug!("LspService: Updating Julia executable path to: {:?}", julia_executable);
        let mut config_guard = self.config.write().unwrap();
        config_guard.julia_executable = julia_executable;
        debug!("LspService: Julia executable path updated successfully");
    }
    
    /// Ensure EmbeddedLspService is created (lazy initialization)
    async fn ensure_service(&self) -> Result<(), String> {
        let mut service_guard = self.service.write().await;
        if service_guard.is_none() {
            debug!("LspService: Creating EmbeddedLspService (lazy initialization)");
            let config = self.config.read().unwrap().clone();
            let service = EmbeddedLspService::new(config);
            *service_guard = Some(service);
            debug!("LspService: EmbeddedLspService created successfully");
        }
        Ok(())
    }
    
    /// Get a reference to the service, ensuring it's created first
    async fn get_service(&self) -> Result<tokio::sync::RwLockReadGuard<'_, Option<EmbeddedLspService>>, String> {
        self.ensure_service().await?;
        Ok(self.service.read().await)
    }
    
    /// Get a mutable reference to the service, ensuring it's created first
    async fn get_service_mut(&self) -> Result<tokio::sync::RwLockWriteGuard<'_, Option<EmbeddedLspService>>, String> {
        self.ensure_service().await?;
        Ok(self.service.write().await)
    }
    
    /// Update project root for the service
    pub async fn update_project_root(&self, project_root: PathBuf) {
        debug!("LspService: Updating project root to: {:?}", project_root);
        let mut current_project = self.current_project.write().unwrap();
        *current_project = Some(project_root.to_string_lossy().to_string());
        debug!("LspService: Project root updated successfully");
    }

    /// Initialize the Julia LSP process (no-op - JuliaLspProvider removed)
    pub async fn initialize_julia_lsp(&self) -> Result<(), String> {
        debug!("LspService: Julia LSP initialization no longer needed (using BaseDocsRegistry instead)");
        Ok(())
    }

    /// Check if LSP is running
    pub fn is_running(&self) -> bool {
        *self.is_running.read().unwrap()
    }

    /// Set running state
    pub fn set_running(&self, running: bool) {
        *self.is_running.write().unwrap() = running;
    }

    /// Get current project
    pub fn current_project(&self) -> Option<String> {
        self.current_project.read().unwrap().clone()
    }

    /// Set current project
    pub fn set_current_project(&self, project: Option<String>) {
        *self.current_project.write().unwrap() = project;
    }

    /// Start LSP server
    pub async fn start_lsp_server(&self, project_path: String) -> Result<(), String> {
        debug!("LspService: Starting LSP server for project: {}", project_path);
        
        // Ensure service is created (lazy initialization - happens after base docs extraction)
        self.ensure_service().await?;
        
        // Check if already running for the same project
        let is_running = *self.is_running.read().unwrap();
        let current_project = self.current_project.read().unwrap().clone();
        if is_running && current_project.as_ref() == Some(&project_path) {
            debug!("LspService: LSP server already running for project {}", project_path);
            return Ok(());
        }
        
        debug!("LspService: Updating project root to: {}", project_path);
        // Update project root
        let project_root = PathBuf::from(&project_path);
        self.update_project_root(project_root.clone()).await;
        debug!("LspService: Project root updated successfully");
        
        // Open project and trigger workspace indexing
        debug!("LspService: Opening project and indexing workspace");
        let mut service_guard = self.get_service_mut().await?;
        let service = service_guard.as_mut().unwrap();
        match service.open_project(project_root) {
            Ok(()) => {
                debug!("LspService: Project opened and indexed successfully");
            }
            Err(e) => {
                // Log warning but don't fail - we can still work without project indexing
                warn!("LspService: Failed to open project for indexing: {}. Continuing without workspace indexing.", e);

            }
        }
        let _ = service; // Release write lock
        
        debug!("LspService: Setting service state to running");
        *self.is_running.write().unwrap() = true;
        *self.current_project.write().unwrap() = Some(project_path);
        
        debug!("LspService: LSP server started successfully");
        Ok(())
    }

    /// Stop LSP server
    pub async fn stop_lsp_server(&self) -> Result<(), String> {
        debug!("LspService: Stopping LSP server");
        
        *self.is_running.write().unwrap() = false;
        *self.current_project.write().unwrap() = None;
        
        // Clear the EmbeddedLspService so it can be recreated with updated config if needed
        let mut service_guard = self.service.write().await;
        if service_guard.is_some() {
            debug!("LspService: Clearing EmbeddedLspService");
            *service_guard = None;
        }
        
        debug!("LspService: LSP server stopped successfully");
        Ok(())
    }
    
    /// Set Julia LSP stderr sender (no-op - JuliaLspProvider removed)
    pub async fn set_julia_lsp_stderr_sender(
        &self,
        _sender: tokio::sync::mpsc::UnboundedSender<String>,
    ) -> Result<(), String> {
        debug!("LspService: Julia LSP stderr sender no longer needed (JuliaLspProvider removed)");
        Ok(())
    }

    // Text document synchronization
    pub async fn notify_did_open(
        &self,
        uri: String,
        content: String,
        _language: String,
    ) -> Result<(), String> {
        let mut service_guard = self.get_service_mut().await?;
        let service = service_guard.as_mut().unwrap();
        // Convert URI to proper file path
        let path_str = self.utils.uri_to_path(&uri);
        let path = PathBuf::from(&path_str);
        
        service.update_document(path.clone(), content)
            .map_err(|e| format!("Failed to update document: {}", e))?;
        Ok(())
    }

    pub async fn notify_did_change(&self, uri: String, content: String) -> Result<(), String> {
        let mut service_guard = self.get_service_mut().await?;
        let service = service_guard.as_mut().unwrap();
        // Convert URI to proper file path
        let path_str = self.utils.uri_to_path(&uri);
        let path = PathBuf::from(&path_str);
        
        service.update_document(path, content)
            .map_err(|e| format!("Failed to update document: {}", e))
    }

    pub async fn notify_did_close(&self, _uri: String) -> Result<(), String> {
        // Note: EmbeddedLspService doesn't have a close method yet
        // This is a no-op for now, but could be implemented if needed
        Ok(())
    }

    pub async fn notify_did_save(&self, _uri: String) -> Result<(), String> {
        // Document save is handled by the update_document method
        // No additional processing needed on save
        Ok(())
    }

    // Completion and IntelliSense
    pub async fn get_completions(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspCompletionItem>, String> {
        let service_guard = self.get_service().await?;
        let service = service_guard.as_ref().unwrap();
        // Convert URI to proper file path
        let path_str = self.utils.uri_to_path(&uri);
        let path = PathBuf::from(&path_str);
        let languageserver_pos = lsp_position_to_position(position);
        
        if let Some(completion_list) = service.complete(&path, languageserver_pos.line, languageserver_pos.character) {
            let items: Vec<LspCompletionItem> = completion_list.items.into_iter()
                .map(completion_item_to_lsp)
                .collect();
            Ok(items)
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_completions_with_content(
        &self,
        uri: String,
        position: LspPosition,
        content: String,
    ) -> Result<Vec<LspCompletionItem>, String> {
        debug!("LspService: get_completions_with_content for {} at {:?} ({} chars)", uri, position, content.len());
        // Note: This method doesn't use the service, it creates a temporary document
        // Convert URI to proper file path
        let path_str = self.utils.uri_to_path(&uri);
        let path = std::path::PathBuf::from(&path_str);
        // Build a temporary Document and parser
        use languageserver::pipeline::parser::JuliaParser;
        use languageserver::pipeline::sources::Document;
        use languageserver::pipeline::types::SourceItem;
        
        let content_len = content.len();
        let mut parser = JuliaParser::new().create_parser().map_err(|e| format!("Failed to create parser: {}", e))?;
        let mut doc = Document::new(uri.clone(), content.clone());
        if let Err(e) = doc.parse(&mut parser) {
            debug!("get_completions_with_content: failed to parse: {}", e);
            return Ok(vec![]);
        }
        // Create a minimal Index for this document using the pipeline
        let source_item = SourceItem {
            path: path.clone(),
            content,
            metadata: languageserver::pipeline::types::FileMetadata::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                content_len as u64,
            ),
        };
        let pipeline = languageserver::pipeline::WorkspacePipeline::new();
        let analysis = pipeline.run_single_file(source_item)
            .map_err(|e| format!("Failed to analyze document: {}", e))?;
        let mut index = languageserver::pipeline::storage::Index::new();
        index.merge_file(&path, analysis)
            .map_err(|e| format!("Failed to build index: {}", e))?;
        
        let pos = languageserver::types::Position {
            line: position.line,
            character: position.character,
        };
        if let Some(completion_list) = languageserver::features::CompletionProvider::complete(&index, &doc, pos) {
            let items: Vec<LspCompletionItem> = completion_list.items.into_iter().map(completion_item_to_lsp).collect();
            debug!("LspService: get_completions_with_content returning {} items", items.len());
            Ok(items)
        } else {
            debug!("LspService: get_completions_with_content: no completions");
            Ok(vec![])
        }
    }

    pub async fn resolve_completion(
        &self,
        completion_item: LspCompletionItem,
    ) -> Result<LspCompletionItem, String> {
        // For now, just return the completion item as-is
        // This could be enhanced to provide more details
        Ok(completion_item)
    }

    // Hover and documentation
    pub async fn get_hover(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Option<LspHover>, String> {
        let service_guard = self.get_service().await?;
        let service = service_guard.as_ref().unwrap();
        // Convert URI to proper file path
        let path_str = self.utils.uri_to_path(&uri);
        let path = PathBuf::from(&path_str);
        let languageserver_pos = lsp_position_to_position(position);
        
        if let Some(hover_content) = service.hover(&path, languageserver_pos.line, languageserver_pos.character).await {
            let hover = LspHover {
                contents: vec![LspMarkedString {
                    language: Some("julia".to_string()),
                    value: hover_content,
                }],
                range: None, // Could be enhanced to include range
            };
            Ok(Some(hover))
        } else {
            Ok(None)
        }
    }

    // Signature help - NOT IMPLEMENTED
    pub async fn get_signature_help(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Option<LspSignatureHelp>, String> {
        warn!("LspService: get_signature_help not implemented yet");
        Ok(None)
    }

    // Navigation and references
    pub async fn get_definition(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspLocation>, String> {
        debug!("LspService: Getting definition for {} at {:?}", uri, position);
        
        let service_guard = self.get_service().await?;
        let service = service_guard.as_ref().unwrap();
        // Convert URI to proper file path
        let path_str = self.utils.uri_to_path(&uri);
        let path = PathBuf::from(&path_str);
        let languageserver_pos = lsp_position_to_position(position);
        
        debug!("LspService: Converted URI {} to path {}", uri, path_str);
        
        if let Some(locations) = service.find_definition(&path, languageserver_pos.line, languageserver_pos.character) {
            let lsp_locations: Vec<LspLocation> = locations.into_iter()
                .map(location_to_lsp)
                .collect();
            debug!("LspService: Returning {} definition locations", lsp_locations.len());
            Ok(lsp_locations)
        } else {
            debug!("LspService: No definition found");
            Ok(vec![])
        }
    }

    pub async fn get_declaration(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspLocation>, String> {
        // For Julia, declaration is the same as definition
        self.get_definition(uri, position).await
    }

    pub async fn get_type_definition(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspLocation>, String> {
        // For now, treat type definition same as definition
        self.get_definition(uri, position).await
    }

    pub async fn get_implementation(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspLocation>, String> {
        // For now, treat implementation same as definition
        self.get_definition(uri, position).await
    }

    pub async fn get_references(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspLocation>, String> {
        debug!("LspService: Getting references for {} at {:?}", uri, position);
        
        // Convert URI to proper file path
        let path_str = self.utils.uri_to_path(&uri);
        let path = PathBuf::from(&path_str);
        let languageserver_pos = lsp_position_to_position(position);
        
        debug!("LspService: Converted URI {} to path {:?}", uri, path);
        
        let service_guard = self.get_service().await?;
        let service = service_guard.as_ref().unwrap();
        
        if let Some(locations) = service.find_references(&path, languageserver_pos.line, languageserver_pos.character, true) {
            let lsp_locations: Vec<LspLocation> = locations.into_iter()
                .map(location_to_lsp)
                .collect();
            debug!("LspService: Returning {} reference locations", lsp_locations.len());
            Ok(lsp_locations)
        } else {
            debug!("LspService: No references found");
            Ok(vec![])
        }
    }

    pub async fn get_document_highlights(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Vec<LspRange>, String> {
        warn!("LspService: get_document_highlights not implemented yet");
        Ok(vec![])
    }

    // Document symbols and workspace symbols - NOT IMPLEMENTED
    pub async fn get_document_symbols(&self, _uri: String) -> Result<Vec<LspDocumentSymbol>, String> {
        warn!("LspService: get_document_symbols not implemented yet");
        Ok(vec![])
    }

    pub async fn get_workspace_symbols(&self, _query: String) -> Result<Vec<LspDocumentSymbol>, String> {
        warn!("LspService: get_workspace_symbols not implemented yet");
        Ok(vec![])
    }

    // Code actions and refactoring - NOT IMPLEMENTED
    pub async fn get_code_actions(
        &self,
        _uri: String,
        _range: LspRange,
        _context: Vec<LspDiagnostic>,
    ) -> Result<Vec<LspCodeAction>, String> {
        warn!("LspService: get_code_actions not implemented yet");
        Ok(vec![])
    }

    pub async fn execute_command(
        &self,
        _command: String,
        _arguments: Option<Vec<serde_json::Value>>,
    ) -> Result<serde_json::Value, String> {
        warn!("LspService: execute_command not implemented yet");
        Err("Not implemented".to_string())
    }

    // Rename and refactoring - NOT IMPLEMENTED
    pub async fn prepare_rename(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Option<LspRange>, String> {
        warn!("LspService: prepare_rename not implemented yet");
        Ok(None)
    }

    pub async fn rename(
        &self,
        _uri: String,
        _position: LspPosition,
        _new_name: String,
    ) -> Result<Option<LspWorkspaceEdit>, String> {
        warn!("LspService: rename not implemented yet");
        Ok(None)
    }

    // Formatting - NOT IMPLEMENTED
    pub async fn format_document(
        &self,
        _uri: String,
        _options: serde_json::Value,
    ) -> Result<Vec<LspTextEdit>, String> {
        warn!("LspService: format_document not implemented yet");
        Ok(vec![])
    }

    pub async fn format_range(
        &self,
        _uri: String,
        _range: LspRange,
        _options: serde_json::Value,
    ) -> Result<Vec<LspTextEdit>, String> {
        warn!("LspService: format_range not implemented yet");
        Ok(vec![])
    }

    pub async fn format_on_type(
        &self,
        _uri: String,
        _position: LspPosition,
        _ch: String,
        _options: serde_json::Value,
    ) -> Result<Vec<LspTextEdit>, String> {
        warn!("LspService: format_on_type not implemented yet");
        Ok(vec![])
    }

    // Folding ranges - NOT IMPLEMENTED
    pub async fn get_folding_ranges(&self, _uri: String) -> Result<Vec<LspRange>, String> {
        warn!("LspService: get_folding_ranges not implemented yet");
        Ok(vec![])
    }

    // Selection ranges - NOT IMPLEMENTED
    pub async fn get_selection_ranges(
        &self,
        _uri: String,
        _positions: Vec<LspPosition>,
    ) -> Result<Vec<LspRange>, String> {
        warn!("LspService: get_selection_ranges not implemented yet");
        Ok(vec![])
    }

    // Call hierarchy - NOT IMPLEMENTED
    pub async fn prepare_call_hierarchy(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Vec<LspCallHierarchyItem>, String> {
        warn!("LspService: prepare_call_hierarchy not implemented yet");
        Ok(vec![])
    }

    pub async fn get_call_hierarchy_incoming_calls(
        &self,
        _item: LspCallHierarchyItem,
    ) -> Result<Vec<LspCallHierarchyItem>, String> {
        warn!("LspService: get_call_hierarchy_incoming_calls not implemented yet");
        Ok(vec![])
    }

    pub async fn get_call_hierarchy_outgoing_calls(
        &self,
        _item: LspCallHierarchyItem,
    ) -> Result<Vec<LspCallHierarchyItem>, String> {
        warn!("LspService: get_call_hierarchy_outgoing_calls not implemented yet");
        Ok(vec![])
    }

    // Semantic tokens - NOT IMPLEMENTED
    pub async fn get_semantic_tokens(
        &self,
        _uri: String,
        _range: Option<LspRange>,
    ) -> Result<Vec<LspSemanticToken>, String> {
        warn!("LspService: get_semantic_tokens not implemented yet");
        Ok(vec![])
    }

    pub async fn get_semantic_tokens_delta(
        &self,
        _uri: String,
        _previous_result_id: String,
    ) -> Result<Vec<LspSemanticToken>, String> {
        warn!("LspService: get_semantic_tokens_delta not implemented yet");
        Ok(vec![])
    }

    // Inlay hints - NOT IMPLEMENTED
    pub async fn get_inlay_hints(
        &self,
        _uri: String,
        _range: LspRange,
    ) -> Result<Vec<LspInlayHint>, String> {
        warn!("LspService: get_inlay_hints not implemented yet");
        Ok(vec![])
    }

    pub async fn resolve_inlay_hint(&self, hint: LspInlayHint) -> Result<LspInlayHint, String> {
        warn!("LspService: resolve_inlay_hint not implemented yet");
        Ok(hint)
    }

    // Diagnostics
    pub async fn get_diagnostics(&self, uri: String) -> Result<Vec<LspDiagnostic>, String> {
        debug!("LspService: Getting diagnostics for URI: {}", uri);
        
        let service_guard = self.get_service().await?;
        let service = service_guard.as_ref().unwrap();
        
        // Convert URI to path
        let path_str = self.utils.uri_to_path(&uri);
        let path = PathBuf::from(&path_str);
        
        debug!("LspService: Converted URI {} to path {:?}", uri, path);
        
        // Get diagnostics from languageserver
        let diagnostics = service.get_diagnostics(&path);
        
        debug!("LspService: Found {} diagnostics", diagnostics.len());
        
        // Convert to LspDiagnostic
        let lsp_diagnostics: Vec<LspDiagnostic> = diagnostics
            .into_iter()
            .map(diagnostic_to_lsp)
            .collect();
        
        Ok(lsp_diagnostics)
    }

    // Document management
    pub async fn update_document(&self, uri: String, content: String) -> Result<(), String> {
        let mut service_guard = self.get_service_mut().await?;
        let service = service_guard.as_mut().unwrap();
        
        // Convert URI to path
        let path_str = self.utils.uri_to_path(&uri);
        let path = PathBuf::from(&path_str);
        
        // Update document in languageserver
        service.update_document(path, content)
            .map_err(|e| format!("Failed to update document: {}", e))?;
        
        Ok(())
    }

    pub async fn invalidate_cache(&self, uri: String) -> Result<(), String> {
        let mut service_guard = self.get_service_mut().await?;
        let service = service_guard.as_mut().unwrap();
        
        // Convert URI to path
        let path_str = self.utils.uri_to_path(&uri);
        let path = PathBuf::from(&path_str);
        
        // Invalidate cache in languageserver
        service.invalidate_cache(&path);
        
        Ok(())
    }

    // Workspace operations - NOT IMPLEMENTED
    pub async fn get_workspace_folders(&self) -> Result<Vec<String>, String> {
        warn!("LspService: get_workspace_folders not implemented yet");
        Ok(vec![])
    }

    pub async fn get_workspace_configuration(
        &self,
        _section: String,
    ) -> Result<serde_json::Value, String> {
        warn!("LspService: get_workspace_configuration not implemented yet");
        Ok(serde_json::Value::Null)
    }

    // Document links - NOT IMPLEMENTED
    pub async fn get_document_links(&self, _uri: String) -> Result<Vec<LspLocation>, String> {
        warn!("LspService: get_document_links not implemented yet");
        Ok(vec![])
    }

    pub async fn resolve_document_link(&self, link: LspLocation) -> Result<LspLocation, String> {
        warn!("LspService: resolve_document_link not implemented yet");
        Ok(link)
    }

    // Color provider - NOT IMPLEMENTED
    pub async fn get_document_colors(&self, _uri: String) -> Result<Vec<serde_json::Value>, String> {
        warn!("LspService: get_document_colors not implemented yet");
        Ok(vec![])
    }

    pub async fn get_color_presentations(
        &self,
        _uri: String,
        _color: serde_json::Value,
        _range: LspRange,
    ) -> Result<Vec<serde_json::Value>, String> {
        warn!("LspService: get_color_presentations not implemented yet");
        Ok(vec![])
    }

    // Type hierarchy - NOT IMPLEMENTED
    pub async fn prepare_type_hierarchy(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Vec<LspCallHierarchyItem>, String> {
        warn!("LspService: prepare_type_hierarchy not implemented yet");
        Ok(vec![])
    }

    pub async fn get_type_hierarchy_supertypes(
        &self,
        _item: LspCallHierarchyItem,
    ) -> Result<Vec<LspCallHierarchyItem>, String> {
        warn!("LspService: get_type_hierarchy_supertypes not implemented yet");
        Ok(vec![])
    }

    pub async fn get_type_hierarchy_subtypes(
        &self,
        _item: LspCallHierarchyItem,
    ) -> Result<Vec<LspCallHierarchyItem>, String> {
        warn!("LspService: get_type_hierarchy_subtypes not implemented yet");
        Ok(vec![])
    }

    // Inline values - NOT IMPLEMENTED
    pub async fn get_inline_values(
        &self,
        _uri: String,
        _range: LspRange,
        _context: serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, String> {
        warn!("LspService: get_inline_values not implemented yet");
        Ok(vec![])
    }

    // Progress reporting - NOT IMPLEMENTED
    pub async fn create_progress(
        &self,
        _token: String,
        _title: String,
        _message: Option<String>,
    ) -> Result<(), String> {
        warn!("LspService: create_progress not implemented yet");
        Ok(())
    }

    pub async fn report_progress(
        &self,
        _token: String,
        _message: String,
        _percentage: Option<u32>,
    ) -> Result<(), String> {
        warn!("LspService: report_progress not implemented yet");
        Ok(())
    }

    pub async fn end_progress(&self, _token: String, _message: Option<String>) -> Result<(), String> {
        warn!("LspService: end_progress not implemented yet");
        Ok(())
    }

    // Custom Julia-specific features
    pub async fn get_julia_symbols(&self, _uri: String, _query: String) -> Result<Vec<String>, String> {
        warn!("LspService: get_julia_symbols not implemented yet");
        Ok(vec![])
    }

    pub async fn get_julia_diagnostics(&self, _uri: String) -> Result<Vec<LspDiagnostic>, String> {
        warn!("LspService: get_julia_diagnostics not implemented yet");
        Ok(vec![])
    }

    pub async fn get_julia_workspace_symbols(
        &self,
        _query: String,
    ) -> Result<Vec<LspDocumentSymbol>, String> {
        warn!("LspService: get_julia_workspace_symbols not implemented yet");
        Ok(vec![])
    }

    pub async fn get_julia_references(
        &self,
        uri: String,
        position: LspPosition,
        include_declaration: bool,
    ) -> Result<Vec<LspLocation>, String> {
        debug!("LspService: Getting Julia references for {} at {:?}", uri, position);
        
        // Convert URI to proper file path
        let path_str = self.utils.uri_to_path(&uri);
        let path = PathBuf::from(&path_str);
        let languageserver_pos = lsp_position_to_position(position);
        
        debug!("LspService: Converted URI {} to path {:?}", uri, path);
        
        let service_guard = self.get_service().await?;
        let service = service_guard.as_ref().unwrap();
        
        if let Some(locations) = service.find_references(&path, languageserver_pos.line, languageserver_pos.character, include_declaration) {
            let lsp_locations: Vec<LspLocation> = locations.into_iter()
                .map(location_to_lsp)
                .collect();
            debug!("LspService: Returning {} Julia reference locations", lsp_locations.len());
            Ok(lsp_locations)
        } else {
            debug!("LspService: No Julia references found");
            Ok(vec![])
        }
    }

    // Additional methods
    pub async fn check_lsp_health(&self) -> Result<bool, String> {
        let is_running = self.is_running.read().map_err(|e| e.to_string())?;
        Ok(*is_running)
    }

    pub async fn send_request(&self, _request: serde_json::Value) -> Result<serde_json::Value, String> {
        warn!("LspService: send_request not implemented yet");
        Err("Not implemented".to_string())
    }

    pub async fn get_capabilities(&self) -> Result<serde_json::Value, String> {
        // Return basic capabilities for implemented features
        let capabilities = serde_json::json!({
            "textDocumentSync": {
                "openClose": true,
                "change": 1, // Full document sync
                "willSave": false,
                "willSaveWaitUntil": false,
                "save": false
            },
            "hoverProvider": true,
            "completionProvider": {
                "resolveProvider": false,
                "triggerCharacters": [".", ":"]
            },
            "definitionProvider": true,
            "referencesProvider": true,
            "declarationProvider": true,
            "implementationProvider": true,
            "typeDefinitionProvider": true,
            "signatureHelpProvider": false,
            "documentSymbolProvider": false,
            "workspaceSymbolProvider": false,
            "codeActionProvider": false,
            "codeLensProvider": false,
            "documentFormattingProvider": false,
            "documentRangeFormattingProvider": false,
            "documentOnTypeFormattingProvider": false,
            "renameProvider": false,
            "documentLinkProvider": false,
            "colorProvider": false,
            "foldingRangeProvider": false,
            "selectionRangeProvider": false,
            "callHierarchyProvider": false,
            "semanticTokensProvider": false,
            "inlayHintProvider": false,
            "diagnosticProvider": false
        });
        
        Ok(capabilities)
    }
}


