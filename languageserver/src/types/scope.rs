use crate::types::{Position, Range};

pub type ScopeId = u32;

#[derive(Debug, Clone)]
pub struct ScopeInfo {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub range: Range,
    pub file_uri: String,
}

impl ScopeInfo {
    pub fn contains_position(&self, pos: Position) -> bool {
        pos.line >= self.range.start.line
            && pos.line <= self.range.end.line
            && (pos.line > self.range.start.line || pos.character >= self.range.start.character)
            && (pos.line < self.range.end.line || pos.character <= self.range.end.character)
    }
}
