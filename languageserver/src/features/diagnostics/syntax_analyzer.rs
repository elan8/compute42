use crate::types::Range;

/// Analyze syntax error to provide more specific feedback
pub fn analyze_syntax_error(
    error_text: &str,
    content: &str,
    range: &Range,
    parent_kind: &str,
) -> String {
    // Heuristics based on full content and current line
    let current_line = content
        .lines()
        .nth(range.start.line as usize)
        .unwrap_or("")
        .trim();
    if current_line == "end" {
        return "Unexpected 'end' keyword. Check for matching block structure.".to_string();
    }

    // Unterminated string: odd number of quotes in document
    if content.matches('"').count() % 2 != 0 {
        return "Unmatched string delimiter".to_string();
    }

    // Unmatched delimiters based on presence without match ahead
    let tail_text = content
        .lines()
        .skip(range.start.line as usize)
        .collect::<Vec<&str>>()
        .join("\n");
    if current_line.contains('(') && !tail_text.contains(')') {
        return "Unmatched opening parenthesis".to_string();
    }
    if current_line.contains('[') && !tail_text.contains(']') {
        return "Unmatched opening bracket".to_string();
    }
    if current_line.contains('{') && !tail_text.contains('}') {
        return "Unmatched opening brace".to_string();
    }

    // Check for common patterns
    if error_text.contains("end") {
        return "Unexpected 'end' keyword. Check for matching block structure.".to_string();
    }
    
    if error_text.contains("function") || error_text.contains("if") || error_text.contains("for") {
        return format!(
            "Syntax error: unexpected keyword '{}' in {}",
            error_text.split_whitespace().next().unwrap_or(""),
            parent_kind
        );
    }
    
    if error_text.contains("=") {
        return "Syntax error near assignment operator. Check expression syntax.".to_string();
    }
    
    if error_text.contains("(") || error_text.contains(")") {
        return "Unmatched parenthesis or incorrect function call syntax.".to_string();
    }
    
    if error_text.contains("[") || error_text.contains("]") {
        return "Unmatched bracket or incorrect array syntax.".to_string();
    }
    
    if error_text.contains("{") || error_text.contains("}") {
        return "Unmatched brace or incorrect syntax.".to_string();
    }
    
    // Generic error with context
    if !error_text.is_empty() {
        let preview = if error_text.len() > 20 {
            format!("{}...", &error_text[..20])
        } else {
            error_text.to_string()
        };
        format!("Syntax error near '{}'", preview)
    } else {
        format!("Syntax error in {}", parent_kind)
    }
}



















