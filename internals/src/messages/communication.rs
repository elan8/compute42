use actix::prelude::*;
use serde::{Deserialize, Serialize};
use super::execution::ExecutionType;

// ============================================================================
// Communication Types
// ============================================================================

/// Unified message protocol for permanent communication channel
#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "Result<(), String>")]
pub enum JuliaMessage {
    // Unified Code Execution Messages
    CodeExecution {
        id: String,
        code: String,
        execution_type: ExecutionType,
        timeout_ms: Option<u64>,
        breakpoints: Option<serde_json::Value>, // Breakpoints for debug execution
    },

    // Unified Execution Completion Message
    ExecutionComplete {
        id: String,
        execution_type: ExecutionType,
        result: Option<String>,
        error: Option<String>,
        success: bool,
        duration_ms: Option<u64>,
        timestamp: i64,
        metadata: Option<serde_json::Value>,
    },

    // Plot/Image Messages
    PlotData {
        id: String,
        mime_type: String,
        data: String, // Base64 encoded
        timestamp: i64,
        title: Option<String>,
        description: Option<String>,
        source_file: Option<String>,
        line_number: Option<u32>,
        code_context: Option<String>,
        session_id: Option<String>,
    },

    // System Messages
    Heartbeat {
        timestamp: i64,
    },

    // System Messages
    SessionStatus {
        status: String,
        details: Option<String>,
    },
    Error {
        error_type: String,
        message: String,
        details: Option<String>,
    },

    // Connection Test Messages
    ConnectionTest {
        id: String,
        message: String,
        timestamp: i64,
    },
    ConnectionTestResponse {
        id: String,
        response: String,
        timestamp: i64,
    },
    
    // Workspace Variables Messages
    GetWorkspaceVariables {
        id: String,
    },
    WorkspaceVariables {
        id: String,
        variables: serde_json::Value,
    },
    
    // Get specific variable value
    GetVariableValue {
        id: String,
        variable_name: String,
    },
    VariableValue {
        id: String,
        variable_name: String,
        value: Option<String>,
    },
}

/// Session status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatus {
    pub status: String,
    pub details: Option<String>,
}

/// Error information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub error_type: String,
    pub message: String,
    pub details: Option<String>,
}

/// Stream output for Julia terminal output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOutput {
    pub content: String,
    pub stream_type: StreamType,
    pub timestamp: u64, // Unix timestamp in milliseconds
}

/// Stream type for distinguishing output types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StreamType {
    Stdout,
    Stderr,
    Debug,
}

/// Message handler trait for processing Julia messages
pub trait MessageHandler: Send + Sync {
    fn handle_message(&self, message: &JuliaMessage) -> Result<(), String>;
}

/// Helper functions for creating messages
impl JuliaMessage {
    pub fn code_execution(
        id: String,
        code: String,
        execution_type: ExecutionType,
        timeout_ms: Option<u64>,
        breakpoints: Option<serde_json::Value>,
    ) -> Self {
        JuliaMessage::CodeExecution {
            id,
            code,
            execution_type,
            timeout_ms,
            breakpoints,
        }
    }

    pub fn execution_complete(
        id: String,
        execution_type: ExecutionType,
        result: Option<String>,
        error: Option<String>,
        success: bool,
        duration_ms: Option<u64>,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        JuliaMessage::ExecutionComplete {
            id,
            execution_type,
            result,
            error,
            success,
            duration_ms,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
            metadata,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn plot_data(
        id: String,
        mime_type: String,
        data: String,
        timestamp: i64,
        title: Option<String>,
        description: Option<String>,
        source_file: Option<String>,
        line_number: Option<u32>,
        code_context: Option<String>,
        session_id: Option<String>,
    ) -> Self {
        JuliaMessage::PlotData {
            id,
            mime_type,
            data,
            timestamp,
            title,
            description,
            source_file,
            line_number,
            code_context,
            session_id,
        }
    }

    pub fn heartbeat(timestamp: i64) -> Self {
        JuliaMessage::Heartbeat { timestamp }
    }

    pub fn session_status(status: String, details: Option<String>) -> Self {
        JuliaMessage::SessionStatus { status, details }
    }

    pub fn error(error_type: String, message: String, details: Option<String>) -> Self {
        JuliaMessage::Error {
            error_type,
            message,
            details,
        }
    }

    pub fn connection_test(id: String, message: String) -> Self {
        JuliaMessage::ConnectionTest {
            id,
            message,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        }
    }

    pub fn connection_test_response(id: String, response: String, timestamp: i64) -> Self {
        JuliaMessage::ConnectionTestResponse {
            id,
            response,
            timestamp,
        }
    }
}

/// Message validation
impl JuliaMessage {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            JuliaMessage::CodeExecution { id, code, .. } => Self::validate_code_execution(id, code),
            JuliaMessage::ExecutionComplete { id, .. } => Self::validate_execution_complete(id),
            JuliaMessage::PlotData { id, mime_type, data, .. } => Self::validate_plot_data(id, mime_type, data),
            JuliaMessage::Heartbeat { .. } => Ok(()),
            JuliaMessage::SessionStatus { status, .. } => Self::validate_session_status(status),
            JuliaMessage::Error { error_type, message, .. } => Self::validate_error(error_type, message),
            JuliaMessage::ConnectionTest { id, message, .. } => Self::validate_connection_test(id, message),
            JuliaMessage::ConnectionTestResponse { id, response, .. } => Self::validate_connection_test_response(id, response),
            JuliaMessage::GetWorkspaceVariables { id } => Self::validate_get_workspace_variables(id),
            JuliaMessage::WorkspaceVariables { id, .. } => Self::validate_workspace_variables(id),
            JuliaMessage::GetVariableValue { id, variable_name } => Self::validate_get_variable_value(id, variable_name),
            JuliaMessage::VariableValue { id, variable_name, .. } => Self::validate_variable_value(id, variable_name),
        }
    }

    // Helper validation functions
    fn validate_id(id: &str, field_name: &str) -> Result<(), String> {
        if id.is_empty() {
            Err(format!("{} cannot be empty", field_name))
        } else {
            Ok(())
        }
    }

    fn validate_non_empty(value: &str, field_name: &str) -> Result<(), String> {
        if value.is_empty() {
            Err(format!("{} cannot be empty", field_name))
        } else {
            Ok(())
        }
    }

    #[allow(dead_code)]
    fn validate_line_number(line: u32, context: &str) -> Result<(), String> {
        if line == 0 {
            Err(format!("{} line must be greater than 0", context))
        } else {
            Ok(())
        }
    }

    // Per-message-type validation methods
    fn validate_code_execution(id: &str, code: &str) -> Result<(), String> {
        Self::validate_id(id, "Code execution ID")?;
        Self::validate_non_empty(code, "Code execution code")?;
        Ok(())
    }

    fn validate_execution_complete(id: &str) -> Result<(), String> {
        Self::validate_id(id, "Execution complete ID")
    }

    fn validate_plot_data(id: &str, mime_type: &str, data: &str) -> Result<(), String> {
        Self::validate_id(id, "Plot data ID")?;
        Self::validate_non_empty(mime_type, "Plot data MIME type")?;
        Self::validate_non_empty(data, "Plot data")?;
        Ok(())
    }

    fn validate_session_status(status: &str) -> Result<(), String> {
        Self::validate_non_empty(status, "Session status")
    }

    fn validate_error(error_type: &str, message: &str) -> Result<(), String> {
        Self::validate_non_empty(error_type, "Error type")?;
        Self::validate_non_empty(message, "Error message")?;
        Ok(())
    }

    fn validate_connection_test(id: &str, message: &str) -> Result<(), String> {
        Self::validate_id(id, "Connection test ID")?;
        Self::validate_non_empty(message, "Connection test message")?;
        Ok(())
    }

    fn validate_connection_test_response(id: &str, response: &str) -> Result<(), String> {
        Self::validate_id(id, "Connection test response ID")?;
        Self::validate_non_empty(response, "Connection test response")?;
        Ok(())
    }


    fn validate_get_workspace_variables(id: &str) -> Result<(), String> {
        Self::validate_id(id, "Get workspace variables ID")
    }

    fn validate_workspace_variables(id: &str) -> Result<(), String> {
        Self::validate_id(id, "Workspace variables ID")
    }

    fn validate_get_variable_value(id: &str, variable_name: &str) -> Result<(), String> {
        Self::validate_id(id, "Get variable value ID")?;
        Self::validate_non_empty(variable_name, "Variable name")?;
        Ok(())
    }

    fn validate_variable_value(id: &str, variable_name: &str) -> Result<(), String> {
        Self::validate_id(id, "Variable value ID")?;
        Self::validate_non_empty(variable_name, "Variable name")?;
        Ok(())
    }

}

// ============================================================================
// CommunicationActor Messages
// ============================================================================

/// Connect to pipes
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ConnectToPipes {
    pub to_julia_pipe: String,
    pub from_julia_pipe: String,
}

/// Connect to Julia pipe (for sending data TO Julia)
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ConnectToJuliaPipe {
    pub to_julia_pipe: String,
}

/// Connect from Julia pipe (for receiving data FROM Julia)
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ConnectFromJuliaPipe {
    pub from_julia_pipe: String,
}

/// Disconnect from pipes
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct DisconnectFromPipes;

/// Execute code
#[derive(Message)]
#[rtype(result = "Result<JuliaMessage, String>")]
pub struct ExecuteCode {
    pub code: String,
    pub execution_type: ExecutionType,
    pub file_path: Option<String>,
    /// If true, suppress emitting backend-busy and backend-done events for this execution
    /// Used for batch executions where we only want to emit busy/done at the start/end
    pub suppress_busy_events: bool,
}

/// Check if connected
#[derive(Message)]
#[rtype(result = "Result<bool, String>")]
pub struct IsConnected;

/// Send debug message
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SendDebugMessage {
    pub message: crate::messages::JuliaMessage,
}

/// Get backend busy status
#[derive(Message)]
#[rtype(result = "Result<bool, String>")]
pub struct GetBackendBusyStatus;

/// Set orchestrator actor address for restart coordination
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetOrchestratorActor {
    pub orchestrator_actor: actix::Addr<crate::actors::orchestrator_actor::OrchestratorActor>,
}
