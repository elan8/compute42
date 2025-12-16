use crate::pipeline::{
    types::{self, SourceItem, ParsedItem, AnalysisResult},
    config::PipelineConfig,
    parser,
    analyzers,
    storage,
};
use crate::types::LspError;

/// Main pipeline orchestrator that coordinates the pipeline stages
pub struct Pipeline {
    config: PipelineConfig,
}

impl Pipeline {
    /// Create a new pipeline with the given configuration
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    /// Run the pipeline: sources → parser → analyzers → storage
    pub fn run(&self, source_items: Vec<SourceItem>) -> Result<storage::Index, LspError> {
        self.run_with_index(source_items, None)
    }

    /// Run the pipeline with an existing index to merge into
    pub fn run_with_index(&self, source_items: Vec<SourceItem>, mut existing_index: Option<storage::Index>) -> Result<storage::Index, LspError> {
        log::trace!("Pipeline: Running with {} files", source_items.len());
        let mut index = existing_index.take().unwrap_or_default();

        // PASS 0: Collect all exports first (needed to filter which symbols to index from dependencies)
        // Sort files so main module files (with exports) are processed first
        let mut sorted_items = source_items.clone();
        sorted_items.sort_by(|a, b| {
            let a_is_main = Self::is_main_module_file(&a.path);
            let b_is_main = Self::is_main_module_file(&b.path);
            match (a_is_main, b_is_main) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });
        
        // First pass: Collect exports only
        for source_item in &sorted_items {
            let parsed = parser::parse(source_item)?;
            let analysis = self.analyze_pass1(&parsed)?;
            
            // Only merge exports in first pass
            if !analysis.exports.is_empty() {
                let module_name = Self::infer_module_name_from_path(&source_item.path);
                index.add_exports(module_name.clone(), analysis.exports.clone(), source_item.path.clone());
            }
        }

        // Extract metadata (symbols, signatures, types, scopes)
        // Now we have all exports, so we can filter which symbols to index
        for source_item in &sorted_items {
            // Parse
            let parsed = parser::parse(source_item)?;

            // Analyze (without type inference)
            let analysis = self.analyze_pass1(&parsed)?;

            // Store (will filter based on exports we collected in PASS 0)
            index.merge_file(&source_item.path, analysis)?;
        }

        log::trace!("Pipeline: Completed - {} symbols indexed", index.get_all_symbols().len());
        Ok(index)
    }
    
    /// Check if a file is a main module file (e.g., DataFrames.jl)
    fn is_main_module_file(path: &std::path::Path) -> bool {
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            // Check if filename matches module name pattern
            // Main module files are typically named after the package (e.g., DataFrames.jl)
            if let Some(stem) = file_name.strip_suffix(".jl") {
                // Check if it's likely a main module file (capitalized, matches package name)
                let first_char = stem.chars().next();
                first_char.map(|c| c.is_uppercase()).unwrap_or(false)
            } else {
                false
            }
        } else {
            false
        }
    }
    
    /// Infer module name from file path (helper)
    fn infer_module_name_from_path(path: &std::path::Path) -> String {
        let path_str = path.to_string_lossy();
        
        // Check if path contains "packages/{PackageName}/"
        if let Some(packages_pos) = path_str.find("packages/") {
            let after_packages = &path_str[packages_pos + 9..];
            if let Some(slash_pos) = after_packages.find('/') {
                let package_name = &after_packages[..slash_pos];
                return package_name.to_string();
            }
        }
        
        // Fallback: use filename without extension
        if let Some(file_stem) = path.file_stem() {
            if let Some(stem_str) = file_stem.to_str() {
                let mut chars = stem_str.chars();
                if let Some(first) = chars.next() {
                    return format!("{}{}", first.to_uppercase(), chars.as_str());
                }
            }
        }
        
        "Main".to_string()
    }

    /// Run the pipeline for a single file and return the analysis result
    pub fn run_single_file(&self, source_item: SourceItem) -> Result<types::AnalysisResult, LspError> {
        // Parse
        let parsed = parser::parse(&source_item)?;

        // Analyze
        let analysis = self.analyze(&parsed)?;

        Ok(analysis)
    }

    /// Analyze a parsed item according to the pipeline configuration (Pass 1 - no type inference)
    fn analyze_pass1(&self, parsed: &ParsedItem) -> Result<AnalysisResult, LspError> {
        let mut result = AnalysisResult::new();

        if self.config.extract_symbols {
            result.symbols = analyzers::symbol::analyze(parsed)?;
        }

        if self.config.extract_references {
            result.references = analyzers::reference::analyze(parsed)?;
        }

        if self.config.extract_types {
            result.types = analyzers::type_analyzer::analyze(parsed)?;
        }

        if self.config.extract_scopes {
            result.scopes = analyzers::scope::analyze(parsed)?;
        }

        if self.config.extract_signatures {
            result.signatures = analyzers::signature::analyze(parsed)?;
        }

        if self.config.extract_exports {
            result.exports = analyzers::export::analyze_legacy(parsed)?;
        }

        Ok(result)
    }

    /// Analyze a parsed item (legacy method - calls analyze_pass1)
    /// Kept for backward compatibility with run_single_file
    fn analyze(&self, parsed: &ParsedItem) -> Result<AnalysisResult, LspError> {
        self.analyze_pass1(parsed)
    }
}

// Note: Tests for the old Pipeline struct have been moved to workspace_pipeline.rs
// The Pipeline struct is kept here for backward compatibility but new code should use WorkspacePipeline

