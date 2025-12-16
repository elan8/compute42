use actix::prelude::*;
use log::{error, warn};

use crate::messages::orchestrator::{StartupStateEntered, StartupEventMessage, StartupEvent};
use crate::messages::process::StartJuliaProcess;
use crate::messages::execution::ActivateProject;
use crate::messages::lsp::RestartLspServer;
use crate::messages::plot::StartPlotServer;
use crate::messages::file_server::StartFileServer;
use crate::messages::communication::DisconnectFromPipes;
use crate::messages::configuration::GetRootFolder;

use super::{OrchestratorActor, StartupPhase};

impl Handler<StartupStateEntered> for OrchestratorActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: StartupStateEntered, ctx: &mut Context<Self>) -> Self::Result {
        match msg.phase {
            // WaitingForAuth phase removed - skip authentication
            StartupPhase::CheckingJulia => {
                self.handle_check_julia(ctx);
            }
            StartupPhase::StartingJuliaProcess => {
                self.handle_start_julia_process(ctx);
            }
            StartupPhase::StartingPlotServer => {
                self.handle_start_plot_server(ctx);
            }
            StartupPhase::StartingFileServer => {
                self.handle_start_file_server(ctx);
            }
            StartupPhase::ActivatingProject => {
                self.handle_activate_project(ctx);
            }
            StartupPhase::StartingLsp => {
                self.handle_start_lsp(ctx);
            }
            StartupPhase::WaitingForLspReady => {
                // Wait for LspReady event - no work to do here, just wait
            }
            _ => {
                // No work handler for this phase
            }
        }
        
        Ok(())
    }
}

impl OrchestratorActor {
    
    /// Handle CheckingJulia phase - ensure Julia is installed
    fn handle_check_julia(&mut self, ctx: &mut Context<Self>) {
        let installation_actor = match &self.installation_actor {
            Some(actor) => actor.clone(),
            None => {
                error!("OrchestratorActor: Installation actor not available");
                ctx.address().do_send(StartupEventMessage {
                    event: StartupEvent::StartupFailed("Installation actor not available".to_string()),
                });
                return;
            }
        };
        
        let orchestrator_addr = ctx.address();
        
        // Send EnsureJuliaInstalled message - InstallationActor will check/install and emit StartupEvent
        // Fire-and-forget: InstallationActor will emit StartupEvent::JuliaCheckComplete or StartupEvent::StartupFailed when done
        installation_actor.do_send(crate::messages::installation::EnsureJuliaInstalled {
            orchestrator_addr,
        });
    }
    
    /// Handle StartingJuliaProcess phase - start Julia process
    fn handle_start_julia_process(&mut self, ctx: &mut Context<Self>) {
        let process_actor = match &self.process_actor {
            Some(actor) => actor.clone(),
            None => {
                error!("OrchestratorActor: Process actor not available");
                ctx.address().do_send(StartupEventMessage {
                    event: StartupEvent::StartupFailed("Process actor not available".to_string()),
                });
                return;
            }
        };
        
        let communication_actor = self.communication_actor.clone();
        let orchestrator_addr = ctx.address();
        
        // Reset connection state
        if let Some(ref comm_actor) = communication_actor {
            comm_actor.do_send(DisconnectFromPipes);
        }
        
        // Send StartJuliaProcess message - ProcessActor will start Julia and emit StartupEvent
        // Fire-and-forget: ProcessActor will emit StartupEvent::JuliaProcessStarted or StartupEvent::StartupFailed when done
        process_actor.do_send(StartJuliaProcess {
            orchestrator_addr: Some(orchestrator_addr),
        });
    }
    
    /// Handle StartingPlotServer phase - start plot server
    fn handle_start_plot_server(&mut self, ctx: &mut Context<Self>) {
        let plot_actor = match &self.plot_actor {
            Some(actor) => actor.clone(),
            None => {
                error!("OrchestratorActor: Plot actor not available");
                ctx.address().do_send(StartupEventMessage {
                    event: StartupEvent::StartupFailed("Plot actor not available".to_string()),
                });
                return;
            }
        };
        
        let orchestrator_addr = ctx.address();
        
        // Send StartPlotServer message - PlotActor will start plot server and emit StartupEvent
        // Fire-and-forget: PlotActor will emit StartupEvent::PlotServerStarted or StartupEvent::StartupFailed when done
        plot_actor.do_send(StartPlotServer {
            orchestrator_addr: Some(orchestrator_addr),
        });
    }
    
    /// Handle StartingFileServer phase - start file server
    fn handle_start_file_server(&mut self, ctx: &mut Context<Self>) {
        let file_server_actor = match &self.file_server_actor {
            Some(actor) => actor.clone(),
            None => {
                error!("OrchestratorActor: File server actor not available");
                ctx.address().do_send(StartupEventMessage {
                    event: StartupEvent::StartupFailed("File server actor not available".to_string()),
                });
                return;
            }
        };
        
        let orchestrator_addr = ctx.address();
        
        // Send StartFileServer message - FileServerActor will get root folder from config_actor and emit StartupEvent
        // Fire-and-forget: FileServerActor will emit StartupEvent::FileServerStarted or StartupEvent::StartupFailed when done
        file_server_actor.do_send(StartFileServer {
            orchestrator_addr: Some(orchestrator_addr),
        });
    }
    
    /// Check for project and activate if found
    /// Called when entering ActivatingProject phase to check if a project exists
    fn handle_check_project(&mut self, ctx: &mut Context<Self>) {
        let config_actor = match &self.config_actor {
            Some(actor) => actor.clone(),
            None => {
                warn!("OrchestratorActor: Config actor not available, no project to activate");
                ctx.address().do_send(StartupEventMessage {
                    event: StartupEvent::ProjectCheckComplete { has_project: false },
                });
                return;
            }
        };
        
        let actor_addr = ctx.address();
        let event_manager = self.event_manager.clone();
        
        ctx.spawn(
            async move {
                // Check for saved project
                let mut project_path = match config_actor.send(GetRootFolder).await {
                    Ok(Ok(Some(folder))) => {
                        let folder_path = std::path::Path::new(&folder);
                        if folder_path.exists() && folder_path.is_dir() {
                            let project_toml = folder_path.join("Project.toml");
                            if project_toml.exists() {
                                Some(folder)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                
                // If no saved project, check for demo
                if project_path.is_none() {
                    // Demo folder is bundled as a Tauri resource at compile time
                    // Just check if it exists and use it
                    if let Some(demo_path) = get_demo_folder_path() {
                        if demo_path.exists() && demo_path.is_dir() {
                            let project_toml = demo_path.join("Project.toml");
                            if project_toml.exists() {
                                let demo_path_str = demo_path.to_string_lossy().to_string();
                                project_path = Some(demo_path_str);
                            } else {
                                warn!("OrchestratorActor: Demo folder exists but Project.toml not found at: {:?}", project_toml);
                            }
                        } else {
                            warn!("OrchestratorActor: Demo folder does not exist at: {:?} (should be bundled as resource)", demo_path);
                        }
                    } else {
                        warn!("OrchestratorActor: Could not determine demo folder path (resource directory not found)");
                    }
                }
                
                // If we have a project, set it on the actor and emit selected-directory event
                if let Some(ref path) = project_path {
                    // Send message to set current project
                    use crate::messages::orchestrator::UpdateCurrentProject;
                    actor_addr.do_send(UpdateCurrentProject {
                        project_path: Some(path.clone()),
                    });
                    
                    // Emit selected-directory event to notify frontend about the project path
                    let project_path_str = path.clone();
                    let event_manager_clone = event_manager.clone();
                    actix::spawn(async move {
                        let payload = serde_json::json!({
                            "path": project_path_str,
                            "is_julia_project": true
                        });
                        let _ = event_manager_clone.emit_with_actor_notification(
                            "orchestrator:selected-directory",
                            payload,
                            &[]
                        ).await;
                    });
                }
                
                // Emit project check complete event
                let has_project = project_path.is_some();
                actor_addr.do_send(StartupEventMessage {
                    event: StartupEvent::ProjectCheckComplete { has_project },
                });
            }
            .into_actor(self)
            .map(|_, _actor, _ctx| {})
        );
    }
    
    /// Handle ActivatingProject phase - check for project and activate if found
    fn handle_activate_project(&mut self, ctx: &mut Context<Self>) {
        // First check if we have a project - if not, check for one
        if self.current_project.is_none() {
            self.handle_check_project(ctx);
            return;
        }
        
        let execution_actor = match &self.execution_actor {
            Some(actor) => actor.clone(),
            None => {
                error!("OrchestratorActor: Execution actor not available");
                ctx.address().do_send(StartupEventMessage {
                    event: StartupEvent::StartupFailed("Execution actor not available".to_string()),
                });
                return;
            }
        };
        
        let project_path = match &self.current_project {
            Some(project) => project.path.clone(),
            None => {
                // Try to get from config
                if let Some(config_actor) = &self.config_actor {
                    let config_actor_clone = config_actor.clone();
                    let execution_actor_clone = execution_actor.clone();
                    let actor_addr = ctx.address();
                    
                    ctx.spawn(
                        async move {
                            match config_actor_clone.send(GetRootFolder).await {
                                Ok(Ok(Some(path))) => {
                                    // Found project path in config - activate it
                                    match execution_actor_clone.send(ActivateProject { project_path: path.clone() }).await {
                                        Ok(Ok(())) => {
                                            // Do NOT emit event here - wait for ProjectActivationComplete event
                                        }
                                        Ok(Err(e)) => {
                                            if !e.contains("Communication service not connected") {
                                                error!("OrchestratorActor: Failed to activate project: {}", e);
                                                actor_addr.do_send(StartupEventMessage {
                                                    event: StartupEvent::StartupFailed(format!("Failed to activate project: {}", e)),
                                                });
                                            }
                                        }
                                        Err(e) => {
                                            error!("OrchestratorActor: Failed to send ActivateProject: {:?}", e);
                                            actor_addr.do_send(StartupEventMessage {
                                                event: StartupEvent::StartupFailed(format!("Failed to send ActivateProject: {:?}", e)),
                                            });
                                        }
                                    }
                                }
                                _ => {
                                    // No project path in config - nothing to activate
                                    // No project to activate - transition to StartingLsp (which will complete startup if no LSP needed)
                                    actor_addr.do_send(StartupEventMessage {
                                        event: StartupEvent::ProjectActivationComplete,
                                    });
                                }
                            }
                        }
                        .into_actor(self)
                        .map(|_, _actor, _ctx| {})
                    );
                    return;
                } else {
                    // No config actor - nothing to activate
                    // No project to activate - transition to StartingLsp (which will complete startup if no LSP needed)
                    ctx.address().do_send(StartupEventMessage {
                        event: StartupEvent::ProjectActivationComplete,
                    });
                    return;
                }
            }
        };
        
        let actor_addr = ctx.address();
        
        ctx.spawn(
            async move {
                match execution_actor.send(ActivateProject { project_path: project_path.clone() }).await {
                    Ok(Ok(())) => {
                        // Do NOT emit event here - wait for ProjectActivationComplete event
                        // The ProjectActivationComplete handler will emit StartupEvent::ProjectActivationComplete
                        // which will transition the state machine from ActivatingProject to StartingLsp
                    }
                    Ok(Err(e)) => {
                        if !e.contains("Communication service not connected") {
                            error!("OrchestratorActor: Failed to activate project: {}", e);
                            actor_addr.do_send(StartupEventMessage {
                                event: StartupEvent::StartupFailed(format!("Failed to activate project: {}", e)),
                            });
                        }
                    }
                    Err(e) => {
                        error!("OrchestratorActor: Failed to send ActivateProject: {:?}", e);
                        actor_addr.do_send(StartupEventMessage {
                            event: StartupEvent::StartupFailed(format!("Failed to send ActivateProject: {:?}", e)),
                        });
                    }
                }
                
                // Do NOT emit event here - wait for ProjectActivationComplete event
                // The state machine will transition from ActivatingProject to StartingLsp when
                // ProjectActivationComplete event is received
            }
            .into_actor(self)
            .map(|_, _actor, _ctx| {})
        );
    }
    
    /// Handle StartingLsp phase - start LSP server
    fn handle_start_lsp(&mut self, ctx: &mut Context<Self>) {
        let lsp_actor = match &self.lsp_actor {
            Some(actor) => actor.clone(),
            None => {
                error!("OrchestratorActor: LSP actor not available");
                ctx.address().do_send(StartupEventMessage {
                    event: StartupEvent::StartupFailed("LSP actor not available".to_string()),
                });
                return;
            }
        };
        
        let project_path = self.current_project.as_ref()
            .map(|p| p.path.clone())
            .unwrap_or_default();
        
        let actor_addr = ctx.address();
        
        ctx.spawn(
            async move {
                match lsp_actor.send(RestartLspServer { project_path }).await {
                    Ok(Ok(())) => {
                        // Will be completed by LspReady event
                        actor_addr.do_send(StartupEventMessage {
                            event: StartupEvent::LspStarted,
                        });
                    }
                    Ok(Err(e)) => {
                        error!("OrchestratorActor: Failed to start LSP server: {}", e);
                        actor_addr.do_send(StartupEventMessage {
                            event: StartupEvent::StartupFailed(format!("Failed to start LSP server: {}", e)),
                        });
                    }
                    Err(e) => {
                        error!("OrchestratorActor: Failed to send RestartLspServer: {:?}", e);
                        actor_addr.do_send(StartupEventMessage {
                            event: StartupEvent::StartupFailed(format!("Failed to send RestartLspServer: {:?}", e)),
                        });
                    }
                }
            }
            .into_actor(self)
            .map(|_, _actor, _ctx| {})
        );
    }
}

/// Get demo folder path from resource directory
/// The demo folder is bundled as a Tauri resource at compile time
fn get_demo_folder_path() -> Option<std::path::PathBuf> {
    // Try to find the demo folder relative to the executable
    // In Tauri, resources are typically in the same directory as the executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // In development, resources might be in target/debug/resources or target/release/resources
            // In production, resources are in the same directory as the executable
            let mut possible_paths = vec![
                exe_dir.join("resources").join("demo"),  // Production path
                exe_dir.join("demo"),  // Alternative production path
            ];
            
            // Add development path if parent exists
            if let Some(parent) = exe_dir.parent() {
                possible_paths.push(parent.join("resources").join("demo"));  // Development path (target/debug/resources/demo)
            }
            
            for path in possible_paths {
                if path.exists() && path.is_dir() {
                    let project_toml = path.join("Project.toml");
                    if project_toml.exists() {
                        return Some(path);
                    }
                }
            }
            
            // Also try workspace root for development
            if let Ok(cwd) = std::env::current_dir() {
                let workspace_demo = cwd.join("demo");
                if workspace_demo.exists() && workspace_demo.is_dir() {
                    let project_toml = workspace_demo.join("Project.toml");
                    if project_toml.exists() {
                        return Some(workspace_demo);
                    }
                }
            }
        }
    }
    
    None
}


