// Message handling for CommunicationActor
// Handles parsing and processing of Julia messages

use crate::services::events::EventService;
use crate::messages::plot::HandlePlotDataReceived;
use actix::prelude::*;
use log::{debug, error, trace};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Message handler for processing Julia messages
pub struct MessageHandler {
    pub event_manager: EventService,
    plot_actor: Option<Addr<crate::actors::PlotActor>>,
    process_actor: Option<Addr<crate::actors::ProcessActor>>,
}

impl MessageHandler {
    pub fn new(
        event_manager: EventService,
        plot_actor: Option<Addr<crate::actors::PlotActor>>,
        process_actor: Option<Addr<crate::actors::ProcessActor>>,
    ) -> Self {
        Self {
            event_manager,
            plot_actor,
            process_actor,
        }
    }
    
    /// Set PlotActor address for routing plot data through actor
    #[allow(dead_code)]
    pub fn set_plot_actor(&mut self, plot_actor: Addr<crate::actors::PlotActor>) {
        self.plot_actor = Some(plot_actor);
    }


    /// Handle messages from Julia
    #[allow(clippy::type_complexity)]
    pub async fn handle_julia_message(
        &self,
        message: &crate::messages::JuliaMessage,
        current_request: &Arc<Mutex<Option<(String, tokio::sync::oneshot::Sender<crate::messages::JuliaMessage>)>>>,
    ) -> Result<(), String> {
        match message {
            crate::messages::JuliaMessage::ExecutionComplete {
                id,
                execution_type,
                result,
                error,
                ..
            } => {
                self.handle_execution_complete(
                    message,
                    id,
                    execution_type,
                    result,
                    error,
                    current_request,
                ).await
            }

            crate::messages::JuliaMessage::ConnectionTestResponse {
                id,
                response,
                timestamp,
            } => self.handle_connection_test_response(id, response, *timestamp).await,
            
            crate::messages::JuliaMessage::PlotData {
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
            } => self.handle_plot_data(
                id, mime_type, data, *timestamp, title, description,
                source_file, line_number, code_context, session_id,
            ).await,
            
            crate::messages::JuliaMessage::SessionStatus { status, details } => {
                self.handle_session_status(status, details.as_deref()).await
            }
            
            crate::messages::JuliaMessage::Error {
                error_type,
                message,
                ..
            } => self.handle_error(error_type, message).await,
            
            crate::messages::JuliaMessage::Heartbeat { .. } => {
                debug!("[CommunicationActor::MessageHandler] Received heartbeat");
                Ok(())
            }
            crate::messages::JuliaMessage::WorkspaceVariables {
                variables,
                ..
            } => self.handle_workspace_variables(variables).await,
            
            crate::messages::JuliaMessage::VariableValue {
                variable_name,
                value,
                ..
            } => self.handle_variable_value(variable_name, value.as_deref()).await,
            _ => {
                debug!(
                    "[CommunicationActor::MessageHandler] Unhandled message type: {:?}",
                    message
                );
                Ok(())
            }
        }
    }

    // Helper methods for processing messages
    
    /// Process pending request and send response if ID matches
    #[allow(clippy::type_complexity)]
    async fn process_pending_request(
        current_request: &Arc<Mutex<Option<(String, tokio::sync::oneshot::Sender<crate::messages::JuliaMessage>)>>>,
        message: &crate::messages::JuliaMessage,
        id: &str,
    ) {
        let mut current_request_guard = current_request.lock().await;
        if let Some((request_id, sender)) = current_request_guard.take() {
            debug!("[CommunicationActor::MessageHandler] Found pending request with ID: {}", request_id);
            if request_id == *id {
                debug!("[CommunicationActor::MessageHandler] Request ID matches, sending response");
                if let Err(e) = sender.send(message.clone()) {
                    error!("[CommunicationActor::MessageHandler] Failed to send response: {:?}", e);
                } else {
                    debug!("[CommunicationActor::MessageHandler] Successfully sent response");
                }
            } else {
                debug!(
                    "[CommunicationActor::MessageHandler] Request ID mismatch: expected {}, got {}",
                    id, request_id
                );
            }
        } else {
            debug!("[CommunicationActor::MessageHandler] No pending request found for ID: {}", id);
        }
    }

    /// Clean array strings in result if present
    fn clean_array_string_result(result: &Option<String>) -> Option<String> {
        if let Some(result_str) = result {
            if result_str.contains("Float32[") || result_str.contains("Int64[") || result_str.contains("UInt8[") {
                // Only log a summary for large arrays to avoid cluttering logs
                let preview = if result_str.len() > 200 {
                    format!("{}... ({} chars)", &result_str[..200], result_str.len())
                } else {
                    result_str.clone()
                };
                trace!("[CommunicationActor::MessageHandler] Detected array with type prefix, attempting to clean: {}", preview);
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(result_str) {
                    if let Some(value_field) = parsed.get("value").and_then(|v| v.as_str()) {
                        use crate::services::base::variable_utils::clean_array_string;
                        let cleaned_value = clean_array_string(value_field);
                        let mut cleaned_parsed = parsed.clone();
                        cleaned_parsed["value"] = serde_json::Value::String(cleaned_value);
                        if let Ok(cleaned_json) = serde_json::to_string(&cleaned_parsed) {
                            return Some(cleaned_json);
                        }
                    }
                }
            }
            result.clone()
        } else {
            None
        }
    }

    // Per-message-type handler methods

    #[allow(clippy::type_complexity)]
    async fn handle_execution_complete(
        &self,
        message: &crate::messages::JuliaMessage,
        id: &str,
        execution_type: &crate::messages::ExecutionType,
        result: &Option<String>,
        error: &Option<String>,
        current_request: &Arc<Mutex<Option<(String, tokio::sync::oneshot::Sender<crate::messages::JuliaMessage>)>>>,
    ) -> Result<(), String> {
        debug!("[CommunicationActor::MessageHandler] Received execution complete: {} (type: {:?})", id, execution_type);
        
        Self::process_pending_request(current_request, message, id).await;
        
        let cleaned_result = Self::clean_array_string_result(result);
        
        self.event_manager
            .emit_communication_execution_complete(id, cleaned_result.as_deref(), error.as_deref())
            .await
            .map_err(|e| format!("Failed to emit execution complete event: {}", e))?;

        // Only emit prompt for REPL and FileExecution, not for ApiCall or NotebookCell
        if execution_type != &crate::messages::ExecutionType::ApiCall {
            // Check if this is a notebook cell execution - don't emit prompt for those
            let is_notebook_cell = matches!(execution_type, crate::messages::ExecutionType::NotebookCell { .. });
            
            if !is_notebook_cell {
                if execution_type == &crate::messages::ExecutionType::ReplExecution {
                    let _ = self.event_manager.emit_julia_output("\n").await;
                }
                let _ = self.event_manager.emit_julia_output("\x1b[1;32mjulia> \x1b[0m").await;
            }
        }
        Ok(())
    }

    async fn handle_connection_test_response(
        &self,
        id: &str,
        response: &str,
        timestamp: i64,
    ) -> Result<(), String> {
        debug!("[CommunicationActor::MessageHandler] Received connection test response: {}", id);
        self.event_manager
            .emit_connection_test_response(id, response, timestamp)
            .await
            .map_err(|e| format!("Failed to emit connection test response event: {}", e))
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_plot_data(
        &self,
        id: &str,
        mime_type: &str,
        data: &str,
        timestamp: i64,
        title: &Option<String>,
        description: &Option<String>,
        source_file: &Option<String>,
        line_number: &Option<u32>,
        code_context: &Option<String>,
        session_id: &Option<String>,
    ) -> Result<(), String> {
        debug!("[CommunicationActor::MessageHandler] Received plot data: {}", id);
        
        // Check if a notebook cell is executing and buffer the plot
        if let Some(process_actor) = &self.process_actor {
            if let Err(e) = process_actor.send(crate::messages::process::BufferNotebookCellPlot {
                mime_type: mime_type.to_string(),
                data: data.to_string(),
            }).await {
                debug!("[CommunicationActor::MessageHandler] Failed to buffer plot for notebook cell: {:?}", e);
            } else {
                debug!("[CommunicationActor::MessageHandler] Buffered plot for notebook cell");
            }
        }
        
        let plot_data = serde_json::json!({
            "id": id,
            "mime_type": mime_type,
            "data": data,
            "timestamp": timestamp,
            "title": title,
            "description": description,
            "source_file": source_file,
            "line_number": line_number,
            "code_context": code_context,
            "session_id": session_id
        });
        
        // Route through PlotActor for proper serialization (for plot pane)
        if let Some(plot_actor) = &self.plot_actor {
            debug!("[CommunicationActor::MessageHandler] Routing plot data through PlotActor for serialization");
            plot_actor.send(HandlePlotDataReceived {
                plot_data_json: plot_data,
            }).await
            .map_err(|e| format!("Failed to send plot data to PlotActor: {:?}", e))?
            .map_err(|e| format!("PlotActor failed to handle plot data: {}", e))?;
            Ok(())
        } else {
            error!("[CommunicationActor::MessageHandler] PlotActor not available, cannot handle plot data");
            Err("PlotActor not available".to_string())
        }
    }

    async fn handle_session_status(
        &self,
        status: &str,
        details: Option<&str>,
    ) -> Result<(), String> {
        debug!("[CommunicationActor::MessageHandler] Session status: {}", status);
        self.event_manager
            .emit_communication_session_status(status, details)
            .await
            .map_err(|e| format!("Failed to emit session status event: {}", e))
    }

    async fn handle_error(
        &self,
        error_type: &str,
        message: &str,
    ) -> Result<(), String> {
        error!("[CommunicationActor::MessageHandler] Julia error: {} - {}", error_type, message);
        self.event_manager
            .emit_julia_error(message)
            .await
            .map_err(|e| format!("Failed to emit error event: {}", e))
    }

    async fn handle_workspace_variables(
        &self,
        variables: &serde_json::Value,
    ) -> Result<(), String> {
        debug!("[CommunicationActor::MessageHandler] Received workspace variables");
        use crate::services::base::variable_utils::process_variables_map;
        let filtered_variables = process_variables_map(variables.clone());
        self.event_manager.emit("workspace:variables-updated", filtered_variables).await
            .map_err(|e| format!("Failed to emit workspace variables event: {}", e))
    }

    async fn handle_variable_value(
        &self,
        variable_name: &str,
        value: Option<&str>,
    ) -> Result<(), String> {
        debug!("[CommunicationActor::MessageHandler] Received variable value for: {}", variable_name);
        use crate::services::base::variable_utils::clean_array_string;
        let cleaned_value = value.map(clean_array_string);
        let payload = serde_json::json!({
            "variable_name": variable_name,
            "value": cleaned_value
        });
        self.event_manager.emit("workspace:variable-value-received", payload).await
            .map_err(|e| format!("Failed to emit variable value event: {}", e))
    }

    /// Parse nested Julia message format
    #[allow(dead_code)]
    pub fn parse_nested_message(&self, buffer: &str) -> Result<Option<crate::messages::JuliaMessage>, String> {
        // Try to parse as a nested structure (Julia format)
        if let Ok(nested_message) = serde_json::from_str::<serde_json::Value>(buffer) {
            trace!(
                "[CommunicationActor::MessageHandler] Parsed as nested structure: {:?}",
                nested_message
            );

            // Handle nested ExecutionComplete
            if let Some(exec_data) = nested_message.get("ExecutionComplete") {
                // Try to parse directly - execution_type should now serialize/deserialize correctly
                if let Ok(exec_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(
                        serde_json::json!({
                            "ExecutionComplete": exec_data
                        }),
                    )
                {
                    debug!("[CommunicationActor::MessageHandler] Parsed ExecutionComplete from nested structure: {:?}", exec_message);
                    return Ok(Some(exec_message));
                } else {
                    return Err(format!("Failed to parse ExecutionComplete from nested structure: {}", buffer));
                }
            }
            // Handle nested ConnectionTestResponse
            else if let Some(conn_data) = nested_message.get("ConnectionTestResponse") {
                if let Ok(conn_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(serde_json::json!({
                        "ConnectionTestResponse": conn_data
                    }))
                {
                    debug!("[CommunicationActor::MessageHandler] Parsed ConnectionTestResponse from nested structure: {:?}", conn_message);
                    return Ok(Some(conn_message));
                } else {
                    return Err(format!("Failed to parse ConnectionTestResponse from nested structure: {}", buffer));
                }
            }
            // Handle nested DebugFrame
            else if let Some(debug_data) = nested_message.get("DebugFrame") {
                if let Ok(debug_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(serde_json::json!({
                        "DebugFrame": debug_data
                    }))
                {
                    debug!("[CommunicationActor::MessageHandler] Parsed DebugFrame from nested structure: {:?}", debug_message);
                    return Ok(Some(debug_message));
                } else {
                    return Err(format!("Failed to parse DebugFrame from nested structure: {}", buffer));
                }
            }
            // Handle nested DebugStep
            else if let Some(step_data) = nested_message.get("DebugStep") {
                if let Ok(step_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(serde_json::json!({
                        "DebugStep": step_data
                    }))
                {
                    debug!("[CommunicationActor::MessageHandler] Parsed DebugStep from nested structure: {:?}", step_message);
                    return Ok(Some(step_message));
                } else {
                    return Err(format!("Failed to parse DebugStep from nested structure: {}", buffer));
                }
            }
            // Handle nested WorkspaceVariables
            else if let Some(workspace_data) = nested_message.get("WorkspaceVariables") {
                if let Ok(workspace_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(serde_json::json!({
                        "WorkspaceVariables": workspace_data
                    }))
                {
                    debug!("[CommunicationActor::MessageHandler] Parsed WorkspaceVariables from nested structure");
                    return Ok(Some(workspace_message));
                } else {
                    return Err(format!("Failed to parse WorkspaceVariables from nested structure: {}", buffer));
                }
            }
            // Handle nested VariableValue
            else if let Some(variable_value_data) = nested_message.get("VariableValue") {
                if let Ok(variable_value_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(serde_json::json!({
                        "VariableValue": variable_value_data
                    }))
                {
                    debug!("[CommunicationActor::MessageHandler] Parsed VariableValue from nested structure");
                    return Ok(Some(variable_value_message));
                } else {
                    return Err(format!("Failed to parse VariableValue from nested structure: {}", buffer));
                }
            }
            // Handle nested BreakpointSet
            else if let Some(bp_set_data) = nested_message.get("BreakpointSet") {
                if let Ok(bp_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(serde_json::json!({
                        "BreakpointSet": bp_set_data
                    }))
                {
                    debug!("[CommunicationActor::MessageHandler] Parsed BreakpointSet from nested structure");
                    return Ok(Some(bp_message));
                } else {
                    return Err(format!("Failed to parse BreakpointSet from nested structure: {}", buffer));
                }
            }
            // Handle nested DebugCompleted
            else if let Some(debug_completed_data) = nested_message.get("DebugCompleted") {
                if let Ok(debug_completed_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(serde_json::json!({
                        "DebugCompleted": debug_completed_data
                    }))
                {
                    trace!("[CommunicationActor::MessageHandler] Parsed DebugCompleted from nested structure");
                    return Ok(Some(debug_completed_message));
                } else {
                    return Err(format!("Failed to parse DebugCompleted from nested structure: {}", buffer));
                }
            }
            // Handle nested DebugStarted
            else if let Some(debug_started_data) = nested_message.get("DebugStarted") {
                if let Ok(debug_started_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(serde_json::json!({
                        "DebugStarted": debug_started_data
                    }))
                {
                    trace!("[CommunicationActor::MessageHandler] Parsed DebugStarted from nested structure");
                    return Ok(Some(debug_started_message));
                } else {
                    return Err(format!("Failed to parse DebugStarted from nested structure: {}", buffer));
                }
            }
            // Handle nested DebugError
            else if let Some(debug_error_data) = nested_message.get("DebugError") {
                if let Ok(debug_error_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(serde_json::json!({
                        "DebugError": debug_error_data
                    }))
                {
                    debug!("[CommunicationActor::MessageHandler] Parsed DebugError from nested structure");
                    return Ok(Some(debug_error_message));
                } else {
                    return Err(format!("Failed to parse DebugError from nested structure: {}", buffer));
                }
            }
            // Handle nested DebugMessageResponse
            else if let Some(debug_response_data) = nested_message.get("DebugMessageResponse") {
                if let Ok(debug_response_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(serde_json::json!({
                        "DebugMessageResponse": debug_response_data
                    }))
                {
                    debug!("[CommunicationActor::MessageHandler] Parsed DebugMessageResponse from nested structure");
                    return Ok(Some(debug_response_message));
                } else {
                    return Err(format!("Failed to parse DebugMessageResponse from nested structure: {}", buffer));
                }
            }
            // Handle nested DebugVariables
            else if let Some(debug_vars_data) = nested_message.get("DebugVariables") {
                if let Ok(debug_vars_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(serde_json::json!({
                        "DebugVariables": debug_vars_data
                    }))
                {
                    debug!("[CommunicationActor::MessageHandler] Parsed DebugVariables from nested structure");
                    return Ok(Some(debug_vars_message));
                } else {
                    return Err(format!("Failed to parse DebugVariables from nested structure: {}", buffer));
                }
            }
            // Handle nested DebugStopped
            else if let Some(debug_stopped_data) = nested_message.get("DebugStopped") {
                if let Ok(debug_stopped_message) =
                    serde_json::from_value::<crate::messages::JuliaMessage>(serde_json::json!({
                        "DebugStopped": debug_stopped_data
                    }))
                {
                    debug!("[CommunicationActor::MessageHandler] Parsed DebugStopped from nested structure");
                    return Ok(Some(debug_stopped_message));
                } else {
                    return Err(format!("Failed to parse DebugStopped from nested structure: {}", buffer));
                }
            } else {
                return Err(format!("Unknown nested message structure: {}", buffer));
            }
        }
        
        Ok(None)
    }
}

