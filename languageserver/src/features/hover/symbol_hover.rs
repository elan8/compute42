use crate::pipeline::storage::Index;
use crate::types::SymbolKind;
use super::helpers::extract_assignment_value;

/// Build hover content for a function symbol
pub fn build_function_hover(symbol: &crate::types::Symbol, has_julia_docs: bool) -> String {
    let mut content = String::new();
    
    // Show function signature
    if let Some(sig) = &symbol.signature {
        if !has_julia_docs {
            content.push_str("```julia\n");
            content.push_str(sig);
            content.push_str("\n```\n\n");
        }
    }
    
    // Add location info with clickable file link
    if !has_julia_docs {
        let file_path = std::path::Path::new(&symbol.file_uri);
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&symbol.file_uri);
        let line_number = symbol.range.start.line + 1; // Convert to 1-based line number
        
        // Format file URI properly for clickable links
        // If file_uri is already a URI (starts with file://), use it as-is
        // Otherwise, convert path to URI format
        let file_link = if symbol.file_uri.starts_with("file://") {
            format!("{}:{}", symbol.file_uri, line_number)
        } else {
            // Convert path to file:// URI, handling Windows paths
            let path = std::path::Path::new(&symbol.file_uri);
            let uri_path = if cfg!(windows) {
                // On Windows, convert backslashes to forward slashes and add leading slash
                path.to_string_lossy().replace('\\', "/")
            } else {
                path.to_string_lossy().to_string()
            };
            format!("file:///{}:{}", uri_path, line_number)
        };
        
        content.push_str(&format!(
            "*Defined in [{}:{}]({})*\n\n",
            file_name, line_number, file_link
        ));
    }
    
    // Add doc comment if available
    if let Some(doc) = &symbol.doc_comment {
        if !has_julia_docs {
            content.push_str("---\n\n");
        }
        content.push_str(doc);
        content.push_str("\n\n");
    }
    
    content
}

/// Build hover content for a type, constant, or macro symbol
pub fn build_type_constant_macro_hover(
    symbol: &crate::types::Symbol,
    symbol_name: &str,
    has_julia_docs: bool,
) -> String {
    let mut content = String::new();
    
    if !has_julia_docs {
        let kind_str = match symbol.kind {
            SymbolKind::Type => "Type",
            SymbolKind::Constant => "Constant",
            SymbolKind::Macro => "Macro",
            _ => "Symbol",
        };
        content.push_str(&format!("```julia\n{}\n```\n\n", symbol_name));
        let file_path = std::path::Path::new(&symbol.file_uri);
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&symbol.file_uri);
        let line_number = symbol.range.start.line + 1; // Convert to 1-based line number
        
        // Format file URI properly for clickable links
        let file_link = if symbol.file_uri.starts_with("file://") {
            format!("{}:{}", symbol.file_uri, line_number)
        } else {
            let path = std::path::Path::new(&symbol.file_uri);
            let uri_path = if cfg!(windows) {
                path.to_string_lossy().replace('\\', "/")
            } else {
                path.to_string_lossy().to_string()
            };
            format!("file:///{}:{}", uri_path, line_number)
        };
        
        content.push_str(&format!(
            "*{} defined in [{}:{}]({})*\n\n",
            kind_str, file_name, line_number, file_link
        ));
    }
    
    if let Some(doc) = &symbol.doc_comment {
        if !has_julia_docs {
            content.push_str("---\n\n");
        }
        content.push_str(doc);
        content.push_str("\n\n");
    }
    
    content
}

/// Build hover content for a module symbol
pub fn build_module_hover(symbol: &crate::types::Symbol, symbol_name: &str, has_julia_docs: bool) -> String {
    let mut content = String::new();
    
    if !has_julia_docs {
        content.push_str(&format!("```julia\nmodule {}\n```\n\n", symbol_name));
        let file_path = std::path::Path::new(&symbol.file_uri);
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&symbol.file_uri);
        let line_number = symbol.range.start.line + 1; // Convert to 1-based line number
        
        // Format file URI properly for clickable links
        let file_link = if symbol.file_uri.starts_with("file://") {
            format!("{}:{}", symbol.file_uri, line_number)
        } else {
            let path = std::path::Path::new(&symbol.file_uri);
            let uri_path = if cfg!(windows) {
                path.to_string_lossy().replace('\\', "/")
            } else {
                path.to_string_lossy().to_string()
            };
            format!("file:///{}:{}", uri_path, line_number)
        };
        
        content.push_str(&format!(
            "*Defined in [{}:{}]({})*\n\n",
            file_name, line_number, file_link
        ));
    }
    
    if let Some(doc) = &symbol.doc_comment {
        if !has_julia_docs {
            content.push_str("---\n\n");
        }
        content.push_str(doc);
        content.push_str("\n\n");
    }
    
    content
}

/// Build hover content for a variable symbol
pub fn build_variable_hover(
    symbol: &crate::types::Symbol,
    symbol_name: &str,
    def_node: Option<tree_sitter::Node>,
    _tree: &tree_sitter::Tree,
    text: &str,
    _index: &Index,
) -> String {
    let mut content = String::new();
    
    // Show assignment value if available
    if let Some(def_node) = def_node {
        if let Some(rhs) = extract_assignment_value(def_node, text) {
            let rendered = format!("```julia\n{} = {}\n```\n\n", symbol_name, rhs);
            content.push_str(&rendered);
        } else {
            let rendered = format!("`{}`\n\n", symbol_name);
            content.push_str(&rendered);
        }
    } else {
        // No definition node - this might be a function parameter
        // Don't show the name again, just show location info below
    }
    
    // Add location info with clickable file link for variables
    let file_path = std::path::Path::new(&symbol.file_uri);
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&symbol.file_uri);
    let line_number = symbol.range.start.line + 1; // Convert to 1-based line number
    
    // Format file URI properly for clickable links
    let file_link = if symbol.file_uri.starts_with("file://") {
        format!("{}:{}", symbol.file_uri, line_number)
    } else {
        let path = std::path::Path::new(&symbol.file_uri);
        let uri_path = if cfg!(windows) {
            path.to_string_lossy().replace('\\', "/")
        } else {
            path.to_string_lossy().to_string()
        };
        format!("file:///{}:{}", uri_path, line_number)
    };
    
    content.push_str(&format!(
        "*Defined in [{}:{}]({})*\n\n",
        file_name, line_number, file_link
    ));
    
    content
}

