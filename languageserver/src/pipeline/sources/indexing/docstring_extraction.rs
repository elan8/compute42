//! Docstring extraction and validation
//! 
//! This module handles extraction of docstrings from Julia source code.
//! It provides two approaches:
//! 1. Extract docstrings for specific nodes (function/type definitions)
//! 2. Extract all docstrings and derive function names from the docstrings themselves

// Range and LspError are not used in this file
use tree_sitter::Node;
use std::collections::HashMap;

/// Extract docstring from a node (function or type definition)
/// Looks for triple-quoted strings immediately before the definition
/// Handles docstrings separated by comments
/// IMPORTANT: Stops if it encounters another function/type definition to avoid picking wrong docstrings
/// Also validates that the docstring is actually for this function by checking function name
pub fn extract_docstring(node: Node, source: &str) -> Option<String> {
    // First, try to extract the function/type name from the node for validation
    let node_name = extract_node_name_for_validation(node, source);
    
    // Get the parent node to look for docstring as a sibling
    let parent = node.parent()?;
    
    // Look through parent's children to find a string literal before our node
    // Skip over comments to find docstrings that may be separated by comments
    // BUT: Stop if we encounter another function/type definition (to avoid wrong docstrings)
    let mut found_docstring: Option<String> = None;
    
    for i in 0..parent.child_count() {
        if let Some(child) = parent.child(i) {
            if child.id() == node.id() {
                // Found our node - validate and return the docstring we found (if any)
                if let Some(doc) = found_docstring {
                    // Validate that this docstring is for our function
                    if validate_docstring_for_node(&doc, &node_name, node, source) {
                        return Some(doc);
                    } else {
                        log::trace!("DocstringExtraction: Rejected docstring for '{}' - validation failed", 
                            node_name.as_ref().unwrap_or(&"<unknown>".to_string()));
                        return None;
                    }
                }
                return None;
            }
            
            // Skip comments but continue looking for docstrings
            if child.kind() == "comment" || child.kind() == "line_comment" {
                continue;
            }
            
            // CRITICAL: If we encounter another function or type definition before our node,
            // stop looking - any docstring we found belongs to that other definition, not ours
            if child.kind() == "function_definition" || 
               child.kind() == "struct_definition" || 
               child.kind() == "abstract_definition" ||
               child.kind() == "assignment" ||
               child.kind() == "macro_definition" {
                // Reset found_docstring - it belongs to the other definition
                found_docstring = None;
                continue;
            }
            
            // Check if this is a string literal (potential docstring)
            if child.kind() == "string" || child.kind() == "string_literal" {
                // Get the string content
                if let Ok(string_text) = child.utf8_text(source.as_bytes()) {
                    let trimmed = string_text.trim();
                    // Check if it's a triple-quoted string (docstring)
                    if trimmed.starts_with("\"\"\"") && trimmed.ends_with("\"\"\"") {
                        // Extract content between triple quotes
                        let content = &trimmed[3..trimmed.len().saturating_sub(3)];
                        // Only store if there's actual content (not just empty docstring)
                        if !content.trim().is_empty() {
                            found_docstring = Some(content.trim().to_string());
                            // Continue to check if there's another definition before our node
                        }
                    }
                }
            }
        }
    }
    
    // Alternative: look at the node's own children for docstring
    // Some parsers might include the docstring as a child
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "string" || child.kind() == "string_literal" {
                if let Ok(string_text) = child.utf8_text(source.as_bytes()) {
                    let trimmed = string_text.trim();
                    if trimmed.starts_with("\"\"\"") && trimmed.ends_with("\"\"\"") {
                        let content = &trimmed[3..trimmed.len().saturating_sub(3)];
                        if !content.trim().is_empty() {
                            let doc = content.trim().to_string();
                            // Validate docstring
                            if validate_docstring_for_node(&doc, &node_name, node, source) {
                                return Some(doc);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Also check previous sibling in the parent
    if let Some(parent) = node.parent() {
        let mut prev_sibling: Option<Node> = None;
        for i in 0..parent.child_count() {
            if let Some(child) = parent.child(i) {
                if child.id() == node.id() {
                    break;
                }
                // Skip other definitions - they would have their own docstrings
                if !matches!(child.kind(), "function_definition" | "struct_definition" | 
                            "abstract_definition" | "assignment" | "macro_definition") {
                    prev_sibling = Some(child);
                } else {
                    prev_sibling = None; // Reset if we hit another definition
                }
            }
        }
        
        if let Some(prev) = prev_sibling {
            if prev.kind() == "string" || prev.kind() == "string_literal" {
                if let Ok(string_text) = prev.utf8_text(source.as_bytes()) {
                    let trimmed = string_text.trim();
                    if trimmed.starts_with("\"\"\"") && trimmed.ends_with("\"\"\"") {
                        let content = &trimmed[3..trimmed.len().saturating_sub(3)];
                        if !content.trim().is_empty() {
                            let doc = content.trim().to_string();
                            // Validate docstring
                            if validate_docstring_for_node(&doc, &node_name, node, source) {
                                return Some(doc);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Check for @doc macro patterns (common in Plots.jl, Flux.jl)
    // Pattern: @doc "docstring" function_name(...) or @doc "docstring" -> function_name
    if let Some(parent) = node.parent() {
        // Check if parent is a macro_call with @doc
        if parent.kind() == "macro_call" || parent.kind() == "macrocall_expression" {
            if let Some(macro_name_node) = parent.child(0) {
                if let Ok(macro_name) = macro_name_node.utf8_text(source.as_bytes()) {
                    if macro_name.trim() == "@doc" {
                        // Extract docstring from @doc macro
                        // @doc "docstring" function_name(...)
                        for i in 1..parent.child_count() {
                            if let Some(arg_node) = parent.child(i) {
                                if arg_node.kind() == "string" || arg_node.kind() == "string_literal" {
                                    if let Ok(doc_text) = arg_node.utf8_text(source.as_bytes()) {
                                        let trimmed = doc_text.trim();
                                        // Remove quotes (single or triple)
                                        let doc_content = if trimmed.starts_with("\"\"\"") && trimmed.ends_with("\"\"\"") {
                                            &trimmed[3..trimmed.len().saturating_sub(3)]
                                        } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
                                            &trimmed[1..trimmed.len().saturating_sub(1)]
                                        } else {
                                            continue;
                                        };
                                        
                                        if !doc_content.trim().is_empty() {
                                            let doc = doc_content.trim().to_string();
                                            // Validate docstring
                                            if validate_docstring_for_node(&doc, &node_name, node, source) {
                                                return Some(doc);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Also check for pattern: function f(x) end\n"""doc"""\nf (docstring attached to identifier)
    if let Some(parent) = node.parent() {
        let mut found_self = false;
        let mut found_docstring: Option<String> = None;
        let mut found_identifier_after_docstring = false;
        
        for i in 0..parent.child_count() {
            if let Some(child) = parent.child(i) {
                if child.id() == node.id() {
                    found_self = true;
                    continue;
                }
                
                // Look for docstring after the function definition
                if found_self {
                    // Skip comments but continue looking for docstrings
                    if child.kind() == "comment" || child.kind() == "line_comment" {
                        continue;
                    }
                    
                    // Check if this is a string literal (potential docstring)
                    if child.kind() == "string" || child.kind() == "string_literal" {
                        if let Ok(string_text) = child.utf8_text(source.as_bytes()) {
                            let trimmed = string_text.trim();
                            if trimmed.starts_with("\"\"\"") && trimmed.ends_with("\"\"\"") {
                                let content = &trimmed[3..trimmed.len().saturating_sub(3)];
                                if !content.trim().is_empty() {
                                    found_docstring = Some(content.trim().to_string());
                                }
                            }
                        }
                    } else if child.kind() == "identifier" {
                        // Check if this identifier comes after a docstring we found
                        // Pattern: function f(x) end\n"""doc"""\nf
                        if found_docstring.is_some() {
                            // If we found a docstring and then an identifier, assume it's the pattern
                            // (We don't need to verify the name matches - if there's a docstring followed by identifier, use it)
                            found_identifier_after_docstring = true;
                            break;
                        } else {
                            // If we hit an identifier without a preceding docstring, stop looking
                            break;
                        }
                    } else {
                        // If we hit a non-comment, non-string, non-identifier node after the function,
                        // stop looking (docstring should be immediately after, possibly with comments)
                        if found_docstring.is_some() && !found_identifier_after_docstring {
                            // We found a docstring but no identifier after it, so use it
                            return found_docstring;
                        }
                        break;
                    }
                }
            }
        }
        
        // If we found a docstring followed by an identifier, return the docstring
        if found_docstring.is_some() && found_identifier_after_docstring {
            return found_docstring;
        } else if found_docstring.is_some() {
            // Found docstring but no identifier after - still use it
            return found_docstring;
        }
    }
    
    None
}

/// Extract function/type name from node for validation purposes
/// Returns None if name cannot be extracted
fn extract_node_name_for_validation(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "function_definition" => {
            // Try to extract name from signature
            if let Some(signature) = node.child(0) {
                // Look for call_expression in signature
                for i in 0..signature.child_count() {
                    if let Some(child) = signature.child(i) {
                        if child.kind() == "call_expression" {
                            // Get first child (identifier or field_access)
                            if let Some(name_node) = child.child(0) {
                                if name_node.kind() == "identifier" {
                                    return name_node.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
                                } else if name_node.kind() == "field_access" {
                                    // Extract qualified name
                                    return extract_field_access_name_simple(name_node, source);
                                }
                            }
                        }
                    }
                }
            }
            None
        }
        "assignment" => {
            // For assignments like f(x) = y, try to find call_expression on left side
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "call_expression" {
                        if let Some(name_node) = child.child(0) {
                            if name_node.kind() == "identifier" {
                                return name_node.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
                            }
                        }
                    }
                }
            }
            None
        }
        "struct_definition" | "abstract_definition" => {
            // Try to find identifier in first child (type name)
            if let Some(first_child) = node.child(0) {
                for i in 0..first_child.child_count() {
                    if let Some(child) = first_child.child(i) {
                        if child.kind() == "identifier" {
                            return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Simple helper to extract field access name (e.g., Base.joinpath)
fn extract_field_access_name_simple(node: Node, source: &str) -> Option<String> {
    let mut parts = Vec::new();
    let mut current = Some(node);
    
    while let Some(n) = current {
        if n.kind() == "field_access" {
            // Get the field name (last child)
            if let Some(field_node) = n.child(n.child_count().saturating_sub(1)) {
                if field_node.kind() == "identifier" {
                    if let Ok(field_name) = field_node.utf8_text(source.as_bytes()) {
                        parts.push(field_name.to_string());
                    }
                }
            }
            // Get the object (first child or previous)
            current = n.child(0);
        } else if n.kind() == "identifier" {
            if let Ok(name) = n.utf8_text(source.as_bytes()) {
                parts.push(name.to_string());
            }
            break;
        } else {
            break;
        }
    }
    
    if parts.is_empty() {
        None
    } else {
        Some(parts.into_iter().rev().collect::<Vec<_>>().join("."))
    }
}

/// Validate that a docstring is actually for the given node
/// This helps prevent wrong documentation matches (e.g., DataFrame showing groupindices docs,
/// or "display" getting "displayable" docstring)
fn validate_docstring_for_node(
    docstring: &str,
    node_name: &Option<String>,
    _node: Node,
    _source: &str,
) -> bool {
    // If we can't extract the node name, be lenient - accept the docstring
    let Some(ref name) = node_name else {
        return true;
    };
    
    let name_lower = name.to_lowercase();
    
    // Get first line and first few lines for analysis
    let first_line = docstring.lines().next().unwrap_or("").trim().to_lowercase();
    let first_lines: Vec<&str> = docstring.lines().take(3).collect();
    let first_lines_text = first_lines.join(" ").to_lowercase();
    
    // STRICT CHECK 1: Check if docstring starts with function name (most common pattern)
    // This is the strongest signal - Julia docstrings often start with the function name
    if first_line.starts_with(&name_lower) {
        // But check if it's actually a word boundary (not a substring)
        // e.g., "display" should match "display(" but not "displayable("
        let after_name = first_line.strip_prefix(&name_lower);
        if let Some(after) = after_name {
            // Check if what comes after is a word boundary (space, paren, newline, etc.)
            // CRITICAL: If the next character is alphanumeric or underscore, it's a substring match - reject!
            if let Some(next_char) = after.chars().next() {
                if next_char.is_alphanumeric() || next_char == '_' {
                    // This is a substring match (e.g., "display" in "displayable") - reject!
                    log::trace!("DocstringExtraction: Rejected docstring for '{}' - starts with '{}' but is a substring match", 
                        name, name_lower);
                    return false;
                }
            }
            // Valid word boundary - accept
            if after.is_empty() || 
               after.starts_with('(') || 
               after.starts_with(' ') || 
               after.starts_with('\n') ||
               after.starts_with('.') {
                return true;
            }
        }
    }
    
    // STRICT CHECK 2: Check for function signature pattern "name(...)" in first line
    // This is a strong signal that the docstring is for this function
    if first_line.contains(&format!("{}(", name_lower)) {
        // Verify it's not a substring match (e.g., "display" in "displayable")
        // Check if there's a word boundary before the name
        if let Some(pos) = first_line.find(&format!("{}(", name_lower)) {
            if pos == 0 || !first_line.chars().nth(pos.saturating_sub(1))
                .map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false) {
                return true;
            }
        }
    }
    
    // STRICT CHECK 3: Check for "function name" pattern
    if first_lines_text.contains(&format!("function {}", name_lower)) {
        return true;
    }
    
    // STRICT CHECK 4: Check for backtick-wrapped name pattern `name` or `Base.name`
    if first_lines_text.contains(&format!("`{}`", name_lower)) ||
       first_lines_text.contains(&format!("`base.{}`", name_lower)) {
        return true;
    }
    
    // STRICT CHECK 5: Check if another function name appears prominently that's different
    // This helps catch cases like "display" getting "displayable" docstring
    // Look for function signatures in the first line that mention OTHER functions
    // Extract function name from first line if it starts with a function signature
    // CRITICAL: This check must happen BEFORE any acceptance to catch wrong docstrings
    if first_line.contains('(') {
        // Try to extract the function name from the signature (e.g., "displayable(mime)" -> "displayable")
        if let Some(open_paren_pos) = first_line.find('(') {
            let potential_func_name = first_line[..open_paren_pos].trim().to_lowercase();
            // If we found a function name in the signature
            if !potential_func_name.is_empty() && potential_func_name != name_lower {
                // Check if this is a substring match issue (e.g., "display" vs "displayable")
                // If the docstring signature mentions a different function, it's likely wrong
                if potential_func_name.contains(&name_lower) || name_lower.contains(&potential_func_name) {
                    // This is likely the wrong docstring - the docstring is for a different but similar function
                    log::trace!("DocstringExtraction: Rejected docstring for '{}' - found different function name '{}' in signature", 
                        name, potential_func_name);
                    return false;
                }
                // Even if not a substring match, if the signature explicitly mentions a different function,
                // it's probably the wrong docstring (unless the docstring is about multiple functions)
                // But to be safe, we only reject substring matches here
            }
        }
    }
    
    // STRICT CHECK 5b: Also check if the first word of the first line is a different function name
    // This catches cases where the docstring starts with a function name that's not ours
    let first_word = first_line.split_whitespace().next().unwrap_or("").to_lowercase();
    if !first_word.is_empty() && first_word != name_lower && first_word.contains('(') {
        // Extract function name from first word if it's a function call pattern
        if let Some(open_paren_pos) = first_word.find('(') {
            let func_name_from_first_word = first_word[..open_paren_pos].trim().to_lowercase();
            if !func_name_from_first_word.is_empty() && func_name_from_first_word != name_lower {
                // Check for substring matches
                if func_name_from_first_word.contains(&name_lower) || name_lower.contains(&func_name_from_first_word) {
                    log::trace!("DocstringExtraction: Rejected docstring for '{}' - first word is different function '{}'", 
                        name, func_name_from_first_word);
                    return false;
                }
            }
        }
    } else if !first_word.is_empty() && first_word != name_lower {
        // First word is a different identifier (not a function call)
        // Check for substring matches (e.g., "display" vs "displayable")
        if first_word.contains(&name_lower) || name_lower.contains(&first_word) {
            log::trace!("DocstringExtraction: Rejected docstring for '{}' - first word '{}' is a substring match", 
                name, first_word);
            return false;
        }
    }
    
    // STRICT CHECK 6: For short docstrings, require explicit mention of function name
    if docstring.len() < 100 {
        // Short docstrings should explicitly mention the function name
        // Check for word boundaries to avoid substring matches
        // Simple word boundary check - look for exact word matches
        let has_exact_match = first_lines_text.split_whitespace()
            .any(|word| {
                let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                clean_word == name_lower
            });
        
        if !has_exact_match {
            log::trace!("DocstringExtraction: Rejected short docstring for '{}' - no explicit mention", name);
            return false;
        }
    }
    
    // STRICT CHECK 7: Check if docstring mentions the function name with word boundaries
    // This is more lenient than the strict checks above, but still requires word boundaries
    // to avoid substring matches
    let words: Vec<&str> = first_lines_text.split_whitespace().collect();
    for word in &words {
        let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if clean_word == name_lower {
            return true;
        }
    }
    
    // STRICT CHECK 8: For longer docstrings, if we haven't found a match yet,
    // check if there's a clear indication it's for a different function
    if docstring.len() >= 100 {
        // Look for function signature patterns that mention other functions
        // If the first line has a function signature that's not our function, reject
        if first_line.contains('(') && !first_line.contains(&name_lower) {
            // The first line has a function signature but doesn't mention our function
            // This is suspicious - likely the wrong docstring
            log::trace!("DocstringExtraction: Rejected docstring for '{}' - first line has different function signature", name);
            return false;
        }
        
        // For longer docstrings, be more lenient - if we haven't found a clear mismatch,
        // But still check if it explicitly mentions another function name
        // (This is a fallback - the main protection is the intervening definition check)
        return true;
    }
    
    // If we haven't found any positive signals and it's not a long docstring, reject
    log::trace!("DocstringExtraction: Rejected docstring for '{}' - no positive signals found", name);
    false
}

/// Extract all docstrings from source code and extract function names from the docstrings themselves
/// This is a more reliable approach than trying to match docstrings to functions
/// 
/// Returns a HashMap mapping function names to their docstrings
/// 
/// The function name is extracted from the first line of the docstring, which typically
/// contains the function signature (e.g., "display(x)" or "displayable(mime) -> Bool")
pub fn extract_docstrings_with_function_names(
    root: Node,
    source: &str,
) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut stack = vec![root];
    
    while let Some(node) = stack.pop() {
        // Check if this node is a string literal (potential docstring)
        if node.kind() == "string" || node.kind() == "string_literal" {
            if let Ok(string_text) = node.utf8_text(source.as_bytes()) {
                let trimmed = string_text.trim();
                // Check if it's a triple-quoted string (docstring)
                if trimmed.starts_with("\"\"\"") && trimmed.ends_with("\"\"\"") {
                    // Extract content between triple quotes
                    let content = &trimmed[3..trimmed.len().saturating_sub(3)];
                    if !content.trim().is_empty() {
                        // Extract function name from the docstring
                        if let Some((func_name, docstring)) = extract_function_name_from_docstring(content.trim()) {
                            // Store with the extracted name (which may be qualified like "CSV.read")
                            result.insert(func_name.clone(), docstring.clone());
                            
                            // If it's a qualified name (e.g., "CSV.read"), also store with bare name for lookup
                            // This allows matching both "CSV.read" and just "read" when the module context is known
                            if let Some(dot_pos) = func_name.rfind('.') {
                                let bare_name = &func_name[dot_pos + 1..];
                                if !bare_name.is_empty() {
                                    result.insert(bare_name.to_string(), docstring.clone());
                                }
                            }
                            
                            // Also store with Base. prefix for Base functions (legacy compatibility)
                            if !func_name.starts_with("Base.") && !func_name.contains('.') {
                                result.insert(format!("Base.{}", func_name), docstring);
                            }
                        }
                    }
                }
            }
        }
        
        // Add children to stack for traversal
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
    
    result
}

/// Validate that a docstring is likely correct for a function
/// Checks:
/// - Minimum length (at least 20 characters to avoid very short/obviously wrong docs)
/// - Mentions the function name (basic sanity check)
fn is_valid_docstring(docstring: &str, function_name: &str) -> bool {
    // Minimum length check - very short docstrings are likely wrong
    if docstring.trim().len() < 20 {
        return false;
    }
    
    // Extract bare function name (without module prefix) for checking
    let bare_name = function_name.split('.').next_back().unwrap_or(function_name);
    let doc_lower = docstring.to_lowercase();
    let name_lower = bare_name.to_lowercase();
    
    // Check if docstring mentions the function name (basic validation)
    // Allow variations: function name, `function name`, or qualified name
    doc_lower.contains(&name_lower) ||
    doc_lower.contains(&format!("`{}`", name_lower)) ||
    doc_lower.contains(function_name) ||
    // For qualified names, also check if bare name appears in context
    (function_name.contains('.') && doc_lower.contains(&format!("`{}`", bare_name)))
}

/// Extract function name from a docstring
/// The first line of a Julia docstring typically contains the function signature
/// Examples:
/// - "display(x)" -> "display"
/// - "displayable(mime) -> Bool" -> "displayable"
/// - "    displayable(mime) -> Bool" -> "displayable" (with indentation)
/// - "Base.Filesystem.joinpath(path::AbstractString, paths::AbstractString...) -> String" -> "Base.Filesystem.joinpath"
fn extract_function_name_from_docstring(docstring: &str) -> Option<(String, String)> {
    let first_line = docstring.lines().next()?.trim();
    
    // Pattern 1: Function signature with parentheses: "function_name(...)" or "Base.function_name(...)"
    if let Some(open_paren_pos) = first_line.find('(') {
        let name_part = first_line[..open_paren_pos].trim();
        if !name_part.is_empty() && is_valid_function_name(name_part) {
            let func_name = name_part.to_string();
            // Validate the docstring before returning
            if is_valid_docstring(docstring, &func_name) {
                return Some((func_name, docstring.to_string()));
            }
        }
    }
    
    // Pattern 2: Function name without parentheses: "function_name" or "Base.function_name"
    // This is less common but can happen
    let first_word = first_line.split_whitespace().next()?;
    if is_valid_function_name(first_word) {
        let func_name = first_word.to_string();
        // Validate the docstring before returning
        if is_valid_docstring(docstring, &func_name) {
            return Some((func_name, docstring.to_string()));
        }
    }
    
    // Pattern 3: Function signature with return type: "function_name(...) -> Type"
    // Extract name from before the arrow
    if let Some(arrow_pos) = first_line.find("->") {
        let before_arrow = first_line[..arrow_pos].trim();
        if let Some(open_paren_pos) = before_arrow.find('(') {
            let name_part = before_arrow[..open_paren_pos].trim();
            if !name_part.is_empty() && is_valid_function_name(name_part) {
                let func_name = name_part.to_string();
                // Validate the docstring before returning
                if is_valid_docstring(docstring, &func_name) {
                    return Some((func_name, docstring.to_string()));
                }
            }
        }
    }
    
    None
}

/// Check if a string is a valid function name
/// Valid names include:
/// - Simple identifiers: "display", "joinpath"
/// - Qualified names: "Base.display", "Base.Filesystem.joinpath"
/// - Operators: "==", "!=", "+", "-"
fn is_valid_function_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    
    // Allow qualified names (with dots)
    let parts: Vec<&str> = name.split('.').collect();
    for part in parts {
        // Each part should be a valid identifier
        if part.is_empty() {
            return false;
        }
        // Check if it's a valid identifier (alphanumeric, _, !, or operator characters)
        if !part.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '!' || 
                            "+-*/%^&|<>=".contains(c)) {
            return false;
        }
        // First character should be alphabetic, underscore, or operator
        if let Some(first_char) = part.chars().next() {
            if !first_char.is_alphabetic() && first_char != '_' && 
               !"+-*/%^&|<>=".contains(first_char) {
                return false;
            }
        }
    }
    
    true
}

