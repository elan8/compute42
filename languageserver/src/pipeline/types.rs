use std::path::PathBuf;
use tree_sitter::Tree;
use crate::types::{Symbol, FunctionSignature, TypeDefinition};

/// Represents a source file item with its content and metadata
#[derive(Debug, Clone)]
pub struct SourceItem {
    pub path: PathBuf,
    pub content: String,
    pub metadata: FileMetadata,
}

/// Metadata about a file
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// Last modified time (Unix timestamp in seconds)
    pub last_modified: u64,
    /// File size in bytes
    pub size: u64,
}

impl FileMetadata {
    pub fn new(last_modified: u64, size: u64) -> Self {
        Self {
            last_modified,
            size,
        }
    }
}

/// Represents a parsed file with its AST and text
#[derive(Debug, Clone)]
pub struct ParsedItem {
    pub path: PathBuf,
    pub tree: Tree,
    pub text: String,
}

/// Result of analysis from analyzers
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub symbols: Vec<Symbol>,
    pub references: Vec<Reference>,
    pub types: Vec<TypeDefinition>,
    pub scopes: ScopeTree,
    pub signatures: Vec<FunctionSignature>,
    pub exports: std::collections::HashSet<String>,
}

/// Represents a reference to a symbol (variable usage, function call, etc.)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Reference {
    pub name: String,
    pub range: crate::types::Range,
    pub file_uri: String,
    pub kind: ReferenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ReferenceKind {
    Variable,
    FunctionCall,
    TypeReference,
    ModuleReference,
}

/// Represents a scope tree hierarchy
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeTree {
    pub root: ScopeNode,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeNode {
    pub id: u32,
    pub parent_id: Option<u32>,
    pub range: crate::types::Range,
    pub file_uri: String,
    pub children: Vec<ScopeNode>,
}

impl AnalysisResult {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
            references: Vec::new(),
            types: Vec::new(),
            scopes: ScopeTree {
                root: ScopeNode {
                    id: 0,
                    parent_id: None,
                    range: crate::types::Range {
                        start: crate::types::Position { line: 0, character: 0 },
                        end: crate::types::Position { line: 0, character: 0 },
                    },
                    file_uri: String::new(),
                    children: Vec::new(),
                },
            },
            signatures: Vec::new(),
            exports: std::collections::HashSet::new(),
        }
    }
}

impl Default for AnalysisResult {
    fn default() -> Self {
        Self::new()
    }
}

