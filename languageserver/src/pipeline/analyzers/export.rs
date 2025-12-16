use crate::pipeline::types::ParsedItem;
use crate::types::LspError;
use tree_sitter::Node;
use std::collections::{HashSet, HashMap};

/// Analyze a parsed item to extract export statements with their module context
/// Returns a map of module name -> set of exported symbols
pub fn analyze(parsed: &ParsedItem) -> Result<HashMap<String, HashSet<String>>, LspError> {
    let mut exports_by_module: HashMap<String, HashSet<String>> = HashMap::new();
    let root = parsed.tree.root_node();
    let text = parsed.text.as_str();
    
    // Infer default module name from path (fallback)
    let default_module = infer_module_name_from_path(&parsed.path);
    
    walk_node(&root, text, &default_module, &mut exports_by_module)?;
    
    // If no module-specific exports were found, use the default module
    if exports_by_module.is_empty() {
        // Return empty map - exports will be empty set for default module
        // This maintains backward compatibility
        return Ok(HashMap::new());
    }
    
    Ok(exports_by_module)
}

/// Legacy function for backward compatibility - extracts exports without module context
/// This is used by code that expects a simple HashSet
pub fn analyze_legacy(parsed: &ParsedItem) -> Result<HashSet<String>, LspError> {
    let exports_by_module = analyze(parsed)?;
    let mut all_exports = HashSet::new();
    for exports in exports_by_module.values() {
        all_exports.extend(exports.iter().cloned());
    }
    Ok(all_exports)
}

fn infer_module_name_from_path(path: &std::path::Path) -> String {
    let path_str = path.to_string_lossy();
    
    // Check if path contains "packages/{PackageName}/"
    if let Some(packages_pos) = path_str.find("packages/") {
        let after_packages = &path_str[packages_pos + 9..]; // Skip "packages/"
        if let Some(slash_pos) = after_packages.find('/') {
            let package_name = &after_packages[..slash_pos];
            return package_name.to_string();
        }
    }
    
    "Main".to_string()
}

fn walk_node(
    node: &Node,
    text: &str,
    current_module: &str,
    exports_by_module: &mut HashMap<String, HashSet<String>>,
) -> Result<(), LspError> {
    // Handle module definitions - update module context
    if node.kind() == "module_definition" {
        if let Some(name_node) = find_first_child_of_type(node, "identifier") {
            if let Ok(module_name_str) = name_node.utf8_text(text.as_bytes()) {
                // If the module name from the file matches the inferred module name from path,
                // use just the file's module name (avoid creating "CSV.CSV" when path says "CSV" and file has "module CSV")
                let nested_module = if current_module.is_empty() || current_module == "Main" {
                    module_name_str.to_string()
                } else if current_module == module_name_str {
                    // Module name matches inferred name - use just the module name from file
                    module_name_str.to_string()
                } else {
                    format!("{}.{}", current_module, module_name_str)
                };
                
                // Walk children with new module context
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        walk_node(&child, text, &nested_module, exports_by_module)?;
                    }
                }
                return Ok(());
            }
        }
    }
    
    // Check for export statement - tree-sitter Julia uses "export" as the node type
    // Structure: export -> identifier (or comma-separated list of identifiers)
    if node.kind() == "export" || node.kind() == "export_statement" {
        let module_exports = exports_by_module
            .entry(current_module.to_string())
            .or_default();
        extract_exports(node, text, module_exports)?;
        // Don't recurse into export node children - we've already extracted them
        return Ok(());
    }

    // Recursively walk children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_node(&child, text, current_module, exports_by_module)?;
        }
    }

    Ok(())
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

fn extract_exports(
    node: &Node,
    text: &str,
    exports: &mut HashSet<String>,
) -> Result<(), LspError> {
    // Export statement structure: export -> identifier (or comma-separated list)
    // Example: export describe, select, dropmissing
    // Or: export DataAPI.describe (re-export)
    // The export node contains identifiers and field_access nodes
    
    // Walk all children to find exported symbols
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "identifier" => {
                    // Simple export: export describe
                    if let Ok(name) = child.utf8_text(text.as_bytes()) {
                        // Skip "export" keyword itself
                        if name != "export" {
                            exports.insert(name.to_string());
                            log::trace!("ExportAnalyzer: Found export '{}'", name);
                        }
                    }
                }
                "field_access" | "field_expression" => {
                    // Qualified export: export DataAPI.describe
                    // For re-exports, we want to track the unqualified name
                    if let Ok(qualified) = child.utf8_text(text.as_bytes()) {
                        let parts: Vec<&str> = qualified.split('.').collect();
                        if parts.len() == 2 {
                            // Export the unqualified name (e.g., "describe" from "DataAPI.describe")
                            exports.insert(parts[1].to_string());
                            log::trace!("ExportAnalyzer: Found re-export '{}' from '{}'", parts[1], parts[0]);
                        } else {
                            exports.insert(qualified.to_string());
                        }
                    }
                }
                "," => {
                    // Comma separator - continue
                    continue;
                }
                _ => {
                    // Recursively check children for nested identifiers
                    extract_exports(&child, text, exports)?;
                }
            }
        }
    }

    Ok(())
}

