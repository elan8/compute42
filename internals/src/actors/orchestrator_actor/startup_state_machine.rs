use actix::prelude::*;
use log::{debug, error, warn};

use crate::messages::orchestrator::{StartupEventMessage, StartupStateEntered, StartupEvent};
use super::{OrchestratorActor, StartupPhase};

impl Handler<StartupEventMessage> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: StartupEventMessage, ctx: &mut Context<Self>) -> Self::Result {
        let next_phase = match (&self.startup_phase, &msg.event) {
            // Entry point: FrontendReady
            (StartupPhase::NotStarted, StartupEvent::FrontendReady) => {
                Some(StartupPhase::CheckingForUpdates)
            }
            
            // Update check complete - skip authentication, go directly to checking Julia
            (StartupPhase::CheckingForUpdates, StartupEvent::UpdateCheckComplete) => {
                Some(StartupPhase::CheckingJulia)
            }
            
            // Work completion: Julia check complete
            (StartupPhase::CheckingJulia, StartupEvent::JuliaCheckComplete) => {
                Some(StartupPhase::StartingJuliaProcess)
            }
            
            // Work completion: Julia process started
            // Stay in StartingJuliaProcess - wait for JuliaMessageLoopReady before moving on
            (StartupPhase::StartingJuliaProcess, StartupEvent::JuliaProcessStarted) => {
                None
            }
            
            // Work completion: Plot server started
            (StartupPhase::StartingPlotServer, StartupEvent::PlotServerStarted) => {
                Some(StartupPhase::StartingFileServer)
            }
            
            // Work completion: File server started
            (StartupPhase::StartingFileServer, StartupEvent::FileServerStarted) => {
                Some(StartupPhase::ActivatingProject)
            }
            
            // File server failed (non-fatal) - continue to next phase
            (StartupPhase::StartingFileServer, StartupEvent::FileServerFailed(_)) => {
                Some(StartupPhase::ActivatingProject)
            }
            
            // Julia message loop ready - Julia is fully ready (process started, pipes connected, message loop active)
            // Transition from StartingJuliaProcess to StartingPlotServer
            // Plot and file servers can start in parallel, but we wait for Julia to be fully ready first
            (StartupPhase::StartingJuliaProcess, StartupEvent::JuliaMessageLoopReady) => {
                Some(StartupPhase::StartingPlotServer)
            }
            
            // Project check complete - if no project, skip to StartingLsp
            (StartupPhase::ActivatingProject, StartupEvent::ProjectCheckComplete { has_project }) => {
                if *has_project {
                    // Project found, stay in ActivatingProject to activate it
                    None
                } else {
                    // No project, skip to StartingLsp
                    Some(StartupPhase::StartingLsp)
                }
            }
            
            // Project activation complete
            (StartupPhase::ActivatingProject, StartupEvent::ProjectActivationComplete) => {
                Some(StartupPhase::StartingLsp)
            }
            
            // Work completion: LSP started
            (StartupPhase::StartingLsp, StartupEvent::LspStarted) => {
                Some(StartupPhase::WaitingForLspReady)
            }
            
            // LSP ready
            (StartupPhase::WaitingForLspReady, StartupEvent::LspReady) => {
                Some(StartupPhase::Completed)
            }
            
            // Error handling - can transition to Failed from any phase
            (_, StartupEvent::StartupFailed(ref error)) => {
                Some(StartupPhase::Failed(error.clone()))
            }
            
            // Invalid transitions
            _ => {
                warn!("OrchestratorActor: Invalid event {:?} for phase {:?}, ignoring", msg.event, self.startup_phase);
                None
            }
        };
        
        if let Some(phase) = next_phase {
            if matches!(phase, StartupPhase::Failed(_)) {
                // Handle failure - set phase and emit error status
                self.startup_phase = phase.clone();
                let progress = self.startup_phase.progress_percentage();
                let event_manager = self.event_manager.clone();
                let status_message = self.startup_phase.status_message();
                actix::spawn(async move {
                    let _ = event_manager.emit_orchestrator_startup_update(status_message, progress).await;
                });
            } else if self.startup_phase.can_transition_to(&phase) {
                self.transition_to_phase(phase, ctx);
            } else {
                error!("OrchestratorActor: Invalid transition from {:?} to {:?} for event {:?}", 
                    self.startup_phase, phase, msg.event);
            }
        }
        
        Ok(())
    }
}

impl OrchestratorActor {
    /// Transition to a new phase and emit state entered event
    fn transition_to_phase(&mut self, next_phase: StartupPhase, ctx: &mut Context<Self>) {
        let old_phase = self.startup_phase.clone();
        self.startup_phase = self.startup_phase.clone().transition_to(next_phase.clone());
        
        if self.startup_phase != old_phase {
            // Emit progress update
            let progress = self.startup_phase.progress_percentage();
            let event_manager = self.event_manager.clone();
            let status_message = self.startup_phase.status_message();
            actix::spawn(async move {
                if let Err(e) = event_manager.emit_orchestrator_startup_update(status_message, progress).await {
                    error!("OrchestratorActor: Failed to emit startup-update event: {}", e);
                }
            });
            
            // If transitioning from ActivatingProject to StartingLsp, emit project-change-complete
            // This happens when Julia project activation completes (during project switching)
            if matches!(old_phase, StartupPhase::ActivatingProject) && matches!(self.startup_phase, StartupPhase::StartingLsp) {
                let event_manager = self.event_manager.clone();
                let project_path = self.current_project.as_ref().map(|p| p.path.clone());
                if let Some(path) = project_path {
                    debug!("OrchestratorActor: Project activation complete, emitting project-change-complete for: {}", path);
                    actix::spawn(async move {
                        if let Err(e) = event_manager.emit_orchestrator_project_change_complete(&path).await {
                            error!("OrchestratorActor: Failed to emit project-change-complete event: {}", e);
                        }
                    });
                }
            }
            
            // If transitioning to Completed, emit startup-ready event to close the modal
            if matches!(self.startup_phase, StartupPhase::Completed) {
                self.state = crate::types::OrchestratorState::Running;
                let startup_message = "Compute42 initialization complete";
                let event_manager = self.event_manager.clone();
                actix::spawn(async move {
                    if let Err(e) = event_manager.emit_orchestrator_startup_ready(startup_message).await {
                        error!("OrchestratorActor: Failed to emit startup-ready event: {}", e);
                    }
                });
            }
            
            // Emit state entered event to trigger work handlers (unless we're in Completed phase)
            if !matches!(self.startup_phase, StartupPhase::Completed) {
                let actor_addr = ctx.address();
                let phase = self.startup_phase.clone();
                actor_addr.do_send(StartupStateEntered { phase });
            }
        }
    }
}

