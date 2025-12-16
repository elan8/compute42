use crate::types::{Position, Range};
use tree_sitter::Node;
use std::collections::{HashMap, HashSet};

/// Recursively find and extract parameters from a signature node
/// Structure: signature -> call_expression -> argument_list (parameters are in argument_list, not parameter_list!)
pub(super) fn find_and_extract_parameters(
    node: Node,
    text: &str,
    defined_symbols: &mut HashSet<String>,
    symbol_definitions: &mut HashMap<String, Range>,
    function_params: &mut HashSet<String>,
) {
    // Check if this node is a parameter_list (for some function syntax)
    if node.kind() == "parameter_list" {
        extract_parameters(node, text, defined_symbols, symbol_definitions, function_params);
        return;
    }
    
    // Check if this node is an argument_list (for function definitions, parameters are in argument_list!)
    // But only extract if this argument_list is part of a function signature, not a function call
    if node.kind() == "argument_list" {
        // Check if this argument_list is part of a function signature
        let mut check_parent = node.parent();
        let mut is_in_signature = false;
        while let Some(parent) = check_parent {
            if parent.kind() == "call_expression" {
                // Check if this call_expression is inside a signature
                let mut check_grandparent = parent.parent();
                while let Some(grandparent) = check_grandparent {
                    if grandparent.kind() == "signature" {
                        is_in_signature = true;
                        break;
                    }
                    if grandparent.kind() == "function_definition" {
                        break; // Reached function_definition, not in signature
                    }
                    check_grandparent = grandparent.parent();
                }
                if is_in_signature {
                    break;
                }
            }
            if parent.kind() == "function_definition" {
                break; // Reached function_definition
            }
            check_parent = parent.parent();
        }
        
        if is_in_signature {
            // Extract parameters from argument_list
            // Parameters in argument_list are identifiers or typed_parameter nodes
            for i in 0..node.child_count() {
                if let Some(param) = node.child(i) {
                    match param.kind() {
                        "identifier" => {
                            if let Ok(name) = param.utf8_text(text.as_bytes()) {
                                let param_name = name.to_string();
                                defined_symbols.insert(param_name.clone());
                                function_params.insert(param_name.clone());
                                let range = Range {
                                    start: Position::from(param.start_position()),
                                    end: Position::from(param.end_position()),
                                };
                                symbol_definitions.insert(param_name, range);
                            }
                        },
                    "typed_parameter" => {
                        // Handle typed parameters like x::Int
                        if let Some(ident) = param.child(0) {
                            if ident.kind() == "identifier" {
                                if let Ok(name) = ident.utf8_text(text.as_bytes()) {
                                    let param_name = name.to_string();
                                    defined_symbols.insert(param_name.clone());
                                    function_params.insert(param_name.clone());
                                    let range = Range {
                                        start: Position::from(ident.start_position()),
                                        end: Position::from(ident.end_position()),
                                    };
                                    symbol_definitions.insert(param_name, range);
                                }
                            }
                        }
                    },
                    "typed_expression" => {
                        // Handle typed expressions like df::DataFrame or col::Symbol in function signatures
                        // Structure: typed_expression -> identifier (parameter name) + type annotation
                        // The first child is typically the identifier
                        for j in 0..param.child_count() {
                            if let Some(child) = param.child(j) {
                                if child.kind() == "identifier" {
                                    if let Ok(name) = child.utf8_text(text.as_bytes()) {
                                        let param_name = name.to_string();
                                        defined_symbols.insert(param_name.clone());
                                        function_params.insert(param_name.clone());
                                        let range = Range {
                                            start: Position::from(child.start_position()),
                                            end: Position::from(child.end_position()),
                                        };
                                        symbol_definitions.insert(param_name, range);
                                        break; // Found the identifier, no need to continue
                                    }
                                }
                            }
                        }
                    },
                    "named_argument" => {
                        // Handle default parameter values like k=5 or learning_rate=0.1
                        // Structure: named_argument -> identifier (parameter name) + = + value
                        // The first child is typically the identifier (parameter name)
                        if let Some(lhs) = param.child(0) {
                            if lhs.kind() == "identifier" {
                                if let Ok(name) = lhs.utf8_text(text.as_bytes()) {
                                    let param_name = name.to_string();
                                    defined_symbols.insert(param_name.clone());
                                    function_params.insert(param_name.clone());
                                    let range = Range {
                                        start: Position::from(lhs.start_position()),
                                        end: Position::from(lhs.end_position()),
                                    };
                                    symbol_definitions.insert(param_name, range);
                                }
                            }
                        }
                    },
                    "assignment" => {
                        // Handle default parameter values like k=5 (fallback for some syntax)
                        if let Some(lhs) = param.child(0) {
                            if lhs.kind() == "identifier" {
                                if let Ok(name) = lhs.utf8_text(text.as_bytes()) {
                                    let param_name = name.to_string();
                                    defined_symbols.insert(param_name.clone());
                                    function_params.insert(param_name.clone());
                                    let range = Range {
                                        start: Position::from(lhs.start_position()),
                                        end: Position::from(lhs.end_position()),
                                    };
                                    symbol_definitions.insert(param_name, range);
                                }
                            } else if lhs.kind() == "typed_parameter" {
                                // Handle typed parameter with default: x::Int = 5
                                if let Some(ident) = lhs.child(0) {
                                    if ident.kind() == "identifier" {
                                        if let Ok(name) = ident.utf8_text(text.as_bytes()) {
                                            let param_name = name.to_string();
                                            defined_symbols.insert(param_name.clone());
                                            function_params.insert(param_name.clone());
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
                    },
                        _ => {
                            // Skip commas, operators, etc.
                        }
                    }
                }
            }
            return; // Extracted parameters, done with this argument_list
        } // end if is_in_signature
        // If not in signature, continue searching (might be a function call argument_list)
    }
    
    // Recursively search children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            find_and_extract_parameters(child, text, defined_symbols, symbol_definitions, function_params);
        }
    }
}

/// Extract parameters from a parameter_list node
pub(super) fn extract_parameters(
    param_list: Node,
    text: &str,
    defined_symbols: &mut HashSet<String>,
    symbol_definitions: &mut HashMap<String, Range>,
    function_params: &mut HashSet<String>,
) {
    for i in 0..param_list.child_count() {
        if let Some(param) = param_list.child(i) {
            match param.kind() {
                "identifier" => {
                    if let Ok(name) = param.utf8_text(text.as_bytes()) {
                        let param_name = name.to_string();
                        defined_symbols.insert(param_name.clone());
                        function_params.insert(param_name.clone());
                        let range = Range {
                            start: Position::from(param.start_position()),
                            end: Position::from(param.end_position()),
                        };
                        symbol_definitions.insert(param_name, range);
                    }
                }
                "typed_parameter" => {
                    // Handle typed parameters like x::Int
                    // The identifier is typically the first child
                    if let Some(ident) = param.child(0) {
                        if ident.kind() == "identifier" {
                            if let Ok(name) = ident.utf8_text(text.as_bytes()) {
                                let param_name = name.to_string();
                                defined_symbols.insert(param_name.clone());
                                function_params.insert(param_name.clone());
                                let range = Range {
                                    start: Position::from(ident.start_position()),
                                    end: Position::from(ident.end_position()),
                                };
                                symbol_definitions.insert(param_name, range);
                            }
                        }
                    }
                }
                "assignment" => {
                    // Handle default parameter values like k=5
                    // The left-hand side is the parameter name
                    if let Some(lhs) = param.child(0) {
                        if lhs.kind() == "identifier" {
                            if let Ok(name) = lhs.utf8_text(text.as_bytes()) {
                                let param_name = name.to_string();
                                defined_symbols.insert(param_name.clone());
                                function_params.insert(param_name.clone());
                                let range = Range {
                                    start: Position::from(lhs.start_position()),
                                    end: Position::from(lhs.end_position()),
                                };
                                symbol_definitions.insert(param_name, range);
                            }
                        } else if lhs.kind() == "typed_parameter" {
                            // Handle typed parameter with default: x::Int = 5
                            if let Some(ident) = lhs.child(0) {
                                if ident.kind() == "identifier" {
                                    if let Ok(name) = ident.utf8_text(text.as_bytes()) {
                                        let param_name = name.to_string();
                                        defined_symbols.insert(param_name.clone());
                                        function_params.insert(param_name.clone());
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
                _ => {
                    // Recursively check for nested parameters (e.g., in default values)
                    extract_parameters(param, text, defined_symbols, symbol_definitions, function_params);
                }
            }
        }
    }
}



