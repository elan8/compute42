//! Base/stdlib documentation and type inference extraction using tree-sitter
//! 
//! This module extracts documentation and type inference information from Julia's Base and stdlib source files
//! by parsing them directly with tree-sitter, without requiring a Julia process.
//! 
//! For Base documentation, we can also parse basedocs.jl directly, which is faster and more accurate
//! than parsing all the source files.

use std::collections::{HashSet, HashMap};
use std::path::Path;
use crate::types::LspError;


/// Parse exports.jl file to extract all exported symbols
/// 
/// Returns a set of symbol names (normalized - both bare names and Base.prefixed versions)
pub fn parse_exports_jl(path: &Path) -> Result<HashSet<String>, LspError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| LspError::InternalError(format!("Failed to read exports.jl: {}", e)))?;
    
    let mut symbols = HashSet::new();
    let mut in_export_block = false;
    let mut in_public_block = false;
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Check for export block start
        if trimmed == "export" {
            in_export_block = true;
            in_public_block = false;
            continue;
        }
        
        // Check for public block start
        if trimmed == "public" {
            in_public_block = true;
            in_export_block = false;
            continue;
        }
        
        // Skip comments (but don't end the block)
        if trimmed.starts_with('#') {
            continue;
        }
        
        // Extract symbols from lines
        if in_export_block || in_public_block {
            // Symbols are indented and end with commas
            // Remove trailing comma and whitespace
            let symbol_line = trimmed.trim_end_matches(',').trim();
            
            // Skip empty lines (they don't end the block in exports.jl)
            if symbol_line.is_empty() {
                continue;
            }
            
            // Split by comma to handle multiple symbols per line (though typically one per line)
            for symbol_str in symbol_line.split(',') {
                let symbol = symbol_str.trim();
                
                // Skip empty symbols
                if symbol.is_empty() {
                    continue;
                }
                
                // Handle special cases:
                // - Operators: !, !=, +, -, etc. (keep as-is)
                // - Macros: @show, @time, etc. (keep as-is)
                // - Unicode: π, √, ∈, etc. (keep as-is)
                // - Constants: VERSION, ARGS, etc. (keep as-is)
                // - Regular identifiers: joinpath, floor, etc.
                
                // Store both bare name and Base.prefixed version
                symbols.insert(symbol.to_string());
                symbols.insert(format!("Base.{}", symbol));
            }
        }
    }
    
    log::info!("BaseDocsExtraction: Parsed {} unique symbols from exports.jl", symbols.len());
    
    Ok(symbols)
}

/// Construct a full module path from module name, parent module name, and symbol name
/// Handles cases where module names are already fully qualified vs. relative
/// Note: This function is currently not used but kept for potential future use
#[allow(dead_code)]
fn construct_module_path(mod_name: &str, parent_module_name: &str, symbol_name: &str) -> String {
    if mod_name.is_empty() {
        // Top-level symbol in the parent module
        if parent_module_name.is_empty() {
            symbol_name.to_string()
        } else {
            format!("{}.{}", parent_module_name, symbol_name)
        }
    } else if mod_name.starts_with(parent_module_name) && !parent_module_name.is_empty() {
        // Module name already includes parent (e.g., "Base.Filesystem")
        format!("{}.{}", mod_name, symbol_name)
    } else {
        // Submodule name (e.g., "Filesystem") - prepend parent (e.g., "Base")
        if parent_module_name.is_empty() {
            format!("{}.{}", mod_name, symbol_name)
        } else {
            format!("{}.{}.{}", parent_module_name, mod_name, symbol_name)
        }
    }
}

/// Parse basedocs.jl file to extract Base documentation
/// 
/// Format: Triple-quoted docstrings followed by function names or signatures
/// Example:
/// ```julia
/// """
///     Documentation here
/// """
/// FunctionName
/// ```
/// 
/// Returns a map from symbol name to documentation string
pub fn parse_basedocs_jl(path: &Path) -> Result<HashMap<String, String>, LspError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| LspError::InternalError(format!("Failed to read basedocs.jl: {}", e)))?;
    
    let mut docs = HashMap::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    
    while i < lines.len() {
        // Look for triple-quoted string start
        if lines[i].trim().starts_with("\"\"\"") {
            // Collect the docstring (may span multiple lines)
            let mut doc_lines = Vec::new();
            
            // Handle opening """
            let first_line = lines[i].trim();
            if first_line == "\"\"\"" {
                // Empty first line, doc starts on next line
                i += 1;
            } else if first_line.starts_with("\"\"\"") && first_line.len() > 3 {
                // Doc starts on same line: """doc here
                let doc_content = &first_line[3..];
                doc_lines.push(doc_content);
                i += 1;
            } else {
                // Malformed, skip
                i += 1;
                continue;
            }
            
            // Collect docstring content until closing """
            while i < lines.len() {
                let line = lines[i];
                if line.trim() == "\"\"\"" || line.trim().ends_with("\"\"\"") {
                    // Found closing """
                    if line.trim() != "\"\"\"" {
                        // Doc ends on same line: ...doc"""
                        let end_pos = line.rfind("\"\"\"").unwrap();
                        let doc_content = &line[..end_pos];
                        if !doc_content.trim().is_empty() {
                            doc_lines.push(doc_content);
                        }
                    }
                    i += 1;
                    break;
                } else {
                    doc_lines.push(line);
                    i += 1;
                }
            }
            
            // Now look for the function name(s) on the following line(s)
            // Skip empty lines and comments
            while i < lines.len() {
                let line = lines[i].trim();
                
                // Skip empty lines
                if line.is_empty() {
                    i += 1;
                    continue;
                }
                
                // Skip comments
                if line.starts_with('#') {
                    i += 1;
                    continue;
                }
                
                // Skip module declarations
                if line.starts_with("module ") || line == "end" {
                    i += 1;
                    continue;
                }
                
                // Check if this is a function name or signature
                // Pattern: FunctionName or FunctionName(...) or kw"name" or Base.FunctionName
                if let Some(symbol_name) = extract_symbol_name_from_line(line) {
                    let doc_string = doc_lines.join("\n").trim().to_string();
                    if !doc_string.is_empty() {
                        // Store with bare name
                        docs.insert(symbol_name.clone(), doc_string.clone());
                        
                        // Also store with Base. prefix for qualified lookup
                        if !symbol_name.starts_with("Base.") {
                            docs.insert(format!("Base.{}", symbol_name), doc_string.clone());
                        }
                        
                        // Handle kw"name" pattern - store both with and without quotes
                        if symbol_name.starts_with("kw\"") && symbol_name.ends_with("\"") {
                            let bare_name = &symbol_name[3..symbol_name.len()-1];
                            docs.insert(bare_name.to_string(), doc_string);
                        }
                    }
                }
                
                // Move to next potential docstring
                i += 1;
                break;
            }
        } else {
            i += 1;
        }
    }
    
    log::info!("BaseDocsExtraction: Parsed {} documentation entries from basedocs.jl", docs.len());
    
    Ok(docs)
}

/// Extract symbol name from a line that follows a docstring
/// Handles patterns like:
/// - FunctionName
/// - FunctionName(...)
/// - kw"name"
/// - Base.FunctionName
/// - Base.FunctionName(...)
fn extract_symbol_name_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    
    // Handle kw"name" pattern
    if trimmed.starts_with("kw\"") && trimmed.contains('"') {
        if let Some(end_quote) = trimmed[3..].find('"') {
            let name = &trimmed[3..3+end_quote];
            return Some(format!("kw\"{}\"", name));
        }
    }
    
    // Handle function signature: FunctionName(...) or Base.FunctionName(...)
    if let Some(open_paren) = trimmed.find('(') {
        let name_part = &trimmed[..open_paren].trim();
        if !name_part.is_empty() {
            return Some(name_part.to_string());
        }
    }
    
    // Handle simple identifier: FunctionName or Base.FunctionName
    // Split by whitespace and take first token
    let first_token = trimmed.split_whitespace().next()?;
    
    // Check if it's a valid identifier (alphanumeric, _, !, with dots for qualified names)
    if first_token.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '!' || c == '.') 
        && !first_token.is_empty()
        && first_token.chars().next().map(|c| c.is_alphabetic() || c == '_' || c == '.').unwrap_or(false) {
        return Some(first_token.to_string());
    }
    
    None
}
