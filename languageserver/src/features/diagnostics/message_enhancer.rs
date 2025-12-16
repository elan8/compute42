use crate::types::{Diagnostic, DiagnosticSeverity, Location};
use crate::pipeline::storage::Index;

/// Enhances diagnostic messages with context and suggestions
pub struct MessageEnhancer;

impl MessageEnhancer {
    /// Enhance a diagnostic with better context and suggestions
    pub fn enhance(
        diagnostic: &mut Diagnostic,
        text: &str,
        index: Option<&Index>,
    ) {
        // Enhance based on diagnostic code
        if let Some(ref code) = diagnostic.code {
            match code.as_str() {
                "undefined_variable" => {
                    Self::enhance_undefined_variable(diagnostic, text, index);
                }
                "missing_end" => {
                    Self::enhance_missing_end(diagnostic, text);
                }
                "unexpected_end" => {
                    Self::enhance_unexpected_end(diagnostic, text);
                }
                "unmatched_parenthesis" | "unmatched_bracket" | "unmatched_brace" => {
                    Self::enhance_unmatched_delimiter(diagnostic, text);
                }
                "unused_variable" => {
                    Self::enhance_unused_variable(diagnostic, text);
                }
                "unresolved_import" => {
                    Self::enhance_unresolved_import(diagnostic, text);
                }
                _ => {
                    // Generic enhancement
                    Self::add_context(diagnostic, text);
                }
            }
        } else {
            // Generic enhancement for diagnostics without codes
            Self::add_context(diagnostic, text);
        }
    }
    
    /// Enhance undefined variable diagnostic
    fn enhance_undefined_variable(
        diagnostic: &mut Diagnostic,
        text: &str,
        index: Option<&Index>,
    ) {
        // Extract variable name from message
        if let Some(var_name) = Self::extract_variable_name(&diagnostic.message) {
            // Try to find similar symbols
            if let Some(idx) = index {
                if let Some(suggestion) = Self::find_similar_in_index(var_name, idx) {
                    let suggestion_msg = format!("\n\nSuggestion: Did you mean `{}`?", suggestion);
                    diagnostic.message.push_str(&suggestion_msg);
                }
            }
        }
        
        // Add context about where this might be defined
        Self::add_context(diagnostic, text);
    }
    
    /// Enhance missing end diagnostic
    fn enhance_missing_end(diagnostic: &mut Diagnostic, text: &str) {
        // Find the opening block keyword
        let line_num = diagnostic.range.start.line as usize;
        if let Some(line) = text.lines().nth(line_num) {
            let trimmed = line.trim_start();
            if trimmed.starts_with("function") {
                diagnostic.message.push_str("\n\nTip: Add `end` after the function body.");
            } else if trimmed.starts_with("if") {
                diagnostic.message.push_str("\n\nTip: Add `end` after the if block.");
            } else if trimmed.starts_with("for") {
                diagnostic.message.push_str("\n\nTip: Add `end` after the for loop body.");
            } else if trimmed.starts_with("while") {
                diagnostic.message.push_str("\n\nTip: Add `end` after the while loop body.");
            }
        }
    }
    
    /// Enhance unexpected end diagnostic
    fn enhance_unexpected_end(diagnostic: &mut Diagnostic, _text: &str) {
        diagnostic.message.push_str("\n\nTip: Check for mismatched block structure. Each `end` must match a `function`, `if`, `for`, `while`, `begin`, `try`, `let`, `struct`, `module`, or `macro`.");
    }
    
    /// Enhance unmatched delimiter diagnostic
    fn enhance_unmatched_delimiter(diagnostic: &mut Diagnostic, text: &str) {
        let line_num = diagnostic.range.start.line as usize;
        if let Some(line) = text.lines().nth(line_num) {
            let open_count = line.matches('(').count() + line.matches('[').count() + line.matches('{').count();
            let close_count = line.matches(')').count() + line.matches(']').count() + line.matches('}').count();
            
            if open_count > close_count {
                diagnostic.message.push_str("\n\nTip: Add closing delimiter(s) to match the opening delimiter(s).");
            } else {
                diagnostic.message.push_str("\n\nTip: Check for extra closing delimiter(s) or missing opening delimiter(s).");
            }
        }
    }
    
    /// Enhance unused variable diagnostic
    fn enhance_unused_variable(diagnostic: &mut Diagnostic, text: &str) {
        diagnostic.message.push_str("\n\nTip: Remove the variable if it's not needed, or use it in your code.");
        
        // Check if it's a parameter (shouldn't suggest removal)
        let line_num = diagnostic.range.start.line as usize;
        if let Some(line) = text.lines().nth(line_num) {
            if line.contains("function") && line.contains(diagnostic.message.split('`').nth(1).unwrap_or("")) {
                diagnostic.message.push_str(" Note: This is a function parameter and may be used by callers.");
            }
        }
    }
    
    /// Enhance unresolved import diagnostic
    fn enhance_unresolved_import(diagnostic: &mut Diagnostic, _text: &str) {
        diagnostic.message.push_str("\n\nTip: Install the package using `using Pkg; Pkg.add(\"PackageName\")` or add it to your Project.toml.");
    }
    
    /// Add context to diagnostic message
    fn add_context(diagnostic: &mut Diagnostic, text: &str) {
        let line_num = diagnostic.range.start.line as usize;
        let start_char = diagnostic.range.start.character as usize;
        
        // Get surrounding lines for context
        let context_lines: Vec<&str> = text
            .lines()
            .enumerate()
            .filter(|(i, _)| {
                let line_idx = *i;
                line_idx >= line_num.saturating_sub(2) && line_idx <= line_num + 2
            })
            .map(|(_, line)| line)
            .collect();
        
        if !context_lines.is_empty() {
            // Add a note about the context
            if context_lines.len() > 1 {
                diagnostic.message.push_str("\n\nContext:");
                for (i, line) in context_lines.iter().enumerate() {
                    let actual_line = line_num.saturating_sub(2) + i;
                    if actual_line == line_num {
                        // Highlight the current line
                        diagnostic.message.push_str(&format!("\n  {}: {}", actual_line + 1, line));
                        if start_char < line.len() {
                            let indicator = " ".repeat(start_char + 8) + "^";
                            diagnostic.message.push_str(&format!("\n  {}", indicator));
                        }
                    } else {
                        diagnostic.message.push_str(&format!("\n  {}: {}", actual_line + 1, line));
                    }
                }
            }
        }
    }
    
    /// Extract variable name from diagnostic message
    fn extract_variable_name(message: &str) -> Option<&str> {
        // Look for patterns like "Undefined variable: `name`" or "`name`"
        if let Some(start) = message.find('`') {
            if let Some(end) = message[start + 1..].find('`') {
                return Some(&message[start + 1..start + 1 + end]);
            }
        }
        None
    }
    
    /// Find similar symbol in symbol table
    fn find_similar_in_index(name: &str, index: &Index) -> Option<String> {
        // This is a simplified implementation
        // In a full implementation, we'd iterate through all symbols
        // and find the closest match using string distance
        
        // For now, try prefix matching
        let symbols = index.find_symbols_with_prefix(name);
        if !symbols.is_empty() {
            return Some(symbols[0].name.clone());
        }
        None
    }
    
    /// Add related information to diagnostic
    #[allow(dead_code)]
    pub fn add_related_info(
        diagnostic: &mut Diagnostic,
        location: Location,
        message: String,
    ) {
        if diagnostic.related_information.is_none() {
            diagnostic.related_information = Some(Vec::new());
        }
        
        if let Some(ref mut related) = diagnostic.related_information {
            related.push(crate::types::DiagnosticRelatedInformation {
                location,
                message,
            });
        }
    }
    
    /// Refine severity based on context
    pub fn refine_severity(diagnostic: &mut Diagnostic, _text: &str) {
        // Some diagnostics might be downgraded from Error to Warning
        // based on context
        
        if let Some(ref code) = diagnostic.code {
            match code.as_str() {
                "unused_variable" => {
                    // Unused variables are warnings, not errors
                    if diagnostic.severity == Some(DiagnosticSeverity::Error) {
                        diagnostic.severity = Some(DiagnosticSeverity::Warning);
                    }
                }
                "unresolved_import" => {
                    // Unresolved imports are warnings (might be available at runtime)
                    if diagnostic.severity == Some(DiagnosticSeverity::Error) {
                        diagnostic.severity = Some(DiagnosticSeverity::Warning);
                    }
                }
                _ => {}
            }
        }
    }
}


