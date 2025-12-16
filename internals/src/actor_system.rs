use actix::prelude::*;
use actix::Supervisor;
use std::sync::Arc;
use log::{debug, error};



use crate::actors::{
    OrchestratorActor, ConfigurationActor, StateActor, ExecutionActor,
    CommunicationActor, ProcessActor, LspActor, PlotActor,
    ProjectActor, FilesystemActor, FileWatcherActor, FileServerActor, InstallationActor,
};
use crate::messages::orchestrator::SetActorAddresses;
use crate::messages::coordination::{ErrorSeverity, ActorError, ActorHealth, DependencyReady, DependencyFailed, ResourceAcquired, ResourceReleased, PerformanceMetric, DebugLog};
use crate::services::events::EventService;
// Service traits no longer needed - all services are constructed internally by actors

/// ActorSystem - central coordinator for all actors
/// This replaces the mutex-based manager architecture with a clean actor model
pub struct ActorSystem {
    // Actor addresses
    pub orchestrator_actor: Addr<OrchestratorActor>,
    pub config_actor: Addr<ConfigurationActor>,
    pub state_actor: Addr<StateActor>,
    pub execution_actor: Addr<ExecutionActor>,
    pub communication_actor: Addr<CommunicationActor>,
    pub process_actor: Addr<ProcessActor>,
    pub lsp_actor: Addr<LspActor>,
    pub plot_actor: Addr<PlotActor>,
    pub file_server_actor: Addr<FileServerActor>,
    pub installation_actor: Addr<InstallationActor>,
    pub filesystem_actor: Addr<FilesystemActor>,
    pub file_watcher_actor: Addr<FileWatcherActor>,
    pub project_actor: Addr<ProjectActor>,
    // pub sysimage_actor: Addr<SysimageActor>,
    
    // Event manager for shared event coordination
    pub event_manager: EventService,
}

impl ActorSystem {
    /// Create a new ActorSystem instance
    /// All services are now constructed internally by their respective actors
    pub async fn new(
        event_manager: EventService,
        _environment_config: crate::config::EnvironmentConfig,
    ) -> Self {
        
        // Create all actors (but don't initialize them yet)
        // Create ConfigurationActor first since other actors depend on it
        // ConfigurationActor now constructs ConfigurationService internally
        let config_actor = ConfigurationActor::new(
            event_manager.clone(),
        ).start();
        
        let state_actor = StateActor::new(
            event_manager.clone(),
        ).start();
        
        // InstallationActor now contains all installation logic internally
        let installation_actor = InstallationActor::new(
            event_manager.clone(),
            Some(config_actor.clone()),
            None, // Will be set later
        ).start();
        
        // ProcessActor manages Julia process internally (no separate service)
        let event_manager_for_proc = event_manager.clone();
        let event_emitter_for_proc = event_manager.get_event_emitter().clone();
        let installation_actor_for_proc = installation_actor.clone();
        let process_actor = Supervisor::start(move |_| {
            ProcessActor::new(
                event_emitter_for_proc.clone(),
                event_manager_for_proc.clone(),
                Some(installation_actor_for_proc.clone()),
            )
        });
        
        // LspActor now constructs LspService internally
        let lsp_actor = LspActor::new(
            event_manager.clone(),
            Some(config_actor.clone()),
            Some(installation_actor.clone()),
        ).start();
        
        // PlotActor contains all plot functionality internally
        let plot_actor = PlotActor::new(
            // Get event_emitter from event_manager (it's a wrapper around EventEmitter)
            event_manager.get_event_emitter().clone(),
            event_manager.clone(),
        ).start();
        
        // CommunicationActor manages communication with Julia processes
        // It needs PlotActor and ProcessActor addresses
        let event_emitter_for_comm = event_manager.get_event_emitter().clone();
        let plot_actor_for_comm = plot_actor.clone();
        let process_actor_for_comm = process_actor.clone();
        let event_manager_for_comm = event_manager.clone();
        let communication_actor = Supervisor::start(move |_| {
            CommunicationActor::new(
                event_emitter_for_comm.clone(),
                plot_actor_for_comm.clone(),
                process_actor_for_comm.clone(),
                event_manager_for_comm.clone(),
            )
        });
        
        // ExecutionActor uses CommunicationActor for code execution
        let communication_actor_for_exec = communication_actor.clone();
        let event_manager_for_exec = event_manager.clone();
        let execution_actor = ExecutionActor::new(
            communication_actor_for_exec,
            event_manager_for_exec,
        ).start();
        
        // PlotActor is set in CommunicationActor constructor for proper plot data routing
        // This ensures all plot data goes through PlotActor's mailbox for serialization
        
        // FileServerActor manages file server internally (no separate service)
        let file_server_actor = FileServerActor::new(
            event_manager.get_event_emitter().clone(),
            event_manager.clone(),
            Some(config_actor.clone()),
        ).start();
        
        // Sysimage actor removed
        
        let orchestrator_actor = OrchestratorActor::new(
            event_manager.clone(),
        ).start();
        let filesystem_actor = FilesystemActor::new().start();
        let file_watcher_actor = FileWatcherActor::new(Arc::new(event_manager.clone())).start();
        let project_actor = ProjectActor::new().start();
        
        // Configuration is now loaded automatically during ConfigurationActor initialization
        
        // Set orchestrator address on LspActor for coordination
        lsp_actor.do_send(crate::actors::lsp_actor::SetOrchestratorActor {
            orchestrator_actor: orchestrator_actor.clone(),
        });
        
        // Set orchestrator address on CommunicationActor for restart coordination
        let _ = communication_actor.send(crate::messages::communication::SetOrchestratorActor {
            orchestrator_actor: orchestrator_actor.clone(),
        }).await;
        
        // Set actor addresses in orchestrator for coordination
        let _ = orchestrator_actor.send(SetActorAddresses {
            config_actor: config_actor.clone(),
            state_actor: state_actor.clone(),
            execution_actor: execution_actor.clone(),
            communication_actor: communication_actor.clone(),
            process_actor: process_actor.clone(),
            lsp_actor: lsp_actor.clone(),
            plot_actor: plot_actor.clone(),
            file_server_actor: file_server_actor.clone(),
            installation_actor: installation_actor.clone(),
        }).await;
        
        // Set orchestrator actor address in installation actor for completion notifications
        let _ = installation_actor.send(crate::messages::installation::SetOrchestratorActor {
            orchestrator_actor: orchestrator_actor.clone(),
        }).await;
        
        // Set orchestrator actor address in process actor for Julia ready notifications
        let _ = process_actor.send(crate::messages::process::SetOrchestratorActor {
            orchestrator_actor: orchestrator_actor.clone(),
        }).await;
        
        // Set communication actor address in process actor for direct pipe coordination
        let _ = process_actor.send(crate::messages::process::SetCommunicationActor {
            communication_actor: communication_actor.clone(),
        }).await;
        
        Self {
            orchestrator_actor,
            config_actor,
            state_actor,
            execution_actor,
            communication_actor,
            process_actor,
            lsp_actor,
            plot_actor,
            file_server_actor,
            installation_actor,
            filesystem_actor,
            file_watcher_actor,
            project_actor,
            // sysimage_actor,
            event_manager,
        }
    }


    /// Read Project.toml via ProjectActor
    pub async fn read_project_toml(&self, project_path: String) -> Result<serde_json::Value, String> {
        use crate::messages::filesystem::ReadProjectToml;
        self.project_actor
            .send(ReadProjectToml { project_path })
            .await
            .map_err(|_| "Failed to communicate with ProjectActor".to_string())?
    }

    /// Write Project.toml via ProjectActor
    pub async fn write_project_toml(&self, config: serde_json::Value) -> Result<(), String> {
        use crate::messages::filesystem::WriteProjectToml;
        self.project_actor
            .send(WriteProjectToml { config })
            .await
            .map_err(|_| "Failed to communicate with ProjectActor".to_string())??;
        Ok(())
    }

    
    /// Initialize the actor system
    pub async fn initialize(&self) -> Result<(), String> {
        
        Ok(())
    }
    
    // ============================================================================
    // Phase 5: Inter-Actor Communication and Message Routing
    // ============================================================================
    
    // Generic helpers removed; use specific actor addresses directly
    
    /// Handle system-wide coordination messages
    pub async fn handle_system_coordination(&self, _message: crate::messages::coordination::SystemReady) -> Result<(), String> {
        debug!("ActorSystem: Handling system ready coordination");
        
        // Coordination disabled in refactor
        Ok(())
    }
    
    /// Handle emergency shutdown coordination
    pub async fn handle_emergency_shutdown(&self, reason: String) -> Result<(), String> {
        error!("ActorSystem: Emergency shutdown requested: {}", reason);
        
        // System shutdown coordination disabled in refactor: just log
        Ok(())
    }
    
    /// Handle actor error propagation
    pub async fn handle_actor_error(&self, actor_name: String, error: String, severity: ErrorSeverity) -> Result<(), String> {
        error!("ActorSystem: Actor error from {}: {} (severity: {:?})", actor_name, error, severity);
        
        let error_msg = ActorError {
            actor_name: actor_name.clone(),
            error: error.clone(),
            severity: severity.clone(),
        };
        
        // Notify orchestrator of the error
        self.orchestrator_actor.send(error_msg).await
            .map_err(|e| format!("Failed to notify orchestrator of actor error: {:?}", e))??;
        
        // For critical errors, consider emergency shutdown
        if matches!(severity, ErrorSeverity::Critical) {
            self.handle_emergency_shutdown(format!("Critical error from actor: {}", actor_name)).await?;
        }
        
        Ok(())
    }
    
    /// Handle health check coordination
    pub async fn perform_system_health_check(&self) -> Result<Vec<ActorHealth>, String> {
        debug!("ActorSystem: Performing system health check");
        
        Ok(Vec::new())
    }
    
    /// Handle dependency coordination
    pub async fn notify_dependency_ready(&self, dependency_name: String) -> Result<(), String> {
        debug!("ActorSystem: Notifying dependency ready: {}", dependency_name);
        
        let dependency_msg = DependencyReady { dependency_name };
        
        // Notify orchestrator of dependency ready
        self.orchestrator_actor.send(dependency_msg).await
            .map_err(|e| format!("Failed to notify orchestrator of dependency ready: {:?}", e))??;
        
        Ok(())
    }
    
    /// Handle dependency failure
    pub async fn notify_dependency_failed(&self, dependency_name: String, error: String) -> Result<(), String> {
        error!("ActorSystem: Notifying dependency failed: {} - {}", dependency_name, error);
        
        let dependency_msg = DependencyFailed { dependency_name, error };
        
        // Notify orchestrator of dependency failure
        self.orchestrator_actor.send(dependency_msg).await
            .map_err(|e| format!("Failed to notify orchestrator of dependency failure: {:?}", e))??;
        
        Ok(())
    }
    
    /// Handle resource management coordination
    pub async fn notify_resource_acquired(&self, resource_type: String, resource_id: String) -> Result<(), String> {
        debug!("ActorSystem: Notifying resource acquired: {} - {}", resource_type, resource_id);
        
        let resource_msg = ResourceAcquired { resource_type, resource_id };
        
        // Notify orchestrator of resource acquisition
        self.orchestrator_actor.send(resource_msg).await
            .map_err(|e| format!("Failed to notify orchestrator of resource acquisition: {:?}", e))??;
        
        Ok(())
    }
    
    /// Handle resource release coordination
    pub async fn notify_resource_released(&self, resource_type: String, resource_id: String) -> Result<(), String> {
        debug!("ActorSystem: Notifying resource released: {} - {}", resource_type, resource_id);
        
        let resource_msg = ResourceReleased { resource_type, resource_id };
        
        // Notify orchestrator of resource release
        self.orchestrator_actor.send(resource_msg).await
            .map_err(|e| format!("Failed to notify orchestrator of resource release: {:?}", e))??;
        
        Ok(())
    }
    
    /// Handle performance monitoring coordination
    pub async fn record_performance_metric(&self, metric_name: String, value: f64, unit: String) -> Result<(), String> {
        debug!("ActorSystem: Recording performance metric: {} = {} {}", metric_name, value, unit);
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        
        let metric_msg = PerformanceMetric {
            metric_name,
            value,
            unit,
            timestamp,
        };
        
        // Send to orchestrator for monitoring
        self.orchestrator_actor.send(metric_msg).await
            .map_err(|e| format!("Failed to record performance metric: {:?}", e))??;
        
        Ok(())
    }
    
    /// Handle debug logging coordination
    pub async fn log_debug_message(&self, level: String, message: String, actor_name: String) -> Result<(), String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        
        let debug_msg = DebugLog {
            level,
            message,
            actor_name,
            timestamp,
        };
        
        // Send to orchestrator for centralized logging
        self.orchestrator_actor.send(debug_msg).await
            .map_err(|e| format!("Failed to log debug message: {:?}", e))??;
        
        Ok(())
    }
    
    /// Get actor addresses for direct communication (for advanced use cases)
    pub fn get_actor_addresses(&self) -> ActorAddresses {
        ActorAddresses {
            orchestrator_actor: self.orchestrator_actor.clone(),
            config_actor: self.config_actor.clone(),
            state_actor: self.state_actor.clone(),
            execution_actor: self.execution_actor.clone(),
            communication_actor: self.communication_actor.clone(),
            process_actor: self.process_actor.clone(),
            lsp_actor: self.lsp_actor.clone(),
            plot_actor: self.plot_actor.clone(),
            file_server_actor: self.file_server_actor.clone(),
            installation_actor: self.installation_actor.clone(),
            // sysimage_actor: self.sysimage_actor.clone(),
        }
    }

    /// Emit backend ready event to frontend
    pub async fn emit_backend_ready(&self) -> Result<(), String> {
        self.event_manager.emit_orchestrator_backend_ready().await
    }

}

/// Helper struct for getting actor addresses
pub struct ActorAddresses {
    pub orchestrator_actor: Addr<OrchestratorActor>,
    pub config_actor: Addr<ConfigurationActor>,
    pub state_actor: Addr<StateActor>,
    pub execution_actor: Addr<ExecutionActor>,
    pub communication_actor: Addr<CommunicationActor>,
    pub process_actor: Addr<ProcessActor>,
    pub lsp_actor: Addr<LspActor>,
    pub plot_actor: Addr<PlotActor>,
    pub file_server_actor: Addr<FileServerActor>,
    pub installation_actor: Addr<InstallationActor>,
   // pub sysimage_actor: Addr<SysimageActor>,
}

impl Drop for ActorSystem {
    fn drop(&mut self) {
        debug!("ActorSystem: Dropping actor system");
        // Actors will be stopped automatically when their addresses are dropped
    }
}
