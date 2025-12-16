#[cfg(debug_assertions)]
use tree_sitter::{Node, Tree};
#[cfg(debug_assertions)]
use std::fs;
#[cfg(debug_assertions)]
use std::path::PathBuf;
#[cfg(debug_assertions)]
use std::env;

/// Serialize tree-sitter CST to a readable text format for debugging
#[cfg(debug_assertions)]
#[allow(dead_code)] // Only used in debug builds via conditional compilation
pub fn dump_cst_to_file(tree: &Tree, text: &str, file_path: &str) {
    let output = serialize_tree(tree, text, 0);
    
    // Try to write to the executable directory first, fallback to current directory
    let output_path = if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            exe_dir.join("debug_cst").join(file_path)
        } else {
            PathBuf::from("debug_cst").join(file_path)
        }
    } else {
        PathBuf::from("debug_cst").join(file_path)
    };
    
    // Create debug_cst directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            log::warn!("Failed to create debug_cst directory: {}", e);
            return;
        }
    }
    
    if let Err(e) = fs::write(&output_path, output) {
        log::warn!("Failed to write CST dump to {:?}: {}", output_path, e);
    } else {
        log::trace!("CST dumped to: {:?}", output_path);
    }
}

/// Serialize a tree-sitter tree to a readable string format
#[cfg(debug_assertions)]
#[allow(dead_code)] // Only used in debug builds
fn serialize_tree(tree: &Tree, text: &str, max_depth: usize) -> String {
    let root = tree.root_node();
    let mut output = String::new();
    output.push_str("=== Tree-sitter CST Dump ===\n\n");
    output.push_str(&format!("Source text length: {} bytes\n", text.len()));
    output.push_str(&format!("Tree has {} errors\n\n", root.has_error()));
    output.push_str("=== Parse Tree Structure ===\n\n");
    serialize_node(&root, text, 0, max_depth, &mut output);
    output
}

/// Recursively serialize a node and its children
#[cfg(debug_assertions)]
#[allow(dead_code)] // Only used in debug builds
fn serialize_node(node: &Node, text: &str, depth: usize, max_depth: usize, output: &mut String) {
    if max_depth > 0 && depth >= max_depth {
        output.push_str(&format!("{}... (truncated at depth {})\n", "  ".repeat(depth), max_depth));
        return;
    }
    
    let kind = node.kind();
    let start = node.start_position();
    let end = node.end_position();
    let is_error = node.is_error();
    let is_missing = node.is_missing();
    
    // Get node text (truncated if too long)
    let node_text = node.utf8_text(text.as_bytes()).unwrap_or("");
    let display_text = if node_text.len() > 100 {
        format!("{}...", &node_text[..100])
    } else {
        node_text.to_string()
    };
    
    // Format the node line
    let indent = "  ".repeat(depth);
    let flags = {
        let mut f = Vec::new();
        if is_error { f.push("ERROR"); }
        if is_missing { f.push("MISSING"); }
        if f.is_empty() { String::new() } else { format!(" [{}]", f.join(",")) }
    };
    
    output.push_str(&format!(
        "{}{} [{},{}] -> [{},{}]{}",
        indent,
        kind,
        start.row,
        start.column,
        end.row,
        end.column,
        flags
    ));
    
    if !node_text.is_empty() && node_text.len() <= 200 {
        output.push_str(&format!(": \"{}\"", display_text.replace('\n', "\\n")));
    }
    output.push('\n');
    
    // Recursively serialize children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            serialize_node(&child, text, depth + 1, max_depth, output);
        }
    }
}

/// No-op in release builds
#[cfg(not(debug_assertions))]
pub fn dump_cst_to_file(_tree: &Tree, _text: &str, _file_path: &str) {
    // No-op in release builds
}

