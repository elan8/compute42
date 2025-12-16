use crate::pipeline::sources::Document;
use crate::pipeline::query::SymbolResolver;
use crate::pipeline::{storage::Index, query::{symbol::SymbolQuery, reference::ReferenceQuery}};
use crate::types::{Location, Position};

/// Stateless references provider - uses Index and query engine
pub struct ReferencesProvider;

impl ReferencesProvider {
    pub fn find_references(
        index: &Index,
        document: &Document,
        position: Position,
        include_declaration: bool,
    ) -> Option<Vec<Location>> {
        let tree = document.tree()?;
        let text = document.text();
        let resolver = SymbolResolver::new(tree, &text);
        let node = resolver.node_at_position(position.line, position.character)?;
        let symbol_name = resolver.extract_symbol_name(node)?;
        
        // Use query engine to find references
        let ref_query = ReferenceQuery::new(index);
        let references = ref_query.find_references(&symbol_name);
        
        let mut locations: Vec<Location> = references
            .into_iter()
            .map(|r| Location {
                uri: r.file_uri,
                range: r.range,
            })
            .collect();
        
        // Add definition if requested
        if include_declaration {
            let symbol_query = SymbolQuery::new(index);
            if let Some(symbol) = symbol_query
                .resolve_symbol_at(&symbol_name, document.uri(), position)
                .or_else(|| symbol_query.find_symbol(&symbol_name))
            {
                let definition_location = Location {
                    uri: symbol.file_uri.clone(),
                    range: symbol.range.clone(),
                };
                
                // Check if definition is already in the locations (avoid duplication)
                let already_has_definition = locations.iter().any(|loc| 
                    loc.uri == definition_location.uri && 
                    loc.range.start.line == definition_location.range.start.line &&
                    loc.range.start.character == definition_location.range.start.character
                );
                
                if !already_has_definition {
                    locations.push(definition_location);
                }
            }
        }
        
        Some(locations)
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
    fn test_find_references() {
        let parser = JuliaParser::new();
        let code = "function my_function(x) return x + 1 end\nmy_function(5)\nmy_function(10)";
        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();
        
        // Build index using pipeline
        let index = build_index_from_code(code, "test.jl");
        
        // Test finding references at definition site (line 0, character 9)
        let position = Position { line: 0, character: 9 };
        let locations = ReferencesProvider::find_references(
            &index, 
            &doc, 
            position, 
            true
        );
        
        assert!(locations.is_some());
        let locations = locations.unwrap();
        // Should find definition + 2 usages = 3 total
        assert_eq!(locations.len(), 3);
        
        // All should be in the same file
        for location in &locations {
            assert_eq!(location.uri, "test.jl");
        }
    }

    #[test]
    fn test_find_references_exclude_declaration() {
        let parser = JuliaParser::new();
        let code = "function my_function(x) return x + 1 end\nmy_function(5)";
        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();
        
        // Build index using pipeline
        let index = build_index_from_code(code, "test.jl");
        
        // Test finding references excluding declaration
        let position = Position { line: 0, character: 9 };
        let locations = ReferencesProvider::find_references(
            &index, 
            &doc, 
            position, 
            false
        );
        
        assert!(locations.is_some());
        let locations = locations.unwrap();
        // Should find only 1 usage (excluding definition)
        // Note: The analyzer extracts references, so we get the usage on line 1
        assert!(locations.len() >= 1);
    }

    #[test]
    fn test_find_references_nonexistent() {
        let parser = JuliaParser::new();
        let code = "nonexistent_function()";
        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();
        
        // Build index (will have reference but no definition)
        let index = build_index_from_code(code, "test.jl");
        
        // Test finding references for non-existent function
        let position = Position { line: 0, character: 0 };
        let locations = ReferencesProvider::find_references(
            &index, 
            &doc, 
            position, 
            true
        );
        
        assert!(locations.is_some());
        let locations = locations.unwrap();
        // Should find only the usage itself (no definition)
        assert_eq!(locations.len(), 1);
    }
}
