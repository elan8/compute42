use crate::types::{CodeAction, Diagnostic, TextEdit, Range, Position, WorkspaceEdit};
use tree_sitter::Tree;

/// Generate code action to fix undefined variable (suggest similar symbol)
pub fn fix_undefined_variable_action(
    diagnostic: &Diagnostic,
    _tree: &Tree,
    text: &str,
) -> Option<CodeAction> {
    // Extract variable name and suggestion from diagnostic message
    let (var_name, suggestion) = extract_variable_and_suggestion(&diagnostic.message)?;
    
    // Find the variable in the text at the diagnostic range
    let line_num = diagnostic.range.start.line as usize;
    let char_start = diagnostic.range.start.character as usize;
    let char_end = diagnostic.range.end.character as usize;
    
    if text.lines().nth(line_num).is_some() {
        // Create edit to replace the undefined variable with the suggestion
        let edit = TextEdit {
            range: Range {
                start: Position {
                    line: line_num as u32,
                    character: char_start as u32,
                },
                end: Position {
                    line: line_num as u32,
                    character: char_end as u32,
                },
            },
            new_text: suggestion.clone(),
        };
        
        return Some(CodeAction {
            title: format!("Replace `{}` with `{}`", var_name, suggestion),
            kind: Some("quickfix".to_string()),
            edit: Some(WorkspaceEdit {
                changes: vec![(String::new(), vec![edit])],
            }),
            command: None,
        });
    }
    
    None
}

fn extract_variable_and_suggestion(message: &str) -> Option<(&str, String)> {
    // Look for pattern like "Undefined variable or function: `name`\nDid you mean `suggestion`?"
    if let Some(start) = message.find('`') {
        if let Some(end) = message[start + 1..].find('`') {
            let var_name = &message[start + 1..start + 1 + end];
            
            // Look for "Did you mean" suggestion
            if let Some(suggestion_start) = message.find("Did you mean `") {
                let suggestion_text = &message[suggestion_start + 14..];
                if let Some(suggestion_end) = suggestion_text.find('`') {
                    let suggestion = suggestion_text[..suggestion_end].to_string();
                    return Some((var_name, suggestion));
                }
            }
        }
    }
    None
}

