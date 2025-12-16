use crate::types::{CodeAction, Diagnostic, TextEdit, Range, Position, WorkspaceEdit};

/// Generate code action to add import statement
pub fn add_import_action(
    diagnostic: &Diagnostic,
    _tree: &tree_sitter::Tree,
    text: &str,
) -> Option<CodeAction> {
    // Extract module name from diagnostic
    let module_name = extract_module_name(&diagnostic.message)?;
    
    // Find a good place to insert the import (usually at the top after other imports)
    let insert_line = find_import_insertion_point(text);
    
    let import_statement = format!("using {}\n", module_name);
    
    let edit = TextEdit {
        range: Range {
            start: Position {
                line: insert_line,
                character: 0,
            },
            end: Position {
                line: insert_line,
                character: 0,
            },
        },
        new_text: import_statement,
    };
    
    Some(CodeAction {
        title: format!("Add 'using {}'", module_name),
        kind: Some("quickfix".to_string()),
        edit: Some(WorkspaceEdit {
            changes: vec![(String::new(), vec![edit])],
        }),
        command: None,
    })
}

fn extract_module_name(message: &str) -> Option<&str> {
    // Look for pattern like "Package `name` may not be available"
    if let Some(start) = message.find('`') {
        if let Some(end) = message[start + 1..].find('`') {
            return Some(&message[start + 1..start + 1 + end]);
        }
    }
    None
}

fn find_import_insertion_point(text: &str) -> u32 {
    // Find the last 'using' or 'import' statement, or return 0
    let mut last_import_line = 0u32;
    
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("using ") || trimmed.starts_with("import ") {
            last_import_line = (i + 1) as u32; // +1 to insert after this line
        }
    }
    
    last_import_line
}

