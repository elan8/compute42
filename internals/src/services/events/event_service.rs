use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use crate::messages::PlotData;
use shared::auth::User;
use log::debug;


/// Unified event structure that can represent all types of events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedEvent {
    /// The category of the event (e.g., "account", "orchestrator", "lsp", "julia")
    pub category: String,
    /// The specific event type within the category
    pub event_type: String,
    /// The event payload as a JSON value
    pub payload: serde_json::Value,
    /// Optional timestamp for when the event was created
    pub timestamp: Option<i64>,
}

/// Event categories for organizing different types of events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventCategory {
    Account,
    Orchestrator,
    Lsp,
    Julia,
    Plot,
    File,
    Communication,
    System,
}

impl std::fmt::Display for EventCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventCategory::Account => write!(f, "account"),
            EventCategory::Orchestrator => write!(f, "orchestrator"),
            EventCategory::Lsp => write!(f, "lsp"),
            EventCategory::Julia => write!(f, "julia"),
            EventCategory::Plot => write!(f, "plot"),
            EventCategory::File => write!(f, "file"),
            EventCategory::Communication => write!(f, "communication"),
            EventCategory::System => write!(f, "system"),
        }
    }
}

/// Account-related event payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AccountEventPayload {
    pub user: Option<User>,
    pub email: Option<String>,
    pub error: Option<String>,
    pub can_retry: Option<bool>,
    pub is_new_user: Option<bool>,
    pub mode: Option<String>,
    pub plans: Option<Vec<serde_json::Value>>,
    pub plan: Option<serde_json::Value>,
}

/// Orchestrator-related event payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct OrchestratorEventPayload {
    pub message: Option<String>,
    pub progress: Option<u8>,
    pub is_error: Option<bool>,
    pub error_details: Option<String>,
    pub status: Option<String>,
    pub packages: Option<Vec<String>>,
    pub project_path: Option<String>,
    pub port: Option<u16>,
}

/// LSP-related event payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct LspEventPayload {
    pub status: Option<String>,
    pub message: Option<String>,
    pub progress: Option<u8>,
    pub error: Option<String>,
    pub port: Option<u16>,
}

/// Julia-related event payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct JuliaEventPayload {
    pub status: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub packages: Option<Vec<String>>,
    pub project_path: Option<String>,
}

/// Plot-related event payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct PlotEventPayload {
    pub plot_id: Option<String>,
    pub message: Option<String>,
    pub port: Option<u16>,
}

/// File-related event payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct FileEventPayload {
    pub file_path: Option<String>,
    pub message: Option<String>,
    pub port: Option<u16>,
    pub error: Option<String>,
}

/// Communication-related event payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct CommunicationEventPayload {
    pub request_id: Option<String>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub connected: Option<bool>,
}

/// Notebook cell output payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct NotebookCellEventPayload {
    pub cell_id: String,
    pub cell_index: usize,
    /// Jupyter-like outputs array (stream / display_data / execute_result / error)
    pub outputs: serde_json::Value,
}

/// System-related event payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct SystemEventPayload {
    pub status: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub health: Option<String>,
}

/// Julia installation-related event payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct JuliaInstallationEventPayload {
    pub installation: Option<crate::types::JuliaInstallation>,
    pub installations: Option<Vec<crate::types::JuliaInstallation>>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub version: Option<String>,
    pub progress: Option<u8>,
}

/// Sysimage-related event payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct SysimageEventPayload {
    pub sysimage: Option<crate::types::SysimageInfo>,
    pub sysimages: Option<Vec<crate::types::SysimageInfo>>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// EventService - manages event emission and coordination

#[derive(Clone)]
pub struct EventService {
    // Event emission service (external communication only)
    event_emitter: Arc<dyn crate::service_traits::EventEmitter>,
    
    // Actor system reference for inter-actor communication
    actor_system: Option<Arc<crate::actor_system::ActorSystem>>,
}

impl EventService {
    /// Create a new EventService instance
    pub fn new(event_emitter: Arc<dyn crate::service_traits::EventEmitter>) -> Self {
        Self {
            event_emitter,
            actor_system: None,
        }
    }
    
    /// Set the actor system reference for inter-actor communication
    pub fn set_actor_system(&mut self, actor_system: Arc<crate::actor_system::ActorSystem>) {
        self.actor_system = Some(actor_system);
    }
    
    /// Get the actor system reference
    pub fn get_actor_system(&self) -> Option<Arc<crate::actor_system::ActorSystem>> {
        self.actor_system.clone()
    }
    
    /// Get the underlying event emitter
    pub fn get_event_emitter(&self) -> Arc<dyn crate::service_traits::EventEmitter> {
        self.event_emitter.clone()
    }
    
    /// Helper to create a unified event with namespaced type
    pub fn create_event(category: EventCategory, event_type: &str, payload: serde_json::Value) -> UnifiedEvent {
        let event_name = format!("{}:{}", category, event_type);
        UnifiedEvent {
            category: category.to_string(),
            event_type: event_name,
            payload,
            timestamp: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
            ),
        }
    }
    
    /// Emit a previously created unified event
    pub async fn emit_event(&self, event: UnifiedEvent) -> Result<(), String> {
        self.event_emitter
            .emit(&event.event_type, event.payload)
            .await
    }
    
    /// Emit an event directly with a full event name (e.g., "workspace:variables-updated")
    pub async fn emit(&self, event_name: &str, payload: serde_json::Value) -> Result<(), String> {
        self.event_emitter.emit(event_name, payload).await
    }
    
    // ============================================================================
    // Phase 5: Actor-Based Event Emission and Coordination
    // ============================================================================
    
    /// Emit an event and notify relevant actors
    pub async fn emit_with_actor_notification(
        &self,
        event_type: &str,
        payload: serde_json::Value,
        _notify_actors: &[&str],
    ) -> Result<(), String> {
        // Emit the event to frontend
        self.event_emitter.emit(event_type, payload.clone()).await?;
        
        // Notify relevant actors if actor system is available
        if let Some(actor_system) = &self.actor_system {
            let _ = actor_system
                .log_debug_message(
                    "info".to_string(),
                    format!("Event emitted: {}", event_type),
                    "EventService".to_string(),
                )
                .await;
        }
        
        Ok(())
    }
    
    /// Emit a system-wide event that all actors should be aware of
    pub async fn emit_system_event(
        &self,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        // Emit to frontend
        self.event_emitter.emit_all(event_type, payload.clone()).await?;
        
        // Notify all actors if actor system is available
        // Actor broadcast skipped in refactor
        
        Ok(())
    }
    
    /// Emit an error event and propagate to actors
    pub async fn emit_error_event(
        &self,
        actor_name: &str,
        error: &str,
        severity: crate::messages::coordination::ErrorSeverity,
    ) -> Result<(), String> {
        let payload = serde_json::json!({
            "actor": actor_name,
            "error": error,
            "severity": severity,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });
        
        // Emit to frontend
        self.event_emitter.emit("actor_error", payload).await?;
        
        // Propagate to actor system
        if let Some(actor_system) = &self.actor_system {
            // forward without moving severity twice
            actor_system.handle_actor_error(
                actor_name.to_string(),
                error.to_string(),
                severity.clone(),
            ).await?;
        }
        
        Ok(())
    }
    
    /// Emit a health check event
    pub async fn emit_health_event(
        &self,
        actor_name: &str,
        status: crate::messages::coordination::HealthStatus,
        message: Option<String>,
    ) -> Result<(), String> {
        let payload = serde_json::json!({
            "actor": actor_name,
            "status": status,
            "message": message,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });
        
        // Emit to frontend
        self.event_emitter.emit("actor_health", payload).await?;
        
        // Notify orchestrator
        if let Some(actor_system) = &self.actor_system {
            let _ = actor_system.perform_system_health_check().await;
        }
        
        Ok(())
    }
    
    /// Emit a performance metric event
    pub async fn emit_performance_event(
        &self,
        metric_name: &str,
        value: f64,
        unit: &str,
    ) -> Result<(), String> {
        let payload = serde_json::json!({
            "metric": metric_name,
            "value": value,
            "unit": unit,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });
        
        // Emit to frontend
        self.event_emitter.emit("performance_metric", payload).await?;
        
        // Record in actor system
        if let Some(actor_system) = &self.actor_system {
            actor_system.record_performance_metric(
                metric_name.to_string(),
                value,
                unit.to_string(),
            ).await?;
        }
        
        Ok(())
    }
    
    /// Emit a resource management event
    pub async fn emit_resource_event(
        &self,
        resource_type: &str,
        resource_id: &str,
        action: &str, // "acquired" or "released"
    ) -> Result<(), String> {
        let payload = serde_json::json!({
            "resource_type": resource_type,
            "resource_id": resource_id,
            "action": action,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });
        
        // Emit to frontend
        self.event_emitter.emit("resource_management", payload).await?;
        
        // Notify actor system
        if let Some(actor_system) = &self.actor_system {
            match action {
                "acquired" => {
                    actor_system.notify_resource_acquired(
                        resource_type.to_string(),
                        resource_id.to_string(),
                    ).await?;
                }
                "released" => {
                    actor_system.notify_resource_released(
                        resource_type.to_string(),
                        resource_id.to_string(),
                    ).await?;
                }
                _ => {
                    return Err(format!("Unknown resource action: {}", action));
                }
            }
        }
        
        Ok(())
    }
    
    /// Emit a dependency event
    pub async fn emit_dependency_event(
        &self,
        dependency_name: &str,
        status: &str, // "ready" or "failed"
        error: Option<String>,
    ) -> Result<(), String> {
        let payload = serde_json::json!({
            "dependency": dependency_name,
            "status": status,
            "error": error,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });
        
        // Emit to frontend
        self.event_emitter.emit("dependency_status", payload).await?;
        
        // Notify actor system
        if let Some(actor_system) = &self.actor_system {
            match status {
                "ready" => {
                    actor_system.notify_dependency_ready(dependency_name.to_string()).await?;
                }
                "failed" => {
                    let error_msg = error.unwrap_or_else(|| "Unknown error".to_string());
                    actor_system.notify_dependency_failed(dependency_name.to_string(), error_msg).await?;
                }
                _ => {
                    return Err(format!("Unknown dependency status: {}", status));
                }
            }
        }
        
        Ok(())
    }
    
    /// Emit a coordination event between actors
    pub async fn emit_coordination_event(
        &self,
        event_type: &str,
        source_actor: &str,
        target_actors: &[&str],
        payload: serde_json::Value,
    ) -> Result<(), String> {
        let coordination_payload = serde_json::json!({
            "event_type": event_type,
            "source_actor": source_actor,
            "target_actors": target_actors,
            "payload": payload,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });
        
        // Emit to frontend
        self.event_emitter.emit("actor_coordination", coordination_payload).await?;
        
        // Send to target actors if actor system is available
        // Actor coordination delivery skipped in refactor
        
        Ok(())
    }
    
    // ============================================================================
    // Existing Event Emission Methods (Enhanced for Actor Integration)
    // ============================================================================
    
    /// Emit account-related event with actor notification
    pub async fn emit_account_event(&self, event_type: &str, payload: AccountEventPayload) -> Result<(), String> {
        let json_payload = serde_json::to_value(payload)
            .map_err(|e| format!("Failed to serialize account event payload: {}", e))?;
        
        // Emit to frontend
        self.event_emitter.emit(event_type, json_payload.clone()).await?;
        
        // Notify relevant actors
        let notify_actors = match event_type {
            "account_login_success" | "account_logout" => vec!["orchestrator", "state"],
            _ => vec!["orchestrator"],
        };
        
        self.emit_with_actor_notification(event_type, json_payload, &notify_actors).await
    }
    
    /// Emit orchestrator-related event with actor notification
    pub async fn emit_orchestrator_event(&self, event_type: &str, payload: OrchestratorEventPayload) -> Result<(), String> {
        let json_payload = serde_json::to_value(payload)
            .map_err(|e| format!("Failed to serialize orchestrator event payload: {}", e))?;
        
        // Emit to frontend
        self.event_emitter.emit(event_type, json_payload.clone()).await?;
        
        // Notify relevant actors based on event type
        let notify_actors = match event_type {
            "orchestrator_startup_complete" => vec!["account", "config", "state"],
            "orchestrator_project_changed" => vec!["state", "execution", "lsp"],
            "orchestrator_julia_started" => vec!["communication", "process"],
            _ => vec![],
        };
        
        self.emit_with_actor_notification(event_type, json_payload, &notify_actors).await
    }
    
    /// Emit LSP-related event with actor notification
    pub async fn emit_lsp_event(&self, event_type: &str, payload: LspEventPayload) -> Result<(), String> {
        let json_payload = serde_json::to_value(payload)
            .map_err(|e| format!("Failed to serialize LSP event payload: {}", e))?;
        
        // Emit to frontend
        self.event_emitter.emit(event_type, json_payload.clone()).await?;
        
        // Notify orchestrator of LSP status changes
        self.emit_with_actor_notification(event_type, json_payload, &["orchestrator"]).await
    }
    
    /// Emit Julia-related event with actor notification
    pub async fn emit_julia_event(&self, event_type: &str, payload: JuliaEventPayload) -> Result<(), String> {
        let json_payload = serde_json::to_value(payload)
            .map_err(|e| format!("Failed to serialize Julia event payload: {}", e))?;
        
        // Emit to frontend
        self.event_emitter.emit(event_type, json_payload.clone()).await?;
        
        // Notify relevant actors
        let notify_actors = match event_type {
            "julia_process_started" => vec!["orchestrator", "communication"],
            "julia_process_stopped" => vec!["orchestrator", "communication"],
            "julia_execution_completed" => vec!["orchestrator", "state"],
            _ => vec!["orchestrator"],
        };
        
        self.emit_with_actor_notification(event_type, json_payload, &notify_actors).await
    }
    
    /// Emit plot-related event with actor notification
    pub async fn emit_plot_event(&self, event_type: &str, payload: PlotEventPayload) -> Result<(), String> {
        let json_payload = serde_json::to_value(payload)
            .map_err(|e| format!("Failed to serialize plot event payload: {}", e))?;
        
        // Emit to frontend
        self.event_emitter.emit(event_type, json_payload.clone()).await?;
        
        // Notify orchestrator of plot events
        self.emit_with_actor_notification(event_type, json_payload, &["orchestrator"]).await
    }
    
    /// Emit file-related event with actor notification
    pub async fn emit_file_event(&self, event_type: &str, payload: FileEventPayload) -> Result<(), String> {
        let json_payload = serde_json::to_value(payload)
            .map_err(|e| format!("Failed to serialize file event payload: {}", e))?;
        
        // Emit to frontend
        self.event_emitter.emit(event_type, json_payload.clone()).await?;
        
        // Notify relevant actors
        let notify_actors = match event_type {
            "file_server_started" | "file_server_stopped" => vec!["orchestrator"],
            "file_changed" => vec!["orchestrator", "state"],
            _ => vec!["orchestrator"],
        };
        
        self.emit_with_actor_notification(event_type, json_payload, &notify_actors).await
    }
    
    /// Emit communication-related event with actor notification
    pub async fn emit_communication_event(&self, event_type: &str, payload: CommunicationEventPayload) -> Result<(), String> {
        let json_payload = serde_json::to_value(payload)
            .map_err(|e| format!("Failed to serialize communication event payload: {}", e))?;
        
        // Emit to frontend
        self.event_emitter.emit(event_type, json_payload.clone()).await?;
        
        // Notify orchestrator of communication status
        self.emit_with_actor_notification(event_type, json_payload, &["orchestrator"]).await
    }

    /// Emit notebook-related event (per-cell outputs)
    pub async fn emit_notebook_event(&self, event_type: &str, payload: NotebookCellEventPayload) -> Result<(), String> {
        let json_payload = serde_json::to_value(payload)
            .map_err(|e| format!("Failed to serialize notebook event payload: {}", e))?;

        // Emit to frontend; event_type should include category prefix (e.g., "julia:notebook-cell-output")
        self.event_emitter.emit(event_type, json_payload.clone()).await?;

        // No actor notifications needed
        Ok(())
    }
    


    // ============================================================================
    // Account Events
    // ============================================================================

    pub async fn emit_account_show_welcome_modal(&self, mode: &str) -> Result<(), String> {
        let payload = serde_json::to_value(AccountEventPayload {
            mode: Some(mode.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize account event: {}", e))?;

        let event = Self::create_event(EventCategory::Account, "show-welcome-modal", payload);
        self.emit_event(event).await
    }

    pub async fn emit_account_login_attempt(&self, email: &str) -> Result<(), String> {
        let payload = serde_json::to_value(AccountEventPayload {
            email: Some(email.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize account event: {}", e))?;

        let event = Self::create_event(EventCategory::Account, "login-attempt", payload);
        self.emit_event(event).await
    }



    pub async fn emit_account_login_failure(&self, error: &str) -> Result<(), String> {
        let payload = serde_json::to_value(AccountEventPayload {
            error: Some(error.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize account event: {}", e))?;

        let event = Self::create_event(EventCategory::Account, "login-failure", payload);
        self.emit_event(event).await
    }

    pub async fn emit_account_authentication_complete(&self, user: User) -> Result<(), String> {
        let payload = serde_json::to_value(AccountEventPayload {
            user: Some(user),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize account event: {}", e))?;

        let event = Self::create_event(EventCategory::Account, "authentication-complete", payload);
        self.emit_event(event).await
    }

    pub async fn emit_account_registration_attempt(&self, email: &str) -> Result<(), String> {
        let payload = serde_json::to_value(AccountEventPayload {
            email: Some(email.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize account event: {}", e))?;

        let event = Self::create_event(EventCategory::Account, "registration-attempt", payload);
        self.emit_event(event).await
    }



    pub async fn emit_account_registration_failure(&self, error: &str) -> Result<(), String> {
        let payload = serde_json::to_value(AccountEventPayload {
            error: Some(error.to_string()),
            can_retry: Some(true),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize account event: {}", e))?;

        let event = Self::create_event(EventCategory::Account, "registration-failure", payload);
        self.emit_event(event).await
    }

    // ============================================================================
    // Orchestrator Events
    // ============================================================================

    pub async fn emit_orchestrator_startup_update(&self, message: &str, progress: u8) -> Result<(), String> {
        let payload = serde_json::to_value(OrchestratorEventPayload {
            message: Some(message.to_string()),
            progress: Some(progress),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize orchestrator event: {}", e))?;

        let event = Self::create_event(EventCategory::Orchestrator, "startup-update", payload);
        self.emit_event(event).await
    }

    pub async fn emit_orchestrator_startup_error(&self, message: &str, progress: u8, error_details: &str) -> Result<(), String> {
        let payload = serde_json::to_value(OrchestratorEventPayload {
            message: Some(message.to_string()),
            progress: Some(progress),
            is_error: Some(true),
            error_details: Some(error_details.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize orchestrator event: {}", e))?;

        let event = Self::create_event(EventCategory::Orchestrator, "startup-error", payload);
        self.emit_event(event).await
    }

    pub async fn emit_orchestrator_backend_ready(&self) -> Result<(), String> {
        let payload = serde_json::to_value(OrchestratorEventPayload {
            message: Some("Backend is ready for frontend handshake".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize orchestrator event: {}", e))?;

        let event = Self::create_event(EventCategory::Orchestrator, "backend-ready", payload);
        self.emit_event(event).await
    }

    pub async fn emit_orchestrator_startup_ready(&self, message: &str) -> Result<(), String> {
        let payload = serde_json::to_value(OrchestratorEventPayload {
            message: Some(message.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize orchestrator event: {}", e))?;

        let event = Self::create_event(EventCategory::Orchestrator, "startup-ready", payload);
        self.emit_event(event).await
    }

    pub async fn emit_orchestrator_julia_restart_started(&self, message: &str) -> Result<(), String> {
        log::debug!("[EventService] Emitting orchestrator:julia_restart_started event with message: {}", message);
        
        let payload = serde_json::to_value(OrchestratorEventPayload {
            message: Some(message.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize orchestrator event: {}", e))?;

        let event = Self::create_event(EventCategory::Orchestrator, "julia_restart_started", payload);
        log::debug!("[EventService] Created event: {}", event.event_type);
        
        let result = self.emit_event(event).await;
        log::debug!("[EventService] Event emission result: {:?}", result);
        result
    }

    pub async fn emit_orchestrator_julia_restart_completed(&self, message: &str) -> Result<(), String> {
        log::debug!("[EventService] Emitting orchestrator:julia_restart_completed event with message: {}", message);
        
        let payload = serde_json::to_value(OrchestratorEventPayload {
            message: Some(message.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize orchestrator event: {}", e))?;

        let event = Self::create_event(EventCategory::Orchestrator, "julia_restart_completed", payload);
        log::debug!("[EventService] Created event: {}", event.event_type);
        
        let result = self.emit_event(event).await;
        log::debug!("[EventService] Event emission result: {:?}", result);
        result
    }

    pub async fn emit_orchestrator_initialization_status(
        &self,
        message: &str,
        progress: u8,
        _is_complete: bool,
        is_error: bool,
        error_details: Option<&str>,
    ) -> Result<(), String> {
        let payload = serde_json::to_value(OrchestratorEventPayload {
            message: Some(message.to_string()),
            progress: Some(progress),
            is_error: Some(is_error),
            error_details: error_details.map(|s| s.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize orchestrator event: {}", e))?;

        let event = Self::create_event(EventCategory::Orchestrator, "initialization-status", payload);
        self.emit_event(event).await
    }

    pub async fn emit_orchestrator_julia_missing_packages(&self, packages: Vec<String>) -> Result<(), String> {
        let payload = serde_json::to_value(OrchestratorEventPayload {
            packages: Some(packages),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize orchestrator event: {}", e))?;

        let event = Self::create_event(EventCategory::Orchestrator, "julia-missing-packages", payload);
        self.emit_event(event).await
    }

    pub async fn emit_orchestrator_project_change_complete(&self, project_path: &str) -> Result<(), String> {
        let payload = serde_json::to_value(OrchestratorEventPayload {
            project_path: Some(project_path.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize orchestrator event: {}", e))?;

        let event = Self::create_event(EventCategory::Orchestrator, "project-change-complete", payload);
        self.emit_event(event).await
    }

    // ============================================================================
    // LSP Events
    // ============================================================================

    pub async fn emit_lsp_installation_started(&self) -> Result<(), String> {
        let payload = serde_json::to_value(LspEventPayload {
            status: Some("started".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize LSP event: {}", e))?;

        let event = Self::create_event(EventCategory::Lsp, "installation-started", payload);
        self.emit_event(event).await
    }

    pub async fn emit_lsp_installation_progress(&self, message: &str, progress: u8) -> Result<(), String> {
        let payload = serde_json::to_value(LspEventPayload {
            message: Some(message.to_string()),
            progress: Some(progress),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize LSP event: {}", e))?;

        let event = Self::create_event(EventCategory::Lsp, "installation-progress", payload);
        self.emit_event(event).await
    }

    pub async fn emit_lsp_installation_error(&self, message: &str, error_details: &str) -> Result<(), String> {
        debug!("[EventService] Emitting lsp:installation-error event: {} - {}", message, error_details);
        
        let payload = serde_json::to_value(LspEventPayload {
            message: Some(message.to_string()),
            error: Some(error_details.to_string()),
            status: Some("error".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize LSP event: {}", e))?;

        let event = Self::create_event(EventCategory::Lsp, "installation-error", payload);
        
        let result = self.emit_event(event).await;
        result
    }

    pub async fn emit_lsp_installation_complete(&self) -> Result<(), String> {
        let payload = serde_json::to_value(LspEventPayload {
            status: Some("complete".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize LSP event: {}", e))?;

        let event = Self::create_event(EventCategory::Lsp, "installation-complete", payload);
        self.emit_event(event).await
    }

    pub async fn emit_lsp_ready(&self) -> Result<(), String> {
        let payload = serde_json::to_value(LspEventPayload {
            status: Some("ready".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize LSP event: {}", e))?;

        let event = Self::create_event(EventCategory::Lsp, "ready", payload);
        self.emit_event(event).await
    }

    pub async fn emit_lsp_status(&self, status: &str, message: &str) -> Result<(), String> {
        let payload = serde_json::to_value(LspEventPayload {
            status: Some(status.to_string()),
            message: Some(message.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize LSP event: {}", e))?;

        let event = Self::create_event(EventCategory::Lsp, "status", payload);
        self.emit_event(event).await
    }

    // Additional LSP event methods that are missing
    pub async fn emit_lsp_server_started(&self, server_info: crate::types::LspServerInfo) -> Result<(), String> {
        let payload = serde_json::to_value(LspEventPayload {
            status: Some("server-started".to_string()),
            port: server_info.port,
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize LSP event: {}", e))?;

        let event = Self::create_event(EventCategory::Lsp, "server-started", payload);
        self.emit_event(event).await
    }

    pub async fn emit_lsp_server_stopped(&self) -> Result<(), String> {
        let payload = serde_json::to_value(LspEventPayload {
            status: Some("server-stopped".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize LSP event: {}", e))?;

        let event = Self::create_event(EventCategory::Lsp, "server-stopped", payload);
        self.emit_event(event).await
    }

    pub async fn emit_lsp_server_error(&self, server_info: crate::types::LspServerInfo) -> Result<(), String> {
        let payload = serde_json::to_value(LspEventPayload {
            status: Some("server-error".to_string()),
            message: server_info.error_message.clone(),
            error: server_info.error_message.clone(),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize LSP event: {}", e))?;

        let event = Self::create_event(EventCategory::Lsp, "server-error", payload);
        self.emit_event(event).await
    }

    pub async fn emit_lsp_initialized(&self, project_path: String) -> Result<(), String> {
        let payload = serde_json::to_value(LspEventPayload {
            status: Some("initialized".to_string()),
            message: Some(format!("LSP initialized for project: {}", project_path)),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize LSP event: {}", e))?;

        let event = Self::create_event(EventCategory::Lsp, "initialized", payload);
        self.emit_event(event).await
    }

    pub async fn emit_lsp_shutdown(&self) -> Result<(), String> {
        let payload = serde_json::to_value(LspEventPayload {
            status: Some("shutdown".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize LSP event: {}", e))?;

        let event = Self::create_event(EventCategory::Lsp, "shutdown", payload);
        self.emit_event(event).await
    }

    pub async fn emit_lsp_server_restarted(&self) -> Result<(), String> {
        let payload = serde_json::to_value(LspEventPayload {
            status: Some("server-restarted".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize LSP event: {}", e))?;

        let event = Self::create_event(EventCategory::Lsp, "server-restarted", payload);
        self.emit_event(event).await
    }

    // ============================================================================
    // Julia Events
    // ============================================================================

    pub async fn emit_julia_process_started(&self) -> Result<(), String> {
        let payload = serde_json::to_value(JuliaEventPayload {
            status: Some("started".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "process-started", payload);
        self.emit_event(event).await
    }

    pub async fn emit_julia_daemon_status_changed(&self, status: &str) -> Result<(), String> {
        let payload = serde_json::to_value(JuliaEventPayload {
            status: Some(status.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "daemon-status-changed", payload);
        self.emit_event(event).await
    }

    pub async fn emit_julia_error(&self, error: &str) -> Result<(), String> {
        let payload = serde_json::to_value(JuliaEventPayload {
            error: Some(error.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "error", payload);
        self.emit_event(event).await
    }

    // Julia installation event methods
    pub async fn emit_julia_installation_found(&self, installations: Vec<crate::types::JuliaInstallation>) -> Result<(), String> {
        let payload = serde_json::to_value(JuliaInstallationEventPayload {
            installations: Some(installations),
            status: Some("found".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia installation event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "installation-found", payload);
        self.emit_event(event).await
    }

    pub async fn emit_julia_installation_not_found(&self) -> Result<(), String> {
        let payload = serde_json::to_value(JuliaInstallationEventPayload {
            status: Some("not-found".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia installation event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "installation-not-found", payload);
        self.emit_event(event).await
    }

    pub async fn emit_julia_installation_started(&self, version: String) -> Result<(), String> {
        let payload = serde_json::to_value(JuliaInstallationEventPayload {
            status: Some("started".to_string()),
            version: Some(version),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia installation event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "installation-started", payload);
        self.emit_event(event).await
    }

    pub async fn emit_julia_installation_completed(&self, installation: crate::types::JuliaInstallation) -> Result<(), String> {
        let payload = serde_json::to_value(JuliaInstallationEventPayload {
            installation: Some(installation),
            status: Some("completed".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia installation event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "installation-completed", payload);
        self.emit_event(event).await
    }

    pub async fn emit_current_installation_changed(&self, installation: crate::types::JuliaInstallation) -> Result<(), String> {
        let payload = serde_json::to_value(JuliaInstallationEventPayload {
            installation: Some(installation),
            status: Some("changed".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia installation event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "current-installation-changed", payload);
        self.emit_event(event).await
    }

    pub async fn emit_julia_installation_progress(&self, message: &str, progress: u8) -> Result<(), String> {
        // Only log every 10% progress to reduce verbosity
        if progress % 10 == 0 {
            debug!("[EventService] Emitting julia:installation-progress event: {} ({}%)", message, progress);
        }
        
        let payload = serde_json::to_value(JuliaInstallationEventPayload {
            message: Some(message.to_string()),
            progress: Some(progress),
            status: Some("progress".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia installation event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "installation-progress", payload);
        
        let result = self.emit_event(event).await;
        result
    }

    pub async fn emit_julia_installation_error(&self, message: &str, error_details: &str) -> Result<(), String> {
        debug!("[EventService] Emitting julia:installation-error event: {} - {}", message, error_details);
        
        let payload = serde_json::to_value(JuliaInstallationEventPayload {
            message: Some(message.to_string()),
            error: Some(error_details.to_string()),
            status: Some("error".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia installation event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "installation-error", payload);
        debug!("[EventService] Created event: {} with payload: {:?}", event.event_type, event.payload);
        
        let result = self.emit_event(event).await;
        debug!("[EventService] Event emission result: {:?}", result);
        result
    }

    pub async fn emit_installations_detected(&self, installations: Vec<crate::types::JuliaInstallation>) -> Result<(), String> {
        let payload = serde_json::to_value(JuliaInstallationEventPayload {
            installations: Some(installations),
            status: Some("detected".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia installation event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "installations-detected", payload);
        self.emit_event(event).await
    }

    pub async fn emit_installation_repair_started(&self, installation: crate::types::JuliaInstallation) -> Result<(), String> {
        let payload = serde_json::to_value(JuliaInstallationEventPayload {
            installation: Some(installation),
            status: Some("repair-started".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia installation event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "installation-repair-started", payload);
        self.emit_event(event).await
    }

    pub async fn emit_installation_repair_completed(&self, installation: crate::types::JuliaInstallation) -> Result<(), String> {
        let payload = serde_json::to_value(JuliaInstallationEventPayload {
            installation: Some(installation),
            status: Some("repair-completed".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize Julia installation event: {}", e))?;

        let event = Self::create_event(EventCategory::Julia, "installation-repair-completed", payload);
        self.emit_event(event).await
    }

    // Sysimage event methods
    pub async fn emit_sysimage_available(&self, sysimages: Vec<crate::types::SysimageInfo>) -> Result<(), String> {
        let payload = serde_json::to_value(SysimageEventPayload {
            sysimages: Some(sysimages),
            status: Some("available".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize sysimage event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "sysimage-available", payload);
        self.emit_event(event).await
    }

    pub async fn emit_sysimage_not_available(&self) -> Result<(), String> {
        let payload = serde_json::to_value(SysimageEventPayload {
            status: Some("not-available".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize sysimage event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "sysimage-not-available", payload);
        self.emit_event(event).await
    }

    pub async fn emit_sysimage_download_started(&self) -> Result<(), String> {
        let payload = serde_json::to_value(SysimageEventPayload {
            status: Some("download-started".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize sysimage event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "sysimage-download-started", payload);
        self.emit_event(event).await
    }

    pub async fn emit_sysimage_download_completed(&self, sysimage: crate::types::SysimageInfo) -> Result<(), String> {
        let payload = serde_json::to_value(SysimageEventPayload {
            sysimage: Some(sysimage),
            status: Some("download-completed".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize sysimage event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "sysimage-download-completed", payload);
        self.emit_event(event).await
    }

    pub async fn emit_sysimage_compilation_started(&self) -> Result<(), String> {
        let payload = serde_json::to_value(SysimageEventPayload {
            status: Some("compilation-started".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize sysimage event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "sysimage-compilation-started", payload);
        self.emit_event(event).await
    }

    pub async fn emit_sysimage_compilation_completed(&self, sysimage: crate::types::SysimageInfo) -> Result<(), String> {
        let payload = serde_json::to_value(SysimageEventPayload {
            sysimage: Some(sysimage),
            status: Some("compilation-completed".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize sysimage event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "sysimage-compilation-completed", payload);
        self.emit_event(event).await
    }

    pub async fn emit_current_sysimage_changed(&self, sysimage: crate::types::SysimageInfo) -> Result<(), String> {
        let payload = serde_json::to_value(SysimageEventPayload {
            sysimage: Some(sysimage),
            status: Some("changed".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize sysimage event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "current-sysimage-changed", payload);
        self.emit_event(event).await
    }

    pub async fn emit_sysimage_loaded(&self, sysimage: crate::types::SysimageInfo) -> Result<(), String> {
        let payload = serde_json::to_value(SysimageEventPayload {
            sysimage: Some(sysimage),
            status: Some("loaded".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize sysimage event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "sysimage-loaded", payload);
        self.emit_event(event).await
    }

    pub async fn emit_sysimage_unloaded(&self, sysimage: crate::types::SysimageInfo) -> Result<(), String> {
        let payload = serde_json::to_value(SysimageEventPayload {
            sysimage: Some(sysimage),
            status: Some("unloaded".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize sysimage event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "sysimage-unloaded", payload);
        self.emit_event(event).await
    }

    pub async fn emit_sysimage_optimization_started(&self, sysimage: crate::types::SysimageInfo) -> Result<(), String> {
        let payload = serde_json::to_value(SysimageEventPayload {
            sysimage: Some(sysimage),
            status: Some("optimization-started".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize sysimage event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "sysimage-optimization-started", payload);
        self.emit_event(event).await
    }

    pub async fn emit_sysimage_optimization_completed(&self, sysimage: crate::types::SysimageInfo) -> Result<(), String> {
        let payload = serde_json::to_value(SysimageEventPayload {
            sysimage: Some(sysimage),
            status: Some("optimization-completed".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize sysimage event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "sysimage-optimization-completed", payload);
        self.emit_event(event).await
    }

    pub async fn emit_sysimage_removed(&self, sysimage: crate::types::SysimageInfo) -> Result<(), String> {
        let payload = serde_json::to_value(SysimageEventPayload {
            sysimage: Some(sysimage),
            status: Some("removed".to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize sysimage event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "sysimage-removed", payload);
        self.emit_event(event).await
    }

    // ============================================================================
    // Plot Events
    // ============================================================================

    pub async fn emit_plot_server_started(&self, port: u16) -> Result<(), String> {
        let payload = serde_json::to_value(PlotEventPayload {
            port: Some(port),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize plot event: {}", e))?;

        let event = Self::create_event(EventCategory::Plot, "server-started", payload);
        self.emit_event(event).await
    }

    pub async fn emit_plot_server_stopped(&self) -> Result<(), String> {
        let payload = serde_json::to_value(PlotEventPayload {
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize plot event: {}", e))?;

        let event = Self::create_event(EventCategory::Plot, "server-stopped", payload);
        self.emit_event(event).await
    }

    pub async fn emit_plot_server_restarted(&self, port: u16) -> Result<(), String> {
        let payload = serde_json::to_value(PlotEventPayload {
            port: Some(port),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize plot event: {}", e))?;

        let event = Self::create_event(EventCategory::Plot, "server-restarted", payload);
        self.emit_event(event).await
    }

    pub async fn emit_plot_added(&self, plot_id: &str) -> Result<(), String> {
        let payload = serde_json::to_value(PlotEventPayload {
            plot_id: Some(plot_id.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize plot event: {}", e))?;

        let event = Self::create_event(EventCategory::Plot, "plot-added", payload);
        self.emit_event(event).await
    }

    pub async fn emit_plot_deleted(&self, plot_id: &str) -> Result<(), String> {
        let payload = serde_json::to_value(PlotEventPayload {
            plot_id: Some(plot_id.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize plot event: {}", e))?;

        let event = Self::create_event(EventCategory::Plot, "plot-deleted", payload);
        self.emit_event(event).await
    }

    pub async fn emit_plot_updated(&self, _old_plot: PlotData, new_plot: PlotData) -> Result<(), String> {
        let payload = serde_json::to_value(PlotEventPayload {
            plot_id: Some(new_plot.id.clone()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize plot event: {}", e))?;

        let event = Self::create_event(EventCategory::Plot, "plot-updated", payload);
        self.emit_event(event).await
    }

    // ============================================================================
    // File Events
    // ============================================================================

    pub async fn emit_file_server_started(&self, port: u16) -> Result<(), String> {
        let payload = serde_json::to_value(FileEventPayload {
            port: Some(port),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize file event: {}", e))?;

        let event = Self::create_event(EventCategory::File, "server-started", payload);
        self.emit_event(event).await
    }

    pub async fn emit_file_server_stopped(&self) -> Result<(), String> {
        let payload = serde_json::to_value(FileEventPayload {
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize file event: {}", e))?;

        let event = Self::create_event(EventCategory::File, "server-stopped", payload);
        self.emit_event(event).await
    }

    pub async fn emit_file_server_error(&self, error: &str) -> Result<(), String> {
        let payload = serde_json::to_value(FileEventPayload {
            error: Some(error.to_string()),
            message: Some(format!("File server error: {}", error)),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize file event: {}", e))?;

        let event = Self::create_event(EventCategory::File, "server-error", payload);
        self.emit_event(event).await
    }
    
    pub async fn emit_file_server_restarted(&self, port: u16) -> Result<(), String> {
        let payload = serde_json::to_value(FileEventPayload {
            port: Some(port),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize file event: {}", e))?;

        let event = Self::create_event(EventCategory::File, "server-restarted", payload);
        self.emit_event(event).await
    }

    // ============================================================================
    // Communication Events
    // ============================================================================

    pub async fn emit_communication_execution_complete(&self, request_id: &str, result: Option<&str>, error: Option<&str>) -> Result<(), String> {
        let payload = serde_json::to_value(CommunicationEventPayload {
            request_id: Some(request_id.to_string()),
            message: result.map(|s| s.to_string()),
            error: error.map(|s| s.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize communication event: {}", e))?;

        let event = Self::create_event(EventCategory::Communication, "execution-complete", payload);
        self.emit_event(event).await
    }

    pub async fn emit_communication_session_status(&self, status: &str, message: Option<&str>) -> Result<(), String> {
        let payload = serde_json::to_value(CommunicationEventPayload {
            status: Some(status.to_string()),
            message: message.map(|s| s.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize communication event: {}", e))?;

        let event = Self::create_event(EventCategory::Communication, "session-status", payload);
        self.emit_event(event).await
    }

    // ============================================================================
    // System Events
    // ============================================================================

    pub async fn emit_system_error(&self, error: &str) -> Result<(), String> {
        let payload = serde_json::to_value(SystemEventPayload {
            error: Some(error.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize system event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "error", payload);
        self.emit_event(event).await
    }

    pub async fn emit_system_status(&self, status: &str, message: Option<&str>) -> Result<(), String> {
        let payload = serde_json::to_value(SystemEventPayload {
            status: Some(status.to_string()),
            message: message.map(|s| s.to_string()),
            ..Default::default()
        }).map_err(|e| format!("Failed to serialize system event: {}", e))?;

        let event = Self::create_event(EventCategory::System, "status", payload);
        self.emit_event(event).await
    }

    // ============================================================================
    // Additional Events for Backward Compatibility
    // ============================================================================

    pub async fn emit_backend_busy(&self, request_id: &str) -> Result<(), String> {
        let payload = serde_json::json!({
            "is_busy": true,
            "request_id": request_id,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        });

        // Emit directly with the event name the frontend expects
        self.event_emitter.emit("backend-busy", payload).await
    }

    pub async fn emit_backend_done(&self, request_id: &str) -> Result<(), String> {
        let payload = serde_json::json!({
            "is_busy": false,
            "request_id": request_id,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        });

        // Emit directly with the event name the frontend expects
        self.event_emitter.emit("backend-done", payload).await
    }

    pub async fn emit_julia_output(&self, content: &str) -> Result<(), String> {
        let payload = serde_json::json!(vec![crate::messages::StreamOutput {
            content: content.to_string(),
            stream_type: crate::messages::StreamType::Stdout,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }]);

        let event = Self::create_event(EventCategory::Julia, "output", payload);
        self.emit_event(event).await
    }

    pub async fn emit_connection_test_response(&self, id: &str, response: &str, timestamp: i64) -> Result<(), String> {
        let payload = serde_json::json!({
            "event_type": "connection_test_response",
            "id": id,
            "response": response,
            "timestamp": timestamp
        });

        let event = Self::create_event(EventCategory::Communication, "connection-test-response", payload);
        self.emit_event(event).await
    }

    pub async fn emit_syntax_diagnostics(&self, uri: String, diagnostics: Vec<crate::types::LspDiagnostic>) -> Result<(), String> {
        let payload = serde_json::json!({
            "uri": uri,
            "diagnostics": diagnostics
        });

        let event = Self::create_event(EventCategory::Lsp, "syntax-diagnostics", payload);
        self.emit_event(event).await
    }
}

// Implement Default for all event payloads










