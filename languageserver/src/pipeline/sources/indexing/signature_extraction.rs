use crate::types::{FunctionSignature, Parameter, TypeExpr, Range, Position};
use crate::types::LspError;
use tree_sitter::Node;
use super::docstring_extraction::extract_docstring;

/// Extract function signature from function_definition node
pub fn extract_function_signature(
    node: Node,
    source: &str,
    module_name: &str,
    file_uri: &str,
    find_first_child_of_type: &dyn for<'a> Fn(Node<'a>, &'a str) -> Result<Node<'a>, LspError>,
) -> Result<Option<FunctionSignature>, LspError> {
    // Find signature node
    let signature_node = find_first_child_of_type(node, "signature")?;
    
    // Helper function to extract call_expression and name from a node
    fn extract_from_node<'a>(
        node: Node<'a>,
        source: &str,
        find_first_child_of_type: &dyn for<'b> Fn(Node<'b>, &'b str) -> Result<Node<'b>, LspError>,
    ) -> Result<Option<(Node<'a>, String)>, LspError> {
        if let Ok(call_node) = find_first_child_of_type(node, "call_expression") {
            let name = if let Ok(name_node) = find_first_child_of_type(call_node, "identifier") {
                name_node.utf8_text(source.as_bytes())
                    .map_err(|e| LspError::ParseError(format!("Failed to extract function name: {}", e)))?
                    .to_string()
            } else {
                // Try field_access (e.g., Base.getindex, Base.:(==))
                if let Ok(field_node) = find_first_child_of_type(call_node, "field_access") {
                    extract_field_access_name(field_node, source, find_first_child_of_type)?
                } else {
                    // Try field_expression as well (some parsers use this)
                    if let Ok(field_node) = find_first_child_of_type(call_node, "field_expression") {
                        extract_field_access_name(field_node, source, find_first_child_of_type)?
                    } else {
                        // Try parenthesized_expression (for callable types like (cp::ColumnProperties)(...))
                        if let Ok(paren_node) = find_first_child_of_type(call_node, "parenthesized_expression") {
                            // Extract the type name from inside the parentheses
                            // For (cp::ColumnProperties), we want "ColumnProperties"
                            if let Ok(typed_expr) = find_first_child_of_type(paren_node, "typed_expression") {
                                // Get the identifier from the typed_expression
                                if let Ok(type_name_node) = find_first_child_of_type(typed_expr, "identifier") {
                                    type_name_node.utf8_text(source.as_bytes())
                                        .map_err(|e| LspError::ParseError(format!("Failed to extract type name: {}", e)))?
                                        .to_string()
                                } else {
                                    // Fallback: try to extract from typed_expression text
                                    let typed_text = typed_expr.utf8_text(source.as_bytes())
                                        .unwrap_or("")
                                        .trim();
                                    // Extract type name after ::
                                    if let Some(colon_pos) = typed_text.find("::") {
                                        typed_text[colon_pos + 2..].trim().to_string()
                                    } else {
                                        typed_text.to_string()
                                    }
                                }
                            } else {
                                // No typed_expression, try to extract identifier directly
                                if let Ok(id_node) = find_first_child_of_type(paren_node, "identifier") {
                                    id_node.utf8_text(source.as_bytes())
                                        .map_err(|e| LspError::ParseError(format!("Failed to extract identifier: {}", e)))?
                                        .to_string()
                                } else {
                                    // Last resort: extract from parenthesized_expression text
                                    let paren_text = paren_node.utf8_text(source.as_bytes())
                                        .unwrap_or("")
                                        .trim_matches(|c| c == '(' || c == ')');
                                    // Extract type name after ::
                                    if let Some(colon_pos) = paren_text.find("::") {
                                        paren_text[colon_pos + 2..].trim().to_string()
                                    } else {
                                        paren_text.to_string()
                                    }
                                }
                            }
                        } else {
                            // Last resort: try to extract name directly from call_expression text
                            // This handles cases like Base.:(==) where the structure might be different
                            let call_text = call_node.utf8_text(source.as_bytes())
                                .unwrap_or("")
                                .chars()
                                .take(100)
                                .collect::<String>();
                            // Try to extract function name from the call_expression text
                            // For Base.names(...), we want "Base.names"
                            // For Base.:(==)(...), we want "Base.:(==)"
                            if let Some(paren_pos) = call_text.find('(') {
                                let name_part = &call_text[..paren_pos].trim();
                                if !name_part.is_empty() {
                                    name_part.to_string()
                                } else {
                                    return Ok(None);
                                }
                            } else {
                                return Ok(None);
                            }
                        }
                    }
                }
            };
            Ok(Some((call_node, name)))
        } else {
            Ok(None)
        }
    }
    
    // Try to find call_expression first (most common case)
    // Check directly in signature_node
    let (call_node, function_name) = if let Some((call, name)) = extract_from_node(signature_node, source, find_first_child_of_type)? {
        (call, name)
    } else if let Ok(where_expr) = find_first_child_of_type(signature_node, "where_expression") {
        // If signature starts with where_expression, the call_expression is inside it
        if let Some((call, name)) = extract_from_node(where_expr, source, find_first_child_of_type)? {
            (call, name)
        } else {
            // Try to find typed_expression inside where_expression (for return type annotations)
            if let Ok(typed_expr) = find_first_child_of_type(where_expr, "typed_expression") {
                if let Some((call, name)) = extract_from_node(typed_expr, source, find_first_child_of_type)? {
                    (call, name)
                } else {
                    // If no call_expression found, continue to operator/error handling
                    return handle_non_call_signature(signature_node, source, module_name, file_uri, find_first_child_of_type);
                }
            } else {
                // If no call_expression found, continue to operator/error handling
                return handle_non_call_signature(signature_node, source, module_name, file_uri, find_first_child_of_type);
            }
        }
    } else if let Ok(typed_expr) = find_first_child_of_type(signature_node, "typed_expression") {
        // If signature starts with typed_expression (return type annotation), call_expression is inside it
        if let Some((call, name)) = extract_from_node(typed_expr, source, find_first_child_of_type)? {
            (call, name)
        } else {
            // If no call_expression found, continue to operator/error handling
            return handle_non_call_signature(signature_node, source, module_name, file_uri, find_first_child_of_type);
        }
    } else {
        // Try to handle operator-based function definitions and other edge cases
        return handle_non_call_signature(signature_node, source, module_name, file_uri, find_first_child_of_type);
    };
    
    // Extract parameters
    let parameters = extract_parameters(call_node, source, find_first_child_of_type)?;
    
    // Extract return type annotation (function f()::ReturnType)
    let return_type = extract_return_type_annotation(signature_node, source, find_first_child_of_type)?;
    
    // Extract docstring (triple-quoted string before function definition)
    let doc_comment = extract_docstring(node, source);
    
    // Log for joinpath specifically with detailed AST information
    if function_name.contains("joinpath") {
        log::trace!(
            "SignatureExtraction: Extracted joinpath signature: module='{}', name='{}', has_doc={}, doc_len={:?}",
            module_name, function_name, doc_comment.is_some(),
            doc_comment.as_ref().map(|d| d.len())
        );
        
        // Log parent node structure for debugging
        if let Some(parent) = node.parent() {
            let parent_kind = parent.kind();
            let parent_text = parent.utf8_text(source.as_bytes())
                .unwrap_or("")
                .chars()
                .take(200)
                .collect::<String>();
            log::trace!(
                "SignatureExtraction: joinpath parent node: kind='{}', child_count={}, preview='{}'",
                parent_kind, parent.child_count(), parent_text
            );
            
            // Log siblings
            let mut siblings = Vec::new();
            for i in 0..parent.child_count() {
                if let Some(sibling) = parent.child(i) {
                    let sibling_kind = sibling.kind();
                    let sibling_text = sibling.utf8_text(source.as_bytes())
                        .unwrap_or("")
                        .chars()
                        .take(100)
                        .collect::<String>();
                    let is_self = sibling.id() == node.id();
                    siblings.push(format!("{}: '{}'{}", sibling_kind, sibling_text, if is_self { " [SELF]" } else { "" }));
                }
            }
            log::trace!("SignatureExtraction: joinpath siblings: {:?}", siblings);
        }
    }
    
    let range = node_to_range(node);
    
    Ok(Some(FunctionSignature {
        module: module_name.to_string(),
        name: function_name,
        parameters,
        return_type,
        doc_comment,
        file_uri: file_uri.to_string(),
        range,
    }))
}

/// Handle function signatures that don't have a call_expression directly
/// This includes operator functions, where_expression-only signatures, etc.
fn handle_non_call_signature(
    signature_node: Node,
    source: &str,
    module_name: &str,
    file_uri: &str,
    find_first_child_of_type: &dyn for<'a> Fn(Node<'a>, &'a str) -> Result<Node<'a>, LspError>,
) -> Result<Option<FunctionSignature>, LspError> {
    // Check for function declarations without body (e.g., "function detect end")
    // These have just an identifier as a child
    if let Ok(id_node) = find_first_child_of_type(signature_node, "identifier") {
        if let Ok(name) = id_node.utf8_text(source.as_bytes()) {
            let range = node_to_range(signature_node);
            return Ok(Some(FunctionSignature {
                module: module_name.to_string(),
                name: name.to_string(),
                parameters: Vec::new(),
                return_type: None,
                doc_comment: None,
                file_uri: file_uri.to_string(),
                range,
            }));
        }
    }
    
    // Check for anonymous functions (e.g., "function(x...)" or "function()")
    // These have just an argument_list as a child
    if let Ok(arg_list) = find_first_child_of_type(signature_node, "argument_list") {
        let parameters = extract_parameters_from_argument_list(arg_list, source, find_first_child_of_type)?;
        let range = node_to_range(signature_node);
        return Ok(Some(FunctionSignature {
            module: module_name.to_string(),
            name: "<anonymous>".to_string(),
            parameters,
            return_type: None,
            doc_comment: None,
            file_uri: file_uri.to_string(),
            range,
        }));
    }
    
    // Try to handle operator-based function definitions (e.g., function +(a, b) end)
    // Look for binary_operator or operator nodes in the signature
    let mut operator_name = None;
    let mut argument_list = None;
    
    // Search in signature_node and its children (where_expression, typed_expression, etc.)
    let mut nodes_to_search = vec![signature_node];
    
    // Also check inside where_expression if present
    if let Ok(where_expr) = find_first_child_of_type(signature_node, "where_expression") {
        nodes_to_search.push(where_expr);
    }
    
    // Also check inside typed_expression if present
    if let Ok(typed_expr) = find_first_child_of_type(signature_node, "typed_expression") {
        nodes_to_search.push(typed_expr);
    }
    
    for node in nodes_to_search {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "binary_operator" | "operator" => {
                        // Extract operator name
                        if let Ok(op_text) = child.utf8_text(source.as_bytes()) {
                            operator_name = Some(op_text.to_string());
                        }
                    }
                    "argument_list" => {
                        argument_list = Some(child);
                    }
                    _ => {}
                }
            }
        }
    }
    
    if let Some(name) = operator_name {
        // For operator functions, we need to construct a pseudo-call_node structure
        // We'll extract parameters from the argument_list if available
        let parameters = if let Some(arg_list) = argument_list {
            extract_parameters_from_argument_list(arg_list, source, find_first_child_of_type)?
        } else {
            Vec::new()
        };
        
        let range = node_to_range(signature_node);
        return Ok(Some(FunctionSignature {
            module: module_name.to_string(),
            name,
            parameters,
            return_type: extract_return_type_annotation(signature_node, source, find_first_child_of_type)?,
            doc_comment: None,
            file_uri: file_uri.to_string(),
            range,
        }));
    }
    
    // If we can't find call_expression or operator, return error to log warning
    let children: Vec<String> = (0..signature_node.child_count())
        .filter_map(|i| {
            signature_node.child(i).map(|c| {
                let kind = c.kind();
                let text = c.utf8_text(source.as_bytes())
                    .unwrap_or("")
                    .chars()
                    .take(50)
                    .collect::<String>();
                format!("{}: '{}'", kind, text)
            })
        })
        .collect();
    
    let signature_text = signature_node.utf8_text(source.as_bytes())
        .unwrap_or("")
        .chars()
        .take(100)
        .collect::<String>();
    
    Err(LspError::ParseError(
        format!(
            "No call_expression or operator found in function signature. Signature node kind: {}, children: [{}], signature text: '{}'",
            signature_node.kind(),
            children.join(", "),
            signature_text
        )
    ))
}

/// Extract parameters directly from an argument_list node
/// Used for operator-based function definitions
fn extract_parameters_from_argument_list(
    arg_list: Node,
    source: &str,
    find_first_child_of_type: &dyn for<'a> Fn(Node<'a>, &'a str) -> Result<Node<'a>, LspError>,
) -> Result<Vec<Parameter>, LspError> {
    let mut parameters = Vec::new();
    
    for i in 0..arg_list.child_count() {
        if let Some(child) = arg_list.child(i) {
            match child.kind() {
                "identifier" => {
                    let name = child.utf8_text(source.as_bytes())
                        .map_err(|e| LspError::ParseError(format!("Failed to extract parameter name: {}", e)))?
                        .to_string();
                    
                    parameters.push(Parameter {
                        name,
                        param_type: None,
                    });
                }
                "typed_expression" => {
                    // Parameter with type annotation: x::Int64
                    // Skip if identifier cannot be extracted (some typed expressions may have complex structures)
                    if let Ok(name) = extract_typed_expression_identifier(child, source, find_first_child_of_type) {
                        let param_type = extract_type_from_typed_expression(child, source, find_first_child_of_type)?;
                        
                        parameters.push(Parameter {
                            name,
                            param_type,
                        });
                    }
                    // Silently skip invalid typed_expression parameters
                }
                _ => {
                    // Skip other nodes like parentheses, commas, etc.
                }
            }
        }
    }
    
    Ok(parameters)
}

/// Extract parameters from call_expression node
fn extract_parameters(
    call_node: Node,
    source: &str,
    find_first_child_of_type: &dyn for<'a> Fn(Node<'a>, &'a str) -> Result<Node<'a>, LspError>,
) -> Result<Vec<Parameter>, LspError> {
    let mut parameters = Vec::new();
    
    // Find argument_list
    if let Ok(arg_list) = find_first_child_of_type(call_node, "argument_list") {
        for i in 0..arg_list.child_count() {
            if let Some(child) = arg_list.child(i) {
                match child.kind() {
                    "identifier" => {
                        let name = child.utf8_text(source.as_bytes())
                            .map_err(|e| LspError::ParseError(format!("Failed to extract parameter name: {}", e)))?
                            .to_string();
                        
                        parameters.push(Parameter {
                            name,
                            param_type: None,
                        });
                    }
                    "typed_expression" => {
                        // Parameter with type annotation: x::Int64
                        // Skip if identifier cannot be extracted (some typed expressions may have complex structures)
                        if let Ok(name) = extract_typed_expression_identifier(child, source, find_first_child_of_type) {
                            let param_type = extract_type_from_typed_expression(child, source, find_first_child_of_type)?;
                            
                            parameters.push(Parameter {
                                name,
                                param_type,
                            });
                        }
                        // Silently skip invalid typed_expression parameters
                    }
                    _ => {
                        // Skip other nodes like parentheses, commas, etc.
                    }
                }
            }
        }
    }
    
    Ok(parameters)
}

/// Extract identifier from typed_expression node (x from x::Int64)
fn extract_typed_expression_identifier(
    typed_expr: Node,
    source: &str,
    find_first_child_of_type: &dyn for<'a> Fn(Node<'a>, &'a str) -> Result<Node<'a>, LspError>,
) -> Result<String, LspError> {
    // The first child should be the identifier
    if let Ok(ident_node) = find_first_child_of_type(typed_expr, "identifier") {
        ident_node.utf8_text(source.as_bytes())
            .map_err(|e| LspError::ParseError(format!("Failed to extract identifier: {}", e)))
            .map(|s| s.to_string())
    } else {
        // Log detailed information about the typed_expression structure
        let children: Vec<String> = (0..typed_expr.child_count())
            .filter_map(|i| {
                typed_expr.child(i).map(|c| {
                    let kind = c.kind();
                    let text = c.utf8_text(source.as_bytes())
                        .unwrap_or("")
                        .chars()
                        .take(50)
                        .collect::<String>();
                    format!("{}: '{}'", kind, text)
                })
            })
            .collect();
        
        let typed_expr_text = typed_expr.utf8_text(source.as_bytes())
            .unwrap_or("")
            .chars()
            .take(100)
            .collect::<String>();
        
        Err(LspError::ParseError(
            format!(
                "No identifier found in typed_expression. Children: [{}], text: '{}'",
                children.join(", "),
                typed_expr_text
            )
        ))
    }
}

/// Extract type from typed_expression node (x::Int64 -> Int64)
fn extract_type_from_typed_expression(
    typed_expr: Node,
    source: &str,
    _find_first_child_of_type: &dyn for<'a> Fn(Node<'a>, &'a str) -> Result<Node<'a>, LspError>,
) -> Result<Option<TypeExpr>, LspError> {
    // Walk children to find the type part (after ::)
    let mut found_colon_colon = false;
    for i in 0..typed_expr.child_count() {
        if let Some(child) = typed_expr.child(i) {
            if child.kind() == "::" {
                found_colon_colon = true;
                continue;
            }
            if found_colon_colon {
                // This is the type expression - parse it locally
                if let Some(type_expr) = parse_type_expression(child, source) {
                    return Ok(Some(type_expr));
                }
            }
        }
    }
    Ok(None)
}

/// Extract return type annotation (function f()::ReturnType)
fn extract_return_type_annotation(
    signature_node: Node,
    source: &str,
    find_first_child_of_type: &dyn for<'a> Fn(Node<'a>, &'a str) -> Result<Node<'a>, LspError>,
) -> Result<Option<TypeExpr>, LspError> {
    // Check if signature is a typed_expression (function f()::ReturnType)
    if signature_node.kind() == "typed_expression" {
        return extract_type_from_typed_expression(signature_node, source, find_first_child_of_type);
    }
    
    // Otherwise, no return type annotation
    Ok(None)
}

/// Extract field access name (e.g., CSV.read -> "CSV.read", Base.:(==) -> "Base.:(==)")
fn extract_field_access_name(
    node: Node,
    source: &str,
    find_first_child_of_type: &dyn for<'a> Fn(Node<'a>, &'a str) -> Result<Node<'a>, LspError>,
) -> Result<String, LspError> {
    let mut parts = Vec::new();
    let mut current = Some(node);
    
    while let Some(n) = current {
        match n.kind() {
            "field_access" | "field_expression" => {
                // Get the field name (rightmost part)
                // For Base.:(==), the rightmost part might be an operator or parenthesized expression
                if let Some(field_node) = n.child(n.child_count().saturating_sub(1)) {
                    match field_node.kind() {
                        "identifier" => {
                            if let Ok(field_name) = field_node.utf8_text(source.as_bytes()) {
                                parts.push(field_name.to_string());
                            }
                        }
                        "operator" | "binary_operator" => {
                            // Handle operators like :(==)
                            if let Ok(op_text) = field_node.utf8_text(source.as_bytes()) {
                                parts.push(op_text.to_string());
                            }
                        }
                        "parenthesized_expression" | "call_expression" => {
                            // For :(==), extract the operator from inside
                            if let Ok(inner_text) = field_node.utf8_text(source.as_bytes()) {
                                parts.push(inner_text.trim_matches(|c| c == '(' || c == ')').to_string());
                            } else {
                                // Try to find operator inside
                                if let Ok(op_node) = find_first_child_of_type(field_node, "operator") {
                                    if let Ok(op_text) = op_node.utf8_text(source.as_bytes()) {
                                        parts.push(format!(":({})", op_text));
                                    }
                                }
                            }
                        }
                        _ => {
                            // Try to extract text directly
                            if let Ok(field_text) = field_node.utf8_text(source.as_bytes()) {
                                parts.push(field_text.to_string());
                            }
                        }
                    }
                }
                // Get the object (left part)
                current = n.child(0);
            }
            "identifier" => {
                if let Ok(name) = n.utf8_text(source.as_bytes()) {
                    parts.push(name.to_string());
                }
                break;
            }
            _ => break,
        }
    }
    
    parts.reverse();
    let result = parts.join(".");
    if result.is_empty() {
        Err(LspError::ParseError("Failed to extract field access name".to_string()))
    } else {
        Ok(result)
    }
}

/// Extract type annotation from a type_annotation node
#[allow(dead_code)]
fn extract_type_annotation(node: Node, text: &str) -> Option<TypeExpr> {
    // Find the type expression within the annotation
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "type_expression" || child.kind() == "identifier" {
                return parse_type_expression(child, text);
            }
        }
    }
    None
}

/// Parse a type expression (identifier, curly_expression, etc.)
fn parse_type_expression(node: Node, text: &str) -> Option<TypeExpr> {
    match node.kind() {
        "identifier" => {
            if let Ok(name) = node.utf8_text(text.as_bytes()) {
                return Some(TypeExpr::Concrete(name.to_string()));
            }
            None
        }
        "curly_expression" => {
            // Handle Vector{Int64}, Union{Int64, Missing}, etc.
            // First child is the base type name
            if let Some(base_node) = node.child(0) {
                if let Ok(base_name) = base_node.utf8_text(text.as_bytes()) {
                    // Collect type parameters
                    let mut params = Vec::new();
                    for i in 1..node.child_count() {
                        if let Some(child) = node.child(i) {
                            if child.kind() != "{" && child.kind() != "}" && child.kind() != "," {
                                if let Some(param_type) = parse_type_expression(child, text) {
                                    params.push(param_type);
                                }
                            }
                        }
                    }
                    
                    if base_name == "Union" {
                        return Some(TypeExpr::Union(params));
                    } else if !params.is_empty() {
                        return Some(TypeExpr::Generic(base_name.to_string(), params));
                    } else {
                        return Some(TypeExpr::Concrete(base_name.to_string()));
                    }
                }
            }
            None
        }
        "parametrized_type_expression" => {
            // Similar to curly_expression but different AST structure
            if let Some(base_node) = find_child_by_kind_for_type_parsing(node, "identifier") {
                if let Ok(base_name) = base_node.utf8_text(text.as_bytes()) {
                    let mut params = Vec::new();
                    // Look for type parameters in children
                    for i in 0..node.child_count() {
                        if let Some(child) = node.child(i) {
                            if child.kind() == "type_expression" || child.kind() == "identifier" {
                                if let Some(param_type) = parse_type_expression(child, text) {
                                    params.push(param_type);
                                }
                            }
                        }
                    }
                    
                    if base_name == "Union" {
                        return Some(TypeExpr::Union(params));
                    } else if !params.is_empty() {
                        return Some(TypeExpr::Generic(base_name.to_string(), params));
                    } else {
                        return Some(TypeExpr::Concrete(base_name.to_string()));
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Find first child node of a specific kind (helper for type parsing)
fn find_child_by_kind_for_type_parsing<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
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
