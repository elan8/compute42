mod session;
pub mod state;  // Make state module public so NotebookCellOutputBuffer can be used in messages
mod file_creation;
mod setup;
mod output_monitoring;
mod lifecycle;

use actix::prelude::*;
use std::sync::Arc;
use log::{debug, error, warn};
use tokio::sync::Mutex;

use crate::messages::process::*;
use crate::messages::installation::GetJuliaPathFromInstallation;
use crate::messages::orchestrator::{JuliaMessageLoopReady, ProjectActivationComplete};
use crate::services::events::EventService;
use crate::types::JuliaInstallation;
use crate::actors::{InstallationActor, OrchestratorActor};

use session::PersistentJuliaSession;
use state::ProcessState;
use lifecycle::{start_julia_with_communication, stop_julia_process, get_pipe_names};

/// ProcessActor - manages Julia process lifecycle
/// This replaces the mutex-based ProcessManager with a clean actor model
pub struct ProcessActor {
    // Actor state
    is_running: bool,
    process_id: Option<u32>,
    to_julia_pipe: Option<String>,
    from_julia_pipe: Option<String>,
    julia_installation: Option<JuliaInstallation>,
    
    // Internal state
    state: Arc<ProcessState>,
    julia_session: Arc<Mutex<Option<PersistentJuliaSession>>>,
    event_emitter: Arc<dyn crate::service_traits::EventEmitter>,
    event_manager: EventService,
    
    // Actor addresses for message passing
    installation_actor: Option<Addr<InstallationActor>>,
    orchestrator_actor: Option<Addr<OrchestratorActor>>,
    communication_actor: Option<Addr<crate::actors::CommunicationActor>>,
}

impl ProcessActor {
    /// Create a new ProcessActor instance
    pub fn new(
        event_emitter: Arc<dyn crate::service_traits::EventEmitter>,
        event_manager: EventService,
        installation_actor: Option<Addr<InstallationActor>>,
    ) -> Self {
        Self {
            is_running: false,
            process_id: None,
            to_julia_pipe: None,
            from_julia_pipe: None,
            julia_installation: None,
            state: Arc::new(ProcessState::new()),
            julia_session: Arc::new(Mutex::new(None)),
            event_emitter,
            event_manager,
            installation_actor,
            orchestrator_actor: None,
            communication_actor: None,
        }
    }

    /// Set orchestrator actor address and configure message channels
    pub fn set_orchestrator_actor(&mut self, orchestrator_actor: Addr<OrchestratorActor>, ctx: &mut Context<Self>) {
        self.orchestrator_actor = Some(orchestrator_actor.clone());
        
        // Store orchestrator in state for access from monitoring
        let state_clone = self.state.clone();
        let orchestrator_clone = orchestrator_actor.clone();
        ctx.spawn(
            async move {
                state_clone.set_orchestrator_actor(orchestrator_clone).await;
                debug!("ProcessActor: Orchestrator actor stored in state");
            }
            .into_actor(self)
        );
        
        // Create channel for message loop ready notification
        let (loop_tx, mut loop_rx) = tokio::sync::mpsc::unbounded_channel();
        let state_clone = self.state.clone();
        let loop_tx_clone = loop_tx.clone();
        ctx.spawn(
            async move {
                state_clone.set_julia_message_loop_ready_sender(loop_tx_clone).await;
                debug!("ProcessActor: Julia message loop ready sender has been set");
            }
            .into_actor(self)
        );
        
        // Spawn task to listen for message loop ready signal and forward to orchestrator
        let orchestrator_for_loop = orchestrator_actor.clone();
        ctx.spawn(
            async move {
                #[allow(clippy::redundant_pattern_matching)]
                while let Some(_) = loop_rx.recv().await {
                    debug!("ProcessActor: Received Julia message loop ready signal, forwarding to orchestrator");
                    if let Err(e) = orchestrator_for_loop.send(JuliaMessageLoopReady).await {
                        debug!("ProcessActor: Failed to send JuliaMessageLoopReady to orchestrator: {:?}", e);
                    }
                }
            }
            .into_actor(self)
        );
        
        // Create channel for project activation complete notification
        let (activation_tx, mut activation_rx) = tokio::sync::mpsc::unbounded_channel();
        let state_clone = self.state.clone();
        let activation_tx_clone = activation_tx.clone();
        ctx.spawn(
            async move {
                state_clone.set_project_activation_complete_sender(activation_tx_clone).await;
                debug!("ProcessActor: Project activation complete sender has been set");
            }
            .into_actor(self)
        );
        
        // Spawn task to listen for project activation complete signal and forward to orchestrator
        let orchestrator_for_activation = orchestrator_actor.clone();
        ctx.spawn(
            async move {
                #[allow(clippy::redundant_pattern_matching)]
                while let Some(_) = activation_rx.recv().await {
                    debug!("ProcessActor: Received project activation complete signal, forwarding to orchestrator");
                    if let Err(e) = orchestrator_for_activation.send(ProjectActivationComplete).await {
                        debug!("ProcessActor: Failed to send ProjectActivationComplete to orchestrator: {:?}", e);
                    }
                }
            }
            .into_actor(self)
        );
    }
    
}

impl Actor for ProcessActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // debug!("ProcessActor: Actor started");
        ctx.set_mailbox_capacity(128);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("ProcessActor: Actor stopped");
    }
}

impl actix::Supervised for ProcessActor {}

// Message handlers
impl Handler<StartJuliaProcess> for ProcessActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: StartJuliaProcess, _ctx: &mut Context<Self>) -> Self::Result {
        let state = self.state.clone();
        let julia_session = self.julia_session.clone();
        let event_emitter = self.event_emitter.clone();
        let installation_actor = self.installation_actor.clone();
        let event_manager = self.event_manager.clone();
        let orchestrator_addr = msg.orchestrator_addr;
        Box::pin(
            async move {
                // Get Julia path from InstallationActor
                debug!("ProcessActor: Checking for installation actor availability");
                if let Some(installation_actor) = installation_actor {
                    debug!("ProcessActor: Installation actor available, sending GetJuliaPath message");
                    match installation_actor.send(GetJuliaPathFromInstallation).await {
                        Ok(Ok(Some(julia_path))) => {
                            debug!("ProcessActor: Julia path found from installation actor: {}", julia_path);
                            // Set the Julia path in the state
                            state.set_julia_executable_path(std::path::PathBuf::from(julia_path)).await;
                        }
                        Ok(Ok(None)) => {
                            debug!("ProcessActor: Julia path not available from installation actor");
                            if let Some(ref orchestrator) = orchestrator_addr {
                                orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                    event: crate::messages::orchestrator::StartupEvent::StartupFailed("Julia path not available from installation actor".to_string()),
                                });
                            }
                            return Err("Julia path not available from installation actor".to_string());
                        }
                        Ok(Err(e)) => {
                            debug!("ProcessActor: Failed to get Julia path from installation actor: {}", e);
                            if let Some(ref orchestrator) = orchestrator_addr {
                                orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                    event: crate::messages::orchestrator::StartupEvent::StartupFailed(format!("Failed to get Julia path from installation actor: {}", e)),
                                });
                            }
                            return Err(format!("Failed to get Julia path from installation actor: {}", e));
                        }
                        Err(e) => {
                            debug!("ProcessActor: Failed to send GetJuliaPath message to installation actor: {}", e);
                            if let Some(ref orchestrator) = orchestrator_addr {
                                orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                    event: crate::messages::orchestrator::StartupEvent::StartupFailed(format!("Failed to send GetJuliaPath message to installation actor: {:?}", e)),
                                });
                            }
                            return Err(format!("Failed to send GetJuliaPath message to installation actor: {:?}", e));
                        }
                    }
                } else {
                    debug!("ProcessActor: Installation actor not available");
                    if let Some(ref orchestrator) = orchestrator_addr {
                        orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                            event: crate::messages::orchestrator::StartupEvent::StartupFailed("Installation actor not available".to_string()),
                        });
                    }
                    return Err("Installation actor not available".to_string());
                }
                
                // Start the process and wait until it has launched
                match start_julia_with_communication(state, event_emitter, julia_session).await {
                    Ok(()) => {
                        // Fire event for external observers
                        let _ = event_manager.emit_julia_process_started().await;
                        
                        // Emit StartupEvent to orchestrator
                        if let Some(ref orchestrator) = orchestrator_addr {
                            orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                event: crate::messages::orchestrator::StartupEvent::JuliaProcessStarted,
                            });
                        }
                        Ok(())
                    }
                    Err(e) => {
                        error!("ProcessActor: Failed to start Julia process: {}", e);
                        if let Some(ref orchestrator) = orchestrator_addr {
                            orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                event: crate::messages::orchestrator::StartupEvent::StartupFailed(format!("Failed to start Julia process: {}", e)),
                            });
                        }
                        Err(e)
                    }
                }
            }
            .into_actor(self)
            .map(|res, actor, _| {
                if res.is_ok() {
                    actor.is_running = true;
                    actor.process_id = None;
                    actor.to_julia_pipe = None;
                    actor.from_julia_pipe = None;
                    debug!("ProcessActor: Julia process started successfully");
                }
                res
            })
        )
    }
}

impl Handler<StopJuliaProcess> for ProcessActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: StopJuliaProcess, ctx: &mut Context<Self>) -> Self::Result {
        debug!("ProcessActor: Received StopJuliaProcess message");
        
        // Spawn async operation
        let julia_session = self.julia_session.clone();
        
        ctx.spawn(
            async move {
                debug!("ProcessActor: Stopping Julia process in async task");
                match stop_julia_process(julia_session).await {
                    Ok(_) => {
                        debug!("ProcessActor: Julia process stopped successfully");
                    }
                    Err(e) => {
                        error!("ProcessActor: Failed to stop Julia process: {}", e);
                    }
                }
            }
            .into_actor(self)
        );
        
        // Update actor state
        self.is_running = false;
        
        Ok(())
    }
}

impl Handler<IsJuliaRunning> for ProcessActor {
    type Result = Result<bool, String>;
    
    fn handle(&mut self, _msg: IsJuliaRunning, _ctx: &mut Context<Self>) -> Self::Result {
        // For now, return a simple check - this could be enhanced later
        Ok(self.is_running)
    }
}

impl Handler<GetPipeNames> for ProcessActor {
    type Result = ResponseActFuture<Self, Result<(String, String), String>>;
    
    fn handle(&mut self, _msg: GetPipeNames, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("ProcessActor: Received GetPipeNames message");
        
        // Get pipe names from the stored session
        // This retrieves the actual pipe names that were used when starting Julia
        let julia_session = self.julia_session.clone();
        
        Box::pin(
            async move {
                match get_pipe_names(julia_session).await {
                    Ok((to_julia_pipe, from_julia_pipe)) => {
                        debug!("ProcessActor: Retrieved pipe names - To Julia: {}, From Julia: {}", to_julia_pipe, from_julia_pipe);
                        Ok((to_julia_pipe, from_julia_pipe))
                    }
                    Err(e) => {
                        warn!("ProcessActor: Failed to get pipe names: {}. This may happen if Julia hasn't started yet.", e);
                        Err(format!("Failed to get pipe names: {}", e))
                    }
                }
            }
            .into_actor(self)
        )
    }
}

impl Handler<RestartJulia> for ProcessActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: RestartJulia, ctx: &mut Context<Self>) -> Self::Result {
        debug!("ProcessActor: Received RestartJulia message");
        
        // Spawn async operation
        let state = self.state.clone();
        let julia_session = self.julia_session.clone();
        let event_emitter = self.event_emitter.clone();
        
        ctx.spawn(
            async move {
                debug!("ProcessActor: Restarting Julia process in async task");
                match stop_julia_process(julia_session.clone()).await {
                    Ok(_) => {
                        debug!("ProcessActor: Julia process stopped for restart");
                        // Wait a bit for cleanup
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        // Start new process
                        match start_julia_with_communication(state, event_emitter, julia_session).await {
                            Ok(_) => {
                                debug!("ProcessActor: Julia process restarted successfully");
                            }
                            Err(e) => {
                                error!("ProcessActor: Failed to restart Julia process: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("ProcessActor: Failed to stop Julia process for restart: {}", e);
                    }
                }
            }
            .into_actor(self)
        );
        
        Ok(())
    }
}

impl Handler<SetOrchestratorActor> for ProcessActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: SetOrchestratorActor, ctx: &mut Context<Self>) -> Self::Result {
        debug!("ProcessActor: Received SetOrchestratorActor message");
        self.set_orchestrator_actor(msg.orchestrator_actor, ctx);
        Ok(())
    }
}

impl Handler<SetOutputSuppression> for ProcessActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: SetOutputSuppression, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("ProcessActor: Received SetOutputSuppression message: {}", msg.suppressed);
        let state = self.state.clone();
        Box::pin(
            async move {
                state.set_output_suppression(msg.suppressed).await;
                Ok(())
            }
            .into_actor(self)
        )
    }
}

impl Handler<SetNotebookCell> for ProcessActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;

    fn handle(&mut self, msg: SetNotebookCell, _ctx: &mut Self::Context) -> Self::Result {
        debug!("ProcessActor: Received SetNotebookCell message: {:?}", msg.cell_id);
        let state = self.state.clone();
        Box::pin(
            async move {
                // Set current cell ID
                {
                    let mut cell_guard = state.current_notebook_cell.lock().await;
                    *cell_guard = msg.cell_id.clone();
                }
                
                // Initialize or clear output buffer
                {
                    let mut buffer_guard = state.notebook_cell_output_buffer.lock().await;
                    if msg.cell_id.is_some() {
                        *buffer_guard = Some(state::NotebookCellOutputBuffer {
                            stdout: Vec::new(),
                            stderr: Vec::new(),
                            plots: Vec::new(),
                        });
                    } else {
                        *buffer_guard = None;
                    }
                }
                
                Ok(())
            }
            .into_actor(self)
        )
    }
}

impl Handler<GetNotebookCellOutput> for ProcessActor {
    type Result = ResponseActFuture<Self, Result<Option<state::NotebookCellOutputBuffer>, String>>;

    fn handle(&mut self, _msg: GetNotebookCellOutput, _ctx: &mut Self::Context) -> Self::Result {
        debug!("ProcessActor: Received GetNotebookCellOutput message");
        let state = self.state.clone();
        Box::pin(
            async move {
                // Get and clear the buffer
                let buffer = {
                    let mut buffer_guard = state.notebook_cell_output_buffer.lock().await;
                    buffer_guard.take()
                };
                
                // Clear current cell
                {
                    let mut cell_guard = state.current_notebook_cell.lock().await;
                    *cell_guard = None;
                }
                
                Ok(buffer)
            }
            .into_actor(self)
        )
    }
}

impl Handler<SetCommunicationActor> for ProcessActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: SetCommunicationActor, ctx: &mut Context<Self>) -> Self::Result {
        debug!("ProcessActor: Received SetCommunicationActor message");
        self.communication_actor = Some(msg.communication_actor.clone());
        
        // Store communication actor in state for access from monitoring
        let state_clone = self.state.clone();
        let comm_actor_clone = msg.communication_actor.clone();
        ctx.spawn(
            async move {
                state_clone.set_communication_actor(comm_actor_clone).await;
                debug!("ProcessActor: Communication actor stored in state");
            }
            .into_actor(self)
        );
        
        Box::pin(async move { Ok(()) }.into_actor(self))
    }
}

impl Handler<BufferNotebookCellPlot> for ProcessActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;

    fn handle(&mut self, msg: BufferNotebookCellPlot, _ctx: &mut Self::Context) -> Self::Result {
        debug!("ProcessActor: Received BufferNotebookCellPlot message");
        let state = self.state.clone();
        Box::pin(
            async move {
                // Check if a notebook cell is currently executing
                let is_notebook_cell = {
                    let cell_guard = state.current_notebook_cell.lock().await;
                    cell_guard.is_some()
                };
                
                if is_notebook_cell {
                    // Buffer the plot
                    let mut buffer_guard = state.notebook_cell_output_buffer.lock().await;
                    if let Some(ref mut buffer) = *buffer_guard {
                        buffer.plots.push((msg.mime_type, msg.data));
                        debug!("ProcessActor: Buffered plot for notebook cell");
                    }
                }
                
                Ok(())
            }
            .into_actor(self)
        )
    }
}

// Clone implementation for async operations
impl Clone for ProcessActor {
    fn clone(&self) -> Self {
        Self {
            is_running: self.is_running,
            process_id: self.process_id,
            to_julia_pipe: self.to_julia_pipe.clone(),
            from_julia_pipe: self.from_julia_pipe.clone(),
            julia_installation: self.julia_installation.clone(),
            state: self.state.clone(),
            julia_session: self.julia_session.clone(),
            event_emitter: self.event_emitter.clone(),
            event_manager: self.event_manager.clone(),
            installation_actor: self.installation_actor.clone(),
            orchestrator_actor: self.orchestrator_actor.clone(),
            communication_actor: self.communication_actor.clone(),
        }
    }
}
