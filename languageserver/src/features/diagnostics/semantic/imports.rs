use crate::types::ImportContext;
use crate::pipeline::storage::Index;
use crate::pipeline::query::symbol::SymbolQuery;
use crate::pipeline::sources::project_context::ManifestToml;
use crate::types::{Diagnostic, DiagnosticSeverity, Position, Range};
use std::path::Path;
use tree_sitter::Node;

use super::utils;

/// Check import/module resolution (enhanced with Index)
pub(super) fn check_import_resolution(
    tree: &tree_sitter::Tree,
    text: &str,
    index: &Index,
    _import_context: Option<&ImportContext>,
    diagnostics: &mut Vec<Diagnostic>,
    depot_path: Option<&Path>,
    manifest: Option<&ManifestToml>,
) {
    let root = tree.root_node();
    check_imports_recursive(root, text, index, _import_context, diagnostics, depot_path, manifest);
}

pub(super) fn check_imports_recursive(
    node: Node,
    text: &str,
    index: &Index,
    _import_context: Option<&ImportContext>,
    diagnostics: &mut Vec<Diagnostic>,
    depot_path: Option<&Path>,
    manifest: Option<&ManifestToml>,
) {
    match node.kind() {
        "using_statement" | "import_statement" => {
            // Collect all module names from comma-separated list
            let mut module_names = Vec::new();
            
            // Walk through children to collect module names
            // Handle comma-separated lists by processing each module separately
            let mut current_module_parts = Vec::new();
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    match child.kind() {
                        "," => {
                            // Comma indicates end of current module, start of next
                            // Build module name from accumulated parts
                            if !current_module_parts.is_empty() {
                                let module_name = current_module_parts.join(".");
                                if !module_name.is_empty() {
                                    module_names.push((module_name, child.start_position()));
                                }
                                current_module_parts.clear();
                            }
                        }
                        "identifier" => {
                            if let Ok(name) = child.utf8_text(text.as_bytes()) {
                                let name_str = name.to_string();
                                if name_str != "using" && name_str != "import" {
                                    current_module_parts.push(name_str);
                                }
                            }
                        }
                        "field_access" | "field_expression" => {
                            // Qualified module name replaces any accumulated parts
                            if let Ok(qualified) = child.utf8_text(text.as_bytes()) {
                                if !current_module_parts.is_empty() {
                                    // If we have accumulated parts, this qualified name is separate
                                    let module_name = current_module_parts.join(".");
                                    if !module_name.is_empty() {
                                        module_names.push((module_name, child.start_position()));
                                    }
                                    current_module_parts.clear();
                                }
                                module_names.push((qualified.to_string(), child.start_position()));
                            }
                        }
                        _ => {}
                    }
                }
            }
            
            // Don't forget the last module if there's no trailing comma
            if !current_module_parts.is_empty() {
                let module_name = current_module_parts.join(".");
                if !module_name.is_empty() {
                    // Use the position of the last identifier if available
                    let last_pos = node.end_position();
                    module_names.push((module_name, last_pos));
                }
            }
            
            // If we didn't find any modules using the comma-separated approach,
            // fall back to the old method (for single module statements)
            if module_names.is_empty() {
                let mut module_name = String::new();
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        match child.kind() {
                            "identifier" => {
                                if let Ok(name) = child.utf8_text(text.as_bytes()) {
                                    if name != "using" && name != "import" {
                                        if module_name.is_empty() {
                                            module_name = name.to_string();
                                        } else {
                                            module_name.push('.');
                                            module_name.push_str(name);
                                        }
                                    }
                                }
                            }
                            "field_access" | "field_expression" => {
                                if let Ok(qualified) = child.utf8_text(text.as_bytes()) {
                                    module_name = qualified.to_string();
                                }
                            }
                            _ => {}
                        }
                    }
                }
                if !module_name.is_empty() {
                    module_names.push((module_name, node.start_position()));
                }
            }
            
            // Check each module individually
            for (module_name, _pos) in module_names {
                // Check if module exists in Index (primary check - this is the source of truth)
                let module_functions = index.get_module_functions(&module_name);
                let module_types = index.get_module_types(&module_name);
                let module_exists = !module_functions.is_empty() || !module_types.is_empty();
                
                // Debug logging to help diagnose missing packages
                if !module_exists {
                    log::trace!("Import check for '{}': not found in Index (functions: {}, types: {})", 
                        module_name, module_functions.len(), module_types.len());
                    
                    // Log what modules ARE in the index (for debugging)
                    let all_modules = index.get_all_modules();
                    if all_modules.len() < 20 {
                        log::trace!("Available modules in Index: {:?}", all_modules);
                    } else {
                        log::trace!("Available modules in Index: {} modules (first 10: {:?})", 
                            all_modules.len(), 
                            all_modules.iter().take(10).collect::<Vec<_>>());
                    }
                }
                
                // Check if module is in symbol table (workspace module)
                let symbol_query = SymbolQuery::new(index);
                let in_workspace = symbol_query.find_symbol(&module_name).is_some();
                
                // Check if it's a standard library module
                let is_stdlib = utils::is_stdlib_module(&module_name);
                
                // Fallback: Check if package exists in depot (only if not in Index)
                // This prevents false warnings for packages that are installed but not indexed
                // (e.g., not listed as direct dependencies in Manifest.toml)
                let exists_in_depot = if !module_exists && !in_workspace && !is_stdlib {
                    if let Some(depot) = depot_path {
                        let exists = crate::pipeline::sources::indexing::resolve_package_path(depot, &module_name, manifest).is_some();
                        if exists {
                            log::trace!("Import check for '{}': not in Index but found in depot (not indexed - may not be a direct dependency)", module_name);
                        }
                        exists
                    } else {
                        false
                    }
                } else {
                    false
                };
                
                if !module_exists && !in_workspace && !is_stdlib && !exists_in_depot {
                    // Package not found - warn
                    // Find the actual range for this specific module in the source
                    let range = find_module_range_in_statement(node, text, &module_name)
                        .unwrap_or_else(|| Range {
                            start: Position::from(node.start_position()),
                            end: Position::from(node.end_position()),
                        });
                    
                    diagnostics.push(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::Warning),
                        code: Some("unresolved_import".to_string()),
                        source: Some("semantic".to_string()),
                        message: format!("Package `{}` may not be available. Ensure it is installed and indexed.", module_name),
                        related_information: None,
                    });
                }
            }
        }
        _ => {}
    }
    
    // Recursively process children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            check_imports_recursive(child, text, index, _import_context, diagnostics, depot_path, manifest);
        }
    }
}

/// Find the range of a specific module name within a using/import statement
pub(super) fn find_module_range_in_statement(statement_node: Node, text: &str, module_name: &str) -> Option<Range> {
    // Search for the module name in the statement's children
    for i in 0..statement_node.child_count() {
        if let Some(child) = statement_node.child(i) {
            match child.kind() {
                "identifier" => {
                    if let Ok(name) = child.utf8_text(text.as_bytes()) {
                        if name == module_name {
                            return Some(Range {
                                start: Position::from(child.start_position()),
                                end: Position::from(child.end_position()),
                            });
                        }
                    }
                }
                "field_access" | "field_expression" => {
                    if let Ok(qualified) = child.utf8_text(text.as_bytes()) {
                        if qualified == module_name {
                            return Some(Range {
                                start: Position::from(child.start_position()),
                                end: Position::from(child.end_position()),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
    None
}

