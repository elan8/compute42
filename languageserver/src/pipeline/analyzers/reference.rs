use crate::pipeline::types::{ParsedItem, Reference, ReferenceKind};
use crate::types::{LspError, Range, Position};
use tree_sitter::Node;

/// Analyze a parsed item to extract references
pub fn analyze(parsed: &ParsedItem) -> Result<Vec<Reference>, LspError> {
    let mut references = Vec::new();
    let root = parsed.tree.root_node();
    let text = parsed.text.as_str();

    walk_node(&root, text, &parsed.path.to_string_lossy(), &mut references)?;

    Ok(references)
}

fn walk_node(
    node: &Node,
    text: &str,
    file_uri: &str,
    references: &mut Vec<Reference>,
) -> Result<(), LspError> {
    match node.kind() {
        "identifier" => {
            // Skip identifiers that are part of call_expressions (handled separately)
            if let Some(parent) = node.parent() {
                if parent.kind() == "call_expression" {
                    // This identifier is the function name in a call - skip it here
                    // It will be handled by the call_expression case
                    return Ok(());
                }
            }
            
            // Check if this is a reference (not a definition)
            if is_reference(node) {
                if let Ok(name) = node.utf8_text(text.as_bytes()) {
                    let range = node_to_range(*node);
                    references.push(Reference {
                        name: name.to_string(),
                        range,
                        file_uri: file_uri.to_string(),
                        kind: ReferenceKind::Variable,
                    });
                }
            }
        }
        "call_expression" => {
            // Extract function call references
            if let Some(function_node) = node.child(0) {
                if function_node.kind() == "identifier" {
                    if let Ok(name) = function_node.utf8_text(text.as_bytes()) {
                        let range = node_to_range(function_node);
                        references.push(Reference {
                            name: name.to_string(),
                            range,
                            file_uri: file_uri.to_string(),
                            kind: ReferenceKind::FunctionCall,
                        });
                    }
                }
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_node(&child, text, file_uri, references)?;
        }
    }

    Ok(())
}

fn is_reference(node: &Node) -> bool {
    // Simple heuristic: if parent is not a definition node, it's likely a reference
    if let Some(parent) = node.parent() {
        match parent.kind() {
            "function_definition" | "assignment" | "struct_definition" | "abstract_definition" => {
                // Check if this identifier is the name being defined
                if let Some(first_child) = parent.child(0) {
                    return first_child.byte_range() != node.byte_range();
                }
            }
            _ => return true,
        }
    }
    true
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::parser;
    use crate::pipeline::sources::file::FileSource;
    use std::path::PathBuf;

    fn parse_code(code: &str) -> ParsedItem {
        let source = FileSource::from_content(PathBuf::from("test.jl"), code.to_string());
        parser::parse(&source).unwrap()
    }

    #[test]
    fn test_analyze_variable_reference() {
        let code = r#"
x = 10
y = x + 5
"#;
        let parsed = parse_code(code);
        let references = analyze(&parsed).unwrap();

        // Should find reference to 'x' in 'y = x + 5'
        assert!(references.iter().any(|r| r.name == "x" && r.kind == ReferenceKind::Variable));
    }

    #[test]
    fn test_analyze_function_call() {
        let code = "println(\"Hello\")";
        let parsed = parse_code(code);
        let references = analyze(&parsed).unwrap();

        assert!(references.iter().any(|r| r.name == "println" && r.kind == ReferenceKind::FunctionCall));
    }
}



