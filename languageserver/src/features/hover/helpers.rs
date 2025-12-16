use tree_sitter::Node;

/// Extract the value/expression from an assignment node
pub fn extract_assignment_value(assignment_node: Node, text: &str) -> Option<String> {
    // Find the right-hand side of the assignment (after the operator)
    let mut found_operator = false;
    for i in 0..assignment_node.child_count() {
        if let Some(child) = assignment_node.child(i) {
            if child.kind() == "operator" && child.utf8_text(text.as_bytes()).unwrap_or("") == "=" {
                found_operator = true;
                continue;
            }
            if found_operator {
                // This is the right-hand side
                let value = child.utf8_text(text.as_bytes()).unwrap_or("");
                // Truncate very long expressions
                if value.len() > 100 {
                    return Some(format!("{}...", &value[..97]));
                }
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Search earlier siblings and ancestors within the current lexical context
/// for a prior assignment to the given symbol name and extract its RHS value.
pub fn find_prior_assignment_in_scope(node: Node, text: &str, symbol_name: &str) -> Option<String> {
    // Walk up to a reasonable scope boundary (function/module)
    let mut current = node;
    while let Some(parent) = current.parent() {
        // In the parent, scan children that come before `current`
        let mut idx_of_current = None;
        for i in 0..parent.child_count() {
            if let Some(ch) = parent.child(i) {
                if ch.id() == current.id() {
                    idx_of_current = Some(i);
                    break;
                }
            }
        }
        if let Some(limit) = idx_of_current {
            let mut i = limit as i32 - 1;
            while i >= 0 {
                if let Some(sibling) = parent.child(i as usize) {
                    if sibling.kind() == "assignment" {
                        // Check LHS identifier matches symbol_name
                        if let Some(lhs_id) = sibling.child(0) {
                            if lhs_id.kind() == "identifier" {
                                if let Ok(lhs_name) = lhs_id.utf8_text(text.as_bytes()) {
                                    if lhs_name == symbol_name {
                                        if let Some(rhs) = extract_assignment_value(sibling, text) {
                                            return Some(rhs);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                i -= 1;
            }
        }

        // Stop at typical scope boundaries
        match parent.kind() {
            "function_definition" | "module_definition" => break,
            _ => {}
        }
        current = parent;
    }
    None
}

/// Extract assignment information for a variable
pub fn extract_assignment_info(node: Node, _tree: &tree_sitter::Tree, text: &str) -> Option<String> {
    // Walk up the tree to find the assignment node
    let mut current = node;
    while let Some(parent) = current.parent() {
        if parent.kind() == "assignment" {
            // Found assignment, extract the right-hand side
            return extract_assignment_value(parent, text);
        }
        current = parent;
    }
    None
}

/// Find the assignment node that defines a given variable symbol by matching
/// the identifier occurrence at the symbol's definition range.
pub fn find_definition_assignment_node<'a>(
    tree: &'a tree_sitter::Tree,
    text: &str,
    symbol: &crate::types::Symbol,
) -> Option<Node<'a>> {
    let root = tree.root_node();
    // Walk and look for an assignment whose first child identifier matches name and range
    fn walk<'b>(node: Node<'b>, text: &str, sym: &crate::types::Symbol) -> Option<Node<'b>> {
        // Check assignment nodes
        if node.kind() == "assignment" {
            if let Some(lhs) = node.child(0) {
                if lhs.kind() == "identifier" {
                    if let Ok(name) = lhs.utf8_text(text.as_bytes()) {
                        if name == sym.name {
                            // Compare ranges
                            let lhs_range = crate::types::Range {
                                start: crate::types::Position::from(lhs.start_position()),
                                end: crate::types::Position::from(lhs.end_position()),
                            };
                            if lhs_range == sym.range {
                                return Some(node);
                            }
                        }
                    }
                }
            }
        }
        // Recurse
        for i in 0..node.child_count() {
            if let Some(ch) = node.child(i) {
                if let Some(found) = walk(ch, text, sym) {
                    return Some(found);
                }
            }
        }
        None
    }
    walk(root, text, symbol)
}

/// Check if a node is part of a function call
/// Returns true if the node is an identifier that is part of a call_expression
pub fn is_function_call(node: Node) -> bool {
    // Check if this node is an identifier
    if node.kind() != "identifier" {
        return false;
    }
    
    // Walk up the tree to find if we're inside a call_expression
    let mut current = node;
    while let Some(parent) = current.parent() {
        match parent.kind() {
            "call_expression" => {
                // Check if this identifier is the function name (first child)
                if let Some(first_child) = parent.child(0) {
                    if first_child.id() == node.id() {
                        return true;
                    }
                    // Or if it's part of a field_access that's the function name
                    if first_child.kind() == "field_access" || first_child.kind() == "field_expression" {
                        // Check if our node is part of this field_access
                        fn is_node_in_tree(root: Node, target_id: usize) -> bool {
                            if root.id() == target_id {
                                return true;
                            }
                            for i in 0..root.child_count() {
                                if let Some(child) = root.child(i) {
                                    if is_node_in_tree(child, target_id) {
                                        return true;
                                    }
                                }
                            }
                            false
                        }
                        if is_node_in_tree(first_child, node.id()) {
                            return true;
                        }
                    }
                }
            }
            "assignment" => {
                // If we hit an assignment before a call, check if we're on the LHS
                // If so, it's not a function call (it's a variable being assigned)
                if let Some(lhs) = parent.child(0) {
                    fn is_node_in_tree(root: Node, target_id: usize) -> bool {
                        if root.id() == target_id {
                            return true;
                        }
                        for i in 0..root.child_count() {
                            if let Some(child) = root.child(i) {
                                if is_node_in_tree(child, target_id) {
                                    return true;
                                }
                            }
                        }
                        false
                    }
                    if is_node_in_tree(lhs, node.id()) {
                        return false;
                    }
                }
                // If we're on the RHS and there's a call_expression above, continue checking
            }
            _ => {}
        }
        current = parent;
    }
    
    false
}

/// Convert value-like string to type (e.g., "= 123" -> "Int64")
pub fn type_of_value_like(s: &str) -> Option<String> {
    // Convert "= 123" -> ::Int64; "= \"abc\"" -> ::String; best-effort
    let trimmed = s.trim();
    if !trimmed.starts_with('=') {
        return None;
    }
    let rhs = trimmed.trim_start_matches('=').trim();
    if rhs.starts_with('"') && rhs.ends_with('"') {
        return Some("String".to_string());
    }
    if rhs.parse::<i64>().is_ok() {
        return Some("Int64".to_string());
    }
    if rhs.parse::<f64>().is_ok() {
        return Some("Float64".to_string());
    }
    if rhs == "true" || rhs == "false" {
        return Some("Bool".to_string());
    }
    None
}

















