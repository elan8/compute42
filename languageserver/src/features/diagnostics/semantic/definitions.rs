use crate::pipeline::storage::Index;
use crate::types::{Position, Range};
use tree_sitter::Node;
use std::collections::{HashMap, HashSet};

use super::parameters;

/// Collect all symbol definitions from the tree
pub(super) fn collect_definitions(
    node: Node,
    text: &str,
    index: &Index,
    defined_symbols: &mut HashSet<String>,
    symbol_definitions: &mut HashMap<String, Range>,
    function_scopes: &mut HashMap<usize, HashSet<String>>,
) {
    match node.kind() {
        "assignment" => {
            if let Some(lhs) = node.child(0) {
                match lhs.kind() {
                    "identifier" => {
                        // Simple assignment: x = ...
                        if let Ok(name) = lhs.utf8_text(text.as_bytes()) {
                            defined_symbols.insert(name.to_string());
                            let range = Range {
                                start: Position::from(lhs.start_position()),
                                end: Position::from(lhs.end_position()),
                            };
                            symbol_definitions.insert(name.to_string(), range);
                        }
                    }
                    "call_expression" => {
                        // Short function syntax: f(x, y) = ...
                        // Extract function name and parameters
                        // Track parameters in function_scopes using assignment node ID
                        let mut short_func_params = HashSet::new();
                        
                        if let Some(func_name_node) = lhs.child(0) {
                            if func_name_node.kind() == "identifier" {
                                if let Ok(name) = func_name_node.utf8_text(text.as_bytes()) {
                                    defined_symbols.insert(name.to_string());
                                    let range = Range {
                                        start: Position::from(func_name_node.start_position()),
                                        end: Position::from(func_name_node.end_position()),
                                    };
                                    symbol_definitions.insert(name.to_string(), range);
                                }
                            }
                        }
                        // Extract parameters from argument_list
                        for j in 0..lhs.child_count() {
                            if let Some(arg_list) = lhs.child(j) {
                                if arg_list.kind() == "argument_list" {
                                    // Extract parameters from argument_list
                                    for k in 0..arg_list.child_count() {
                                        if let Some(param) = arg_list.child(k) {
                                            if param.kind() == "identifier" {
                                                if let Ok(name) = param.utf8_text(text.as_bytes()) {
                                                    let param_name = name.to_string();
                                                    defined_symbols.insert(param_name.clone());
                                                    short_func_params.insert(param_name.clone());
                                                    let range = Range {
                                                        start: Position::from(param.start_position()),
                                                        end: Position::from(param.end_position()),
                                                    };
                                                    symbol_definitions.insert(param_name, range);
                                                }
                                            } else if param.kind() == "typed_parameter" {
                                                // Handle typed parameters like x::Int
                                                if let Some(ident) = param.child(0) {
                                                    if ident.kind() == "identifier" {
                                                        if let Ok(name) = ident.utf8_text(text.as_bytes()) {
                                                            let param_name = name.to_string();
                                                            defined_symbols.insert(param_name.clone());
                                                            short_func_params.insert(param_name.clone());
                                                            let range = Range {
                                                                start: Position::from(ident.start_position()),
                                                                end: Position::from(ident.end_position()),
                                                            };
                                                            symbol_definitions.insert(param_name, range);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Store function scope for short function syntax using assignment node ID
                        if !short_func_params.is_empty() {
                            function_scopes.insert(node.id(), short_func_params);
                        }
                    }
                    "tuple_expression" | "parenthesized_expression" => {
                        // Tuple assignment: a, b = ... or (a, b) = ...
                        extract_identifiers_from_tuple(lhs, text, defined_symbols, symbol_definitions);
                    }
                    _ => {
                        // Handle comma-separated identifiers in assignments like: lr_W, lr_b = ...
                        // Tree-sitter might parse this differently - the lhs might have multiple identifier children
                        // separated by commas, or the assignment node might have identifiers before the = operator
                        // Try extracting identifiers from lhs children first (handles comma-separated list)
                        extract_identifiers_from_children(lhs, text, defined_symbols, symbol_definitions);
                    }
                }
            }
        }
        "function_definition" => {
            // Track function parameters in a scope for this function
            let mut function_params = HashSet::new();
            
            // Extract function name and parameters
            // First, find the function name (identifier)
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "identifier" {
                        if let Ok(name) = child.utf8_text(text.as_bytes()) {
                            defined_symbols.insert(name.to_string());
                            let range = Range {
                                start: Position::from(child.start_position()),
                                end: Position::from(child.end_position()),
                            };
                            symbol_definitions.insert(name.to_string(), range);
                        }
                        break;
                    }
                }
            }
            
            // Now find parameter_list - it's inside a signature node
            // Structure: function_definition -> signature -> (call_expression or identifier) -> parameter_list
            // Or: function_definition -> signature -> parameter_list (direct)
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "signature" {
                        // Recursively search for parameter_list within signature
                        parameters::find_and_extract_parameters(child, text, defined_symbols, symbol_definitions, &mut function_params);
                        break; // Found signature, no need to continue
                    }
                }
            }
            
            // Store function scope (parameters) for this function node
            if !function_params.is_empty() {
                function_scopes.insert(node.id(), function_params);
            }
            // Also handle short function syntax: f(x, y) = ...
            // In this case, the function name is in a call_expression on the left side of assignment
            // Structure: assignment -> call_expression -> identifier (function name) + argument_list (parameters)
            let mut short_func_params = HashSet::new();
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "call_expression" {
                        // Extract function name and parameters from call_expression
                        if let Some(func_name_node) = child.child(0) {
                            if func_name_node.kind() == "identifier" {
                                if let Ok(name) = func_name_node.utf8_text(text.as_bytes()) {
                                    defined_symbols.insert(name.to_string());
                                    let range = Range {
                                        start: Position::from(func_name_node.start_position()),
                                        end: Position::from(func_name_node.end_position()),
                                    };
                                    symbol_definitions.insert(name.to_string(), range);
                                }
                            }
                        }
                        // Find argument_list in the call_expression (this contains the parameters)
                        for j in 0..child.child_count() {
                            if let Some(arg_list) = child.child(j) {
                                if arg_list.kind() == "argument_list" {
                                    // Extract parameters from argument_list
                                    // Parameters are identifiers in the argument_list (not in a parameter_list node)
                                    for k in 0..arg_list.child_count() {
                                        if let Some(param) = arg_list.child(k) {
                                            if param.kind() == "identifier" {
                                                if let Ok(name) = param.utf8_text(text.as_bytes()) {
                                                    let param_name = name.to_string();
                                                    defined_symbols.insert(param_name.clone());
                                                    short_func_params.insert(param_name.clone());
                                                    let range = Range {
                                                        start: Position::from(param.start_position()),
                                                        end: Position::from(param.end_position()),
                                                    };
                                                    symbol_definitions.insert(param_name, range);
                                                }
                                            } else if param.kind() == "typed_parameter" {
                                                // Handle typed parameters like x::Int
                                                if let Some(ident) = param.child(0) {
                                                    if ident.kind() == "identifier" {
                                                        if let Ok(name) = ident.utf8_text(text.as_bytes()) {
                                                            let param_name = name.to_string();
                                                            defined_symbols.insert(param_name.clone());
                                                            short_func_params.insert(param_name.clone());
                                                            let range = Range {
                                                                start: Position::from(ident.start_position()),
                                                                end: Position::from(ident.end_position()),
                                                            };
                                                            symbol_definitions.insert(param_name, range);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Store function scope for short function syntax
            if !short_func_params.is_empty() {
                function_scopes.insert(node.id(), short_func_params);
            }
        }
        "for_statement" => {
            // Extract loop variables from for loops
            // Handle: for i in 1:10
            // Handle: for (i, idx) in enumerate(...)
            // Handle: for (model_name, (train_acc, val_acc)) in ...
            // Structure: for_statement -> for_binding -> (identifier or tuple_expression)
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "for_binding" {
                        // Extract variables from for_binding
                        // The first child of for_binding is the loop variable(s)
                        for j in 0..child.child_count() {
                            if let Some(var_node) = child.child(j) {
                                if var_node.kind() == "identifier" {
                                    // Simple case: for i in ...
                                    if let Ok(name) = var_node.utf8_text(text.as_bytes()) {
                                        defined_symbols.insert(name.to_string());
                                        let range = Range {
                                            start: Position::from(var_node.start_position()),
                                            end: Position::from(var_node.end_position()),
                                        };
                                        symbol_definitions.insert(name.to_string(), range);
                                    }
                                } else if var_node.kind() == "tuple_expression" || var_node.kind() == "parenthesized_expression" {
                                    // Tuple case: for (i, idx) in ... or for (a, (b, c)) in ...
                                    // Use helper function to handle nested tuples
                                    extract_identifiers_from_tuple(var_node, text, defined_symbols, symbol_definitions);
                                }
                                // Stop when we hit "in" operator
                                if var_node.kind() == "operator" {
                                    if let Ok(op) = var_node.utf8_text(text.as_bytes()) {
                                        if op == "in" {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
        "generator" | "comprehension" | "comprehension_expression" => {
            // Extract variables from generator expressions
            // Handle: [x for x in 1:10]
            // Handle: Dict(species => i for (i, species) in enumerate(...))
            // Structure: comprehension -> for_clause -> for_binding -> (identifier or tuple_expression)
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    // Look for the "for" part which contains the loop variables
                    if child.kind() == "for_clause" || child.kind() == "for" {
                        // Find for_binding within for_clause
                        for j in 0..child.child_count() {
                            if let Some(binding_node) = child.child(j) {
                                if binding_node.kind() == "for_binding" {
                                    // Extract variables from for_binding
                                    // The first child of for_binding is the loop variable(s)
                                    for k in 0..binding_node.child_count() {
                                        if let Some(var_node) = binding_node.child(k) {
                                            if var_node.kind() == "identifier" {
                                                if let Ok(name) = var_node.utf8_text(text.as_bytes()) {
                                                    defined_symbols.insert(name.to_string());
                                                    let range = Range {
                                                        start: Position::from(var_node.start_position()),
                                                        end: Position::from(var_node.end_position()),
                                                    };
                                                    symbol_definitions.insert(name.to_string(), range);
                                                }
                                            } else if var_node.kind() == "tuple_expression" || var_node.kind() == "parenthesized_expression" {
                                                // Tuple case: (i, species) in ... or (a, (b, c)) in ...
                                                // Use helper function to handle nested tuples
                                                extract_identifiers_from_tuple(var_node, text, defined_symbols, symbol_definitions);
                                            }
                                            // Stop when we hit "in" operator
                                            if var_node.kind() == "operator" {
                                                if let Ok(op) = var_node.utf8_text(text.as_bytes()) {
                                                    if op == "in" {
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        "using_statement" | "import_statement" => {
            // When we see a using/import statement, we should mark that module as available
            // For now, we'll just skip checking identifiers that might come from these modules
            // by tracking which modules are imported
            // This is a simplified approach - in a full implementation, we'd track
            // which symbols are exported by each imported module
        }
        _ => {}
    }
    
    // Recursively process children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_definitions(child, text, index, defined_symbols, symbol_definitions, function_scopes);
        }
    }
}

/// Extract identifiers from a tuple expression (handles nested tuples)
pub(super) fn extract_identifiers_from_tuple(
    tuple_node: Node,
    text: &str,
    defined_symbols: &mut HashSet<String>,
    symbol_definitions: &mut HashMap<String, Range>,
) {
    for i in 0..tuple_node.child_count() {
        if let Some(child) = tuple_node.child(i) {
            match child.kind() {
                "identifier" => {
                    if let Ok(name) = child.utf8_text(text.as_bytes()) {
                        defined_symbols.insert(name.to_string());
                        let range = Range {
                            start: Position::from(child.start_position()),
                            end: Position::from(child.end_position()),
                        };
                        symbol_definitions.insert(name.to_string(), range);
                    }
                }
                "tuple_expression" | "parenthesized_expression" => {
                    // Recursively handle nested tuples: (a, (b, c))
                    extract_identifiers_from_tuple(child, text, defined_symbols, symbol_definitions);
                }
                _ => {
                    // Skip commas, operators, etc.
                }
            }
        }
    }
}

/// Extract identifiers from children of a node (handles comma-separated lists)
/// This is used as a fallback when tree-sitter doesn't create a tuple_expression node
pub(super) fn extract_identifiers_from_children(
    node: Node,
    text: &str,
    defined_symbols: &mut HashSet<String>,
    symbol_definitions: &mut HashMap<String, Range>,
) {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "identifier" => {
                    if let Ok(name) = child.utf8_text(text.as_bytes()) {
                        defined_symbols.insert(name.to_string());
                        let range = Range {
                            start: Position::from(child.start_position()),
                            end: Position::from(child.end_position()),
                        };
                        symbol_definitions.insert(name.to_string(), range);
                    }
                }
                "tuple_expression" | "parenthesized_expression" => {
                    // Recursively handle nested tuples
                    extract_identifiers_from_tuple(child, text, defined_symbols, symbol_definitions);
                }
                _ => {
                    // Skip commas, operators, etc., but recurse into other nodes
                    // This handles cases where identifiers might be nested
                    if !matches!(child.kind(), "," | "operator" | "=") {
                        extract_identifiers_from_children(child, text, defined_symbols, symbol_definitions);
                    }
                }
            }
        }
    }
}


