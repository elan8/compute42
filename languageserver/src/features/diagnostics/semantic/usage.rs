use crate::pipeline::storage::Index;
use crate::types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tree_sitter::{Node, Tree};
use std::collections::{HashMap, HashSet};

use super::utils;

/// Check for unused variables
pub(super) fn check_unused_variables(
    tree: &Tree,
    text: &str,
    _index: &Index,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let root = tree.root_node();
    let mut variable_definitions = HashMap::new();
    let mut variable_usages = HashSet::new();
    
    // Collect variable definitions and usages
    collect_variable_usage(root, text, &mut variable_definitions, &mut variable_usages);
    
    // Report unused variables
    for (name, (range, is_parameter)) in variable_definitions {
        // Skip parameters (they might be used externally)
        if is_parameter {
            continue;
        }
        
        if !variable_usages.contains(&name) {
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::Warning),
                code: Some("unused_variable".to_string()),
                source: Some("semantic".to_string()),
                message: format!("Unused variable: `{}`", name),
                related_information: None,
            });
        }
    }
}

/// Check if an assignment node is actually a keyword argument (not a variable assignment)
/// In Julia, keyword arguments like `pkg=DecisionTree` in macro calls are parsed as assignments
/// by tree-sitter, but they're not variable definitions.
pub(super) fn is_keyword_argument_assignment(assignment_node: Node, _text: &str) -> bool {
    // Walk up the tree to find if we're inside a macro_call or keyword_argument
    let mut current = assignment_node.parent();
    let mut found_macro_call = false;
    
    while let Some(n) = current {
        let kind = n.kind();
        
        // If we're inside a keyword_argument node, this is definitely a keyword arg
        if kind == "keyword_argument" {
            return true;
        }
        
        // Check for named_field or pair nodes (used in some contexts for keyword-like syntax)
        if kind == "named_field" || kind == "pair" {
            return true;
        }
        
        // Check for macro_argument_list - assignments inside macro argument lists are keyword args
        // This is the most direct check since tree-sitter Julia parses macro args this way
        if kind == "macro_argument_list" {
            return true;
        }
        
        // Track if we've seen a macro_call or macrocall_expression
        // Tree-sitter Julia uses "macrocall_expression" as the node type
        if kind == "macro_call" || kind == "macrocall_expression" {
            found_macro_call = true;
        }
        
        // If we hit a block structure BEFORE finding a macro_call, this is a real assignment
        // Block structures that indicate real code (not macro arguments):
        if matches!(kind, "begin_statement" | "block" | "if_statement" | "for_statement" | "while_statement" | "function_definition" | "module_definition" | "struct_definition") {
            // If we haven't found a macro_call yet, this is a real assignment
            if !found_macro_call {
                return false;
            }
            // If we found a macro_call but then hit a block, check if the block is inside the macro
            // or if the assignment is inside the block (which would make it a real assignment)
            // For now, if we hit a block after a macro, assume it's still in macro context
            // (this handles cases like @macro begin x=1 end where x=1 is still a keyword arg)
        }
        
        current = n.parent();
    }
    
    // If we found a macro_call and didn't hit a blocking structure, this is a keyword argument
    found_macro_call
}

/// Collect variable definitions and usages
pub(super) fn collect_variable_usage(
    node: Node,
    text: &str,
    definitions: &mut HashMap<String, (Range, bool)>,
    usages: &mut HashSet<String>,
) {
    match node.kind() {
        "assignment" => {
            // Skip assignments that are actually keyword arguments in macro calls or function calls
            if !is_keyword_argument_assignment(node, text) {
                // This is a real variable assignment
                if let Some(identifier) = node.child(0) {
                    if identifier.kind() == "identifier" {
                        if let Ok(name) = identifier.utf8_text(text.as_bytes()) {
                            let range = Range {
                                start: Position::from(identifier.start_position()),
                                end: Position::from(identifier.end_position()),
                            };
                            definitions.insert(name.to_string(), (range, false));
                        }
                    }
                }
            }
        }
        "function_definition" => {
            // Collect parameters
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "parameter_list" {
                        for j in 0..child.child_count() {
                            if let Some(param) = child.child(j) {
                                if param.kind() == "identifier" {
                                    if let Ok(name) = param.utf8_text(text.as_bytes()) {
                                        let range = Range {
                                            start: Position::from(param.start_position()),
                                            end: Position::from(param.end_position()),
                                        };
                                        definitions.insert(name.to_string(), (range, true));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        "identifier" => {
            // Check if this is a usage (not a definition)
            if let Some(parent) = node.parent() {
                if !matches!(parent.kind(), "assignment" | "function_definition" | "parameter_list") {
                    if let Ok(name) = node.utf8_text(text.as_bytes()) {
                        if !utils::is_builtin_or_keyword(name) {
                            usages.insert(name.to_string());
                        }
                    }
                }
            }
        }
        _ => {}
    }
    
    // Recursively process children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_variable_usage(child, text, definitions, usages);
        }
    }
}



