use crate::pipeline::sources::{Document, BaseDocsRegistry};
use crate::pipeline::storage::CacheManager;
use crate::pipeline::storage::Index;
use crate::types::{Position, SymbolKind};
use tree_sitter::Node;
use regex::Regex;
use super::symbol_hover::{
    build_function_hover, build_type_constant_macro_hover, build_module_hover, build_variable_hover,
};
use super::variable_analysis::infer_variable_type;
use super::helpers::{extract_assignment_info, find_prior_assignment_in_scope, find_definition_assignment_node, is_function_call};

/// Clean and normalize documentation formatting
/// Removes metadata markers, normalizes whitespace, and ensures consistent formatting
fn clean_documentation(doc: &str) -> String {
    let mut cleaned = doc.to_string();
    
    // Remove metadata markers like $METADATA_FIXED
    cleaned = cleaned.replace("$METADATA_FIXED", "");
    cleaned = cleaned.replace("$METADATA", "");
    
    // Normalize section headers - ensure consistent markdown heading levels
    // Keep original heading levels but normalize spacing
    cleaned = Regex::new(r"(?m)^(#{1,6})\s+").unwrap()
        .replace_all(&cleaned, "$1 ")
        .to_string();
    
    // Normalize code blocks - ensure consistent language tags and formatting
    // Ensure julia code blocks have proper formatting
    cleaned = Regex::new(r"```\s*julia\s*\n").unwrap()
        .replace_all(&cleaned, "```julia\n")
        .to_string();
    
    // Normalize spacing - ensure single blank line between sections
    // Replace multiple blank lines (3+) with double blank line
    cleaned = Regex::new(r"\n{3,}").unwrap()
        .replace_all(&cleaned, "\n\n")
        .to_string();
    
    // Normalize note/warning callouts - ensure consistent formatting
    cleaned = Regex::new(r"(?m)^!!!\s*(note|warning|tip|danger)\s*\n").unwrap()
        .replace_all(&cleaned, "!!! $1\n")
        .to_string();
    
    // Remove trailing whitespace from lines
    cleaned = cleaned.lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    
    // Trim overall whitespace
    cleaned.trim().to_string()
}

/// Build hover content for a symbol, prioritizing documentation from Index and package docs
/// NOTE: BaseDocsRegistry parameter removed - Index now contains all Base/stdlib documentation
pub async fn build_hover_content<'a>(
    symbol: Option<&crate::types::Symbol>,
    symbol_name: &str,
    node: Node<'a>,
    tree: &'a tree_sitter::Tree,
    text: &str,
    document: &Document,
    _position: Position,
    index: &Index,
    _cache: Option<&CacheManager>,
    _base_docs: Option<&BaseDocsRegistry>, // Kept for API compatibility but no longer used
    package_docs: Option<&std::collections::HashMap<String, BaseDocsRegistry>>,
) -> (String, bool) {
    let mut content = String::new();
    let mut has_julia_docs = false;
    
    log::trace!("build_hover_content: symbol_name={}, has_symbol={}", 
        symbol_name, symbol.is_some());

    // Try to get documentation from Index first (for indexed packages)
    // For qualified names (e.g., "CSV.read"), use find_function_by_qualified_name first
    let doc = if symbol_name.contains('.') {
        // Qualified name - try find_function_by_qualified_name first
        if let Some(signatures) = index.find_function_by_qualified_name(symbol_name) {
            // Get documentation from first signature with a docstring
            signatures.iter()
                .find_map(|sig| {
                    sig.doc_comment.as_ref()
                        .filter(|doc_str| !doc_str.trim().is_empty())
                        .cloned()
                })
        } else {
            // Fallback to get_documentation for qualified names
            index.get_documentation(symbol_name)
        }
    } else {
        // Unqualified name - try get_documentation first, then cross-module search
        index.get_documentation(symbol_name)
    };
    
    // If that fails and symbol_name doesn't contain '.', try searching across all modules
    // This handles cases where functions are used without module prefix (e.g., "describe" instead of "DataFrames.describe")
    // Try to determine preferred modules from import context in the document
    // NOTE: We extract preferred modules here for Index lookup, but we'll extract again later for package docs
    // This is fine - it's a cheap operation
    let preferred_modules_for_index = extract_preferred_modules_from_document(document, tree, text);
    let doc = doc.or_else(|| {
        if !symbol_name.contains('.') {
            // Convert Vec<String> to Vec<&str> for the function call
            let preferred_strs: Option<Vec<&str>> = preferred_modules_for_index.as_ref().map(|v| v.iter().map(|s| s.as_str()).collect());
            index.find_documentation_by_name(symbol_name, preferred_strs.as_deref())
        } else {
            None
        }
    });
    
    if let Some(doc) = doc {
        if !doc.trim().is_empty() {
            let cleaned_doc = clean_documentation(&doc);
            content.push_str(&cleaned_doc);
            content.push_str("\n\n");
            has_julia_docs = true;
        }
    }

    // NOTE: BaseDocsRegistry fallback removed - Index should contain all Base/stdlib documentation
    // The improved get_documentation() and find_documentation_by_name() functions now search across
    // all modules, so BaseDocsRegistry is no longer needed as a fallback
    
    // Fallback to package docs for external packages (backward compatibility)
    // NOTE: Package data should now be in the Index, so this fallback is rarely needed
    // For external packages (e.g., CSV.read, DataFrames.select)
    if !has_julia_docs {
        if let Some(package_docs) = package_docs {
            // For qualified names (e.g., "CSV.read"), use multi-tier lookup strategy
            if symbol_name.contains('.') {
                if let Some(dot_pos) = symbol_name.rfind('.') {
                    let module_name = &symbol_name[..dot_pos];
                    let func_name = &symbol_name[dot_pos + 1..];
                    
                    // Strategy 1: Direct qualified lookup across all packages
                    for (_package_name, registry) in package_docs.iter() {
                        if let Some(doc) = registry.get_documentation(symbol_name) {
                            let cleaned_doc = clean_documentation(&doc);
                            content.push_str(&cleaned_doc);
                            content.push_str("\n\n");
                            has_julia_docs = true;
                            break;
                        }
                    }
                    
                    // Strategy 2: Module+name lookup across all packages (if strategy 1 failed)
                    if !has_julia_docs {
                        for (_package_name, registry) in package_docs.iter() {
                            if let Some(doc) = registry.get_documentation_by_module(module_name, func_name) {
                                let cleaned_doc = clean_documentation(&doc);
                                content.push_str(&cleaned_doc);
                                content.push_str("\n\n");
                                has_julia_docs = true;
                                break;
                            }
                        }
                    }
                    
                    // Strategy 3: Try with package name as module (CSV package → CSV module)
                    // If module_name matches a package name, try that package's registry
                    if !has_julia_docs {
                        if let Some(registry) = package_docs.get(module_name) {
                            if let Some(doc) = registry.get_documentation_by_module(module_name, func_name) {
                                let cleaned_doc = clean_documentation(&doc);
                                content.push_str(&cleaned_doc);
                                content.push_str("\n\n");
                                has_julia_docs = true;
                            } else {
                                // Also try bare function name in this package
                                if let Some(doc) = registry.get_documentation(func_name) {
                                    let cleaned_doc = clean_documentation(&doc);
                                    content.push_str(&cleaned_doc);
                                    content.push_str("\n\n");
                                    has_julia_docs = true;
                                }
                            }
                        }
                    }
                }
            } else {
                // For unqualified names, search all package registries
                // Try to use import context to prioritize packages
                let preferred_modules = extract_preferred_modules_from_document(document, tree, text);
                
                // First try preferred modules (from imports)
                if let Some(ref preferred) = preferred_modules {
                    for module_name in preferred {
                        if let Some(registry) = package_docs.get(module_name) {
                            // Try module+name lookup first (searches submodules too) - important for functions in submodules
                            if let Some(doc) = registry.get_documentation_by_module(module_name, symbol_name) {
                                let cleaned_doc = clean_documentation(&doc);
                                content.push_str(&cleaned_doc);
                                content.push_str("\n\n");
                                has_julia_docs = true;
                                break;
                            }
                            // Try bare name (searches across all modules in the package)
                            if let Some(doc) = registry.get_documentation(symbol_name) {
                                let cleaned_doc = clean_documentation(&doc);
                                content.push_str(&cleaned_doc);
                                content.push_str("\n\n");
                                has_julia_docs = true;
                                break;
                            }
                            // Also try qualified name (e.g., "DataFrames.select")
                            let qualified = format!("{}.{}", module_name, symbol_name);
                            if let Some(doc) = registry.get_documentation(&qualified) {
                                let cleaned_doc = clean_documentation(&doc);
                                content.push_str(&cleaned_doc);
                                content.push_str("\n\n");
                                has_julia_docs = true;
                                break;
                            }
                        }
                    }
                }
                
                // Fallback: search all packages (if preferred modules didn't work)
                if !has_julia_docs {
                    for (package_name, registry) in package_docs.iter() {
                        // Try module+name lookup with package name as module first (searches submodules too)
                        // This is important for functions like "select" which are in "DataFrames.Selection"
                        if let Some(doc) = registry.get_documentation_by_module(package_name, symbol_name) {
                            let cleaned_doc = clean_documentation(&doc);
                            content.push_str(&cleaned_doc);
                            content.push_str("\n\n");
                            has_julia_docs = true;
                            break;
                        }
                        // Try bare name (searches across all modules in the package)
                        if let Some(doc) = registry.get_documentation(symbol_name) {
                            let cleaned_doc = clean_documentation(&doc);
                            content.push_str(&cleaned_doc);
                            content.push_str("\n\n");
                            has_julia_docs = true;
                            break;
                        }
                        // Also try qualified name (e.g., "DataFrames.select") as fallback
                        let qualified = format!("{}.{}", package_name, symbol_name);
                        if let Some(doc) = registry.get_documentation(&qualified) {
                            let cleaned_doc = clean_documentation(&doc);
                            content.push_str(&cleaned_doc);
                            content.push_str("\n\n");
                            has_julia_docs = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    // If we have a user-defined symbol, add local context
    // BUT: If we have a qualified function name (e.g., "CSV.read") and found Julia docs,
    // don't add module hover content - we want function docs, not module docs
    if let Some(symbol) = symbol {
        // Check if this is a qualified function name - if so, prioritize function docs over module docs
        let is_qualified_function = symbol_name.contains('.') && has_julia_docs;
        
        match symbol.kind {
            SymbolKind::Function => {
                content.push_str(&build_function_hover(symbol, has_julia_docs));
            }
            SymbolKind::Type | SymbolKind::Constant | SymbolKind::Macro => {
                content.push_str(&build_type_constant_macro_hover(symbol, symbol_name, has_julia_docs));
            }
            SymbolKind::Variable => {
                let def_node = find_definition_assignment_node(tree, text, symbol);
                let var_content = build_variable_hover(
                    symbol,
                    symbol_name,
                    def_node,
                    tree,
                    text,
                    index,
                );
                content.push_str(&var_content);
            }
            SymbolKind::Module => {
                // If we have a qualified function name and found docs, skip module hover
                // The docs we found are for the function, not the module
                if !is_qualified_function {
                    content.push_str(&build_module_hover(symbol, symbol_name, has_julia_docs));
                }
            }
        }
    } else if !has_julia_docs {
        // No Julia docs and not in our symbol table
        // This might be a local variable, parameter, or function call
        // First check if this is a function call - if so, don't show assignment context
        if is_function_call(node) {
            // This is a function call, don't show assignment context
            // Just show the identifier name
            content.push_str(&format!("`{}`", symbol_name));
        } else {
            // Not a function call - might be a local variable or parameter
            // Try type inference first (for function parameters with type annotations)
            if let Some(type_info) = infer_variable_type(node, tree, text, index).await {
                // Check if this is a parameter by looking for function definition context
                let is_parameter = {
                    let mut current = node;
                    let mut found_function = false;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "function_definition" {
                            found_function = true;
                            break;
                        }
                        if parent.kind() == "module_definition" {
                            break;
                        }
                        current = parent;
                    }
                    found_function
                };
                
                if is_parameter {
                    content.push_str(&format!("```julia\n{}::{}\n```\n\n", symbol_name, type_info));
                    content.push_str("Parameter of function\n\n");
                } else {
                    content.push_str(&format!("```julia\n{}::{}\n```\n\n", symbol_name, type_info));
                }
            } else if let Some(assignment_info) = extract_assignment_info(node, tree, text) {
                content.push_str(&format!("```julia\n{} = {}\n```\n\n", symbol_name, assignment_info));
            } else if let Some(prior_value) = find_prior_assignment_in_scope(node, text, symbol_name) {
                content.push_str(&format!("```julia\n{} = {}\n```\n\n", symbol_name, prior_value));
            } else {
                // Check if this might be a function parameter without type annotation
                let is_parameter = {
                    let mut current = node;
                    let mut found_function = false;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "function_definition" {
                            found_function = true;
                            break;
                        }
                        if parent.kind() == "module_definition" {
                            break;
                        }
                        current = parent;
                    }
                    found_function
                };
                
                if is_parameter {
                    content.push_str(&format!("```julia\n{}\n```\n\n", symbol_name));
                    content.push_str("Parameter of function\n\n");
                } else {
                    // Last resort - just show the identifier
                    content.push_str(&format!("`{}`", symbol_name));
                }
            }
        }
    }

    (content, has_julia_docs)
}

/// Extract preferred modules from import/using statements in the document
/// This helps prioritize the correct module when a function name appears in multiple modules
fn extract_preferred_modules_from_document(
    _document: &Document,
    tree: &tree_sitter::Tree,
    text: &str,
) -> Option<Vec<String>> {
    let mut modules = Vec::new();
    let root = tree.root_node();
    
    // Walk the tree to find using/import statements
    fn collect_modules(node: tree_sitter::Node, text: &str, modules: &mut Vec<String>) {
        match node.kind() {
            "using_statement" | "import_statement" => {
                // Extract module names from the statement
                // Handle both "using A, B, C" and "using A.B.C" formats
                let mut current_module_parts = Vec::new();
                let mut after_colon = false;
                
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        match child.kind() {
                            "identifier" => {
                                if let Ok(name) = child.utf8_text(text.as_bytes()) {
                                    let name_str = name.to_string();
                                    if name_str != "using" && name_str != "import" && name_str != "as" {
                                        if after_colon {
                                            // This is a specific symbol, not a module
                                            continue;
                                        }
                                        current_module_parts.push(name_str);
                                    }
                                }
                            }
                            "." => {
                                // Dot indicates qualified module name (e.g., "Base.Filesystem")
                                // Continue building current module
                            }
                            ":" => {
                                after_colon = true;
                            }
                            "," => {
                                // Comma indicates end of current module
                                if !current_module_parts.is_empty() {
                                    let module_name = current_module_parts.join(".");
                                    if !module_name.is_empty() && !modules.contains(&module_name) {
                                        modules.push(module_name);
                                    }
                                    current_module_parts.clear();
                                }
                                after_colon = false;
                            }
                            _ => {}
                        }
                    }
                }
                
                // Add the last module if any
                if !current_module_parts.is_empty() {
                    let module_name = current_module_parts.join(".");
                    if !module_name.is_empty() && !modules.contains(&module_name) {
                        modules.push(module_name);
                    }
                }
            }
            _ => {}
        }
        
        // Recursively process children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                collect_modules(child, text, modules);
            }
        }
    }
    
    collect_modules(root, text, &mut modules);
    
    // Always include Base and Core as preferred modules for Base/stdlib functions
    // This ensures Base functions are found even if not explicitly imported
    if !modules.contains(&"Base".to_string()) {
        modules.insert(0, "Base".to_string());
    }
    if !modules.contains(&"Core".to_string()) {
        // Insert after Base if Base was added, otherwise at position 0 or 1
        let insert_pos = if modules.contains(&"Base".to_string()) { 1 } else { 0 };
        modules.insert(insert_pos, "Core".to_string());
    }
    
    if modules.is_empty() {
        None
    } else {
        Some(modules)
    }
}

