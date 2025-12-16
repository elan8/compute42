use crate::types::{Diagnostic, Position, Range};
use tree_sitter::{Node, Tree};
use super::error_analysis::create_diagnostic_from_node;

/// Recursively extract diagnostics from the parse tree
pub fn extract_diagnostics_from_tree(tree: &Tree, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    let root_node = tree.root_node();
    
    // Special case: check for standalone 'end' keyword
    // If the content is just "end", it's unexpected
    let mut found_unexpected_end = false;
    let trimmed_content = content.trim();
    if trimmed_content == "end" || (trimmed_content.len() <= 5 && trimmed_content.contains("end")) {
        // Check if there's an 'end' node or ERROR node as a direct child of root
        for i in 0..root_node.child_count() {
            if let Some(child) = root_node.child(i) {
                let child_kind = child.kind();
                let is_end_node = child_kind == "end";
                let is_error_end = if child_kind == "ERROR" {
                    child.utf8_text(content.as_bytes())
                        .map(|s| s.trim() == "end" || s.contains("end"))
                        .unwrap_or(false)
                } else {
                    false
                };
                // Also check if it's an identifier named "end" (tree-sitter might parse "end" as identifier)
                let is_end_identifier = if child_kind == "identifier" {
                    child.utf8_text(content.as_bytes())
                        .map(|s| s.trim() == "end")
                        .unwrap_or(false)
                } else {
                    false
                };
                
                if is_end_node || is_error_end || is_end_identifier {
                    // Create a diagnostic for unexpected end
                    let start = node_to_position(child.start_position());
                    let end = node_to_position(child.end_position());
                    let range = Range { start, end };
                    let diagnostic = Diagnostic {
                        range,
                        severity: Some(crate::types::DiagnosticSeverity::Error),
                        code: Some("unexpected_end".to_string()),
                        source: Some("tree-sitter".to_string()),
                        message: "Unexpected 'end' keyword. Check for matching block structure.".to_string(),
                        related_information: None,
                    };
                    diagnostics.push(diagnostic);
                    found_unexpected_end = true;
                    break;
                }
            }
        }
        
        // Also check if root itself is an ERROR node containing "end"
        if !found_unexpected_end && root_node.kind() == "ERROR" {
            if let Ok(root_text) = root_node.utf8_text(content.as_bytes()) {
                if root_text.trim() == "end" || (root_text.contains("end") && root_text.len() <= 5) {
                    let start = node_to_position(root_node.start_position());
                    let end = node_to_position(root_node.end_position());
                    let range = Range { start, end };
                    let diagnostic = Diagnostic {
                        range,
                        severity: Some(crate::types::DiagnosticSeverity::Error),
                        code: Some("unexpected_end".to_string()),
                        source: Some("tree-sitter".to_string()),
                        message: "Unexpected 'end' keyword. Check for matching block structure.".to_string(),
                        related_information: None,
                    };
                    diagnostics.push(diagnostic);
                    found_unexpected_end = true;
                }
            }
        }
    }
    
    // Extract other diagnostics (but skip if we already handled unexpected end to avoid duplicates)
    if !found_unexpected_end {
        extract_diagnostics_from_node(&root_node, content, diagnostics);
    } else {
        // Still check for other errors, but skip ERROR nodes that are just "end"
        extract_diagnostics_from_node_skip_end(&root_node, content, diagnostics);
    }
}

/// Extract diagnostics from a specific node and its children
fn extract_diagnostics_from_node(node: &Node, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    // Check if this node represents an error
    if node.is_error() || node.is_missing() {
        // Special handling for "end" - check if this ERROR node is about "end"
        if node.kind() == "ERROR" {
            if let Ok(error_text) = node.utf8_text(content.as_bytes()) {
                let trimmed = error_text.trim();
                // If this is an ERROR node containing just "end", create unexpected_end diagnostic
                if trimmed == "end" {
                    let start = node_to_position(node.start_position());
                    let end = node_to_position(node.end_position());
                    let range = Range { start, end };
                    let diagnostic = Diagnostic {
                        range,
                        severity: Some(crate::types::DiagnosticSeverity::Error),
                        code: Some("unexpected_end".to_string()),
                        source: Some("tree-sitter".to_string()),
                        message: "Unexpected 'end' keyword. Check for matching block structure.".to_string(),
                        related_information: None,
                    };
                    diagnostics.push(diagnostic);
                    return; // Don't process children for this case
                }
            }
        }
        
        // Check for missing 'end' nodes
        if node.is_missing() && node.kind() == "end" {
            let diagnostic = create_diagnostic_from_node(node, content);
            diagnostics.push(diagnostic);
            return; // Don't process children for missing nodes
        }
        
        let diagnostic = create_diagnostic_from_node(node, content);
        diagnostics.push(diagnostic);
    }
    
    // Recursively check children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_diagnostics_from_node(&child, content, diagnostics);
        }
    }
}

/// Extract diagnostics but skip ERROR nodes that are just "end" (already handled)
fn extract_diagnostics_from_node_skip_end(node: &Node, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    // Check if this node represents an error (but skip if it's just "end")
    if node.is_error() || node.is_missing() {
        // Skip if this is an ERROR node containing just "end"
        if node.kind() == "ERROR" {
            if let Ok(text) = node.utf8_text(content.as_bytes()) {
                let trimmed = text.trim();
                if trimmed == "end" || (trimmed.len() <= 5 && trimmed.contains("end") && !trimmed.contains("function") && !trimmed.contains("if") && !trimmed.contains("for")) {
                    return; // Skip this, already handled
                }
            }
        }
        let diagnostic = create_diagnostic_from_node(node, content);
        diagnostics.push(diagnostic);
    }
    
    // Recursively check children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_diagnostics_from_node_skip_end(&child, content, diagnostics);
        }
    }
}

/// Convert tree-sitter position to LSP position
pub fn node_to_position(point: tree_sitter::Point) -> Position {
    Position {
        line: point.row as u32,
        character: point.column as u32,
    }
}

/// Extract text at a specific range for context
pub fn extract_text_at_range(content: &str, range: &Range) -> String {
    let lines: Vec<&str> = content.lines().collect();
    
    if range.start.line as usize >= lines.len() {
        return String::new();
    }
    
    if range.start.line == range.end.line {
        // Single line
        let line = lines[range.start.line as usize];
        let start = range.start.character as usize;
        let end = range.end.character as usize;
        
        if start < line.len() {
            line[start..end.min(line.len())].to_string()
        } else {
            String::new()
        }
    } else {
        // Multi-line - just get the first line for now
        let line = lines[range.start.line as usize];
        let start = range.start.character as usize;
        
        if start < line.len() {
            line[start..].to_string()
        } else {
            String::new()
        }
    }
}




