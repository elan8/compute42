use crate::pipeline::sources::Document;
use crate::pipeline::query::SymbolResolver;
use crate::pipeline::{storage::Index, query::symbol::SymbolQuery};
use crate::types::{Location, Position};

/// Stateless definition provider - uses Index and query engine
pub struct DefinitionProvider;

impl DefinitionProvider {
    pub fn find_definition(
        index: &Index,
        document: &Document,
        position: Position,
    ) -> Option<Vec<Location>> {
        let tree = document.tree()?;
        let text = document.text();
        let resolver = SymbolResolver::new(tree, &text);
        let node = resolver.node_at_position(position.line, position.character)?;
        let symbol_name = resolver.extract_symbol_name(node)?;
        
        // Use query engine with scope-aware resolution
        let symbol_query = SymbolQuery::new(index);
        let symbol = symbol_query
            .resolve_symbol_at(&symbol_name, document.uri(), position)
            .or_else(|| symbol_query.find_symbol(&symbol_name))?;
        
        Some(vec![Location {
            uri: symbol.file_uri.clone(),
            range: symbol.range.clone(),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::parser::JuliaParser;
    use crate::pipeline::{WorkspacePipeline, sources::file::FileSource};
    use std::path::PathBuf;

    fn build_index_from_code(code: &str, file_path: &str) -> Index {
        let source_item = FileSource::from_content(PathBuf::from(file_path), code.to_string());
        let pipeline = WorkspacePipeline::new();
        pipeline.run(vec![source_item]).unwrap()
    }

    #[test]
    fn test_find_definition() {
        let parser = JuliaParser::new();
        let code = "function my_function(x) return x + 1 end\nmy_function(5)";
        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();
        
        // Build index using pipeline
        let index = build_index_from_code(code, "test.jl");
        
        // Test finding definition at usage site (line 1, character 0)
        let position = Position { line: 1, character: 0 };
        let locations = DefinitionProvider::find_definition(&index, &doc, position);
        
        assert!(locations.is_some());
        let locations = locations.unwrap();
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].uri, "test.jl");
        // The definition should be at line 0
        assert_eq!(locations[0].range.start.line, 0);
    }

    #[test]
    fn test_find_definition_nonexistent() {
        let parser = JuliaParser::new();
        let code = "nonexistent_function()";
        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();
        
        // Build index (empty since no definitions)
        let index = build_index_from_code(code, "test.jl");
        
        // Test finding definition for non-existent function
        let position = Position { line: 0, character: 0 };
        let locations = DefinitionProvider::find_definition(&index, &doc, position);
        
        assert!(locations.is_none());
    }
}
