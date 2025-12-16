use crate::types::{CodeAction, Diagnostic, TextEdit, Range, Position, WorkspaceEdit};
use tree_sitter::{Tree, Node};

/// Generate code action to add missing 'end' keyword
pub fn add_missing_end_action(
    diagnostic: &Diagnostic,
    tree: &Tree,
    text: &str,
) -> Option<CodeAction> {
    // Find the block that needs the 'end'
    let root = tree.root_node();
    let target_line = diagnostic.range.start.line as usize;
    
    // Find the opening block (function, if, for, while, etc.)
    let block_node = find_block_node(root, target_line, text)?;
    let block_kind = block_node.kind();
    
    // Find where to insert the 'end'
    let insert_position = find_end_insertion_point(&block_node, text)?;
    
    // Determine indentation
    let indentation = get_indentation_for_line(text, target_line);
    
    // Create the edit
    let edit = TextEdit {
        range: Range {
            start: insert_position,
            end: insert_position,
        },
        new_text: format!("{}\n{}end", if needs_newline_before_end(&block_node, text) { "\n" } else { "" }, indentation),
    };
    
    Some(CodeAction {
        title: format!("Add missing 'end' for {}", block_kind),
        kind: Some("quickfix".to_string()),
        edit: Some(WorkspaceEdit {
            changes: vec![(String::new(), vec![edit])], // URI will be filled by caller
        }),
        command: None,
    })
}

fn find_block_node<'a>(root: Node<'a>, target_line: usize, _text: &str) -> Option<Node<'a>> {
    find_block_node_recursive(root, target_line)
}

fn find_block_node_recursive<'a>(node: Node<'a>, target_line: usize) -> Option<Node<'a>> {
    let start = node.start_position();
    let end = node.end_position();
    
    // Check if target line is within this node
    if start.row <= target_line && target_line <= end.row {
        // Check if this is a block node
        if matches!(
            node.kind(),
            "function_definition" | "if_statement" | "for_statement" | "while_statement"
                | "begin_statement" | "try_statement" | "let_statement" | "struct_definition"
                | "module_definition" | "macro_definition"
        ) {
            return Some(node);
        }
        
        // Check children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if let Some(found) = find_block_node_recursive(child, target_line) {
                    return Some(found);
                }
            }
        }
    }
    
    None
}

fn find_end_insertion_point(block_node: &Node, text: &str) -> Option<Position> {
    // Find the last statement in the block
    let end_pos = block_node.end_position();
    
    // Look for the last non-empty line before the end
    let lines: Vec<&str> = text.lines().collect();
    let mut last_line_idx = end_pos.row;
    
    // Skip empty lines
    while last_line_idx > 0 && lines.get(last_line_idx).map(|l| l.trim().is_empty()).unwrap_or(true) {
        last_line_idx -= 1;
    }
    
    if let Some(last_line) = lines.get(last_line_idx) {
        let line_end = last_line.len();
        Some(Position {
            line: last_line_idx as u32,
            character: line_end as u32,
        })
    } else {
        Some(Position::from(end_pos))
    }
}

fn get_indentation_for_line(text: &str, line_num: usize) -> String {
    if let Some(line) = text.lines().nth(line_num) {
        let indent_len = line.len() - line.trim_start().len();
        " ".repeat(indent_len)
    } else {
        String::new()
    }
}

fn needs_newline_before_end(node: &Node, text: &str) -> bool {
    let end_pos = node.end_position();
    if let Some(line) = text.lines().nth(end_pos.row) {
        !line.trim().is_empty() && !line.trim_end().ends_with('\n')
    } else {
        true
    }
}

