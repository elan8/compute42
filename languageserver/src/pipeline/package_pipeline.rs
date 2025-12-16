use crate::pipeline::sources::package::PackageSource;
use crate::pipeline::sources::{ProjectContext, indexing::should_skip_entry};
use crate::pipeline::pipeline_trait::Pipeline;
use crate::pipeline::{
    types::{ParsedItem, AnalysisResult},
    parser,
    analyzers,
    storage::{self, persistence},
    sources::file::FileSource,
};
use crate::types::LspError;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Input for PackagePipeline::run()
/// 
/// Contains the depot path and project context needed to discover and process packages.
#[derive(Debug, Clone)]
pub struct PackagePipelineInput {
    pub depot_path: PathBuf,
    pub project_context: ProjectContext,
}

/// Package pipeline for extracting docstrings from external packages
/// 
/// This pipeline extracts docstrings from package source files to create a BaseDocsRegistry
/// for hover documentation. It does NOT create a full Index (no symbols, references, etc.).
pub struct PackagePipeline;

impl PackagePipeline {
    /// Create a new package pipeline
    pub fn new() -> Self {
        Self
    }

    /// Process a single package and extract metadata (signatures, types, exports)
    /// 
    /// Returns a lightweight Index containing signatures, types, and exports from the package.
    pub fn process_package(
        &self,
        package_path: &Path,
        package_name: &str,
    ) -> Result<storage::Index, LspError> {
        // Discover all Julia files in the package
        let mut source_items = Vec::new();
        for entry in WalkDir::new(package_path)
            .into_iter()
            .filter_entry(|e| !should_skip_entry(e.path()))
        {
            let entry = entry.map_err(|e| LspError::InternalError(format!("Failed to walk directory: {}", e)))?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("jl") {
                let file_source = FileSource::new(path.to_path_buf());
                match file_source.load() {
                    Ok(source_item) => source_items.push(source_item),
                    Err(e) => {
                        log::warn!("PackagePipeline: Failed to load file {:?}: {}", path, e);
                        continue;
                    }
                }
            }
        }
        
        log::info!("PackagePipeline: Processing {} files from package '{}'", source_items.len(), package_name);
        
        // Run lightweight analysis pipeline (signatures, types, exports only)
        let mut index = storage::Index::new();
        
        // First pass: Collect exports with module context
        for source_item in &source_items {
            let parsed = parser::parse(source_item)?;
            // Use module-aware export analyzer
            let exports_by_module = analyzers::export::analyze(&parsed)?;
            
            // Add exports for each module found in the file
            for (module_name, exports) in exports_by_module {
                if !exports.is_empty() {
                    index.add_exports(module_name.clone(), exports, source_item.path.clone());
                }
            }
        }
        
        // Second pass: Extract signatures and types (filtered by exports)
        for source_item in &source_items {
            let parsed = parser::parse(source_item)?;
            let analysis = self.analyze(&parsed)?;
            
            // Merge into index (will filter based on exports we collected in first pass)
            index.merge_file(&source_item.path, analysis)?;
        }
        
        // Promote submodule functions to parent module (e.g., Flux.Losses.crossentropy -> Flux.crossentropy)
        index.promote_submodule_functions();
        
        log::info!("PackagePipeline: Extracted metadata from package '{}' with {} function signatures", 
            package_name, index.get_all_modules().iter()
                .map(|m| index.get_module_functions(m).len())
                .sum::<usize>());
        
        Ok(index)
    }

    /// Discover and process all packages from project dependencies
    /// 
    /// This discovers direct dependencies and extracts metadata from each package.
    /// Returns a unified Index containing all package metadata.
    /// Automatically checks and uses cache for each package if available.
    pub fn discover_and_process(
        &self,
        depot_path: PathBuf,
        project_context: ProjectContext,
    ) -> Result<storage::Index, LspError> {
        // Discover packages (this handles the discovery logic)
        let Some(dependencies) = project_context.dependencies() else {
            log::info!("PackagePipeline: No dependencies found in project");
            return Ok(storage::Index::new());
        };
        
        log::info!("PackagePipeline: Found {} direct dependencies to index", dependencies.len());
        let manifest = project_context.manifest_toml.as_ref();
        
        let mut unified_index = storage::Index::new();
        
        // Process each package with cache checking
        for package_name in dependencies.keys() {
            // Try to get UUID and git-tree-sha1 from manifest to compute slug
            let slug = if let Some(manifest) = manifest {
                if let Some(entries) = manifest.packages.get(package_name) {
                    entries.iter().find_map(|entry| {
                        if let (Some(uuid), Some(git_tree_sha1)) = (&entry.uuid, &entry.git_tree_sha1) {
                            match crate::pipeline::sources::indexing::compute_package_slug(uuid, git_tree_sha1) {
                                Ok(s) => Some(s),
                                Err(e) => {
                                    log::warn!("PackagePipeline: Failed to compute slug for {}: {}", package_name, e);
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            } else {
                None
            };
            
            // Try to resolve package path
            let package_path = crate::pipeline::sources::indexing::resolve_package_path(
                &depot_path,
                package_name,
                manifest,
            );
            
            let Some(package_path) = package_path else {
                log::warn!("PackagePipeline: Could not resolve path for package '{}' from depot {:?}", 
                    package_name, depot_path);
                log::trace!("PackagePipeline: Package '{}' is in Project.toml but not found in depot. Check if:", package_name);
                log::trace!("  - Package is installed: {}/packages/{}/", depot_path.display(), package_name);
                log::trace!("  - Manifest.toml has entry for '{}' with UUID", package_name);
                continue;
            };
            
            log::trace!("PackagePipeline: Resolved package '{}' to path: {:?}", package_name, package_path);
            
            // Check cache if we have a slug
            let mut loaded_from_cache = false;
            if let Some(ref slug) = slug {
                let cache_path = PackageSource::get_package_cache_path(package_name, slug);
                
                // Check if cache is valid
                if cache_path.exists() {
                    if let Some(cached_index) = self.load_package_cache(&cache_path, &package_path) {
                        unified_index.merge(cached_index);
                        loaded_from_cache = true;
                        log::trace!("PackagePipeline: Loaded package '{}' from cache", package_name);
                    }
                }
            }
            
            // If not loaded from cache, process the package
            if !loaded_from_cache {
                match self.process_package(&package_path, package_name) {
                    Ok(package_index) => {
                        // Save to cache if slug is available
                        if let Some(ref slug) = slug {
                            let cache_path = PackageSource::get_package_cache_path(package_name, slug);
                            if let Err(e) = self.save_package_cache(&package_index, &cache_path) {
                                log::warn!("PackagePipeline: Failed to save cache for package {}: {}", package_name, e);
                            }
                        }
                        
                        // Merge package index into unified index
                        unified_index.merge(package_index);
                    }
                    Err(e) => {
                        log::warn!("PackagePipeline: Failed to process package {}: {}", package_name, e);
                    }
                }
            }
        }
        
        Ok(unified_index)
    }
    
    /// Load package index from cache if valid
    fn load_package_cache(&self, cache_path: &Path, package_path: &Path) -> Option<storage::Index> {
        if !cache_path.exists() {
            return None;
        }
        
        // Check if cache is valid (package directory hasn't changed)
        if !self.is_cache_valid(cache_path, package_path) {
            return None;
        }
        
        // Load Index from cache
        match persistence::deserialize_from_json(cache_path) {
            Ok(index) => {
                log::info!("PackagePipeline: Loaded package index from cache: {:?}", cache_path);
                Some(index)
            }
            Err(e) => {
                log::warn!("PackagePipeline: Failed to load cache from {:?}: {}", cache_path, e);
                None
            }
        }
    }
    
    /// Check if cache is valid by comparing cache timestamp with package source files
    fn is_cache_valid(&self, cache_path: &Path, package_path: &Path) -> bool {
        // Get cache modification time
        let cache_modified = match std::fs::metadata(cache_path) {
            Ok(metadata) => {
                match metadata.modified() {
                    Ok(modified) => modified,
                    Err(_) => return false,
                }
            }
            Err(_) => return false,
        };
        
        // Find most recent source file modification time
        let mut most_recent_source: Option<std::time::SystemTime> = None;
        if let Ok(walker) = WalkDir::new(package_path).into_iter().collect::<Result<Vec<_>, _>>() {
            for entry in walker {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("jl") {
                    if let Ok(metadata) = std::fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            most_recent_source = Some(most_recent_source
                                .map(|prev| prev.max(modified))
                                .unwrap_or(modified));
                        }
                    }
                }
            }
        }
        
        // If we found source files, check if cache is newer than the most recent source file
        if let Some(most_recent) = most_recent_source {
            cache_modified >= most_recent
        } else {
            // No source files found, consider cache valid if it exists
            true
        }
    }
    
    /// Save package index to cache
    fn save_package_cache(&self, index: &storage::Index, cache_path: &Path) -> Result<(), LspError> {
        // Create cache directory if it doesn't exist
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| LspError::InternalError(format!("Failed to create cache directory: {}", e)))?;
        }

        // Save using persistence module
        persistence::serialize_to_json(index, cache_path)?;
        
        log::info!("PackagePipeline: Saved package index to cache: {:?}", cache_path);
        Ok(())
    }
    
    /// Infer module name from file path (helper)
    #[allow(dead_code)]
    fn infer_module_name_from_path(path: &std::path::Path, package_name: &str) -> String {
        // For packages, try to extract module name from path
        let path_str = path.to_string_lossy();
        
        // Check if path contains package name pattern (e.g., packages/DataFrames/.../src/DataFrames.jl)
        if let Some(packages_pos) = path_str.find("packages/") {
            let after_packages = &path_str[packages_pos + 9..];
            if let Some(slash_pos) = after_packages.find('/') {
                let extracted_name = &after_packages[..slash_pos];
                // Check if it matches the package name
                if extracted_name == package_name {
                    // Check if file is in a subdirectory (e.g., packages/Flux/.../src/losses/functions.jl -> Flux.Losses)
                    if let Some(src_pos) = path_str.find("/src/") {
                        let after_src = &path_str[src_pos + 5..]; // Skip "/src/"
                        // Get the directory containing the file (if any)
                        if let Some(file_name_pos) = after_src.rfind('/') {
                            let dir_path = &after_src[..file_name_pos];
                            if !dir_path.is_empty() {
                                // Extract directory name and capitalize first letter
                                let dir_name = dir_path.split('/').last().unwrap_or(dir_path);
                                let dir_name = dir_path.split('\\').last().unwrap_or(dir_name);
                                if !dir_name.is_empty() && dir_name != package_name {
                                    let mut chars = dir_name.chars();
                                    if let Some(first) = chars.next() {
                                        let capitalized = format!("{}{}", first.to_uppercase(), chars.as_str());
                                        return format!("{}.{}", package_name, capitalized);
                                    }
                                }
                            }
                        }
                    }
                    return package_name.to_string();
                }
            }
        }
        
        // Check if filename matches package name (e.g., DataFrames.jl)
        if let Some(file_stem) = path.file_stem() {
            if let Some(stem_str) = file_stem.to_str() {
                if stem_str == package_name {
                    return package_name.to_string();
                }
            }
        }
        
        // Fallback to package name
        package_name.to_string()
    }
    
    /// Analyze a parsed item with lightweight analysis (signatures, types, exports only)
    fn analyze(&self, parsed: &ParsedItem) -> Result<AnalysisResult, LspError> {
        let mut result = AnalysisResult::new();
        
        // Extract only what we need for packages: signatures, types, exports
        // NOT symbols/references/scopes (workspace-only)
        result.types = analyzers::type_analyzer::analyze(parsed)?;
        result.signatures = analyzers::signature::analyze(parsed)?;
        // For AnalysisResult, we still need HashSet<String> for backward compatibility
        // But we collect module-aware exports separately in the first pass
        let exports_by_module = analyzers::export::analyze(parsed)?;
        // Flatten into HashSet for AnalysisResult (backward compatibility)
        for exports in exports_by_module.values() {
            result.exports.extend(exports.iter().cloned());
        }
        
        Ok(result)
    }
}

impl Pipeline for PackagePipeline {
    type Input = PackagePipelineInput;
    type Output = storage::Index;
    
    fn run(&self, input: Self::Input) -> Result<Self::Output, LspError> {
        self.discover_and_process(input.depot_path, input.project_context)
    }
    
    fn name(&self) -> &'static str {
        "PackagePipeline"
    }
    
    fn description(&self) -> &'static str {
        "Extracts signatures, types, and exports from external packages. Discovers direct dependencies and extracts metadata from each package to create a unified Index for type inference and semantic analysis."
    }
}

impl Default for PackagePipeline {
    fn default() -> Self {
        Self::new()
    }
}

