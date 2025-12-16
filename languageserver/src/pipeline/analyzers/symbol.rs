use crate::pipeline::types::ParsedItem;
use crate::types::{Symbol, SymbolKind, LspError, Range, Position};
use tree_sitter::Node;

/// Analyze a parsed item to extract symbols
pub fn analyze(parsed: &ParsedItem) -> Result<Vec<Symbol>, LspError> {
    let mut symbols = Vec::new();
    let root = parsed.tree.root_node();
    let text = parsed.text.as_str();

    walk_node(&root, text, &parsed.path.to_string_lossy(), 0, &mut symbols)?;

    Ok(symbols)
}

fn walk_node(
    node: &Node,
    text: &str,
    file_uri: &str,
    scope_id: u32,
    symbols: &mut Vec<Symbol>,
) -> Result<(), LspError> {
    match node.kind() {
        "function_definition" => {
            if let Some(symbol) = extract_function_symbol(node, text, file_uri, scope_id)? {
                symbols.push(symbol);
            }
            // Extract function parameters as symbols
            // Set scope_id to 0 so resolve_symbol_at will find the scope by range
            // The parameter's range will be within the function's scope, so it will be found correctly
            extract_function_parameters(node, text, file_uri, 0, symbols)?;
        }
        "assignment" => {
            if let Some(symbol) = extract_assignment_symbol(node, text, file_uri, scope_id)? {
                symbols.push(symbol);
            }
        }
        "struct_definition" => {
            if let Some(symbol) = extract_struct_symbol(node, text, file_uri, scope_id)? {
                symbols.push(symbol);
            }
        }
        "abstract_definition" => {
            if let Some(symbol) = extract_abstract_symbol(node, text, file_uri, scope_id)? {
                symbols.push(symbol);
            }
        }
        "module_definition" => {
            if let Some(symbol) = extract_module_symbol(node, text, file_uri, scope_id)? {
                symbols.push(symbol);
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_node(&child, text, file_uri, scope_id, symbols)?;
        }
    }

    Ok(())
}

/// Extract function parameters as symbols
fn extract_function_parameters(
    node: &Node,
    text: &str,
    file_uri: &str,
    function_scope_id: u32,
    symbols: &mut Vec<Symbol>,
) -> Result<(), LspError> {
    // Find the signature node
    if let Some(signature_node) = find_first_child_of_type(node, "signature") {
        // Find parameter_list - it can be in different places:
        // 1. signature -> call_expression -> argument_list
        // 2. signature -> argument_list (direct)
        if let Some(call_node) = find_first_child_of_type(&signature_node, "call_expression") {
            if let Some(param_list) = find_first_child_of_type(&call_node, "argument_list") {
                extract_parameters_from_list(&param_list, text, file_uri, function_scope_id, symbols)?;
            }
        } else if let Some(param_list) = find_first_child_of_type(&signature_node, "argument_list") {
            extract_parameters_from_list(&param_list, text, file_uri, function_scope_id, symbols)?;
        }
    }
    Ok(())
}

/// Extract parameters from an argument_list node
fn extract_parameters_from_list(
    param_list: &Node,
    text: &str,
    file_uri: &str,
    scope_id: u32,
    symbols: &mut Vec<Symbol>,
) -> Result<(), LspError> {
    for i in 0..param_list.child_count() {
        if let Some(param) = param_list.child(i) {
            match param.kind() {
                "identifier" => {
                    // Simple parameter: x
                    if let Ok(name) = param.utf8_text(text.as_bytes()) {
                        let range = node_to_range(param);
                        symbols.push(Symbol {
                            name: name.to_string(),
                            kind: SymbolKind::Variable,
                            range,
                            scope_id,
                            doc_comment: None,
                            signature: None,
                            file_uri: file_uri.to_string(),
                        });
                    }
                }
                "typed_parameter" => {
                    // Typed parameter: x::Int
                    if let Some(ident) = find_first_child_of_type(&param, "identifier") {
                        if let Ok(name) = ident.utf8_text(text.as_bytes()) {
                            let range = node_to_range(ident);
                            symbols.push(Symbol {
                                name: name.to_string(),
                                kind: SymbolKind::Variable,
                                range,
                                scope_id,
                                doc_comment: None,
                                signature: None,
                                file_uri: file_uri.to_string(),
                            });
                        }
                    }
                }
                "assignment" => {
                    // Default parameter: x=5
                    if let Some(lhs) = param.child(0) {
                        if lhs.kind() == "identifier" {
                            if let Ok(name) = lhs.utf8_text(text.as_bytes()) {
                                let range = node_to_range(lhs);
                                symbols.push(Symbol {
                                    name: name.to_string(),
                                    kind: SymbolKind::Variable,
                                    range,
                                    scope_id,
                                    doc_comment: None,
                                    signature: None,
                                    file_uri: file_uri.to_string(),
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn extract_function_symbol(
    node: &Node,
    text: &str,
    file_uri: &str,
    scope_id: u32,
) -> Result<Option<Symbol>, LspError> {
    // Function name is in: function_definition -> signature -> call_expression -> identifier
    if let Some(signature_node) = find_first_child_of_type(node, "signature") {
        if let Some(call_node) = find_first_child_of_type(&signature_node, "call_expression") {
            if let Some(name_node) = find_first_child_of_type(&call_node, "identifier") {
                let name = name_node.utf8_text(text.as_bytes())
                    .map_err(|e| LspError::ParseError(format!("Failed to extract function name: {}", e)))?
                    .to_string();

                let range = node_to_range(name_node);
                let doc_comment = extract_doc_comment(node, text)?;

                return Ok(Some(Symbol {
                    name,
                    kind: SymbolKind::Function,
                    range,
                    scope_id,
                    doc_comment,
                    signature: None,
                    file_uri: file_uri.to_string(),
                }));
            }
        }
    }

    Ok(None)
}

fn extract_assignment_symbol(
    node: &Node,
    text: &str,
    file_uri: &str,
    scope_id: u32,
) -> Result<Option<Symbol>, LspError> {
    // Check for regular identifier first
    if let Some(identifier) = find_first_child_of_type(node, "identifier") {
        let name = identifier.utf8_text(text.as_bytes())
            .map_err(|e| LspError::ParseError(format!("Failed to extract variable name: {}", e)))?
            .to_string();

        let range = node_to_range(identifier);

        return Ok(Some(Symbol {
            name,
            kind: SymbolKind::Variable,
            range,
            scope_id,
            doc_comment: None,
            signature: None,
            file_uri: file_uri.to_string(),
        }));
    }
    
    // Check for typed_identifier (e.g., x::Int64 = 42)
    if let Some(typed_identifier) = find_first_child_of_type(node, "typed_identifier") {
        // Find the identifier child within typed_identifier
        if let Some(identifier) = find_first_child_of_type(&typed_identifier, "identifier") {
            let name = identifier.utf8_text(text.as_bytes())
                .map_err(|e| LspError::ParseError(format!("Failed to extract variable name: {}", e)))?
                .to_string();

            let range = node_to_range(identifier);

            return Ok(Some(Symbol {
                name,
                kind: SymbolKind::Variable,
                range,
                scope_id,
                doc_comment: None,
                signature: None,
                file_uri: file_uri.to_string(),
            }));
        }
    }
    
    // Check for typed_expression (e.g., x::Int64 = 42) - tree-sitter might use this
    if let Some(typed_expression) = find_first_child_of_type(node, "typed_expression") {
        // Find the identifier child within typed_expression
        if let Some(identifier) = find_first_child_of_type(&typed_expression, "identifier") {
            let name = identifier.utf8_text(text.as_bytes())
                .map_err(|e| LspError::ParseError(format!("Failed to extract variable name: {}", e)))?
                .to_string();

            let range = node_to_range(identifier);

            return Ok(Some(Symbol {
                name,
                kind: SymbolKind::Variable,
                range,
                scope_id,
                doc_comment: None,
                signature: None,
                file_uri: file_uri.to_string(),
            }));
        }
    }

    Ok(None)
}

fn extract_struct_symbol(
    node: &Node,
    text: &str,
    file_uri: &str,
    scope_id: u32,
) -> Result<Option<Symbol>, LspError> {
    if let Some(type_head) = find_first_child_of_type(node, "type_head") {
        if let Some(name_node) = find_first_child_of_type(&type_head, "identifier") {
            let name = name_node.utf8_text(text.as_bytes())
                .map_err(|e| LspError::ParseError(format!("Failed to extract struct name: {}", e)))?
                .to_string();

            let range = node_to_range(name_node);
            let doc_comment = extract_doc_comment(node, text)?;

            return Ok(Some(Symbol {
                name,
                kind: SymbolKind::Type,
                range,
                scope_id,
                doc_comment,
                signature: None,
                file_uri: file_uri.to_string(),
            }));
        }
    }

    Ok(None)
}

fn extract_abstract_symbol(
    node: &Node,
    text: &str,
    file_uri: &str,
    scope_id: u32,
) -> Result<Option<Symbol>, LspError> {
    if let Some(type_head) = find_first_child_of_type(node, "type_head") {
        if let Some(name_node) = find_first_child_of_type(&type_head, "identifier") {
            let name = name_node.utf8_text(text.as_bytes())
                .map_err(|e| LspError::ParseError(format!("Failed to extract abstract type name: {}", e)))?
                .to_string();

            let range = node_to_range(name_node);
            let doc_comment = extract_doc_comment(node, text)?;

            return Ok(Some(Symbol {
                name,
                kind: SymbolKind::Type,
                range,
                scope_id,
                doc_comment,
                signature: None,
                file_uri: file_uri.to_string(),
            }));
        }
    }

    Ok(None)
}

fn extract_module_symbol(
    node: &Node,
    text: &str,
    file_uri: &str,
    scope_id: u32,
) -> Result<Option<Symbol>, LspError> {
    if let Some(name_node) = find_first_child_of_type(node, "identifier") {
        let name = name_node.utf8_text(text.as_bytes())
            .map_err(|e| LspError::ParseError(format!("Failed to extract module name: {}", e)))?
            .to_string();

        let range = node_to_range(name_node);
        let doc_comment = extract_doc_comment(node, text)?;

        return Ok(Some(Symbol {
            name,
            kind: SymbolKind::Module,
            range,
            scope_id,
            doc_comment,
            signature: None,
            file_uri: file_uri.to_string(),
        }));
    }

    Ok(None)
}

fn find_first_child_of_type<'a>(node: &'a Node<'a>, kind: &str) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == kind {
                return Some(child);
            }
        }
    }
    None
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

fn extract_doc_comment(node: &Node, text: &str) -> Result<Option<String>, LspError> {
    // Look for docstring comment before the node
    let start_byte = node.start_byte();
    let before_text = &text[..start_byte.min(text.len())];

    // Simple heuristic: look for """...""" pattern before the node
    if let Some(doc_start) = before_text.rfind("\"\"\"") {
        if let Some(doc_end) = text[doc_start + 3..].find("\"\"\"") {
            let doc = text[doc_start + 3..doc_start + 3 + doc_end].trim().to_string();
            if !doc.is_empty() {
                return Ok(Some(doc));
            }
        }
    }

    Ok(None)
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
    fn test_analyze_function() {
        let code = "function test() return 42 end";
        let parsed = parse_code(code);
        let symbols = analyze(&parsed).unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "test");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_analyze_variable() {
        let code = "x = 10";
        let parsed = parse_code(code);
        let symbols = analyze(&parsed).unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "x");
        assert_eq!(symbols[0].kind, SymbolKind::Variable);
    }

    #[test]
    fn test_analyze_struct() {
        let code = "struct MyStruct x::Int end";
        let parsed = parse_code(code);
        let symbols = analyze(&parsed).unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "MyStruct");
        assert_eq!(symbols[0].kind, SymbolKind::Type);
    }

    #[test]
    fn test_analyze_multiple_symbols() {
        let code = r#"
function f1() end
x = 1
function f2() end
y = 2
"#;
        let parsed = parse_code(code);
        let symbols = analyze(&parsed).unwrap();

        assert_eq!(symbols.len(), 4);
        assert!(symbols.iter().any(|s| s.name == "f1"));
        assert!(symbols.iter().any(|s| s.name == "f2"));
        assert!(symbols.iter().any(|s| s.name == "x"));
        assert!(symbols.iter().any(|s| s.name == "y"));
    }
}

