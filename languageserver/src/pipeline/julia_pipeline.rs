use crate::pipeline::sources::base::BaseSource;
use crate::pipeline::pipeline_trait::Pipeline;
use crate::pipeline::{
    types::{ParsedItem, AnalysisResult},
    parser,
    analyzers,
    storage::{self, persistence},
};
use crate::types::LspError;
use std::path::{Path, PathBuf};
use dirs;

/// Julia pipeline for extracting docstrings from Base and stdlib
/// 
/// This pipeline extracts docstrings from Julia Base and stdlib source files to create
/// a BaseDocsRegistry for hover documentation. It does NOT create a full Index.
pub struct JuliaPipeline;

impl JuliaPipeline {
    /// Create a new Julia pipeline
    pub fn new() -> Self {
        Self
    }

    /// Extract Base/stdlib metadata (signatures, types, exports)
    /// 
    /// This function:
    /// 1. Discovers Base and stdlib files
    /// 2. Extracts signatures, types, and exports to create a lightweight Index
    /// 
    /// Note: This extracts only signatures, types, and exports (not symbols/references/scopes).
    /// The Index can be merged into the main workspace Index for type inference and semantic analysis.
    pub fn index_base(&self, julia_executable: &Path) -> Result<storage::Index, LspError> {
        let base_source = BaseSource::new(julia_executable)?;
        
        let mut base_items = Vec::new();
        
        // Discover Base files
        match base_source.discover_base() {
            Ok(mut items) => {
                log::info!("JuliaPipeline: Discovered {} Base files", items.len());
                base_items.append(&mut items);
            }
            Err(e) => {
                log::warn!("JuliaPipeline: Failed to discover Base files: {}. Continuing without Base types.", e);
            }
        }
        
        // Discover stdlib files
        match base_source.discover_stdlib() {
            Ok(mut items) => {
                log::info!("JuliaPipeline: Discovered {} stdlib files", items.len());
                base_items.append(&mut items);
            }
            Err(e) => {
                log::warn!("JuliaPipeline: Failed to discover stdlib files: {}. Continuing without stdlib types.", e);
            }
        }
        
        if base_items.is_empty() {
            return Err(LspError::InternalError("No base items found to index".to_string()));
        }
        
        // Run lightweight analysis pipeline (signatures, types, exports only)
        let mut index = storage::Index::new();
        
        // First pass: Collect exports
        for source_item in &base_items {
            let parsed = parser::parse(source_item)?;
            let analysis = self.analyze(&parsed)?;
            
            // Only merge exports in first pass
            if !analysis.exports.is_empty() {
                let module_name = Self::infer_module_name_from_path(&source_item.path);
                index.add_exports(module_name.clone(), analysis.exports.clone(), source_item.path.clone());
            }
        }
        
        // Second pass: Extract signatures and types (filtered by exports)
        for source_item in &base_items {
            let parsed = parser::parse(source_item)?;
            let analysis = self.analyze(&parsed)?;
            
            // Merge into index (will filter based on exports we collected in first pass)
            index.merge_file(&source_item.path, analysis)?;
        }
        
        // Count total signatures by iterating through modules
        let signature_count: usize = index.get_all_modules().iter()
            .map(|module| index.get_module_functions(module).len())
            .sum();
        log::info!("JuliaPipeline: Indexed {} Base/stdlib files with {} function signatures", 
            base_items.len(), signature_count);
        
        Ok(index)
    }
    
    /// Infer module name from file path (helper)
    fn infer_module_name_from_path(path: &std::path::Path) -> String {
        let path_str = path.to_string_lossy();
        
        // Check if path contains "base/" or "stdlib/"
        if path_str.contains("/base/") || path_str.contains("\\base\\") {
            // Extract module name from path (e.g., "base/array.jl" -> "Base.Array")
            if let Some(file_stem) = path.file_stem() {
                if let Some(stem_str) = file_stem.to_str() {
                    let mut chars = stem_str.chars();
                    if let Some(first) = chars.next() {
                        return format!("Base.{}{}", first.to_uppercase(), chars.as_str());
                    }
                }
            }
            return "Base".to_string();
        } else if path_str.contains("/stdlib/") || path_str.contains("\\stdlib\\") {
            // Extract stdlib module name from path
            if let Some(file_stem) = path.file_stem() {
                if let Some(stem_str) = file_stem.to_str() {
                    let mut chars = stem_str.chars();
                    if let Some(first) = chars.next() {
                        return format!("Base.{}{}", first.to_uppercase(), chars.as_str());
                    }
                }
            }
            return "Base".to_string();
        }
        
        "Base".to_string()
    }
    
    /// Analyze a parsed item with lightweight analysis (signatures, types, exports only)
    fn analyze(&self, parsed: &ParsedItem) -> Result<AnalysisResult, LspError> {
        let mut result = AnalysisResult::new();
        
        // Extract only what we need for Base/stdlib: signatures, types, exports
        // NOT symbols/references/scopes (workspace-only)
        result.types = analyzers::type_analyzer::analyze(parsed)?;
        result.signatures = analyzers::signature::analyze(parsed)?;
        result.exports = analyzers::export::analyze_legacy(parsed)?;
        
        Ok(result)
    }

    /// Save base index to file
    /// 
    /// Saves:
    /// - base_index.json: Index format (for type inference and semantic analysis)
    pub fn save_base_index(
        &self,
        index: &storage::Index,
        output_path: Option<PathBuf>,
    ) -> Result<PathBuf, LspError> {
        let data_dir = dirs::data_local_dir()
            .map(|dir| dir.join("com.compute42.dev"))
            .unwrap_or_else(|| {
                log::warn!("Failed to get user data directory, falling back to current directory");
                PathBuf::from(".")
            });
        
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| LspError::InternalError(format!("Failed to create data directory: {}", e)))?;
        
        // Save Index as base_index.json using persistence module
        let base_index_path = output_path.unwrap_or_else(|| data_dir.join("base_index.json"));
        persistence::serialize_to_json(index, &base_index_path)
            .map_err(|e| LspError::InternalError(format!("Failed to save base_index.json: {}", e)))?;
        
        // Count total signatures for logging
        let signature_count: usize = index.get_all_modules().iter()
            .map(|module| index.get_module_functions(module).len())
            .sum();
        log::info!("JuliaPipeline: Saved base_index.json with {} function signatures to {:?}", 
            signature_count, base_index_path);
        
        Ok(base_index_path)
    }

    /// Check if base_index.json exists and is recent (to skip re-indexing)
    /// 
    /// Returns true if base_index.json exists and is within the last 7 days
    /// 
    /// **Deprecated**: Cache checking is now handled automatically in `run()`.
    /// This method is kept for backward compatibility.
    #[deprecated(note = "Cache checking is now handled automatically in run(). Use run() instead.")]
    pub fn should_skip_base_indexing(&self) -> bool {
        let data_dir = dirs::data_local_dir()
            .map(|dir| dir.join("com.compute42.dev"))
            .unwrap_or_else(|| {
                log::warn!("Failed to get user data directory, falling back to current directory");
                PathBuf::from(".")
            });
        
        let docs_path = data_dir.join("base_index.json");
        
        if !docs_path.exists() {
            return false;
        }
        
        match std::fs::metadata(&docs_path) {
            Ok(metadata) => {
                match metadata.modified() {
                    Ok(modified) => {
                        match modified.elapsed() {
                            Ok(elapsed) => {
                                // If base_index.json is recent (within 7 days), skip re-indexing
                                let is_recent = elapsed.as_secs() <= 7 * 24 * 60 * 60;
                                if is_recent {
                                    log::info!("JuliaPipeline: base_index.json exists and is recent ({} days old), skipping re-indexing", 
                                        elapsed.as_secs() / (24 * 60 * 60));
                                    true
                                } else {
                                    log::info!("JuliaPipeline: base_index.json is outdated ({} days old), will rebuild", 
                                        elapsed.as_secs() / (24 * 60 * 60));
                                    false
                                }
                            }
                            Err(_) => false
                        }
                    }
                    Err(_) => false
                }
            }
            Err(_) => false
        }
    }
}

impl Pipeline for JuliaPipeline {
    type Input = PathBuf;
    type Output = storage::Index;
    
    fn run(&self, input: Self::Input) -> Result<Self::Output, LspError> {
        // Check cache first
        let data_dir = dirs::data_local_dir()
            .map(|dir| dir.join("com.compute42.dev"))
            .unwrap_or_else(|| {
                log::warn!("Failed to get user data directory, falling back to current directory");
                PathBuf::from(".")
            });
        
        let base_index_path = data_dir.join("base_index.json");
        
        // Check if cache exists and is recent (within 7 days)
        if base_index_path.exists() {
            match std::fs::metadata(&base_index_path) {
                Ok(metadata) => {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(elapsed) = modified.elapsed() {
                            // If base_index.json is recent (within 7 days), load from cache
                            let is_recent = elapsed.as_secs() <= 7 * 24 * 60 * 60;
                            if is_recent {
                                log::info!("JuliaPipeline: Loading base_index.json from cache ({} days old)", 
                                    elapsed.as_secs() / (24 * 60 * 60));
                                match persistence::deserialize_from_json(&base_index_path) {
                                    Ok(index) => {
                                        log::info!("JuliaPipeline: Loaded Base/stdlib index from cache");
                                        return Ok(index);
                                    }
                                    Err(e) => {
                                        log::warn!("JuliaPipeline: Failed to load base_index.json: {}. Will rebuild.", e);
                                        // Fall through to rebuild
                                    }
                                }
                            } else {
                                log::info!("JuliaPipeline: base_index.json is outdated ({} days old), will rebuild", 
                                    elapsed.as_secs() / (24 * 60 * 60));
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("JuliaPipeline: Failed to check base_index.json metadata: {}. Will rebuild.", e);
                }
            }
        }
        
        // Cache doesn't exist or is invalid - rebuild
        log::info!("JuliaPipeline: Rebuilding Base/stdlib index...");
        let index = self.index_base(&input)?;
        
        // Save to cache
        if let Err(e) = self.save_base_index(&index, Some(base_index_path.clone())) {
            log::warn!("JuliaPipeline: Failed to save cache: {}", e);
        }
        
        Ok(index)
    }
    
    fn name(&self) -> &'static str {
        "JuliaPipeline"
    }
    
    fn description(&self) -> &'static str {
        "Extracts signatures, types, and exports from Julia Base and stdlib. Creates a lightweight Index that can be merged into the main workspace Index for type inference and semantic analysis. Automatically checks and uses cache if available."
    }
}

impl Default for JuliaPipeline {
    fn default() -> Self {
        Self::new()
    }
}

