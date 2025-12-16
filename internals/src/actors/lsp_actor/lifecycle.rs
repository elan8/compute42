// Server lifecycle methods

use log::{debug, error};

use crate::messages::coordination::LspReady;
use crate::messages::installation::GetJuliaPathFromInstallation;
use crate::types::LspServerInfo;
use super::state::LspActorState;

impl LspActorState {
    /// Start LSP server
    pub async fn start_lsp_server(&mut self, project_path: String) -> Result<(), String> {
        debug!("LspActor: ===== STARTING LSP SERVER =====");
        debug!("LspActor: Starting LSP server for project {}", project_path);
        debug!("LspActor: Current state - is_running: {}, current_project: {:?}", self.is_running, self.current_project);
        
        // If already running for the same project, return success
        if self.is_running && self.current_project.as_ref() == Some(&project_path) {
            debug!("LspActor: LSP server already running for project {}", project_path);
            return Ok(());
        }
        
        // If running for a different project, stop it first
        if self.is_running {
            debug!("LspActor: Stopping existing LSP server before starting new one");
            if let Err(e) = self.stop_lsp_server().await {
                debug!("LspActor: Warning - failed to stop existing LSP server: {}", e);
                // Continue anyway - the new server will handle the cleanup
            }
        }
        
        debug!("LspActor: Setting status to starting");
        // Set status to starting
        let starting_server_info = LspServerInfo {
            project_path: project_path.clone(),
            port: None, // Rust LSP doesn't use a port
            is_running: false,
            error_message: None,
            status: crate::types::LspServerStatus::Starting,
        };
        
        debug!("LspActor: Emitting lsp:server-started event (starting status)");
        // Emit starting event
        if let Err(e) = self.event_manager.emit_lsp_server_started(starting_server_info.clone()).await {
            debug!("LspActor: WARNING - Failed to emit lsp:server-started event: {}", e);
        } else {
            debug!("LspActor: Successfully emitted lsp:server-started event");
        }
        
        // Get the Julia path from InstallationActor BEFORE starting the server
        // This is critical because EmbeddedLspService is lazy-initialized and needs the correct path
        if let Some(installation_actor) = &self.installation_actor {
            match installation_actor.send(GetJuliaPathFromInstallation).await {
                Ok(Ok(Some(julia_path_str))) => {
                    let julia_path = std::path::PathBuf::from(julia_path_str);
                    debug!("LspActor: Got Julia path from InstallationActor: {:?}", julia_path);
                    // Update the Julia executable path in LspService config before EmbeddedLspService is created
                    self.lsp_service.update_julia_executable(julia_path);
                }
                Ok(Ok(None)) | Ok(Err(_)) | Err(_) => {
                    debug!("LspActor: Warning - failed to get Julia path from InstallationActor, using default");
                }
            }
        } else {
            debug!("LspActor: Warning - InstallationActor not available, using default Julia path");
        }
        
        debug!("LspActor: Calling lsp_service.start_lsp_server(project_path: {})", project_path);
        
        // Emit status message for native LSP startup (no separate Julia process, so we emit messages manually)
        debug!("LspActor: Emitting lsp:status event - opening project");
        let _ = self.event_manager.emit_lsp_status("opening", &format!("Opening project at {}", project_path)).await;
        
        // Emit status event before starting pipelines
        debug!("LspActor: Emitting lsp:status event - indexing Base/stdlib and packages");
        let _ = self.event_manager.emit_lsp_status("indexing", "Indexing Base/stdlib and packages...").await;
        
        // Try to start LSP server using the Rust-based service
        // This will run the pipelines synchronously (JuliaPipeline and PackagePipeline)
        match self.lsp_service.start_lsp_server(project_path.clone()).await {
            Ok(()) => {
                debug!("LspActor: ===== LSP SERVICE STARTED SUCCESSFULLY =====");
                debug!("LspActor: LSP service started successfully, pipelines completed");
                
                // Emit status message for pipeline completion
                debug!("LspActor: Emitting lsp:status event - pipelines complete");
                let _ = self.event_manager.emit_lsp_status("indexing-complete", "Base/stdlib and packages indexed successfully").await;
                
                debug!("LspActor: About to initialize Julia LSP process");
                
                // Set up stderr channel for Julia LSP process before initializing
                // This uses message-based communication instead of callbacks (more Actix-idiomatic)
                debug!("LspActor: Setting up stderr channel for Julia LSP process");
                let (_tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                let event_manager = self.event_manager.clone();
                
                // Spawn a task to handle stderr lines from the channel
                tokio::spawn(async move {
                    while let Some(line) = rx.recv().await {
                        // Log stderr to log file
                        debug!("[Julia LSP stderr] {}", line);
                        
                        // Emit lsp:output-detailed event for stderr output
                        let stream_output = vec![crate::messages::StreamOutput {
                            content: format!("{}\n", line),
                            stream_type: crate::messages::StreamType::Stderr,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                        }];
                        
                        match serde_json::to_value(stream_output) {
                            Ok(json_value) => {
                                let event = crate::services::events::EventService::create_event(
                                    crate::services::events::EventCategory::Lsp,
                                    "output-detailed",
                                    json_value
                                );
                                if let Err(e) = event_manager.emit_event(event).await {
                                    debug!("LspActor: Failed to emit lsp:output-detailed from Julia LSP stderr: {}", e);
                                }
                            }
                            Err(e) => {
                                debug!("LspActor: Failed to serialize StreamOutput for Julia LSP stderr: {}", e);
                            }
                        }
                    }
                });
                
                // Julia LSP stderr sender no longer needed (JuliaLspProvider removed)
                // Documentation is now provided via BaseDocsRegistry loaded from file
                debug!("LspActor: Documentation service uses BaseDocsRegistry (no Julia process needed)");
                
                // Emit status message for documentation initialization
                debug!("LspActor: Emitting lsp:status event - documentation ready");
                let _ = self.event_manager.emit_lsp_status("docs-ready", "Documentation service ready (BaseDocsRegistry)").await;
                
                // Always emit lsp:ready event - documentation is ready via BaseDocsRegistry
                debug!("LspActor: Emitting lsp:ready event (documentation via BaseDocsRegistry)");
                if let Err(e) = self.event_manager.emit_lsp_ready().await {
                    debug!("LspActor: WARNING - Failed to emit lsp:ready event: {}", e);
                } else {
                    debug!("LspActor: Successfully emitted lsp:ready event");
                }
                
                // Also emit lsp:status event with 'ready' status to ensure UI updates
                debug!("LspActor: Emitting lsp:status event - ready");
                let _ = self.event_manager.emit_lsp_status("ready", "Language Server is ready").await;
                
                // Also notify orchestrator that LSP is ready
                if let Some(orchestrator_actor) = &self.orchestrator_actor {
                    debug!("LspActor: Notifying orchestrator that LSP is ready");
                    if let Err(e) = orchestrator_actor.send(LspReady).await {
                        debug!("LspActor: WARNING - Failed to send LspReady message to orchestrator: {:?}", e);
                    } else {
                        debug!("LspActor: Successfully notified orchestrator that LSP is ready");
                    }
                } else {
                    debug!("LspActor: Orchestrator actor not available, skipping LspReady notification");
                }
                
                debug!("LspActor: Updating actor state");
                // Create server info for successful start
                let server_info = LspServerInfo {
                    project_path: project_path.clone(),
                    port: None, // Rust LSP doesn't use a port
                    is_running: true,
                    error_message: None,
                    status: crate::types::LspServerStatus::Running,
                };
                
                // Update actor state
                self.is_running = true;
                self.server_info = Some(server_info.clone());
                self.current_project = Some(project_path.clone());
                
                debug!("LspActor: Emitting lsp:server-started event (running status)");
                if let Err(e) = self.event_manager.emit_lsp_server_started(server_info).await {
                    debug!("LspActor: WARNING - Failed to emit lsp:server-started event: {}", e);
                } else {
                    debug!("LspActor: Successfully emitted lsp:server-started event");
                }
                
                debug!("LspActor: ===== LSP SERVER STARTED SUCCESSFULLY =====");
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to start LSP server: {}", e);
                error!("LspActor: ===== LSP SERVER START FAILED =====");
                error!("LspActor: {}", error_msg);
                
                let error_server_info = LspServerInfo {
                    project_path: project_path.clone(),
                    port: None,
                    is_running: false,
                    error_message: Some(error_msg.clone()),
                    status: crate::types::LspServerStatus::Error,
                };
                
                self.server_info = Some(error_server_info.clone());
                self.event_manager.emit_lsp_server_error(error_server_info).await?;
                Err(error_msg)
            }
        }
    }
    
    /// Stop LSP server
    pub async fn stop_lsp_server(&mut self) -> Result<(), String> {
        debug!("LspActor: Stopping LSP server");
        
        // Check if running
        if !self.is_running {
            debug!("LspActor: LSP server is not running, nothing to stop");
            return Ok(());
        }
        
        // Use LSP service for stopping
        if let Err(e) = self.lsp_service.stop_lsp_server().await {
            debug!("LspActor: Warning - failed to stop LSP server gracefully: {}", e);
            // Continue with cleanup even if stop failed
        }
        
        // Update actor state
        self.is_running = false;
        self.server_info = None;
        self.current_project = None;
        
        // Emit stopped event
        if let Err(e) = self.event_manager.emit_lsp_server_stopped().await {
            debug!("LspActor: Warning - failed to emit LSP stopped event: {}", e);
        }
        
        debug!("LspActor: LSP server stopped successfully");
        Ok(())
    }
}

