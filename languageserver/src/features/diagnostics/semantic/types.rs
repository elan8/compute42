use crate::pipeline::storage::Index;
use crate::types::Diagnostic;
use tree_sitter::{Node, Tree};

/// Check for type mismatches using stored inferred types from Index
/// Types are pre-computed during indexing, so we can check for mismatches
pub(super) fn check_type_mismatches(
    tree: &Tree,
    text: &str,
    index: &Index,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let root = tree.root_node();
    check_type_mismatches_recursive(root, text, index, diagnostics);
}

/// Recursively check for type mismatches in AST nodes
pub(super) fn check_type_mismatches_recursive(
    node: Node,
    text: &str,
    index: &Index,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check function calls for type mismatches
    if node.kind() == "call_expression" {
        check_function_call_types(node, text, index, diagnostics);
    }
    
    // Check assignments for type mismatches
    if node.kind() == "assignment" {
        check_assignment_types(node, text, index, diagnostics);
    }
    
    // Recursively check children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            check_type_mismatches_recursive(child, text, index, diagnostics);
        }
    }
}

/// Check function call for type mismatches
pub(super) fn check_function_call_types(
    call_node: Node,
    text: &str,
    index: &Index,
    _diagnostics: &mut Vec<Diagnostic>,
) {
    // Extract function name
    let func_name = extract_function_name(call_node, text);
    if func_name.is_none() {
        return;
    }
    let func_name = func_name.unwrap();
    
    // Find function signatures in Index
    // Try qualified name first (e.g., "CSV.read")
    let (module, name): (&str, &str) = if let Some((m, n)) = func_name.split_once('.') {
        (m, n)
    } else {
        ("Main", func_name.as_str())
    };
    
    let signatures = index.find_signatures(module, name);
    if signatures.is_empty() {
        // Function not found in index - skip type checking
    }
    
    // Get argument types from Index (if available)
    // For now, we just check if function exists - full type checking can be enhanced later
    // TODO: Compare argument types with function signature parameter types
}

/// Check assignment for type mismatches
pub(super) fn check_assignment_types(
    _assignment_node: Node,
    _text: &str,
    _index: &Index,
    _diagnostics: &mut Vec<Diagnostic>,
) {
    // Type inference is not supported for Julia
}

/// Validate a function call: check if function exists, argument count, and argument types
/// Type inference is not supported for Julia
#[allow(dead_code)]
pub(super) fn validate_function_call(
    _call_node: Node,
    _text: &str,
    _index: &Index,
    _diagnostics: &mut Vec<Diagnostic>,
) {
    // Type inference is not supported for Julia
}

/// Extract function name from a call_expression node
pub(super) fn extract_function_name(call_node: Node, text: &str) -> Option<String> {
    // Find the function identifier or field_access
    for i in 0..call_node.child_count() {
        if let Some(child) = call_node.child(i) {
            match child.kind() {
                "identifier" => {
                    if let Ok(name) = child.utf8_text(text.as_bytes()) {
                        return Some(name.to_string());
                    }
                }
                "field_access" | "field_expression" => {
                    // Extract qualified name like "CSV.read"
                    if let Ok(qualified) = child.utf8_text(text.as_bytes()) {
                        return Some(qualified.to_string());
                    }
                }
                _ => {}
            }
        }
    }
    None
}

