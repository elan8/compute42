use crate::pipeline::storage::Index;
use crate::types::{Symbol, Position};
use std::path::PathBuf;

/// Query symbols from the index
pub struct SymbolQuery<'a> {
    index: &'a Index,
}

impl<'a> SymbolQuery<'a> {
    pub fn new(index: &'a Index) -> Self {
        Self { index }
    }

    /// Find symbols by exact name
    pub fn find_by_name(&self, name: &str) -> Vec<Symbol> {
        self.index.find_symbols(name)
    }

    /// Find first symbol by name (for backward compatibility)
    pub fn find_symbol(&self, name: &str) -> Option<Symbol> {
        self.index.find_symbol(name)
    }

    /// Resolve a symbol by name at a specific position within a file using lexical scopes
    /// This implements scope-aware resolution similar to SymbolTable::resolve_symbol_at
    pub fn resolve_symbol_at(&self, name: &str, file_uri: &str, position: Position) -> Option<Symbol> {
        let candidates = self.index.find_symbols(name);
        
        // Filter to same file
        let same_file: Vec<&Symbol> = candidates.iter()
            .filter(|s| s.file_uri == file_uri)
            .collect();
        
        if same_file.is_empty() {
            return None;
        }

        // Get scope tree for the file
        let file_path = PathBuf::from(file_uri);
        let scope_tree = self.index.get_file_scopes(&file_path)?;

        // Find scopes that contain the position
        // Choose the candidate whose scope is the most specific (deepest) that contains the position
        // IMPORTANT: We want the symbol defined in the most specific scope that contains the position,
        // not just any symbol whose scope contains the position. This ensures we get the correct
        // symbol when there are multiple symbols with the same name in different scopes.
        let mut best: Option<(&Symbol, usize)> = None;
        
        // First, find the most specific scope that contains the position
        let position_scope = find_most_specific_scope_containing_position(&scope_tree.root, position);
        
        for sym in same_file {
            // Find the scope that contains this symbol's definition
            // Use the symbol's scope_id if available, otherwise find by range
            let scope_id = if sym.scope_id > 0 {
                Some(sym.scope_id)
            } else {
                find_scope_for_symbol(&scope_tree.root, &sym.range)
            };
            
            if let Some(scope_id) = scope_id {
                // Check if this symbol's scope contains the position
                if scope_contains_position(&scope_tree.root, scope_id, position) {
                    // Compute depth of this scope (distance to root)
                    let depth = scope_depth(&scope_tree.root, scope_id);
                    
                    // Prefer symbols defined in the same scope as the position
                    // If no symbol in the same scope, prefer symbols in the most specific (deepest) parent scope
                    let is_in_position_scope = position_scope.map(|ps| ps == scope_id).unwrap_or(false);
                    
                    match &best {
                        Some((best_sym, best_depth)) => {
                            // Check if current best is in position scope
                            let best_scope_id = if best_sym.scope_id > 0 {
                                Some(best_sym.scope_id)
                            } else {
                                find_scope_for_symbol(&scope_tree.root, &best_sym.range)
                            };
                            let best_in_position_scope = best_scope_id
                                .and_then(|bs| position_scope.map(|ps| bs == ps))
                                .unwrap_or(false);
                            
                            // Always prefer symbols in the same scope as the position
                            if is_in_position_scope && !best_in_position_scope {
                                best = Some((sym, depth));
                            } else if !is_in_position_scope && !best_in_position_scope {
                                // Both are in parent scopes - prefer the deeper one
                                if depth > *best_depth {
                                    best = Some((sym, depth));
                                }
                            }
                            // If both are in position scope or both are not, keep the first one found
                        }
                        None => best = Some((sym, depth)),
                    }
                }
            } else {
                // Fallback: if no scope found, check if symbol range contains position
                // Only use this if we haven't found a better match with scope
                if position_in_range(position, &sym.range) && best.is_none() {
                    best = Some((sym, 0));
                }
            }
        }
        
        best.map(|(s, _)| s.clone())
    }

    /// Find symbols with prefix (for completion)
    pub fn find_by_prefix(&self, prefix: &str) -> Vec<Symbol> {
        self.index.find_symbols_with_prefix(prefix)
    }

    /// Find symbols at a specific position
    pub fn find_at_position(&self, file_path: &std::path::Path, position: Position) -> Vec<Symbol> {
        self.index
            .get_all_symbols()
            .into_iter()
            .filter(|symbol| {
                symbol.file_uri == file_path.to_string_lossy()
                    && position_in_range(position, &symbol.range)
            })
            .collect()
    }

    /// Get all symbols in a file
    pub fn find_in_file(&self, file_path: &std::path::Path) -> Vec<Symbol> {
        self.index
            .get_all_symbols()
            .into_iter()
            .filter(|symbol| symbol.file_uri == file_path.to_string_lossy())
            .collect()
    }
}

/// Helper function to find scope ID for a symbol based on its range
fn find_scope_for_symbol(scope_node: &crate::pipeline::types::ScopeNode, symbol_range: &crate::types::Range) -> Option<u32> {
    // Check if this scope contains the symbol
    if position_in_range(symbol_range.start, &scope_node.range) {
        // Check children first (deeper scopes)
        for child in &scope_node.children {
            if let Some(id) = find_scope_for_symbol(child, symbol_range) {
                return Some(id);
            }
        }
        // If no child contains it, this scope does
        return Some(scope_node.id);
    }
    None
}

/// Helper function to check if a scope contains a position
fn scope_contains_position(scope_node: &crate::pipeline::types::ScopeNode, scope_id: u32, position: Position) -> bool {
    if scope_node.id == scope_id {
        return position_in_range(position, &scope_node.range);
    }
    
    for child in &scope_node.children {
        if scope_contains_position(child, scope_id, position) {
            return true;
        }
    }
    
    false
}

/// Helper function to compute scope depth
fn scope_depth(scope_node: &crate::pipeline::types::ScopeNode, scope_id: u32) -> usize {
    fn depth_recursive(node: &crate::pipeline::types::ScopeNode, target_id: u32, current_depth: usize) -> Option<usize> {
        if node.id == target_id {
            return Some(current_depth);
        }
        
        for child in &node.children {
            if let Some(d) = depth_recursive(child, target_id, current_depth + 1) {
                return Some(d);
            }
        }
        
        None
    }
    
    depth_recursive(scope_node, scope_id, 0).unwrap_or(0)
}

/// Find the most specific (deepest) scope that contains the position
fn find_most_specific_scope_containing_position(scope_node: &crate::pipeline::types::ScopeNode, position: Position) -> Option<u32> {
    // Check if this scope contains the position
    if !position_in_range(position, &scope_node.range) {
        return None;
    }
    
    // Check children first (deeper scopes)
    for child in &scope_node.children {
        if let Some(id) = find_most_specific_scope_containing_position(child, position) {
            return Some(id);
        }
    }
    
    // No child contains it, so this scope is the most specific
    Some(scope_node.id)
}

fn position_in_range(position: Position, range: &crate::types::Range) -> bool {
    (position.line > range.start.line
        || (position.line == range.start.line && position.character >= range.start.character))
        && (position.line < range.end.line
            || (position.line == range.end.line && position.character <= range.end.character))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::types::AnalysisResult;

    #[test]
    fn test_find_by_name() {
        let mut index = Index::new();
        let mut analysis = AnalysisResult::new();
        
        let symbol = Symbol {
            name: "test".to_string(),
            kind: crate::types::SymbolKind::Function,
            range: crate::types::Range {
                start: crate::types::Position { line: 0, character: 0 },
                end: crate::types::Position { line: 0, character: 10 },
            },
            scope_id: 0,
            doc_comment: None,
            signature: None,
            file_uri: "test.jl".to_string(),
        };
        analysis.symbols.push(symbol);

        let file_path = PathBuf::from("test.jl");
        index.merge_file(&file_path, analysis).unwrap();

        let query = SymbolQuery::new(&index);
        let results = query.find_by_name("test");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_by_prefix() {
        let mut index = Index::new();
        let mut analysis = AnalysisResult::new();
        
        let symbol1 = Symbol {
            name: "test_function".to_string(),
            kind: crate::types::SymbolKind::Function,
            range: crate::types::Range {
                start: crate::types::Position { line: 0, character: 0 },
                end: crate::types::Position { line: 0, character: 10 },
            },
            scope_id: 0,
            doc_comment: None,
            signature: None,
            file_uri: "test.jl".to_string(),
        };
        let symbol2 = Symbol {
            name: "test_variable".to_string(),
            kind: crate::types::SymbolKind::Variable,
            range: crate::types::Range {
                start: crate::types::Position { line: 1, character: 0 },
                end: crate::types::Position { line: 1, character: 10 },
            },
            scope_id: 0,
            doc_comment: None,
            signature: None,
            file_uri: "test.jl".to_string(),
        };
        analysis.symbols.push(symbol1);
        analysis.symbols.push(symbol2);

        let file_path = PathBuf::from("test.jl");
        index.merge_file(&file_path, analysis).unwrap();

        let query = SymbolQuery::new(&index);
        let results = query.find_by_prefix("test_");
        assert_eq!(results.len(), 2);
    }
}

