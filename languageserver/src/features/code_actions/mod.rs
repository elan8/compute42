use crate::types::{Diagnostic, CodeAction};
use tree_sitter::Tree;

mod missing_end;
mod delimiters;
mod unused_vars;
mod imports;
mod undefined_vars;

pub use missing_end::add_missing_end_action;
pub use delimiters::fix_delimiter_action;
pub use unused_vars::remove_unused_variable_action;
pub use imports::add_import_action;
pub use undefined_vars::fix_undefined_variable_action;

/// Code actions provider
pub struct CodeActionsProvider;

impl CodeActionsProvider {
    /// Get code actions for a diagnostic
    pub fn get_actions(
        diagnostic: &Diagnostic,
        tree: &Tree,
        text: &str,
    ) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        
        if let Some(ref code) = diagnostic.code {
            match code.as_str() {
                "missing_end" => {
                    if let Some(action) = add_missing_end_action(diagnostic, tree, text) {
                        actions.push(action);
                    }
                }
                "unmatched_parenthesis" | "unmatched_bracket" | "unmatched_brace" => {
                    if let Some(action) = fix_delimiter_action(diagnostic, tree, text) {
                        actions.push(action);
                    }
                }
                "unused_variable" => {
                    if let Some(action) = remove_unused_variable_action(diagnostic, tree, text) {
                        actions.push(action);
                    }
                }
                "unresolved_import" => {
                    if let Some(action) = add_import_action(diagnostic, tree, text) {
                        actions.push(action);
                    }
                }
                "undefined_variable" => {
                    if let Some(action) = fix_undefined_variable_action(diagnostic, tree, text) {
                        actions.push(action);
                    }
                }
                _ => {}
            }
        }
        
        actions
    }
    
    /// Get all code actions for a set of diagnostics
    pub fn get_actions_for_diagnostics(
        diagnostics: &[Diagnostic],
        tree: &Tree,
        text: &str,
    ) -> Vec<CodeAction> {
        let mut all_actions = Vec::new();
        
        for diagnostic in diagnostics {
            let actions = Self::get_actions(diagnostic, tree, text);
            all_actions.extend(actions);
        }
        
        all_actions
    }
}



