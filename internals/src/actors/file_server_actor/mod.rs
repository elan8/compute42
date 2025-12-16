mod server;
mod handlers;
mod csv;

use actix::prelude::*;
use std::sync::Arc;
use log::{debug, error, warn};

use crate::messages::file_server::*;
use crate::messages::configuration::GetRootFolder;
use crate::services::events::EventService;
use crate::types::FileServerInfo;
use crate::actors::ConfigurationActor;

use server::{FileServerState, start_server_internal, stop_server_internal};

/// FileServerActor - manages file server lifecycle
/// This replaces the mutex-based FileServer with a clean actor model
pub struct FileServerActor {
    // Actor state
    is_running: bool,
    server_info: Option<FileServerInfo>,
    current_root_path: Option<String>,
    current_port: Option<u16>,
    error_message: Option<String>, // Store error message for file explorer to display
    
    // Internal server state
    server_state: Arc<tokio::sync::Mutex<FileServerState>>,
    event_manager: EventService,
    config_actor: Option<actix::Addr<ConfigurationActor>>,
}

impl FileServerActor {
    /// Create a new FileServerActor instance
    pub fn new(
        _event_emitter: Arc<dyn crate::service_traits::EventEmitter>,
        event_manager: EventService,
        config_actor: Option<actix::Addr<ConfigurationActor>>,
    ) -> Self {
        Self {
            is_running: false,
            server_info: None,
            current_root_path: None,
            current_port: None,
            error_message: None,
            server_state: Arc::new(tokio::sync::Mutex::new(FileServerState::default())),
            event_manager,
            config_actor,
        }
    }
}

impl Actor for FileServerActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // debug!("FileServerActor: Actor started");
        ctx.set_mailbox_capacity(64);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("FileServerActor: Actor stopped");
    }
}

// Message handlers
impl Handler<StartFileServer> for FileServerActor {
    type Result = ResponseActFuture<Self, Result<u16, String>>;
    
    fn handle(&mut self, msg: StartFileServer, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("FileServerActor: Received StartFileServer message");
        let server_state = self.server_state.clone();
        let event_manager = self.event_manager.clone();
        let orchestrator_addr = msg.orchestrator_addr;
        let config_actor = self.config_actor.clone();
        Box::pin(
            async move {
                // Get root folder from config_actor
                let root_path = if let Some(ref config) = config_actor {
                    match config.send(GetRootFolder).await {
                        Ok(Ok(folder)) => folder.unwrap_or_default(),
                        Ok(Err(e)) => {
                            warn!("FileServerActor: Failed to get root folder: {}, using default", e);
                            String::new()
                        }
                        Err(e) => {
                            warn!("FileServerActor: Failed to send GetRootFolder: {:?}, using default", e);
                            String::new()
                        }
                    }
                } else {
                    debug!("FileServerActor: Config actor not available, using default root folder");
                    String::new()
                };
                
                debug!("FileServerActor: Starting file server with root path: {}", root_path);
                
                let path = std::path::Path::new(&root_path);
                if !root_path.is_empty() && (!path.exists() || !path.is_dir()) {
                    let error = format!("Root path does not exist or is not a directory: {}", root_path);
                    warn!("FileServerActor: {}", error);
                    // Emit non-fatal FileServerFailed event instead of StartupFailed
                    // This allows startup to continue and the error will be shown in the file explorer
                    if let Some(ref orchestrator) = orchestrator_addr {
                        orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                            event: crate::messages::orchestrator::StartupEvent::FileServerFailed(error.clone()),
                        });
                    }
                    // Store error state for later retrieval by file explorer
                    event_manager.emit_file_server_error(&error).await.ok();
                    // Return error but don't fail startup
                    return Err(error);
                }
                
                match start_server_internal(server_state, root_path.clone()).await {
                    Ok(port) => {
                        event_manager.emit_file_server_started(port).await.ok();
                        
                        // Emit StartupEvent to orchestrator
                        if let Some(ref orchestrator) = orchestrator_addr {
                            orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                event: crate::messages::orchestrator::StartupEvent::FileServerStarted,
                            });
                        }
                        
                        Ok::<(u16, String), String>((port, root_path))
                    }
                    Err(e) => {
                        error!("FileServerActor: Failed to start file server: {}", e);
                        // Emit non-fatal FileServerFailed event instead of StartupFailed
                        if let Some(ref orchestrator) = orchestrator_addr {
                            orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                event: crate::messages::orchestrator::StartupEvent::FileServerFailed(format!("Failed to start file server: {}", e)),
                            });
                        }
                        // Store error state for later retrieval by file explorer
                        event_manager.emit_file_server_error(&e).await.ok();
                        Err(e)
                    }
                }
            }
            .into_actor(self)
            .map(|res, actor, _| {
                match res {
                    Ok((port, root_path_used)) => {
                        actor.is_running = true;
                        actor.current_root_path = Some(root_path_used.clone());
                        actor.current_port = Some(port);
                        actor.error_message = None; // Clear any previous error
                        actor.server_info = Some(FileServerInfo { port, root_path: root_path_used.clone(), is_running: true });
                        debug!("FileServerActor: File server started successfully on port {}", port);
                        Ok(port)
                    }
                    Err(e) => {
                        // Store error message for file explorer to display
                        actor.error_message = Some(e.clone());
                        actor.is_running = false;
                        Err(e)
                    }
                }
            })
        )
    }
}

impl Handler<StopFileServer> for FileServerActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: StopFileServer, ctx: &mut Context<Self>) -> Self::Result {
        debug!("FileServerActor: Received StopFileServer message");
        
        // Spawn async operation
        let server_state = self.server_state.clone();
        
        ctx.spawn(
            async move {
                debug!("FileServerActor: Stopping file server in async task");
                match stop_server_internal(server_state).await {
                    Ok(_) => {
                        debug!("FileServerActor: File server stopped successfully");
                    }
                    Err(e) => {
                        error!("FileServerActor: Failed to stop file server: {}", e);
                    }
                }
            }
            .into_actor(self)
        );
        
        // Update actor state
        self.is_running = false;
        self.current_port = None;
        self.current_root_path = None;
        self.server_info = None;
        
        Ok(())
    }
}

impl Handler<GetFileServerPort> for FileServerActor {
    type Result = Result<Option<u16>, String>;
    
    fn handle(&mut self, _msg: GetFileServerPort, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("FileServerActor: Received GetFileServerPort message");
        Ok(self.current_port)
    }
}

impl Handler<IsFileServerRunning> for FileServerActor {
    type Result = Result<bool, String>;
    
    fn handle(&mut self, _msg: IsFileServerRunning, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("FileServerActor: Received IsFileServerRunning message");
        Ok(self.is_running)
    }
}

impl Handler<GetFileServerUrl> for FileServerActor {
    type Result = Result<Option<String>, String>;
    
    fn handle(&mut self, _msg: GetFileServerUrl, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("FileServerActor: Received GetFileServerUrl message");
        if let Some(port) = self.current_port {
            Ok(Some(format!("http://127.0.0.1:{}", port)))
        } else {
            Ok(None)
        }
    }
}

// Clone implementation for async operations
impl Clone for FileServerActor {
    fn clone(&self) -> Self {
        Self {
            is_running: self.is_running,
            server_info: self.server_info.clone(),
            current_root_path: self.current_root_path.clone(),
            current_port: self.current_port,
            error_message: self.error_message.clone(),
            server_state: self.server_state.clone(),
            event_manager: self.event_manager.clone(),
            config_actor: self.config_actor.clone(),
        }
    }
}
