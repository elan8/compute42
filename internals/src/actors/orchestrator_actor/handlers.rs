use actix::prelude::*;
use log::{debug, error};

use crate::messages::orchestrator::*;
use crate::messages::orchestrator::{StartupEventMessage, StartupEvent};
use crate::messages::coordination::*;

use super::{OrchestratorActor, StartupPhase};

// Message handlers for OrchestratorActor
impl Handler<GetOrchestratorState> for OrchestratorActor {
    type Result = Result<String, String>;
    
    fn handle(&mut self, _msg: GetOrchestratorState, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(format!("{:?}", self.state))
    }
}

impl Handler<GetStartupPhase> for OrchestratorActor {
    type Result = Result<String, String>;
    
    fn handle(&mut self, _msg: GetStartupPhase, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(format!("{:?}", self.startup_phase))
    }
}

impl Handler<GetCurrentProject> for OrchestratorActor {
    type Result = Result<Option<crate::types::ProjectInfo>, String>;
    
    fn handle(&mut self, _msg: GetCurrentProject, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(self.current_project.clone())
    }
}

// StartOrchestrator handler removed - use FrontendReady instead

impl Handler<ContinueOrchestratorStartup> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: ContinueOrchestratorStartup, ctx: &mut Context<Self>) -> Self::Result {
        // Guard against duplicate scheduling: if already completed or failed, ignore
        if self.startup_phase.is_completed() || self.startup_phase.is_failed() {
            return Ok(());
        }

        // Handle transition from CheckingForUpdates to CheckingJulia (authentication removed)
        if matches!(self.startup_phase, StartupPhase::CheckingForUpdates) {
            ctx.address().do_send(StartupEventMessage {
                event: StartupEvent::UpdateCheckComplete,
            });
            return Ok(());
        }
        
        // Authentication phase removed - startup goes directly from CheckingForUpdates to CheckingJulia
        Ok(())
    }
}

impl Handler<FrontendReady> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: FrontendReady, ctx: &mut Context<Self>) -> Self::Result {
        // Guard: if startup is already completed, skip processing
        if self.startup_phase.is_completed() {
            return Ok(());
        }
        
        // Only process FrontendReady if we're in NotStarted phase
        if !matches!(self.startup_phase, StartupPhase::NotStarted) {
            return Ok(());
        }
        
        // Emit startup event to state machine handler
        ctx.address().do_send(StartupEventMessage {
            event: StartupEvent::FrontendReady,
        });
        Ok(())
    }
}

impl Handler<ShutdownOrchestrator> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: ShutdownOrchestrator, ctx: &mut Context<Self>) -> Self::Result {
        let mut actor = self.clone();
        ctx.spawn(
            async move {
                match actor.shutdown().await {
                    Ok(_) => {
                        debug!("OrchestratorActor: Orchestrator shut down successfully");
                    }
                    Err(e) => {
                        error!("OrchestratorActor: Failed to shut down orchestrator: {}", e);
                    }
                }
            }
            .into_actor(self)
        );
        Ok(())
    }
}

impl Handler<RestartJuliaOrchestrator> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: RestartJuliaOrchestrator, ctx: &mut Context<Self>) -> Self::Result {
        debug!("OrchestratorActor: Received RestartJuliaOrchestrator message");
        
        // Health check removed
        
        let mut actor = self.clone();
        ctx.spawn(
            async move {
                debug!("OrchestratorActor: Restarting Julia in async task");
                let _ = actor.restart_julia().await;
            }
            .into_actor(self)
            .map(|_, _actor, _ctx| {})
        );
        Ok(())
    }
}

// UpdateStartupPhase and PhaseTimeout handlers removed - state machine handles transitions directly

impl Handler<UpdateCurrentProject> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: UpdateCurrentProject, ctx: &mut Context<Self>) -> Self::Result {
        debug!("OrchestratorActor: Received UpdateCurrentProject message, updating project to {:?}", msg.project_path);
        
        if let Some(ref path) = msg.project_path {
            self.current_project = Some(crate::types::ProjectInfo {
                path: path.clone(),
                name: std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                julia_version: None,
                packages: Vec::new(),
            });
            debug!("OrchestratorActor: Current project updated to: {}", path);
            
            // If we're in ActivatingProject phase and just set the project, trigger activation
            if matches!(self.startup_phase, super::StartupPhase::ActivatingProject) {
                debug!("OrchestratorActor: In ActivatingProject phase, triggering project activation");
                // Emit StartupStateEntered to trigger work handler again
                use crate::messages::orchestrator::StartupStateEntered;
                ctx.address().do_send(StartupStateEntered {
                    phase: super::StartupPhase::ActivatingProject,
                });
            }
        } else {
            self.current_project = None;
            debug!("OrchestratorActor: Current project cleared");
        }
        
        Ok(())
    }
}

impl Handler<ChangeProjectDirectory> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: ChangeProjectDirectory, ctx: &mut Context<Self>) -> Self::Result {
        debug!("OrchestratorActor: Received ChangeProjectDirectory message for: {}", msg.project_path);
        
        let mut actor = self.clone();
        ctx.spawn(
            async move {
                match actor.change_project_directory(msg.project_path).await {
                    Ok(_) => {
                        debug!("OrchestratorActor: Project directory changed successfully");
                    }
                    Err(e) => {
                        error!("OrchestratorActor: Failed to change project directory: {}", e);
                    }
                }
            }
            .into_actor(self)
            .map(|_, _actor, _ctx| {})
        );
        Ok(())
    }
}

// Julia activity update removed

impl Handler<SetActorAddresses> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: SetActorAddresses, _ctx: &mut Context<Self>) -> Self::Result {
        self.set_actor_addresses(
            msg.config_actor,
            msg.state_actor,
            msg.execution_actor,
            msg.communication_actor,
            msg.process_actor,
            msg.lsp_actor,
            msg.plot_actor,
            msg.file_server_actor,
            msg.installation_actor,
           // msg.sysimage_actor,
        );
        Ok(())
    }
}

// Handle coordination and telemetry messages sent to orchestrator
impl Handler<DependencyReady> for OrchestratorActor {
    type Result = Result<(), String>;
    fn handle(&mut self, _msg: DependencyReady, _ctx: &mut Context<Self>) -> Self::Result { Ok(()) }
}

impl Handler<DependencyFailed> for OrchestratorActor {
    type Result = Result<(), String>;
    fn handle(&mut self, _msg: DependencyFailed, _ctx: &mut Context<Self>) -> Self::Result { Ok(()) }
}

impl Handler<ResourceAcquired> for OrchestratorActor {
    type Result = Result<(), String>;
    fn handle(&mut self, _msg: ResourceAcquired, _ctx: &mut Context<Self>) -> Self::Result { Ok(()) }
}

impl Handler<ResourceReleased> for OrchestratorActor {
    type Result = Result<(), String>;
    fn handle(&mut self, _msg: ResourceReleased, _ctx: &mut Context<Self>) -> Self::Result { Ok(()) }
}

impl Handler<PerformanceMetric> for OrchestratorActor {
    type Result = Result<(), String>;
    fn handle(&mut self, _msg: PerformanceMetric, _ctx: &mut Context<Self>) -> Self::Result { Ok(()) }
}

impl Handler<DebugLog> for OrchestratorActor {
    type Result = Result<(), String>;
    fn handle(&mut self, _msg: DebugLog, _ctx: &mut Context<Self>) -> Self::Result { Ok(()) }
}

impl Handler<JuliaInstallationCompleted> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: JuliaInstallationCompleted, ctx: &mut Context<Self>) -> Self::Result {
        debug!("OrchestratorActor: Received JuliaInstallationCompleted message for version: {}, current phase: {:?}", msg.version, self.startup_phase);
        
        // Only handle if we're in InstallingJulia phase
        if matches!(self.startup_phase, StartupPhase::InstallingJulia) {
            debug!("OrchestratorActor: Julia installation completed, continuing startup sequence");
            
            let mut actor = self.clone();
            let actor_addr = ctx.address();
            ctx.spawn(
                async move {
                    debug!("OrchestratorActor: Continuing startup after Julia installation completion");
                    match actor.continue_startup_after_julia_installation(Some(actor_addr.clone())).await {
                        Ok(_) => {
                            debug!("OrchestratorActor: Startup continued successfully after Julia installation");
                        }
                        Err(e) => {
                            error!("OrchestratorActor: Failed to continue startup after Julia installation: {}", e);
                        }
                    }
                }
                .into_actor(self)
                .map(|_, _actor, _ctx| {})
            );
        } else {
            debug!("OrchestratorActor: Julia installation completed, but not in InstallingJulia phase (current: {:?}) - ignoring", self.startup_phase);
        }
        
        Ok(())
    }
}

impl Handler<ActorError> for OrchestratorActor {
    type Result = Result<(), String>;
    fn handle(&mut self, _msg: ActorError, _ctx: &mut Context<Self>) -> Self::Result { Ok(()) }
}

impl Handler<JuliaPipesReady> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: JuliaPipesReady, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("OrchestratorActor: Received JuliaPipesReady message (no longer used in state machine, pipes are connected directly)");
        // JuliaPipesReady is no longer used - pipes are connected directly by ProcessActor
        // This handler is kept for backward compatibility but does nothing
        Ok(())
    }
}

impl Handler<JuliaMessageLoopReady> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: JuliaMessageLoopReady, ctx: &mut Context<Self>) -> Self::Result {
        debug!("OrchestratorActor: Received JuliaMessageLoopReady message, emitting StartupEvent::JuliaMessageLoopReady");
        ctx.address().do_send(StartupEventMessage {
            event: StartupEvent::JuliaMessageLoopReady,
        });
        Ok(())
    }
}

impl Handler<LspReady> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: LspReady, ctx: &mut Context<Self>) -> Self::Result {
        debug!("OrchestratorActor: Received LspReady message, emitting StartupEvent::LspReady");
        ctx.address().do_send(StartupEventMessage {
            event: StartupEvent::LspReady,
        });
        Ok(())
    }
}

impl Handler<ProjectActivationComplete> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: ProjectActivationComplete, ctx: &mut Context<Self>) -> Self::Result {
        debug!("OrchestratorActor: Received ProjectActivationComplete message, emitting StartupEvent::ProjectActivationComplete");
        ctx.address().do_send(StartupEventMessage {
            event: StartupEvent::ProjectActivationComplete,
        });
        Ok(())
    }
}
