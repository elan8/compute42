use actix::prelude::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// ============================================================================
// Execution Types
// ============================================================================

/// Execution type for code execution
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionType {
    ApiCall,
    ReplExecution,
    FileExecution,
    NotebookCell { cell_id: String },
}

// Custom serialization to use Display format (string representation)
impl Serialize for ExecutionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use std::fmt::Write;
        let mut s = String::new();
        write!(&mut s, "{}", self).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&s)
    }
}

// Custom deserialization to parse from string
impl<'de> Deserialize<'de> for ExecutionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(ExecutionType::from(s.as_str()))
    }
}

impl std::fmt::Display for ExecutionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionType::ApiCall => write!(f, "api_call"),
            ExecutionType::ReplExecution => write!(f, "repl_execution"),
            ExecutionType::FileExecution => write!(f, "file_execution"),
            ExecutionType::NotebookCell { cell_id } => write!(f, "notebook_cell:{}", cell_id),
        }
    }
}

impl From<&str> for ExecutionType {
    fn from(s: &str) -> Self {
        match s {
            "api_call" => ExecutionType::ApiCall,
            "repl_execution" => ExecutionType::ReplExecution,
            "file_execution" => ExecutionType::FileExecution,
            s if s.starts_with("notebook_cell:") => {
                let cell_id = s.strip_prefix("notebook_cell:").unwrap_or("").to_string();
                ExecutionType::NotebookCell { cell_id }
            }
            _ => ExecutionType::FileExecution, // Default fallback
        }
    }
}

// ============================================================================
// ExecutionActor Messages
// ============================================================================

/// Execute API request
#[derive(Message)]
#[rtype(result = "Result<String, String>")]
pub struct ExecuteApiRequest {
    pub code: String,
}

/// Execute REPL request
#[derive(Message)]
#[rtype(result = "Result<String, String>")]
pub struct ExecuteReplRequest {
    pub code: String,
}

/// Execute notebook cell
#[derive(Message)]
#[rtype(result = "Result<String, String>")]
pub struct ExecuteNotebookCell {
    pub cell_id: String,
    pub code: String,
    pub notebook_path: Option<String>,
}

/// Notebook cell for batch execution
#[derive(Debug, Clone)]
pub struct NotebookCellBatchItem {
    pub cell_id: String,
    pub code: String,
    pub notebook_path: Option<String>,
}

/// Execute multiple notebook cells in batch (emits busy/done only at start/end)
#[derive(Message)]
#[rtype(result = "Result<Vec<(String, Result<String, String>)>, String>")]
pub struct ExecuteNotebookCellsBatch {
    pub cells: Vec<NotebookCellBatchItem>,
}

/// Execute file
#[derive(Message)]
#[rtype(result = "Result<String, String>")]
pub struct ExecuteFile {
    pub file_path: String,
}

/// Activate project
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ActivateProject {
    pub project_path: String,
}

/// Deactivate project
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct DeactivateProject;

