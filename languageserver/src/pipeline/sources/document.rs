use ropey::Rope;
use tree_sitter::{Parser, Tree};
use crate::types::LspError;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Document {
    uri: String,
    text: Rope,
    tree: Option<Tree>,
    version: i32,
    /// Timestamp when document was last modified (Unix timestamp in seconds)
    last_modified: u64,
    /// Flag indicating if document has unparsed changes
    dirty: bool,
}

impl Document {
    pub fn new(uri: String, content: String) -> Self {
        let text = Rope::from_str(&content);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            uri,
            text,
            tree: None,
            version: 0,
            last_modified: now,
            dirty: true,
        }
    }
    
    pub fn parse(&mut self, parser: &mut Parser) -> Result<(), LspError> {
        let source = self.text.to_string();
        self.tree = parser.parse(&source, None);
        self.dirty = false;
        Ok(())
    }
    
    pub fn text(&self) -> String {
        self.text.to_string()
    }
    
    pub fn tree(&self) -> Option<&Tree> {
        self.tree.as_ref()
    }
    
    pub fn uri(&self) -> &str {
        &self.uri
    }
    
    pub fn version(&self) -> i32 {
        self.version
    }
    
    pub fn update_version(&mut self) {
        self.version += 1;
    }
    
    pub fn get_line(&self, line: usize) -> Option<String> {
        if line < self.text.len_lines() {
            Some(self.text.line(line).to_string())
        } else {
            None
        }
    }
    
    pub fn line_count(&self) -> usize {
        self.text.len_lines()
    }
    
    /// Get last modified timestamp
    pub fn last_modified(&self) -> u64 {
        self.last_modified
    }
    
    /// Update last modified timestamp to now
    pub fn touch(&mut self) {
        self.last_modified = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.dirty = true;
    }
    
    /// Check if document is dirty (needs reparsing)
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// Mark document as dirty
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    
    /// Update document content
    pub fn update_content(&mut self, content: String) {
        self.text = Rope::from_str(&content);
        self.touch();
    }
}








