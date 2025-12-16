use crate::types::{TypeDefinition, TypeDefinitionKind, Range, Position};
use crate::types::LspError;
use tree_sitter::Node;
use super::docstring_extraction::extract_docstring;

/// Extract struct definition
pub fn extract_struct_definition(
    node: Node,
    source: &str,
    module_name: &str,
    file_uri: &str,
    find_first_child_of_type: &dyn for<'a> Fn(Node<'a>, &'a str) -> Result<Node<'a>, LspError>,
) -> Result<Option<TypeDefinition>, LspError> {
    // Find struct name
    if let Ok(name_node) = find_first_child_of_type(node, "identifier") {
        let name = name_node.utf8_text(source.as_bytes())
            .map_err(|e| LspError::ParseError(format!("Failed to extract struct name: {}", e)))?
            .to_string();
        
        let doc_comment = extract_docstring(node, source);
        let range = node_to_range(node);
        
        Ok(Some(TypeDefinition {
            module: module_name.to_string(),
            name,
            kind: TypeDefinitionKind::Struct,
            doc_comment,
            file_uri: file_uri.to_string(),
            range,
        }))
    } else {
        Ok(None)
    }
}

/// Extract abstract type definition
pub fn extract_abstract_definition(
    node: Node,
    source: &str,
    module_name: &str,
    file_uri: &str,
    find_first_child_of_type: &dyn for<'a> Fn(Node<'a>, &'a str) -> Result<Node<'a>, LspError>,
) -> Result<Option<TypeDefinition>, LspError> {
    // Find abstract type name
    if let Ok(name_node) = find_first_child_of_type(node, "identifier") {
        let name = name_node.utf8_text(source.as_bytes())
            .map_err(|e| LspError::ParseError(format!("Failed to extract abstract type name: {}", e)))?
            .to_string();
        
        let doc_comment = extract_docstring(node, source);
        let range = node_to_range(node);
        
        Ok(Some(TypeDefinition {
            module: module_name.to_string(),
            name,
            kind: TypeDefinitionKind::Abstract,
            doc_comment,
            file_uri: file_uri.to_string(),
            range,
        }))
    } else {
        Ok(None)
    }
}

fn node_to_range(node: Node) -> Range {
    let start_pos = node.start_position();
    let end_pos = node.end_position();

    Range {
        start: Position {
            line: start_pos.row as u32,
            character: start_pos.column as u32,
        },
        end: Position {
            line: end_pos.row as u32,
            character: end_pos.column as u32,
        },
    }
}

