use crate::pipeline::types::ParsedItem;
use crate::types::{FunctionSignature, Parameter};
use crate::types::{LspError, Range, Position};
use crate::pipeline::sources::indexing::extract_docstrings_with_function_names;
use tree_sitter::Node;
use std::collections::HashMap;

/// Analyze a parsed item to extract function signatures
/// Uses docstring-first approach: extract all docstrings and derive function names from them,
/// then match to function definitions for additional metadata (parameters, types, etc.)
pub fn analyze(parsed: &ParsedItem) -> Result<Vec<FunctionSignature>, LspError> {
    let root = parsed.tree.root_node();
    let text = parsed.text.as_str();

    // Try to infer module name from file path (package name)
    let default_module = infer_module_name_from_path(&parsed.path);
    
    // Step 1: Extract all docstrings and derive function names from them (docstring-first approach)
    let docstring_map = extract_docstrings_with_function_names(root, text);
    
    // Step 2: Extract function signatures from AST (for parameters, types, etc.)
    let mut signatures_from_ast = Vec::new();
    walk_node(&root, text, &parsed.path.to_string_lossy(), &default_module, &mut signatures_from_ast)?;
    
    // Step 3: Match docstrings to function signatures
    // Create a map of function signature indices by module.name for quick lookup
    let mut sig_map: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, sig) in signatures_from_ast.iter().enumerate() {
        let key = format!("{}.{}", sig.module, sig.name);
        sig_map.entry(key).or_default().push(idx);
    }
    
    // Step 4: Apply docstrings to matching signatures
    for (func_name, docstring) in &docstring_map {
        // Try to find matching signature(s)
        // First try exact match with module prefix (e.g., "CSV.read" matches module="CSV", name="read")
        if let Some(indices) = sig_map.get(func_name as &str) {
            // Apply docstring to all matching signatures (in case of multiple methods)
            for &idx in indices {
                if signatures_from_ast[idx].doc_comment.is_none() {
                    signatures_from_ast[idx].doc_comment = Some(docstring.clone());
                }
            }
        } else {
            // If func_name is qualified (e.g., "CSV.read"), try to match by splitting into module and name
            if let Some(dot_pos) = func_name.rfind('.') {
                let module_part = &func_name[..dot_pos];
                let name_part = &func_name[dot_pos + 1..];
                
                // Try exact match with extracted module and name
                let key = format!("{}.{}", module_part, name_part);
                if let Some(indices) = sig_map.get(&key) {
                    for &idx in indices {
                        if signatures_from_ast[idx].doc_comment.is_none() {
                            signatures_from_ast[idx].doc_comment = Some(docstring.clone());
                        }
                    }
                } else {
                    // Try matching just the name in the extracted module
                    // This handles cases where the module name matches but signature wasn't found with full key
                    for (key, indices) in &sig_map {
                        if key.starts_with(&format!("{}.", module_part)) && key.ends_with(&format!(".{}", name_part)) {
                            for &idx in indices {
                                if signatures_from_ast[idx].doc_comment.is_none() {
                                    signatures_from_ast[idx].doc_comment = Some(docstring.clone());
                                }
                            }
                        }
                    }
                    
                    // Also try matching by name only in any module (fallback)
                    // This handles cases where module inference was wrong
                    for (key, indices) in &sig_map {
                        if key.ends_with(&format!(".{}", name_part)) {
                            // Prefer matches in the same file or with matching module prefix
                            for &idx in indices {
                                let sig = &signatures_from_ast[idx];
                                // Check if module matches or if it's in the same file
                                if (sig.module == module_part || sig.file_uri == parsed.path.to_string_lossy())
                                    && sig.doc_comment.is_none() {
                                    signatures_from_ast[idx].doc_comment = Some(docstring.clone());
                                    break; // Only apply to first matching signature
                                }
                            }
                        }
                    }
                }
            } else {
                // No module prefix - try to match with any module (bare function name)
                let name = func_name;
                for (key, indices) in &sig_map {
                    if key.ends_with(&format!(".{}", name)) {
                        for &idx in indices {
                            if signatures_from_ast[idx].doc_comment.is_none() {
                                signatures_from_ast[idx].doc_comment = Some(docstring.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Step 5: For docstrings that don't match any existing signature, create new signatures
    // This handles cases where docstrings exist but function definitions weren't found
    for (func_name, docstring) in &docstring_map {
        let key = func_name.clone();
        if !sig_map.contains_key(&key) {
            // Extract module and name from func_name
            // IMPORTANT: Never use "Base" from docstring - always use default_module from path
            // Docstrings may reference "Base.function" in examples, but that doesn't mean
            // the function is in Base - it's just referencing Base's version
            let (module, name) = if let Some(dot_pos) = func_name.rfind('.') {
                let module_part = &func_name[..dot_pos];
                let name_part = &func_name[dot_pos + 1..];
                // Only use module_part if it's not "Base" and matches the inferred module
                // Otherwise, use default_module to avoid storing functions under "Base" incorrectly
                if module_part == "Base" {
                    // Never store under Base - use default_module from path instead
                    (default_module.clone(), name_part.to_string())
                } else if module_part == default_module || default_module.ends_with(&format!(".{}", module_part)) {
                    // Module matches inferred module - use it
                    (module_part.to_string(), name_part.to_string())
                } else {
                    // Module doesn't match - use default_module to avoid wrong module assignment
                    (default_module.clone(), name_part.to_string())
                }
            } else {
                // No module prefix - use default module
                (default_module.clone(), func_name.clone())
            };
            
            // Create a minimal signature from the docstring
            let sig = FunctionSignature {
                module: module.clone(),
                name: name.clone(),
                parameters: Vec::new(), // Parameters would need to be parsed from docstring signature
                return_type: None,
                doc_comment: Some(docstring.clone()),
                file_uri: parsed.path.to_string_lossy().to_string(),
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
            };
            signatures_from_ast.push(sig);
        }
    }

    Ok(signatures_from_ast)
}

/// Infer module name from file path
/// For package files like "CSV/src/CSV.jl", extract "CSV"
/// For files like "Base/filesystem.jl", extract "Base"
/// For files in subdirectories like "packages/Flux/.../src/losses/functions.jl", extract "Losses"
fn infer_module_name_from_path(path: &std::path::Path) -> String {
    // Try to extract from path components
    // Common patterns:
    // - packages/CSV/{uuid}/src/CSV.jl -> CSV
    // - packages/DataFrames/{uuid}/src/DataFrames.jl -> DataFrames
    // - packages/Flux/{uuid}/src/losses/functions.jl -> Losses (from subdirectory)
    // - Base/filesystem.jl -> Base
    
    let path_str = path.to_string_lossy();
    
    // Check if path contains "packages/{PackageName}/" or "packages\{PackageName}\"
    // Handle both Unix and Windows paths
    let packages_pos = path_str.find("packages/").or_else(|| path_str.find("packages\\"));
    if let Some(packages_pos) = packages_pos {
        let after_packages = &path_str[packages_pos + 9..]; // Skip "packages/" or "packages\"
        // Try both forward and back slashes to find package name
        let slash_pos = after_packages.find('/').or_else(|| after_packages.find('\\'));
        if let Some(slash_pos) = slash_pos {
            let package_name = &after_packages[..slash_pos];
            
            // Check if file is in a subdirectory (e.g., packages/Flux/.../src/losses/functions.jl)
            // Look for "/src/" or "\src\" pattern and check if there's a subdirectory after it
            // Handle both Unix and Windows paths
            let src_pos = path_str.find("/src/")
                .or_else(|| path_str.find("\\src\\"));
            if let Some(src_pos) = src_pos {
                // Both "/src/" and "\src\" are 5 characters, so skip 5
                let after_src = &path_str[src_pos + 5..];
                // Get the directory containing the file (if any)
                // Try both forward and back slashes
                let file_name_pos = after_src.rfind('/').or_else(|| after_src.rfind('\\'));
                if let Some(file_name_pos) = file_name_pos {
                    let dir_path = &after_src[..file_name_pos];
                    if !dir_path.is_empty() {
                        // Extract directory name and capitalize first letter
                        // Handle both forward and back slashes
                        let dir_name = dir_path.split('/').last()
                            .or_else(|| dir_path.split('\\').last())
                            .unwrap_or(dir_path);
                        if !dir_name.is_empty() && dir_name != package_name {
                            let mut chars = dir_name.chars();
                            if let Some(first) = chars.next() {
                                let capitalized = format!("{}{}", first.to_uppercase(), chars.as_str());
                                // Return as "PackageName.SubModule" (e.g., "Flux.Losses")
                                return format!("{}.{}", package_name, capitalized);
                            }
                        }
                    }
                }
            }
            
            return package_name.to_string();
        }
    }
    
    // Check if path contains "base/" or "base\" (case-insensitive, both slashes)
    // This handles both Unix and Windows paths
    let path_lower = path_str.to_lowercase();
    if path_lower.contains("/base/") || path_lower.contains("\\base\\") {
        return "Base".to_string();
    }
    
    // Check for Core
    if path_lower.contains("/core/") || path_lower.contains("\\core\\") {
        return "Core".to_string();
    }
    
    // Check for other stdlib modules (case-insensitive)
    for stdlib_module in ["Statistics", "LinearAlgebra"] {
        let module_lower = stdlib_module.to_lowercase();
        if path_lower.contains(&format!("/{}/", module_lower)) || 
           path_lower.contains(&format!("\\{}\\", module_lower)) {
            return stdlib_module.to_string();
        }
    }
    
    // Fallback: try to use filename without extension
    if let Some(file_stem) = path.file_stem() {
        if let Some(stem_str) = file_stem.to_str() {
            // Capitalize first letter (Julia module convention)
            let mut chars = stem_str.chars();
            if let Some(first) = chars.next() {
                return format!("{}{}", first.to_uppercase(), chars.as_str());
            }
        }
    }
    
    "Main".to_string()
}

fn walk_node(
    node: &Node,
    text: &str,
    file_uri: &str,
    module_name: &str,
    signatures: &mut Vec<FunctionSignature>,
) -> Result<(), LspError> {
    // Handle module definitions - update module context
    if node.kind() == "module_definition" {
        if let Some(name_node) = find_first_child_of_type(node, "identifier") {
            if let Ok(module_name_str) = name_node.utf8_text(text.as_bytes()) {
                // If the module name from the file matches the inferred module name from path,
                // use just the file's module name (avoid creating "CSV.CSV" when path says "CSV" and file has "module CSV")
                let nested_module = if module_name.is_empty() || module_name == "Main" {
                    module_name_str.to_string()
                } else if module_name == module_name_str {
                    // Module name matches inferred name - use just the module name from file
                    module_name_str.to_string()
                } else {
                    format!("{}.{}", module_name, module_name_str)
                };
                
                // Walk children with new module context
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        walk_node(&child, text, file_uri, &nested_module, signatures)?;
                    }
                }
                return Ok(());
            }
        }
    }
    
    if node.kind() == "function_definition" {
        if let Some(sig) = extract_function_signature(node, text, file_uri, module_name)? {
            signatures.push(sig);
        }
    } else if node.kind() == "macro_definition" {
        // Handle macro definitions: macro name(args...) ... end
        if let Some(sig) = extract_macro_signature(node, text, file_uri, module_name)? {
            signatures.push(sig);
        }
    } else if node.kind() == "assignment" {
        // Handle short-form function definitions: f(x) = y
        // Also handle operators: (op)(args...) = y
        if let Some(left_node) = node.child(0) {
            if left_node.kind() == "call_expression" {
                if let Some(sig) = extract_short_form_signature(&left_node, node, text, file_uri, module_name)? {
                    signatures.push(sig);
                }
            }
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_node(&child, text, file_uri, module_name, signatures)?;
        }
    }

    Ok(())
}

fn extract_function_signature(
    node: &Node,
    text: &str,
    file_uri: &str,
    module_name: &str,
) -> Result<Option<FunctionSignature>, LspError> {
    // Find function name - it's in: function_definition -> signature -> call_expression
    // The function name can be either:
    // 1. An identifier (e.g., "describe")
    // 2. A field_access/field_expression (e.g., "DataAPI.describe")
    let (name, qualified_module) = if let Some(signature_node) = find_first_child_of_type(node, "signature") {
        if let Some(call_node) = find_first_child_of_type(&signature_node, "call_expression") {
            // Check if function name is a qualified access (Module.function)
            if let Some(field_node) = find_first_child_of_type(&call_node, "field_access") {
                // Extract qualified name like "DataAPI.describe"
                if let Ok(qualified_name) = field_node.utf8_text(text.as_bytes()) {
                    let parts: Vec<&str> = qualified_name.split('.').collect();
                    if parts.len() == 2 {
                        let module_part = parts[0];
                        let name_part = parts[1];
                        (name_part.to_string(), Some(module_part.to_string()))
                    } else {
                        // Fallback: use last part as name
                        (parts.last().unwrap_or(&"").to_string(), None)
                    }
                } else {
                    return Ok(None);
                }
            } else if let Some(field_node) = find_first_child_of_type(&call_node, "field_expression") {
                // Alternative field expression format
                if let Ok(qualified_name) = field_node.utf8_text(text.as_bytes()) {
                    let parts: Vec<&str> = qualified_name.split('.').collect();
                    if parts.len() == 2 {
                        let module_part = parts[0];
                        let name_part = parts[1];
                        (name_part.to_string(), Some(module_part.to_string()))
                    } else {
                        (parts.last().unwrap_or(&"").to_string(), None)
                    }
                } else {
                    return Ok(None);
                }
            } else if let Some(name_node) = find_first_child_of_type(&call_node, "identifier") {
                // Simple identifier (unqualified function name)
                (name_node.utf8_text(text.as_bytes())
                    .map_err(|e| LspError::ParseError(format!("Failed to extract function name: {}", e)))?
                    .to_string(), None)
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    let mut parameters = Vec::new();
    let return_type = None;

    // Find parameter list - it's in: function_definition -> signature -> call_expression -> argument_list
    if let Some(signature_node) = find_first_child_of_type(node, "signature") {
        if let Some(call_node) = find_first_child_of_type(&signature_node, "call_expression") {
            if let Some(param_list) = find_first_child_of_type(&call_node, "argument_list") {
                for i in 0..param_list.child_count() {
                    if let Some(param_node) = param_list.child(i) {
                        if param_node.kind() == "identifier" {
                            let param_name = param_node.utf8_text(text.as_bytes())
                                .map_err(|e| LspError::ParseError(format!("Failed to extract parameter name: {}", e)))?
                                .to_string();

                            parameters.push(Parameter {
                                name: param_name,
                                param_type: None, // Type inference would be done separately
                            });
                        }
                    }
                }
            }
        }
    }

    let range = node_to_range(*node);
    // Docstrings will be matched from docstring-first extraction, not extracted here
    let doc_comment = None;

    // Use the module name from the qualified function definition if present (e.g., DataAPI.describe -> DataAPI)
    // Otherwise use the module name from context (extracted from file path or module definitions)
    // IMPORTANT: Never use "Base" from qualified_module unless we're actually in a Base file
    // Functions like "Base.crossentropy(...)" in package files are extending Base, not defining Base functions
    let module = if let Some(ref qualified_mod) = qualified_module {
        // Only use qualified_module if it's not "Base" or if module_name indicates we're in Base
        if qualified_mod == "Base" && !module_name.contains("Base") && module_name != "Base" {
            // This is a Base extension in a non-Base file - use the file's module instead
            if module_name.is_empty() {
                "Main".to_string()
            } else {
                module_name.to_string()
            }
        } else {
            qualified_mod.clone()
        }
    } else if module_name.is_empty() {
        "Main".to_string()
    } else {
        module_name.to_string()
    };

    let sig = FunctionSignature {
        module: module.clone(),
        name: name.clone(),
        parameters,
        return_type,
        doc_comment,
        file_uri: file_uri.to_string(),
        range,
    };
    
    // Log function signature extraction for debugging
    if module != "Main" && !module.is_empty() {
        log::trace!("SignatureAnalyzer: Extracted function '{}' in module '{}' from file {:?}", 
            name, module, file_uri);
    }
    
    Ok(Some(sig))
}

fn find_first_child_of_type<'a>(node: &'a Node<'a>, kind: &str) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == kind {
                return Some(child);
            }
        }
    }
    None
}

fn node_to_range(node: Node) -> Range {
    let start_pos = node.start_position();
    let end_pos = node.end_position();

    Range {
        start: Position {
            line: start_pos.row as u32,
            character: start_pos.column as u32,
        },
        end: Position {
            line: end_pos.row as u32,
            character: end_pos.column as u32,
        },
    }
}

/// Extract function signature from short-form definition: f(x) = y
fn extract_short_form_signature(
    call_node: &Node,
    assignment_node: &Node,
    text: &str,
    file_uri: &str,
    module_name: &str,
) -> Result<Option<FunctionSignature>, LspError> {
    // Extract function name and module from call_expression
    // Handle both regular functions: f(x) = y
    // and operators: (op)(args...) = y
    let (name, qualified_module) = {
        // Check if function name is a qualified access (Module.function)
        if let Some(field_node) = find_first_child_of_type(call_node, "field_access") {
            if let Ok(qualified_name) = field_node.utf8_text(text.as_bytes()) {
                let parts: Vec<&str> = qualified_name.split('.').collect();
                if parts.len() == 2 {
                    (parts[1].to_string(), Some(parts[0].to_string()))
                } else {
                    (parts.last().unwrap_or(&"").to_string(), None)
                }
            } else {
                return Ok(None);
            }
        } else if let Some(field_node) = find_first_child_of_type(call_node, "field_expression") {
            if let Ok(qualified_name) = field_node.utf8_text(text.as_bytes()) {
                let parts: Vec<&str> = qualified_name.split('.').collect();
                if parts.len() == 2 {
                    (parts[1].to_string(), Some(parts[0].to_string()))
                } else {
                    (parts.last().unwrap_or(&"").to_string(), None)
                }
            } else {
                return Ok(None);
            }
        } else if let Some(paren_expr) = find_first_child_of_type(call_node, "parenthesized_expression") {
            // Handle operators: (op)(args...) = y
            // The operator is inside the parenthesized_expression
            if let Some(op_node) = find_first_child_of_type(&paren_expr, "operator") {
                (op_node.utf8_text(text.as_bytes())
                    .map_err(|e| LspError::ParseError(format!("Failed to extract operator name: {}", e)))?
                    .to_string(), None)
            } else {
                return Ok(None);
            }
        } else if let Some(name_node) = find_first_child_of_type(call_node, "identifier") {
            (name_node.utf8_text(text.as_bytes())
                .map_err(|e| LspError::ParseError(format!("Failed to extract function name: {}", e)))?
                .to_string(), None)
        } else {
            return Ok(None);
        }
    };

    // Extract parameters from argument_list
    let mut parameters = Vec::new();
    if let Some(param_list) = find_first_child_of_type(call_node, "argument_list") {
        for i in 0..param_list.child_count() {
            if let Some(param_node) = param_list.child(i) {
                if param_node.kind() == "identifier" {
                    let param_name = param_node.utf8_text(text.as_bytes())
                        .map_err(|e| LspError::ParseError(format!("Failed to extract parameter name: {}", e)))?
                        .to_string();
                    parameters.push(Parameter {
                        name: param_name,
                        param_type: None,
                    });
                }
            }
        }
    }

    let range = node_to_range(*assignment_node);
    // Docstrings will be matched from docstring-first extraction, not extracted here
    let doc_comment = None;

    // Use the module name from the qualified function definition if present
    // IMPORTANT: Never use "Base" from qualified_module unless we're actually in a Base file
    let module = if let Some(ref qualified_mod) = qualified_module {
        if qualified_mod == "Base" && !module_name.contains("Base") && module_name != "Base" {
            // This is a Base extension in a non-Base file - use the file's module instead
            if module_name.is_empty() {
                "Main".to_string()
            } else {
                module_name.to_string()
            }
        } else {
            qualified_mod.clone()
        }
    } else if module_name.is_empty() {
        "Main".to_string()
    } else {
        module_name.to_string()
    };

    let sig = FunctionSignature {
        module: module.clone(),
        name: name.clone(),
        parameters,
        return_type: None,
        doc_comment,
        file_uri: file_uri.to_string(),
        range,
    };
    
    if module != "Main" && !module.is_empty() {
        log::trace!("SignatureAnalyzer: Extracted short-form function '{}' in module '{}' from file {:?}", 
            name, module, file_uri);
    }
    
    Ok(Some(sig))
}

/// Extract function signature from macro definition: macro name(args...) ... end
fn extract_macro_signature(
    node: &Node,
    text: &str,
    file_uri: &str,
    module_name: &str,
) -> Result<Option<FunctionSignature>, LspError> {
    // Find macro name - it's in: macro_definition -> signature -> call_expression -> identifier
    // Similar structure to function definitions
    let (name, qualified_module) = if let Some(signature_node) = find_first_child_of_type(node, "signature") {
        if let Some(call_node) = find_first_child_of_type(&signature_node, "call_expression") {
            // Check if macro name is a qualified access (Module.macro)
            if let Some(field_node) = find_first_child_of_type(&call_node, "field_access") {
                if let Ok(qualified_name) = field_node.utf8_text(text.as_bytes()) {
                    let parts: Vec<&str> = qualified_name.split('.').collect();
                    if parts.len() == 2 {
                        (parts[1].to_string(), Some(parts[0].to_string()))
                    } else {
                        (parts.last().unwrap_or(&"").to_string(), None)
                    }
                } else {
                    return Ok(None);
                }
            } else if let Some(field_node) = find_first_child_of_type(&call_node, "field_expression") {
                if let Ok(qualified_name) = field_node.utf8_text(text.as_bytes()) {
                    let parts: Vec<&str> = qualified_name.split('.').collect();
                    if parts.len() == 2 {
                        (parts[1].to_string(), Some(parts[0].to_string()))
                    } else {
                        (parts.last().unwrap_or(&"").to_string(), None)
                    }
                } else {
                    return Ok(None);
                }
            } else if let Some(name_node) = find_first_child_of_type(&call_node, "identifier") {
                // Simple identifier (unqualified macro name)
                (name_node.utf8_text(text.as_bytes())
                    .map_err(|e| LspError::ParseError(format!("Failed to extract macro name: {}", e)))?
                    .to_string(), None)
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    // Extract parameters from argument_list (same as functions)
    let mut parameters = Vec::new();
    if let Some(signature_node) = find_first_child_of_type(node, "signature") {
        if let Some(call_node) = find_first_child_of_type(&signature_node, "call_expression") {
            if let Some(param_list) = find_first_child_of_type(&call_node, "argument_list") {
                for i in 0..param_list.child_count() {
                    if let Some(param_node) = param_list.child(i) {
                        // Handle regular parameters
                        if param_node.kind() == "identifier" {
                            let param_name = param_node.utf8_text(text.as_bytes())
                                .map_err(|e| LspError::ParseError(format!("Failed to extract macro parameter name: {}", e)))?
                                .to_string();
                            parameters.push(Parameter {
                                name: param_name,
                                param_type: None,
                            });
                        } else if param_node.kind() == "splat_expression" {
                            // Handle splat parameters like `args...`
                            if let Some(splat_id) = find_first_child_of_type(&param_node, "identifier") {
                                let param_name = splat_id.utf8_text(text.as_bytes())
                                    .map_err(|e| LspError::ParseError(format!("Failed to extract splat parameter name: {}", e)))?
                                    .to_string();
                                parameters.push(Parameter {
                                    name: format!("{}...", param_name),
                                    param_type: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    let range = node_to_range(*node);
    // Docstrings will be matched from docstring-first extraction, not extracted here
    let doc_comment = None;

    // Use the module name from the qualified macro definition if present
    // IMPORTANT: Never use "Base" from qualified_module unless we're actually in a Base file
    let module = if let Some(ref qualified_mod) = qualified_module {
        if qualified_mod == "Base" && !module_name.contains("Base") && module_name != "Base" {
            // This is a Base extension in a non-Base file - use the file's module instead
            if module_name.is_empty() {
                "Main".to_string()
            } else {
                module_name.to_string()
            }
        } else {
            qualified_mod.clone()
        }
    } else if module_name.is_empty() {
        "Main".to_string()
    } else {
        module_name.to_string()
    };

    // For macros, prepend @ to the name to match Julia convention
    let macro_name = if name.starts_with('@') {
        name.clone()
    } else {
        format!("@{}", name)
    };

    let sig = FunctionSignature {
        module: module.clone(),
        name: macro_name.clone(),
        parameters,
        return_type: None,
        doc_comment,
        file_uri: file_uri.to_string(),
        range,
    };
    
    if module != "Main" && !module.is_empty() {
        log::trace!("SignatureAnalyzer: Extracted macro '{}' in module '{}' from file {:?}", 
            macro_name, module, file_uri);
    }
    
    Ok(Some(sig))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::parser;
    use crate::pipeline::sources::file::FileSource;
    use std::path::PathBuf;

    fn parse_code(code: &str) -> ParsedItem {
        let source = FileSource::from_content(PathBuf::from("test.jl"), code.to_string());
        parser::parse(&source).unwrap()
    }

    #[test]
    fn test_analyze_function_signature() {
        let code = "function test(x, y) return x + y end";
        let parsed = parse_code(code);
        let signatures = analyze(&parsed).unwrap();

        assert_eq!(signatures.len(), 1);
        assert_eq!(signatures[0].name, "test");
        assert_eq!(signatures[0].parameters.len(), 2);
        assert_eq!(signatures[0].parameters[0].name, "x");
        assert_eq!(signatures[0].parameters[1].name, "y");
    }

    #[test]
    fn test_analyze_no_parameters() {
        let code = "function test() return 42 end";
        let parsed = parse_code(code);
        let signatures = analyze(&parsed).unwrap();

        assert_eq!(signatures.len(), 1);
        assert_eq!(signatures[0].name, "test");
        assert_eq!(signatures[0].parameters.len(), 0);
    }
}

