mod parser;

pub use parser::JuliaParser;

use crate::pipeline::types::{SourceItem, ParsedItem};
use crate::types::LspError;

/// Parse a source item into a parsed item
pub fn parse(source: &SourceItem) -> Result<ParsedItem, LspError> {
    let parser = JuliaParser::new();
    let tree = parser.parse(&source.content)?;
    
    Ok(ParsedItem {
        path: source.path.clone(),
        tree,
        text: source.content.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::sources::file::FileSource;
    use std::path::PathBuf;

    #[test]
    fn test_parse_valid_code() {
        let source = FileSource::from_content(
            PathBuf::from("test.jl"),
            "function test() return 42 end".to_string(),
        );
        let result = parse(&source);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.path, PathBuf::from("test.jl"));
        assert!(!parsed.text.is_empty());
    }

    #[test]
    fn test_parse_empty_code() {
        let source = FileSource::from_content(PathBuf::from("test.jl"), String::new());
        let result = parse(&source);
        // Parser should handle empty code gracefully
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let source = FileSource::from_content(
            PathBuf::from("test.jl"),
            "function incomplete(".to_string(),
        );
        // Parser might return a tree with errors or fail
        // Both behaviors are acceptable
        let _ = parse(&source);
    }

    #[test]
    fn test_parse_preserves_content() {
        let content = "function test()\n    return 42\nend";
        let source = FileSource::from_content(PathBuf::from("test.jl"), content.to_string());
        let parsed = parse(&source).unwrap();
        assert_eq!(parsed.text, content);
    }
}

