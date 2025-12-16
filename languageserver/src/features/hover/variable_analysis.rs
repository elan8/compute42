use crate::pipeline::storage::Index;
use tree_sitter::{Node, Tree};

/// Infer variable type from assignment or context
/// For function parameters, extracts type annotations from function signature
pub async fn infer_variable_type<'a>(
    node: Node<'a>,
    tree: &Tree,
    text: &str,
    _index: &Index,
) -> Option<String> {
    // Check if this is a function parameter
    if let Some(param_type) = extract_parameter_type(node, tree, text) {
        return Some(param_type);
    }
    
    // Type inference for other cases is not supported
    None
}

/// Extract parameter type from function signature if node is a parameter
fn extract_parameter_type<'a>(node: Node<'a>, _tree: &Tree, text: &str) -> Option<String> {
    // Walk up to find if we're in a function definition
    let mut current = node;
    let mut function_def: Option<Node> = None;
    
    while let Some(parent) = current.parent() {
        if parent.kind() == "function_definition" {
            function_def = Some(parent);
            break;
        }
        // Stop at module boundaries
        if parent.kind() == "module_definition" {
            break;
        }
        current = parent;
    }
    
    let function_def = function_def?;
    
    // Extract parameter name from node
    let param_name = node.utf8_text(text.as_bytes()).ok()?;
    
    // Find signature node
    let signature_node = find_first_child_of_type(function_def, "signature")?;
    
    // Find parameter list - it can be in call_expression -> argument_list or directly argument_list
    let param_list = find_first_child_of_type(signature_node, "call_expression")
        .and_then(|call| find_first_child_of_type(call, "argument_list"))
        .or_else(|| find_first_child_of_type(signature_node, "argument_list"))?;
    
    // Search for parameter with matching name
    for i in 0..param_list.child_count() {
        if let Some(param_node) = param_list.child(i) {
            match param_node.kind() {
                "identifier" => {
                    if let Ok(name) = param_node.utf8_text(text.as_bytes()) {
                        if name == param_name {
                            // Found parameter without type annotation
                            return None; // No type info available
                        }
                    }
                }
                "typed_expression" => {
                    // Parameter with type annotation: x::Int64
                    if let Some(id_node) = find_first_child_of_type(param_node, "identifier") {
                        if let Ok(name) = id_node.utf8_text(text.as_bytes()) {
                            if name == param_name {
                                // Extract type annotation
                                if let Some(type_node) = find_first_child_of_type(param_node, "type_expression") {
                                    if let Ok(type_str) = type_node.utf8_text(text.as_bytes()) {
                                        return Some(type_str.to_string());
                                    }
                                }
                                // Try alternative: look for :: followed by type
                                if let Ok(full_text) = param_node.utf8_text(text.as_bytes()) {
                                    if let Some(colon_pos) = full_text.find("::") {
                                        let type_part = &full_text[colon_pos + 2..];
                                        return Some(type_part.trim().to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    None
}

/// Helper to find first child of a specific type
fn find_first_child_of_type<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == kind {
                return Some(child);
            }
        }
    }
    None
}
