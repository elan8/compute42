mod helpers;
mod variable_analysis;
mod location_hints;
mod symbol_hover;
mod content_builder;

pub use helpers::{
    extract_assignment_value, find_prior_assignment_in_scope, extract_assignment_info,
    find_definition_assignment_node, type_of_value_like, is_function_call,
};
pub use variable_analysis::infer_variable_type;
pub use location_hints::location_sensitivity_hint;
pub use symbol_hover::{
    build_function_hover, build_type_constant_macro_hover, build_module_hover, build_variable_hover,
};
pub use content_builder::build_hover_content;

use crate::pipeline::sources::{Document, BaseDocsRegistry};
use crate::pipeline::query::SymbolResolver;
use crate::pipeline::storage::CacheManager;
use crate::pipeline::{storage::Index, query::symbol::SymbolQuery};
use crate::types::{HoverResult, Position};

/// Stateless hover provider - uses Index and query engine
pub struct HoverProvider;

impl HoverProvider {
    pub async fn hover(
        index: &Index,
        document: &Document,
        position: Position,
        cache: Option<&CacheManager>,
        base_docs: Option<&BaseDocsRegistry>,
        package_docs: Option<&std::collections::HashMap<String, BaseDocsRegistry>>,
    ) -> Option<HoverResult> {
        log::trace!("Hover: Processing request at {}:{}", position.line, position.character);

        // 1. Find node at position
        let tree = document.tree()?;

        let text = document.text();
        let resolver = SymbolResolver::new(tree, &text);
        let node = resolver.node_at_position(position.line, position.character)?;

        // 2. Extract symbol name - check if we're in a field_access first
        // Also check if we're in a using statement - if so, treat as external module
        let symbol_name = if node.kind() == "identifier" {
            // Check if this identifier is part of a field_access (e.g., "read" in "CSV.read")
            if let Some(parent) = node.parent() {
                if parent.kind() == "field_access" || parent.kind() == "field_expression" {
                    // Extract the full qualified name (e.g., "CSV.read")
                    if let Ok(qualified_name) = parent.utf8_text(text.as_bytes()) {
                        // Remove any trailing parentheses or arguments
                        let name = qualified_name.split('(').next().unwrap_or(qualified_name).trim();
                        name.to_string()
                    } else {
                        resolver.extract_symbol_name(node)?
                    }
                } else {
                    resolver.extract_symbol_name(node)?
                }
            } else {
                resolver.extract_symbol_name(node)?
            }
        } else {
            resolver.extract_symbol_name(node)?
        };
        
        // Check if this identifier is in a using statement - if so, it's an external module
        let is_in_using_statement = {
            let mut current = node;
            let mut found_using = false;
            while let Some(parent) = current.parent() {
                if parent.kind() == "using_statement" || parent.kind() == "import_statement" {
                    found_using = true;
                    break;
                }
                // Stop at function/module boundaries
                if parent.kind() == "function_definition" || parent.kind() == "module_definition" {
                    break;
                }
                current = parent;
            }
            found_using
        };

        // 3. Find definition using query engine with scope-aware resolution
        // If identifier is in a using statement, don't resolve as internal symbol
        let symbol_query = SymbolQuery::new(index);
        let symbol = if is_in_using_statement {
            // In using statement - treat as external module, don't resolve as internal
            None
        } else {
            symbol_query
                .resolve_symbol_at(&symbol_name, document.uri(), position)
                .or_else(|| symbol_query.find_symbol(&symbol_name))
        };

        // 4. Check if identifier is part of a field access (e.g., CSV in CSV.read)
        // If so, try to get function documentation first before treating as module
        let mut is_module_part = false;
        if node.kind() == "identifier" {
            if let Some(parent) = node.parent() {
                if parent.kind() == "field_access" || parent.kind() == "field_expression" {
                    // Check if this identifier is the left part (object) of the field access
                    // In field_access, the first child is the object, rest is the field
                    if let Some(first_child) = parent.child(0) {
                        if first_child.id() == node.id() {
                            // This identifier is the object part of a field access
                            // Check if it's a variable first
                            match &symbol {
                                Some(s) if s.kind == crate::types::SymbolKind::Variable => {
                                    // It's a variable - let normal variable handling proceed
                                    // Don't treat as module
                                }
                                Some(s) if s.kind == crate::types::SymbolKind::Module => {
                                    // Already a module - let normal module handling proceed
                                }
                                _ => {
                                    // Not found or other kind - likely a module/package
                                    // But first, try to get function documentation for the qualified name
                                    // Extract the full qualified name (e.g., "CSV.read")
                                    if parent.utf8_text(text.as_bytes()).is_ok() {
                                        // Try to find documentation for the qualified function name
                                        // This will be checked in build_hover_content, so we just mark it
                                        is_module_part = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // If this is the module part of a qualified call, try to get the function docs instead
        // We'll let build_hover_content handle it, but we need to extract the qualified name
        let final_symbol_name = if is_module_part && node.kind() == "identifier" {
            if let Some(parent) = node.parent() {
                if parent.kind() == "field_access" || parent.kind() == "field_expression" {
                    // Extract the full qualified name (e.g., "CSV.read")
                    if let Ok(qualified_name) = parent.utf8_text(text.as_bytes()) {
                        let qualified = qualified_name.split('(').next().unwrap_or(qualified_name).trim();
                        qualified.to_string()
                    } else {
                        symbol_name.clone()
                    }
                } else {
                    symbol_name.clone()
                }
            } else {
                symbol_name.clone()
            }
        } else {
            symbol_name.clone()
        };

        // 5. Build hover content (use final_symbol_name which may be qualified if hovering on module part)
        let (content, _has_julia_docs) = content_builder::build_hover_content(
            symbol.as_ref(),
            &final_symbol_name,
            node,
            tree,
            &text,
            document,
            position,
            index,
            cache,
            base_docs,
            package_docs,
        )
        .await;

        // If we have no content, log and return None instead of empty string
        if content.is_empty() {
            log::debug!("Hover: No content found for symbol '{}' at {}:{}", 
                symbol_name, position.line, position.character);
            return None;
        }
        
        log::trace!("Hover: Generated content for '{}' at {}:{} ({} chars)", 
            symbol_name, position.line, position.character, content.len());

        // Use symbol range if available, otherwise use node range
        let range = if let Some(symbol) = symbol {
            Some(symbol.range.clone())
        } else {
            Some(crate::types::Range {
                start: Position::from(node.start_position()),
                end: Position::from(node.end_position()),
            })
        };

        Some(HoverResult { contents: content, range })
    }
}

