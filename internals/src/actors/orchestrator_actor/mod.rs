use actix::prelude::*;
use log::{debug, error, warn};
// std::time::Instant removed - no longer needed for watchdog timers

use crate::messages::process::*;
use crate::messages::plot::*;
use crate::messages::file_server::*;
use crate::messages::communication::*;
use crate::messages::execution::*;
use crate::messages::lsp::*;
// UpdateStartupPhase and PhaseTimeout removed - state machine handles transitions

use crate::services::events::{EventService, OrchestratorEventPayload};
use crate::types::{OrchestratorState, ProjectInfo};

// Include message handlers from separate file
mod handlers;
mod startup_state;
mod startup_phases;
mod startup_state_machine;
mod work_handlers;

pub use startup_state::StartupPhase;
pub use startup_phases::{StartupPhaseContext, PhaseTimeouts};

/// OrchestratorActor - manages application lifecycle and coordination
/// This replaces the mutex-based Orchestrator with a clean actor model
pub struct OrchestratorActor {
    // Actor state (no mutexes needed)
    state: OrchestratorState,
    current_project: Option<ProjectInfo>,
    shutdown_requested: bool,
    
    // Startup phase state machine (replaces multiple boolean flags)
    startup_phase: StartupPhase,
    
    // Phase tracking for watchdog timer
    // Watchdog timer fields removed - user doesn't want timeouts
    
    // External communication services only (no mutex-based managers)
    event_manager: EventService,
    
    // Actor addresses for coordination
    config_actor: Option<Addr<crate::actors::ConfigurationActor>>,
    state_actor: Option<Addr<crate::actors::StateActor>>,
    execution_actor: Option<Addr<crate::actors::ExecutionActor>>,
    communication_actor: Option<Addr<crate::actors::CommunicationActor>>,
    process_actor: Option<Addr<crate::actors::ProcessActor>>,
    lsp_actor: Option<Addr<crate::actors::LspActor>>,
    plot_actor: Option<Addr<crate::actors::PlotActor>>,
    file_server_actor: Option<Addr<crate::actors::FileServerActor>>,
    installation_actor: Option<Addr<crate::actors::InstallationActor>>,
}

impl OrchestratorActor {
    /// Create a new OrchestratorActor instance
    pub fn new(
        event_manager: EventService,
    ) -> Self {
        
        Self {
            state: OrchestratorState::Initializing,
            current_project: None,
            shutdown_requested: false,
            startup_phase: StartupPhase::NotStarted,
            // Watchdog timer fields removed
            event_manager,
            config_actor: None,
            state_actor: None,
            execution_actor: None,
            communication_actor: None,
            process_actor: None,
            lsp_actor: None,
            plot_actor: None,
            file_server_actor: None,
            installation_actor: None,
        }
    }
    
    /// Set actor addresses for coordination
    #[allow(clippy::too_many_arguments)]
    pub fn set_actor_addresses(
        &mut self,
        config_actor: Addr<crate::actors::ConfigurationActor>,
        state_actor: Addr<crate::actors::StateActor>,
        execution_actor: Addr<crate::actors::ExecutionActor>,
        communication_actor: Addr<crate::actors::CommunicationActor>,
        process_actor: Addr<crate::actors::ProcessActor>,
        lsp_actor: Addr<crate::actors::LspActor>,
        plot_actor: Addr<crate::actors::PlotActor>,
        file_server_actor: Addr<crate::actors::FileServerActor>,
        installation_actor: Addr<crate::actors::InstallationActor>,
    ) {
        self.config_actor = Some(config_actor);
        self.state_actor = Some(state_actor);
        self.execution_actor = Some(execution_actor);
        self.communication_actor = Some(communication_actor);
        self.process_actor = Some(process_actor);
        self.lsp_actor = Some(lsp_actor);
        self.plot_actor = Some(plot_actor);
        self.file_server_actor = Some(file_server_actor);
        self.installation_actor = Some(installation_actor);
    }
    
    
    // Watchdog timer methods removed - user doesn't want timeouts
    // State machine handles all transitions now
    
    /// Transition startup phase if valid (simplified - no watchdog timers)
    /// Note: This is kept for backward compatibility with methods that still use it
    /// The state machine is the primary mechanism for transitions
    fn transition_startup_phase(&mut self, next_phase: StartupPhase) -> bool {
        let old_phase = self.startup_phase.clone();
        self.startup_phase = self.startup_phase.clone().transition_to(next_phase.clone());
        
        if self.startup_phase != old_phase {
            debug!("OrchestratorActor: Startup phase transitioned from {:?} to {:?}", old_phase, self.startup_phase);
            true
        } else {
            false
        }
    }
    
    
    /// Continue orchestrator startup using state machine
    /// 
    /// Note: Actor message handlers that spawn async tasks return immediately,
    /// so we only await the message send, not the actual operation completion.
    #[allow(dead_code)]
    async fn continue_startup(&mut self, _actor_addr: Option<actix::Addr<Self>>) -> Result<(), String> {
        debug!("OrchestratorActor: Continuing startup from phase: {:?}", self.startup_phase);

        // Idempotency guard: if startup already completed or failed, skip
        if self.startup_phase.is_completed() || self.startup_phase.is_failed() {
            debug!("OrchestratorActor: Startup already completed/failed; ignoring continue request");
            return Ok(());
        }
        
        // Transition to CheckingJulia phase if needed (authentication phase removed)
        if matches!(self.startup_phase, StartupPhase::CheckingForUpdates) {
            self.transition_startup_phase(StartupPhase::CheckingJulia);
            // Phase updates now handled by state machine
        } else if !matches!(self.startup_phase, StartupPhase::CheckingJulia | StartupPhase::InstallingJulia | StartupPhase::StartingJuliaProcess) {
            debug!("OrchestratorActor: Not in expected phase for startup continuation, current: {:?}", self.startup_phase);
            return Ok(());
        }
        
        self.state = OrchestratorState::Initializing;
        self.event_manager.emit_orchestrator_event(
            "orchestrator_state_changed",
            OrchestratorEventPayload { status: Some(format!("{:?}", self.state)), ..Default::default() }
        ).await?;
        
        // Clone event manager and actor addresses before creating phase context
        let event_manager = self.event_manager.clone();
        let installation_actor = self.installation_actor.clone();
        let process_actor = self.process_actor.clone();
        let communication_actor = self.communication_actor.clone();
        let execution_actor = self.execution_actor.clone();
        let lsp_actor = self.lsp_actor.clone();
        let plot_actor = self.plot_actor.clone();
        let file_server_actor = self.file_server_actor.clone();
        let config_actor = self.config_actor.clone();
        
        let phase_context = StartupPhaseContext {
            event_manager: &event_manager,
            config_actor: config_actor.as_ref(),
            installation_actor: installation_actor.as_ref(),
            process_actor: process_actor.as_ref(),
            communication_actor: communication_actor.as_ref(),
            execution_actor: execution_actor.as_ref(),
            lsp_actor: lsp_actor.as_ref(),
            plot_actor: plot_actor.as_ref(),
            file_server_actor: file_server_actor.as_ref(),
        };
        
        // Step 1: Check and install Julia if needed
        let next_phase = match phase_context.check_and_install_julia().await {
            Ok(next_phase) => next_phase,
            Err(e) => {
                error!("OrchestratorActor: Failed to check/install Julia: {}", e);
                self.startup_phase = StartupPhase::Failed(e.clone());
                let _ = event_manager.emit_orchestrator_startup_error("Failed to check Julia installation", 20, &e).await;
                return Err(e);
            }
        };
        
        self.startup_phase = next_phase.clone();
        
        // Phase updates now handled by state machine
        
        // If we're installing Julia, wait for installation to complete
        if matches!(self.startup_phase, StartupPhase::InstallingJulia) {
            debug!("OrchestratorActor: Julia installation started, waiting for completion");
            return Ok(());
        }
        
        // Step 2: Start Julia-dependent services
        if let Err(e) = phase_context.start_julia_dependent_services().await {
            error!("OrchestratorActor: Failed to start Julia-dependent services: {}", e);
            self.startup_phase = StartupPhase::Failed(e.clone());
            // Phase updates now handled by state machine
            return Err(e);
        }
        
        // Stay in StartingJuliaProcess - JuliaMessageLoopReady will transition us
        // Phase updates now handled by state machine
        
        debug!("OrchestratorActor: Startup phase execution completed, waiting for events to continue");
        Ok(())
    }

    
    /// Shutdown the orchestrator
    async fn shutdown(&mut self) -> Result<(), String> {
        debug!("OrchestratorActor: Shutting down orchestrator");
        
        // Health check removed
        
        self.shutdown_requested = true;
        self.state = OrchestratorState::Stopped;
        self.event_manager.emit_orchestrator_event(
            "orchestrator_state_changed",
            OrchestratorEventPayload { status: Some(format!("{:?}", self.state)), ..Default::default() }
        ).await?;
        
        // Shutdown LSP server
        if let Some(lsp_actor) = &self.lsp_actor {
            let _ = lsp_actor.send(ShutdownLsp).await;
        }
        
        // Stop Julia process
        if let Some(process_actor) = &self.process_actor {
            let _ = process_actor.send(StopJuliaProcess).await;
        }
        
        // Stop plot server
        if let Some(plot_actor) = &self.plot_actor {
            let _ = plot_actor.send(StopPlotServer).await;
        }
        
        // Stop file server
        if let Some(file_server_actor) = &self.file_server_actor {
            let _ = file_server_actor.send(StopFileServer).await;
        }
        
        // Disconnect from pipes
        if let Some(communication_actor) = &self.communication_actor {
            let _ = communication_actor.send(DisconnectFromPipes).await;
        }
        
        self.state = OrchestratorState::Stopped;
        self.event_manager.emit_orchestrator_event(
            "orchestrator_state_changed",
            OrchestratorEventPayload { status: Some(format!("{:?}", self.state)), ..Default::default() }
        ).await?;
        self.event_manager.emit_orchestrator_event(
            "orchestrator_shutdown_completed",
            OrchestratorEventPayload { message: Some("Shutdown completed".to_string()), ..Default::default() }
        ).await?;
        
        debug!("OrchestratorActor: Shutdown completed");
        Ok(())
    }
    
    // Julia health check removed

    /// Restart Julia orchestrator
    /// 
    /// Note: Actor message handlers that spawn async tasks return immediately,
    /// so we only await the message send, not the actual operation completion.
    async fn restart_julia(&mut self) -> Result<(), String> {
        debug!("OrchestratorActor: Restarting Julia");
        
        // Generate a unique request ID for this restart operation
        let request_id = format!("restart_julia_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis());
        
        // Emit backend busy event to disable all frontend buttons
        debug!("OrchestratorActor: Emitting backend busy event for restart");
        self.event_manager.emit_backend_busy(&request_id).await?;
        
        // Emit restart started event
        debug!("OrchestratorActor: About to emit restart started event");
        self.event_manager.emit_orchestrator_julia_restart_started("Julia restart started").await?;
        debug!("OrchestratorActor: Restart started event emitted successfully");
        
        // Stop Julia process
        if let Some(process_actor) = &self.process_actor {
            process_actor
                .send(StopJuliaProcess)
                .await
                .map_err(|_| "Failed to stop Julia process")??;
        }
        
        // Disconnect from pipes
        if let Some(communication_actor) = &self.communication_actor {
            communication_actor
                .send(DisconnectFromPipes)
                .await
                .map_err(|_| "Failed to disconnect from pipes")??;
        }
        
        // Start Julia process
        if let Some(process_actor) = &self.process_actor {
            process_actor
                .send(StartJuliaProcess { orchestrator_addr: None })
                .await
                .map_err(|_| "Failed to start Julia process")??;
        }
        
        // Pipe connections will be initiated when Julia signals TO_JULIA_PIPE_READY and FROM_JULIA_PIPE_READY
        // We connect early to unblock Julia's accept() calls, but wait for MESSAGE_LOOP_READY
        // before considering the connection ready for use
        debug!("OrchestratorActor: Waiting for pipe ready signals to connect, then MESSAGE_LOOP_READY for readiness after restart");
        
        // Reactivate the current project if one was active before restart
        // Note: Project activation in main Julia process will happen when JuliaMessageLoopReady is received
        // The new Julia process will go through the same initialization sequence and emit MESSAGE_LOOP_READY
        if let Some(current_project) = &self.current_project {
            debug!("OrchestratorActor: Project will be reactivated when JuliaMessageLoopReady is received after restart: {}", current_project.path);
            
            // Restart LSP server for the project (LSP is independent and can start immediately)
            if let Some(lsp_actor) = &self.lsp_actor {
                lsp_actor
                    .send(StartLspServer { project_path: current_project.path.clone() })
                    .await
                    .map_err(|_| "Failed to restart LSP server after restart")??;
            }
            
            debug!("OrchestratorActor: LSP server restart initiated after Julia restart");
        } else {
            debug!("OrchestratorActor: No project was active before restart, skipping reactivation");
        }
        
        // Emit restart completed event
        debug!("OrchestratorActor: About to emit restart completed event");
        self.event_manager.emit_orchestrator_julia_restart_completed("Julia restart completed").await?;
        debug!("OrchestratorActor: Restart completed event emitted successfully");
        
        // Emit backend done event to re-enable all frontend buttons
        debug!("OrchestratorActor: Emitting backend done event for restart");
        self.event_manager.emit_backend_done(&request_id).await?;
        
        debug!("OrchestratorActor: Julia restarted successfully");
        Ok(())
    }
    
    /// Check if we can emit startup-ready event based on current phase
    /// Transition to Completed phase when all prerequisites are met
    #[allow(dead_code)]
    async fn check_and_emit_startup_ready(&mut self) -> Result<(), String> {
        // Check if we can transition to Completed phase
        let can_complete = match &self.startup_phase {
            // Can complete if we're in ActivatingProject and there's no Julia project
            StartupPhase::ActivatingProject => {
                // Check if we have a Julia project that needs activation/LSP
                let has_julia_project = self.current_project.as_ref()
                    .map(|p| std::path::Path::new(&p.path).join("Project.toml").exists())
                    .unwrap_or(false);
                !has_julia_project
            }
            // Can complete if LSP is ready
            StartupPhase::WaitingForLspReady => {
                // This will be set to true when LspReady message is received
                false // Will be handled by LspReady handler
            }
            // Already completed or failed
            StartupPhase::Completed | StartupPhase::Failed(_) => false,
            // Other phases - not ready yet
            _ => false,
        };
        
        if can_complete {
            self.transition_startup_phase(StartupPhase::Completed);
            self.state = OrchestratorState::Running;
            
            let startup_message = "Compute42 initialization complete";
            self.event_manager
                .emit_orchestrator_startup_ready(startup_message)
                .await?;
            
            debug!("OrchestratorActor: Startup-ready event emitted, phase: {:?}", self.startup_phase);
        } else {
            debug!("OrchestratorActor: Cannot complete startup yet, current phase: {:?}", self.startup_phase);
        }
        
        Ok(())
    }
    
    /// Continue startup after Julia installation completes
    async fn continue_startup_after_julia_installation(&mut self, _actor_addr: Option<actix::Addr<Self>>) -> Result<(), String> {
        debug!("OrchestratorActor: Continuing startup after Julia installation, current phase: {:?}", self.startup_phase);
        
        // Should be in InstallingJulia phase
        if !matches!(self.startup_phase, StartupPhase::InstallingJulia) {
            debug!("OrchestratorActor: Not in InstallingJulia phase, ignoring");
            return Ok(());
        }
        
        // Transition to StartingJuliaProcess
        self.transition_startup_phase(StartupPhase::StartingJuliaProcess);
        
        // Phase updates now handled by state machine
        
        // Start Julia-dependent services
        // Clone actor addresses before creating phase context to avoid borrow checker issues
        let event_manager = self.event_manager.clone();
        let process_actor = self.process_actor.clone();
        let communication_actor = self.communication_actor.clone();
        let execution_actor = self.execution_actor.clone();
        let plot_actor = self.plot_actor.clone();
        let file_server_actor = self.file_server_actor.clone();
        
        let phase_context = StartupPhaseContext {
            event_manager: &event_manager,
            config_actor: None,
            installation_actor: None,
            process_actor: process_actor.as_ref(),
            communication_actor: communication_actor.as_ref(),
            execution_actor: execution_actor.as_ref(),
            lsp_actor: None,
            plot_actor: plot_actor.as_ref(),
            file_server_actor: file_server_actor.as_ref(),
        };
        
        if let Err(e) = phase_context.start_julia_dependent_services().await {
            error!("OrchestratorActor: Failed to start Julia-dependent services: {}", e);
            self.startup_phase = StartupPhase::Failed(e.clone());
            // Phase updates now handled by state machine
            return Err(e);
        }
        
        // Stay in StartingJuliaProcess - JuliaMessageLoopReady will transition us to ActivatingProject
        debug!("OrchestratorActor: Startup continued after Julia installation, waiting for JuliaMessageLoopReady");
        Ok(())
    }
    
    /// Change project directory
    /// Uses the state machine to ensure proper initialization sequence
    /// 
    /// Note: Actor message handlers that spawn async tasks return immediately,
    /// so we only await the message send, not the actual operation completion.
    async fn change_project_directory(&mut self, project_path: String) -> Result<(), String> {
        debug!("OrchestratorActor: Changing project directory to {}, current phase: {:?}", project_path, self.startup_phase);
        
        // Only allow project switching if system is running (Completed state)
        // During startup, project activation is handled by the startup sequence
        if !matches!(self.startup_phase, StartupPhase::Completed) {
            debug!("OrchestratorActor: System not ready for project switching (phase: {:?}), project will be activated during startup", self.startup_phase);
            // Still update the project path for when startup completes
            self.current_project = Some(ProjectInfo {
                path: project_path.clone(),
                name: std::path::Path::new(&project_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                julia_version: None,
                packages: Vec::new(),
            });
            return Ok(());
        }
        
        // Deactivate current project
        if let Some(execution_actor) = &self.execution_actor {
            execution_actor
                .send(DeactivateProject)
                .await
                .map_err(|_| "Failed to deactivate current project")??;
        }
        
        // Stop LSP server
        if let Some(lsp_actor) = &self.lsp_actor {
            lsp_actor
                .send(StopLspServer)
                .await
                .map_err(|_| "Failed to stop LSP server")??;
        }
        
        // Update current project
        self.current_project = Some(ProjectInfo {
            path: project_path.clone(),
            name: std::path::Path::new(&project_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string(),
            julia_version: None,
            packages: Vec::new(),
        });
        
        // Check if it's a Julia project
        let project_toml = std::path::Path::new(&project_path).join("Project.toml");
        let is_julia_project = project_toml.exists();
        
        // Clone actor addresses
        let execution_actor = self.execution_actor.clone();
        let lsp_actor = self.lsp_actor.clone();
        let process_actor = self.process_actor.clone();
        
        if is_julia_project {
            // Transition to ActivatingProject phase (from Completed)
            self.transition_startup_phase(StartupPhase::ActivatingProject);
            
            // Enable output suppression before activation
            if let Some(process_actor) = &process_actor {
                let _ = process_actor.send(SetOutputSuppression { suppressed: true }).await;
                debug!("OrchestratorActor: Enabled output suppression for project activation");
            }
            
            // Activate project - will transition to StartingLsp after ProjectActivationComplete
            if let Some(execution_actor) = &execution_actor {
                debug!("OrchestratorActor: Activating Julia project: {}", project_path);
                match execution_actor.send(ActivateProject { project_path: project_path.clone() }).await {
                    Ok(Ok(())) => {
                        debug!("OrchestratorActor: Project activation initiated, waiting for ProjectActivationComplete");
                        // State machine will handle transition to StartingLsp → WaitingForLspReady → Completed
                    }
                    Ok(Err(e)) => {
                        if e.contains("Communication service not connected") {
                            warn!("OrchestratorActor: Communication not ready, cannot activate project: {}", e);
                            // Transition back to Completed since we can't proceed
                            self.transition_startup_phase(StartupPhase::Completed);
                            return Err(format!("Communication not ready: {}", e));
                        } else {
                            error!("OrchestratorActor: Failed to activate project: {}", e);
                            self.startup_phase = StartupPhase::Failed(e.clone());
                            return Err(format!("Failed to activate project: {}", e));
                        }
                    }
                    Err(e) => {
                        error!("OrchestratorActor: Failed to send ActivateProject message: {:?}", e);
                        self.startup_phase = StartupPhase::Failed(format!("Failed to send ActivateProject: {:?}", e));
                        return Err(format!("Failed to send ActivateProject message: {:?}", e));
                    }
                }
            }
        } else {
            // Not a Julia project - start LSP immediately and return to Completed
            if let Some(lsp_actor) = &lsp_actor {
                debug!("OrchestratorActor: Starting LSP for non-Julia project: {}", project_path);
                self.transition_startup_phase(StartupPhase::StartingLsp);
                
                match lsp_actor.send(RestartLspServer { project_path: project_path.clone() }).await {
                    Ok(Ok(())) => {
                        debug!("OrchestratorActor: LSP server restart message sent successfully");
                        // Transition to WaitingForLspReady, then Completed when LspReady is received
                        self.transition_startup_phase(StartupPhase::WaitingForLspReady);
                    }
                    Ok(Err(e)) => {
                        error!("OrchestratorActor: Failed to restart LSP server: {}", e);
                        self.startup_phase = StartupPhase::Failed(e.clone());
                        return Err(format!("Failed to restart LSP server: {}", e));
                    }
                    Err(e) => {
                        error!("OrchestratorActor: Failed to send RestartLspServer message: {:?}", e);
                        self.startup_phase = StartupPhase::Failed(format!("Failed to send RestartLspServer: {:?}", e));
                        return Err(format!("Failed to send RestartLspServer message: {:?}", e));
                    }
                }
            } else {
                // No LSP actor, just return to Completed
                self.transition_startup_phase(StartupPhase::Completed);
            }
        }
        
        self.event_manager.emit_orchestrator_event(
            "project_changed",
            OrchestratorEventPayload { project_path: self.current_project.as_ref().map(|p| p.path.clone()), ..Default::default() }
        ).await?;
        
        // Emit unified selected-directory for frontend to update project path state
        if let Some(current) = &self.current_project {
            let payload = serde_json::json!({
                "path": current.path,
                "is_julia_project": is_julia_project
            });
            let _ = self.event_manager.emit_with_actor_notification(
                "orchestrator:selected-directory",
                payload,
                &[]
            ).await;
        }
        
        debug!("OrchestratorActor: Project directory changed successfully, phase: {:?}", self.startup_phase);
        Ok(())
    }
}

impl Actor for OrchestratorActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // debug!("OrchestratorActor: Actor started");
        ctx.set_mailbox_capacity(128);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("OrchestratorActor: Actor stopped");
    }
}

// Clone implementation for async operations
impl Clone for OrchestratorActor {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            current_project: self.current_project.clone(),
            shutdown_requested: self.shutdown_requested,
            startup_phase: self.startup_phase.clone(),
            // Watchdog timer fields removed
            event_manager: self.event_manager.clone(),
            config_actor: self.config_actor.clone(),
            state_actor: self.state_actor.clone(),
            execution_actor: self.execution_actor.clone(),
            communication_actor: self.communication_actor.clone(),
            process_actor: self.process_actor.clone(),
            lsp_actor: self.lsp_actor.clone(),
            plot_actor: self.plot_actor.clone(),
            file_server_actor: self.file_server_actor.clone(),
            installation_actor: self.installation_actor.clone(),
        }
    }
}
