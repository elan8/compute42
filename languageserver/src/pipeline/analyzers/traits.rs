/// Trait for analyzers that process parsed items
pub trait Analyzer<T> {
    /// Analyze a parsed item and return the result
    fn analyze(parsed: &crate::pipeline::types::ParsedItem) -> Result<T, crate::types::LspError>;
}


















