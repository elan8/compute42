use crate::types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tree_sitter::Node;
use super::extractor::{node_to_position, extract_text_at_range};
use super::syntax_analyzer::analyze_syntax_error;

/// Create a diagnostic from an error or missing node
pub fn create_diagnostic_from_node(node: &Node, content: &str) -> Diagnostic {
    // Prefer a very small range for error nodes to avoid painting the whole file
    let start = node_to_position(node.start_position());
    let end = if node.is_error() {
        Position { line: start.line, character: start.character.saturating_add(1) }
    } else if start.line + 50 < node_to_position(node.end_position()).line {
        // For non-error nodes, clamp excessively large spans
        Position { line: start.line, character: start.character.saturating_add(1) }
    } else {
        node_to_position(node.end_position())
    };

    let range = Range { start, end };
    let error_text = extract_text_at_range(content, &range);

    let (message, code) = if node.is_missing() {
        create_missing_error_message(node, &error_text)
    } else if node.is_error() {
        create_syntax_error_message(node, &error_text, content, &range)
    } else {
        ("Unknown syntax error".to_string(), None)
    };

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::Error),
        code,
        source: Some("tree-sitter".to_string()),
        message,
        related_information: None,
    }
}

/// Create error message for missing nodes
fn create_missing_error_message(node: &Node, _error_text: &str) -> (String, Option<String>) {
    // Check if this is a missing "end" keyword
    if node.kind() == "end" {
        let message = if let Some(parent) = node.parent() {
            match parent.kind() {
                "function_definition" => "Missing 'end' for function definition".to_string(),
                "if_statement" => "Missing 'end' for if statement".to_string(),
                "for_statement" => "Missing 'end' for for loop".to_string(),
                "while_statement" => "Missing 'end' for while loop".to_string(),
                _ => format!("Missing 'end' for {}", parent.kind()),
            }
        } else {
            "Missing 'end'".to_string()
        };
        return (message, Some("missing_end".to_string()));
    }
    
    let expected = if let Some(parent) = node.parent() {
        format!("Expected '{}' in {}", node.kind(), parent.kind())
    } else {
        format!("Expected '{}'", node.kind())
    };
    
    (expected, Some("missing_node".to_string()))
}

/// Create error message for syntax errors
fn create_syntax_error_message(
    node: &Node,
    error_text: &str,
    content: &str,
    range: &Range,
) -> (String, Option<String>) {
    // Check for unexpected 'end' keyword
    let current_line = content
        .lines()
        .nth(range.start.line as usize)
        .unwrap_or("");
    if current_line.trim() == "end" || error_text.trim() == "end" {
        return ("Unexpected 'end' keyword. Check for matching block structure.".to_string(), Some("unexpected_end".to_string()));
    }
    
    // Check if this ERROR node contains a block type that's missing 'end'
    // This check should happen before parent checks to catch root-level ERROR nodes
    if node.kind() == "ERROR" {
        // Check for complete block structures (function_definition, if_statement, etc.)
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let child_kind = child.kind();
                if child_kind == "function_definition" || child_kind == "if_statement" || 
                   child_kind == "for_statement" || child_kind == "while_statement" ||
                   child_kind == "begin_statement" || child_kind == "try_statement" ||
                   child_kind == "let_statement" || child_kind == "struct_definition" ||
                   child_kind == "module_definition" || child_kind == "macro_definition" {
                    // Check if this block is missing an 'end'
                    let has_end = {
                        let mut has_end = false;
                        for j in 0..child.child_count() {
                            if let Some(grandchild) = child.child(j) {
                                if grandchild.kind() == "end" && !grandchild.is_missing() {
                                    has_end = true;
                                    break;
                                }
                            }
                        }
                        has_end
                    };
                    if !has_end {
                        return match child_kind {
                            "function_definition" => ("Missing 'end' for function definition".to_string(), Some("missing_end".to_string())),
                            "if_statement" => ("Missing 'end' for if statement".to_string(), Some("missing_end".to_string())),
                            "for_statement" => ("Missing 'end' for for loop".to_string(), Some("missing_end".to_string())),
                            "while_statement" => ("Missing 'end' for while loop".to_string(), Some("missing_end".to_string())),
                            _ => ("Missing 'end'".to_string(), Some("missing_end".to_string())),
                        };
                    }
                }
            }
        }
        
        // Check for incomplete block structures (e.g., "function" keyword + "signature" but no "end")
        // Pattern: ERROR contains "function" keyword followed by "signature"
        let mut has_function_keyword = false;
        let mut has_signature = false;
        let mut has_end_in_error = false;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "function" => has_function_keyword = true,
                    "signature" => has_signature = true,
                    "end" if !child.is_missing() => has_end_in_error = true,
                    _ => {}
                }
            }
        }
        if has_function_keyword && has_signature && !has_end_in_error {
            return ("Missing 'end' for function definition".to_string(), Some("missing_end".to_string()));
        }
        
        // Check for "if" keyword pattern
        let mut has_if_keyword = false;
        let mut has_end_for_if = false;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "if" => has_if_keyword = true,
                    "end" if !child.is_missing() => has_end_for_if = true,
                    _ => {}
                }
            }
        }
        if has_if_keyword && !has_end_for_if {
            return ("Missing 'end' for if statement".to_string(), Some("missing_end".to_string()));
        }
        
        // Check for "for" keyword pattern
        let mut has_for_keyword = false;
        let mut has_end_for_for = false;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "for" => has_for_keyword = true,
                    "end" if !child.is_missing() => has_end_for_for = true,
                    _ => {}
                }
            }
        }
        if has_for_keyword && !has_end_for_for {
            return ("Missing 'end' for for loop".to_string(), Some("missing_end".to_string()));
        }
        
        // Check for "while" keyword pattern
        let mut has_while_keyword = false;
        let mut has_end_for_while = false;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "while" => has_while_keyword = true,
                    "end" if !child.is_missing() => has_end_for_while = true,
                    _ => {}
                }
            }
        }
        if has_while_keyword && !has_end_for_while {
            return ("Missing 'end' for while loop".to_string(), Some("missing_end".to_string()));
        }
    }
    
    // Try to provide context-specific error messages
    let message = if let Some(parent) = node.parent() {
        let parent_kind = parent.kind();
        
        // Check if this is a missing 'end' case (parent is a block type)
        if parent_kind == "function_definition" || parent_kind == "if_statement" || 
           parent_kind == "for_statement" || parent_kind == "while_statement" ||
           parent_kind == "begin_statement" || parent_kind == "try_statement" ||
           parent_kind == "let_statement" || parent_kind == "struct_definition" ||
           parent_kind == "module_definition" || parent_kind == "macro_definition" {
            // Check if the parent block is missing an 'end' child
            let has_end_child = {
                let mut has_end = false;
                for i in 0..parent.child_count() {
                    if let Some(child) = parent.child(i) {
                        if child.kind() == "end" && !child.is_missing() {
                            has_end = true;
                            break;
                        }
                    }
                }
                has_end
            };
            
            // Check if the error is at the end of the block (likely missing 'end')
            let parent_end = parent.end_position();
            let node_start = node.start_position();
            // If error is near the end of the parent block, it's likely a missing 'end'
            // Also check if the error node is the last child or near the end
            let is_near_end = node_start.row >= parent_end.row.saturating_sub(1);
            let is_last_child = {
                let mut is_last = false;
                if let Some(parent_node) = node.parent() {
                    let child_count = parent_node.child_count();
                    for i in 0..child_count {
                        if let Some(child) = parent_node.child(i) {
                            if child.id() == node.id() && i == child_count - 1 {
                                is_last = true;
                                break;
                            }
                        }
                    }
                }
                is_last
            };
            
            // If the block doesn't have an 'end' child and the error is at/near the end, it's missing 'end'
            if is_last_child || !has_end_child && is_near_end {
                return match parent_kind {
                    "function_definition" => ("Missing 'end' for function definition".to_string(), Some("missing_end".to_string())),
                    "if_statement" => ("Missing 'end' for if statement".to_string(), Some("missing_end".to_string())),
                    "for_statement" => ("Missing 'end' for for loop".to_string(), Some("missing_end".to_string())),
                    "while_statement" => ("Missing 'end' for while loop".to_string(), Some("missing_end".to_string())),
                    _ => ("Missing 'end'".to_string(), Some("missing_end".to_string())),
                };
            }
        }
        
        // Detect common Julia syntax errors
        match parent_kind {
            "function_definition" => {
                "Syntax error in function definition. Check for missing 'end' or incorrect syntax.".to_string()
            }
            "if_statement" => {
                "Syntax error in if statement. Check for missing 'end' or incorrect condition syntax.".to_string()
            }
            "for_statement" => {
                "Syntax error in for loop. Check for missing 'end' or incorrect loop syntax.".to_string()
            }
            "while_statement" => {
                "Syntax error in while loop. Check for missing 'end' or incorrect condition.".to_string()
            }
            "assignment" => {
                // Detect common string issues within assignment
                if current_line.matches('"').count() % 2 != 0 || content.matches('"').count() % 2 != 0 {
                    "Unmatched string delimiter".to_string()
                } else {
                    "Syntax error in assignment. Check for correct variable name and value syntax.".to_string()
                }
            }
            _ => {
                // Always try analysis for better hints
                analyze_syntax_error(error_text, content, range, parent_kind)
            }
        }
    } else {
        // Fall back, but keep it actionable
        "Syntax error detected. Check for a missing 'end' or unmatched delimiters.".to_string()
    };
    
    (message, Some("syntax_error".to_string()))
}

