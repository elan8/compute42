use tree_sitter::{Tree, Node};
use crate::types::{Position, Symbol, SymbolKind};

pub struct SymbolResolver<'a> {
    tree: &'a Tree,
    source: &'a str,
}

impl<'a> SymbolResolver<'a> {
    pub fn new(tree: &'a Tree, source: &'a str) -> Self {
        Self { tree, source }
    }
    
    /// Find the smallest node containing the position
    pub fn node_at_position(&self, line: u32, col: u32) -> Option<Node<'a>> {
        let root = self.tree.root_node();
        self.find_smallest_node(root, line, col)
    }
    
    fn find_smallest_node(&self, node: Node<'a>, line: u32, col: u32) -> Option<Node<'a>> {
        let start = node.start_position();
        let end = node.end_position();
        
        // Check if position is within this node
        if line < start.row as u32 || line > end.row as u32 {
            return None;
        }
        if line == start.row as u32 && col < start.column as u32 {
            return None;
        }
        if line == end.row as u32 && col > end.column as u32 {
            return None;
        }
        
        // Look for a smaller child node that contains the position
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if let Some(found) = self.find_smallest_node(child, line, col) {
                    return Some(found);
                }
            }
        }
        
        // No smaller child contains the position, return this node
        Some(node)
    }
    
    /// Extract symbol name from identifier node
    pub fn extract_symbol_name(&self, node: Node) -> Option<String> {
        if node.kind() == "identifier" {
            return node.utf8_text(self.source.as_bytes()).ok().map(|s| s.to_string());
        }
        // If not directly an identifier, try parent then a shallow search among children
        if let Some(parent) = node.parent() {
            if parent.kind() == "identifier" {
                return parent.utf8_text(self.source.as_bytes()).ok().map(|s| s.to_string());
            }
            if let Some(id) = self.find_first_child_of_type(parent, "identifier") {
                return id.utf8_text(self.source.as_bytes()).ok().map(|s| s.to_string());
            }
        }
        // As a last resort, scan immediate children of the node
        if let Some(id) = self.find_first_child_of_type(node, "identifier") {
            return id.utf8_text(self.source.as_bytes()).ok().map(|s| s.to_string());
        }
        None
    }
    
    /// Find the definition node for this symbol
    pub fn find_definition(&self, symbol_name: &str) -> Option<Node<'a>> {
        let root = self.tree.root_node();
        self.find_definition_recursive(root, symbol_name)
    }
    
    fn find_definition_recursive(&self, node: Node<'a>, symbol_name: &str) -> Option<Node<'a>> {
        // Check if this node defines the symbol
        match node.kind() {
            "function_definition" | "assignment" | "struct_definition" | 
            "abstract_definition" | "module_definition" | "macro_definition" => {
                if let Some(name_node) = self.find_first_child_of_type(node, "identifier") {
                    if let Ok(name) = name_node.utf8_text(self.source.as_bytes()) {
                        if name == symbol_name {
                            return Some(node);
                        }
                    }
                }
            }
            _ => {}
        }
        
        // Search children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if let Some(found) = self.find_definition_recursive(child, symbol_name) {
                    return Some(found);
                }
            }
        }
        
        None
    }
    
    fn find_first_child_of_type<'b>(&self, node: Node<'b>, kind: &str) -> Option<Node<'b>> {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == kind {
                    return Some(child);
                }
            }
        }
        None
    }
    
    /// Get symbol information at a specific position
    pub fn symbol_at_position(&self, position: Position) -> Option<Symbol> {
        let node = self.node_at_position(position.line, position.character)?;
        
        // Try to extract symbol name from the node or its parent
        let symbol_name = self.extract_symbol_name(node)
            .or_else(|| {
                // If current node is not an identifier, look for identifier in parent
                if let Some(parent) = node.parent() {
                    self.extract_symbol_name(parent)
                } else {
                    None
                }
            })?;
        
        // Determine symbol kind based on node type
        let kind = self.determine_symbol_kind(node);
        
        Some(Symbol {
            name: symbol_name,
            kind,
            range: crate::types::Range {
                start: Position::from(node.start_position()),
                end: Position::from(node.end_position()),
            },
            scope_id: 0, // TODO: Implement proper scope tracking
            doc_comment: None,
            signature: None,
            file_uri: "unknown".to_string(), // TODO: Pass file_uri from caller
        })
    }
    
    fn determine_symbol_kind(&self, node: Node) -> SymbolKind {
        // Check the current node and its parents to determine the symbol kind
        let mut current = Some(node);
        while let Some(n) = current {
            match n.kind() {
                "function_definition" => return SymbolKind::Function,
                "struct_definition" | "abstract_definition" => return SymbolKind::Type,
                "assignment" => return SymbolKind::Variable,
                "module_definition" => return SymbolKind::Module,
                "macro_definition" => return SymbolKind::Macro,
                _ => {
                    current = n.parent();
                }
            }
        }
        
        // Default fallback
        SymbolKind::Variable
    }
}
