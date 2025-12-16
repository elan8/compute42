use tree_sitter::Node;

/// Debug helper: Dump CST structure for function definitions
#[cfg(debug_assertions)]
#[allow(dead_code)]
pub(super) fn dump_function_definitions_cst(node: Node, text: &str) -> String {
    let mut output = String::new();
    output.push_str("=== Function Definitions CST Dump ===\n\n");
    dump_function_definitions_recursive(node, text, 0, &mut output);
    output
}

/// Recursively dump function definitions
#[cfg(debug_assertions)]
#[allow(dead_code)]
pub(super) fn dump_function_definitions_recursive(node: Node, text: &str, depth: usize, output: &mut String) {
    let indent = "  ".repeat(depth);
    let kind = node.kind();
    let start = node.start_position();
    let end = node.end_position();
    
    // Dump function_definition nodes and ALL their children in detail
    if kind == "function_definition" {
        let node_text = node.utf8_text(text.as_bytes()).unwrap_or("");
        let preview = if node_text.len() > 80 {
            format!("{}...", &node_text[..80])
        } else {
            node_text.to_string()
        };
        
        output.push_str(&format!(
            "{}[{}] {} [{}:{} - {}:{}]",
            indent, node.id(), kind, start.row, start.column, end.row, end.column
        ));
        if !preview.is_empty() {
            output.push_str(&format!(": {}", preview.replace('\n', "\\n")));
        }
        output.push_str(&format!(" ({} children)", node.child_count()));
        output.push('\n');
        
        // Dump ALL children with their kinds
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let child_kind = child.kind();
                let child_start = child.start_position();
                let child_end = child.end_position();
                let child_text = child.utf8_text(text.as_bytes()).unwrap_or("");
                let child_preview = if child_text.len() > 60 {
                    format!("{}...", &child_text[..60])
                } else {
                    child_text.to_string()
                };
                output.push_str(&format!(
                    "{}  [{}] {} [{}:{} - {}:{}]",
                    indent, child.id(), child_kind, child_start.row, child_start.column, child_end.row, child_end.column
                ));
                if !child_preview.is_empty() {
                    output.push_str(&format!(": {}", child_preview.replace('\n', "\\n")));
                }
                output.push_str(&format!(" ({} children)", child.child_count()));
                output.push('\n');
                
                // Recursively dump children of important nodes
                if child_kind == "signature" || child_kind == "parameter_list" || child_kind == "typed_parameter" || child_kind == "assignment" || child_kind == "identifier" {
                    dump_function_definitions_recursive(child, text, depth + 2, output);
                }
            }
        }
    } else if kind == "signature" || kind == "parameter_list" || kind == "typed_parameter" || kind == "assignment" || kind == "call_expression" || kind == "argument_list" {
        // Dump these nodes in detail too
        let node_text = node.utf8_text(text.as_bytes()).unwrap_or("");
        let preview = if node_text.len() > 80 {
            format!("{}...", &node_text[..80])
        } else {
            node_text.to_string()
        };
        
        output.push_str(&format!(
            "{}[{}] {} [{}:{} - {}:{}]",
            indent, node.id(), kind, start.row, start.column, end.row, end.column
        ));
        if !preview.is_empty() {
            output.push_str(&format!(": {}", preview.replace('\n', "\\n")));
        }
        output.push_str(&format!(" ({} children)", node.child_count()));
        output.push('\n');
        
        // Dump all children with their kinds
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let child_kind = child.kind();
                let child_start = child.start_position();
                let child_end = child.end_position();
                let child_text = child.utf8_text(text.as_bytes()).unwrap_or("");
                let child_preview = if child_text.len() > 60 {
                    format!("{}...", &child_text[..60])
                } else {
                    child_text.to_string()
                };
                output.push_str(&format!(
                    "{}  [{}] {} [{}:{} - {}:{}]",
                    indent, child.id(), child_kind, child_start.row, child_start.column, child_end.row, child_end.column
                ));
                if !child_preview.is_empty() {
                    output.push_str(&format!(": {}", child_preview.replace('\n', "\\n")));
                }
                output.push_str(&format!(" ({} children)", child.child_count()));
                output.push('\n');
                
                // Recursively dump children
                dump_function_definitions_recursive(child, text, depth + 2, output);
            }
        }
    } else {
        // For other nodes, only recurse if they might contain function definitions
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                dump_function_definitions_recursive(child, text, depth, output);
            }
        }
    }
}

/// No-op in release builds
#[cfg(not(debug_assertions))]
pub(super) fn dump_function_definitions_cst(_node: Node, _text: &str) -> String {
    String::new()
}


