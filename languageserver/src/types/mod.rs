// Type modules
pub mod type_expr;
pub mod scope;
pub mod module_exports;
pub mod base_docs;

// Re-export types for convenience
pub use type_expr::*;
pub use scope::*;
pub use module_exports::*;
pub use base_docs::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LspError {
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),
    
    #[error("Invalid position: line {line}, character {character}")]
    InvalidPosition { line: u32, character: u32 },
    
    #[error("Document not found: {0}")]
    DocumentNotFound(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SymbolKind {
    Function,
    Type,
    Variable,
    Constant,
    Module,
    Macro,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub scope_id: u32,
    pub doc_comment: Option<String>,
    pub signature: Option<String>,
    pub file_uri: String,
}

#[derive(Debug, Clone)]
pub struct HoverResult {
    pub contents: String,
    pub range: Option<Range>,
}

// Completion types
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompletionItemKind {
    Function = 3,
    Variable = 6,
    Module = 9,
    Type = 22,
    Constant = 21,
    Macro = 15,
}

#[derive(Debug, Clone)]
pub struct CompletionList {
    pub is_incomplete: bool,
    pub items: Vec<CompletionItem>,
}

// Location type for definitions and references
#[derive(Debug, Clone)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

// Diagnostic types
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<DiagnosticSeverity>,
    pub code: Option<String>,
    pub source: Option<String>,
    pub message: String,
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

#[derive(Debug, Clone)]
pub struct DiagnosticRelatedInformation {
    pub location: Location,
    pub message: String,
}

// Code action types for LSP quick fixes
#[derive(Debug, Clone)]
pub struct CodeAction {
    pub title: String,
    pub kind: Option<String>,
    pub edit: Option<WorkspaceEdit>,
    pub command: Option<Command>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceEdit {
    pub changes: Vec<(String, Vec<TextEdit>)>, // (uri, edits)
}

#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub title: String,
    pub command: String,
    pub arguments: Option<Vec<serde_json::Value>>,
}

impl From<tree_sitter::Point> for Position {
    fn from(point: tree_sitter::Point) -> Self {
        Self {
            line: point.row as u32,
            character: point.column as u32,
        }
    }
}

impl From<Position> for tree_sitter::Point {
    fn from(pos: Position) -> Self {
        Self {
            row: pos.line as usize,
            column: pos.character as usize,
        }
    }
}

// LspRequest and LspResponse removed - no longer needed without JuliaLspProvider