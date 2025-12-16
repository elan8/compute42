use crate::types::{CodeAction, Diagnostic, TextEdit, Range, Position, WorkspaceEdit};

/// Generate code action to fix unmatched delimiters
pub fn fix_delimiter_action(
    diagnostic: &Diagnostic,
    _tree: &tree_sitter::Tree,
    text: &str,
) -> Option<CodeAction> {
    let line_num = diagnostic.range.start.line as usize;
    let char_num = diagnostic.range.start.character as usize;
    
    if let Some(line) = text.lines().nth(line_num) {
        // Count delimiters on this line
        let open_paren = line[..char_num.min(line.len())].matches('(').count();
        let close_paren = line[..char_num.min(line.len())].matches(')').count();
        let open_bracket = line[..char_num.min(line.len())].matches('[').count();
        let close_bracket = line[..char_num.min(line.len())].matches(']').count();
        let open_brace = line[..char_num.min(line.len())].matches('{').count();
        let close_brace = line[..char_num.min(line.len())].matches('}').count();
        
        let missing_paren = open_paren.saturating_sub(close_paren);
        let missing_bracket = open_bracket.saturating_sub(close_bracket);
        let missing_brace = open_brace.saturating_sub(close_brace);
        
        if missing_paren > 0 || missing_bracket > 0 || missing_brace > 0 {
            let mut closing = String::new();
            closing.push_str(&")".repeat(missing_paren));
            closing.push_str(&"]".repeat(missing_bracket));
            closing.push_str(&"}".repeat(missing_brace));
            
            let closing_for_title = closing.clone();
            
            let insert_pos = Position {
                line: line_num as u32,
                character: line.len() as u32,
            };
            
            let edit = TextEdit {
                range: Range {
                    start: insert_pos,
                    end: insert_pos,
                },
                new_text: closing,
            };
            
            return Some(CodeAction {
                title: format!("Add missing closing delimiter(s): {}", closing_for_title),
                kind: Some("quickfix".to_string()),
                edit: Some(WorkspaceEdit {
                    changes: vec![(String::new(), vec![edit])],
                }),
                command: None,
            });
        }
    }
    
    None
}

