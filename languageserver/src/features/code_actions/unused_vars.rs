use crate::types::{CodeAction, Diagnostic, TextEdit, Range, Position, WorkspaceEdit};

/// Generate code action to remove unused variable
pub fn remove_unused_variable_action(
    diagnostic: &Diagnostic,
    _tree: &tree_sitter::Tree,
    text: &str,
) -> Option<CodeAction> {
    let line_num = diagnostic.range.start.line as usize;
    
    if let Some(line) = text.lines().nth(line_num) {
        // Find the assignment statement
        if line.contains('=') {
            // Extract variable name from diagnostic message
            let var_name = extract_variable_name(&diagnostic.message)?;
            
            // Find the full assignment line
            let trimmed = line.trim();
            if trimmed.starts_with(&format!("{} =", var_name)) {
                // Remove the entire line
                let edit = TextEdit {
                    range: Range {
                        start: diagnostic.range.start,
                        end: Position {
                            line: diagnostic.range.start.line,
                            character: (line.len() as u32).max(diagnostic.range.end.character),
                        },
                    },
                    new_text: String::new(),
                };
                
                return Some(CodeAction {
                    title: format!("Remove unused variable '{}'", var_name),
                    kind: Some("quickfix".to_string()),
                    edit: Some(WorkspaceEdit {
                        changes: vec![(String::new(), vec![edit])],
                    }),
                    command: None,
                });
            }
        }
    }
    
    None
}

fn extract_variable_name(message: &str) -> Option<&str> {
    // Look for pattern like "Unused variable: `name`"
    if let Some(start) = message.find('`') {
        if let Some(end) = message[start + 1..].find('`') {
            return Some(&message[start + 1..start + 1 + end]);
        }
    }
    None
}

