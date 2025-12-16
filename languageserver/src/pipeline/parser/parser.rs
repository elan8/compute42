use tree_sitter::{Parser, Language};
use tree_sitter_julia::LANGUAGE;
use crate::types::LspError;

pub struct JuliaParser {
    language: Language,
}

impl JuliaParser {
    pub fn new() -> Self {
        Self {
            language: LANGUAGE.into(),
        }
    }
    
    pub fn create_parser(&self) -> Result<Parser, LspError> {
        let mut parser = Parser::new();
        parser.set_language(&self.language)
            .map_err(|e| LspError::ParseError(format!("Failed to set Julia language: {}", e)))?;
        Ok(parser)
    }
    
    pub fn parse(&self, source: &str) -> Result<tree_sitter::Tree, LspError> {
        let mut parser = self.create_parser()?;
        parser.parse(source, None)
            .ok_or_else(|| LspError::ParseError("Failed to parse source".to_string()))
    }
}
