use crate::pipeline::{
    types::{self, SourceItem, ParsedItem, AnalysisResult},
    parser,
    analyzers,
    storage,
    pipeline_trait::Pipeline,
};
use crate::types::LspError;

/// Workspace pipeline for full analysis of workspace files
/// 
/// This pipeline extracts all metadata (symbols, references, types, scopes, signatures, exports)
/// and creates an Index for LSP features like hover, go-to-definition, and completion.
pub struct WorkspacePipeline;

impl WorkspacePipeline {
    /// Create a new workspace pipeline
    pub fn new() -> Self {
        Self
    }

    /// Run the pipeline: sources → parser → analyzers → storage
    pub fn run(&self, source_items: Vec<SourceItem>) -> Result<storage::Index, LspError> {
        self.run_impl(source_items)
    }
    
    /// Internal implementation of run (used by both inherent method and trait)
    fn run_impl(&self, source_items: Vec<SourceItem>) -> Result<storage::Index, LspError> {
        self.run_with_index(source_items, None)
    }

    /// Run the pipeline with an existing index to merge into
    pub fn run_with_index(&self, source_items: Vec<SourceItem>, mut existing_index: Option<storage::Index>) -> Result<storage::Index, LspError> {
        log::trace!("WorkspacePipeline: Running with {} files", source_items.len());
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
            let analysis = self.analyze(&parsed)?;
            
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

            // Analyze (full analysis for workspace files)
            let analysis = self.analyze(&parsed)?;

            // Store (will filter based on exports we collected in PASS 0)
            index.merge_file(&source_item.path, analysis)?;
        }

        log::trace!("WorkspacePipeline: Completed - {} symbols indexed", index.get_all_symbols().len());
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

        // Analyze (full analysis)
        let analysis = self.analyze(&parsed)?;

        Ok(analysis)
    }

    /// Analyze a parsed item with full analysis (all analyzers enabled)
    fn analyze(&self, parsed: &ParsedItem) -> Result<AnalysisResult, LspError> {
        let mut result = AnalysisResult::new();

        // Extract all metadata for workspace files
        result.symbols = analyzers::symbol::analyze(parsed)?;
        result.references = analyzers::reference::analyze(parsed)?;
        result.types = analyzers::type_analyzer::analyze(parsed)?;
        result.scopes = analyzers::scope::analyze(parsed)?;
        result.signatures = analyzers::signature::analyze(parsed)?;
        result.exports = analyzers::export::analyze_legacy(parsed)?;

        Ok(result)
    }
}

impl Pipeline for WorkspacePipeline {
    type Input = Vec<SourceItem>;
    type Output = storage::Index;
    
    fn run(&self, input: Self::Input) -> Result<Self::Output, LspError> {
        // Call the internal implementation to avoid recursion
        self.run_impl(input)
    }
    
    fn name(&self) -> &'static str {
        "WorkspacePipeline"
    }
    
    fn description(&self) -> &'static str {
        "Full analysis pipeline for workspace files. Extracts all metadata (symbols, references, types, scopes, signatures, exports) and creates an Index for LSP features."
    }
}

impl Default for WorkspacePipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::sources::file::FileSource;
    use std::path::PathBuf;

    fn create_test_source_item(code: &str) -> SourceItem {
        FileSource::from_content(PathBuf::from("test.jl"), code.to_string())
    }

    #[test]
    fn test_run_workspace_pipeline() {
        let source_items = vec![
            create_test_source_item("function test1() return 42 end"),
            create_test_source_item("struct MyStruct x::Int end"),
        ];

        let pipeline = WorkspacePipeline::new();
        let index = pipeline.run(source_items).unwrap();

        // Should extract symbols, types, signatures, etc.
        assert!(!index.get_all_symbols().is_empty());
    }

    #[test]
    fn test_run_single_file() {
        let source_item = create_test_source_item("function test() return 42 end");
        let pipeline = WorkspacePipeline::new();
        let analysis = pipeline.run_single_file(source_item).unwrap();

        assert!(!analysis.symbols.is_empty());
        assert_eq!(analysis.symbols[0].name, "test");
    }

    #[test]
    fn test_run_empty_input() {
        let source_items = vec![];
        let pipeline = WorkspacePipeline::new();
        let index = pipeline.run(source_items).unwrap();

        assert_eq!(index.get_all_symbols().len(), 0);
    }

    #[test]
    fn test_run_multiple_files() {
        let source_items = vec![
            FileSource::from_content(PathBuf::from("test1.jl"), "function func1() end".to_string()),
            FileSource::from_content(PathBuf::from("test2.jl"), "function func2() end".to_string()),
            FileSource::from_content(PathBuf::from("test3.jl"), "function func3() end".to_string()),
        ];

        let pipeline = WorkspacePipeline::new();
        let index = pipeline.run(source_items).unwrap();

        // Should have symbols from all files
        let symbols = index.get_all_symbols();
        assert!(symbols.len() >= 3);
    }
}

