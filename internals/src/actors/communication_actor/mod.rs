use actix::prelude::*;
use log::{debug, error};

use crate::messages::communication::*;
use crate::service_traits::CommunicationService as CommunicationServiceTrait;
use crate::messages::{ExecutionType, JuliaMessage};

mod state;
mod connection;
mod execution;
mod io_operations;
mod message_handler;

use state::State;

/// CommunicationActor - manages Julia process communication
/// This replaces the mutex-based CommunicationManager with a clean actor model
pub struct CommunicationActor {
    // Actor state (no mutexes needed for simple fields)
    is_connected: bool,
    to_julia_pipe: Option<String>,
    from_julia_pipe: Option<String>,
    #[allow(dead_code)]
    message_queue: Vec<JuliaMessage>,
    
    // Internal state (with mutexes for spawned tasks)
    state: std::sync::Arc<State>,
    event_manager: crate::services::events::EventService,
    
    // Actor addresses for inter-actor communication
    #[allow(dead_code)]
    plot_actor: Option<actix::Addr<crate::actors::PlotActor>>,
    #[allow(dead_code)]
    process_actor: Option<actix::Addr<crate::actors::ProcessActor>>,
    
    // Orchestrator actor for restart coordination
    orchestrator_actor: Option<actix::Addr<crate::actors::orchestrator_actor::OrchestratorActor>>,
}

impl CommunicationActor {
    /// Create a new CommunicationActor instance
    pub fn new(
        _event_emitter: std::sync::Arc<dyn crate::service_traits::EventEmitter>,
        plot_actor: actix::Addr<crate::actors::PlotActor>,
        process_actor: actix::Addr<crate::actors::ProcessActor>,
        event_manager: crate::services::events::EventService,
    ) -> Self {
        // Create state directly
        let state = std::sync::Arc::new(State::new(
            event_manager.clone(),
            plot_actor.clone(),
            process_actor.clone(),
        ));
        
        Self {
            is_connected: false,
            to_julia_pipe: None,
            from_julia_pipe: None,
            message_queue: Vec::new(),
            state,
            event_manager,
            plot_actor: Some(plot_actor),
            process_actor: Some(process_actor),
            orchestrator_actor: None,
        }
    }
    
    /// Disconnect from pipes
    #[allow(dead_code)]
    async fn disconnect_from_pipes(&mut self) -> Result<(), String> {
        debug!("CommunicationActor: Disconnecting from pipes");
        
        // Use connection module for external pipe operations
        connection::disconnect_from_pipes(&self.state).await?;
        
        // Update actor state
        self.is_connected = false;
        self.to_julia_pipe = None;
        self.from_julia_pipe = None;
        
        self.event_manager.emit_communication_event(
            "communication_disconnected",
            crate::services::events::CommunicationEventPayload { status: Some("disconnected".to_string()), connected: Some(false), request_id: None, message: None, error: None }
        ).await?;
        
        debug!("CommunicationActor: Disconnected from pipes successfully");
        Ok(())
    }
    
    /// Execute code
    #[allow(dead_code)]
    async fn execute_code(&mut self, code: String, execution_type: ExecutionType, file_path: Option<String>) -> Result<JuliaMessage, String> {
        debug!("CommunicationActor: Executing code with type {:?}", execution_type);
        
        // Check if connected
        if !self.is_connected {
            return Err("Not connected to Julia process".to_string());
        }
        
        // Use execution module for code execution
        let message = execution::execute_code(&self.state, code, execution_type, file_path, false).await?;
        
        // Queue message for processing
        self.message_queue.push(message.clone());
        
        debug!("CommunicationActor: Code executed successfully");
        Ok(message)
    }
    
    /// Check if connected
    async fn is_connected_internal(&self) -> bool {
        // Check if code pipe is connected - that's sufficient for sending messages (activation, etc.)
        // We don't need to wait for from_julia pipe for basic operations
        let code_connected = {
            let code_stream_guard = self.state.code_stream.lock().await;
            code_stream_guard.is_some()
        };
        
        // Also check the flag for backwards compatibility and to ensure we've gone through connection setup
        let flag_connected = *self.state.is_connected.lock().await;
        
        // Consider connected if code pipe is available (we can send messages)
        // OR if the flag is set (both pipes connected)
        code_connected || flag_connected
    }
    
    /// Check if busy
    async fn is_busy_internal(&self) -> bool {
        execution::is_busy(&self.state).await
    }
    
    /// Send debug message
    async fn send_debug_message(&self, message: JuliaMessage) -> Result<(), String> {
        let sender_guard = self.state.message_sender.lock().await;
        if let Some(ref sender) = *sender_guard {
            sender.send(message).await
                .map_err(|e| format!("Failed to send debug message: {}", e))?;
            Ok(())
        } else {
            Err("Not connected to Julia process".to_string())
        }
    }
}

impl Actor for CommunicationActor {
    type Context = Context<Self>;
    
    fn started(&mut self, ctx: &mut Context<Self>) {
        // debug!("CommunicationActor: Actor started");
        // Limit mailbox to avoid unbounded growth under high-throughput
        ctx.set_mailbox_capacity(256);
    }
    
    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("CommunicationActor: Actor stopped");
    }
}

impl actix::Supervised for CommunicationActor {}

// Message handlers
impl Handler<ConnectToPipes> for CommunicationActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: ConnectToPipes, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("CommunicationActor: Received ConnectToPipes message");
        let state = self.state.clone();
        let event_manager = self.event_manager.clone();
        let to_julia_pipe = msg.to_julia_pipe;
        let from_julia_pipe = msg.from_julia_pipe;
        Box::pin(
            async move {
                let message_sender = connection::connect_to_pipes(&state, to_julia_pipe.clone(), from_julia_pipe.clone()).await?;
                
                // Store the message sender for use in execution
                {
                    let mut sender_guard = state.message_sender.lock().await;
                    *sender_guard = Some(message_sender);
                }
                
                let _ = event_manager.emit_communication_event(
                    "communication_connected",
                    crate::services::events::CommunicationEventPayload { status: Some("connected".to_string()), connected: Some(true), request_id: None, message: None, error: None }
                ).await;
                Ok::<(String, String), String>((to_julia_pipe, from_julia_pipe))
            }
            .into_actor(self)
            .map(|res, actor, _| {
                match res {
                    Ok((to_julia, from_julia)) => {
                        actor.is_connected = true;
                        actor.to_julia_pipe = Some(to_julia);
                        actor.from_julia_pipe = Some(from_julia);
                        debug!("CommunicationActor: Connected to pipes successfully");
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            })
        )
    }
}

impl Handler<ConnectToJuliaPipe> for CommunicationActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: ConnectToJuliaPipe, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("CommunicationActor: Received ConnectToJuliaPipe message");
        let state = self.state.clone();
        let event_manager = self.event_manager.clone();
        let to_julia_pipe = msg.to_julia_pipe.clone();
        Box::pin(
            async move {
                let message_sender = connection::connect_to_julia_pipe(&state, msg.to_julia_pipe.clone()).await?;
                
                // Store the message sender for use in execution (if not already stored)
                {
                    let mut sender_guard = state.message_sender.lock().await;
                    if sender_guard.is_none() {
                        *sender_guard = Some(message_sender);
                    }
                }
                
                let _ = event_manager.emit_communication_event(
                    "communication_connected",
                    crate::services::events::CommunicationEventPayload { status: Some("to_julia_pipe_connected".to_string()), connected: Some(true), request_id: None, message: None, error: None }
                ).await;
                Ok(to_julia_pipe)
            }
            .into_actor(self)
            .map(|res, actor, _| {
                match res {
                    Ok(pipe_name) => {
                        actor.to_julia_pipe = Some(pipe_name);
                        debug!("CommunicationActor: To Julia pipe connected successfully");
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            })
        )
    }
}

impl Handler<ConnectFromJuliaPipe> for CommunicationActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: ConnectFromJuliaPipe, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("CommunicationActor: Received ConnectFromJuliaPipe message");
        let state = self.state.clone();
        let event_manager = self.event_manager.clone();
        let from_julia_pipe = msg.from_julia_pipe.clone();
        Box::pin(
            async move {
                connection::connect_from_julia_pipe(&*state, msg.from_julia_pipe.clone()).await?;
                
                // Check if both pipes are now connected
                let code_connected = {
                    let code_stream_guard = state.code_stream.lock().await;
                    code_stream_guard.is_some()
                };
                let from_julia_connected = {
                    let from_julia_read_stream_guard = state.from_julia_read_stream.lock().await;
                    from_julia_read_stream_guard.is_some()
                };
                
                if code_connected && from_julia_connected {
                    let mut is_connected_guard = state.is_connected.lock().await;
                    *is_connected_guard = true;
                    debug!("[CommunicationActor] Both pipes connected - marking as fully connected");
                }
                
                let _ = event_manager.emit_communication_event(
                    "communication_connected",
                    crate::services::events::CommunicationEventPayload { status: Some("from_julia_pipe_connected".to_string()), connected: Some(true), request_id: None, message: None, error: None }
                ).await;
                Ok(from_julia_pipe)
            }
            .into_actor(self)
            .map(|res, actor, _| {
                match res {
                    Ok(pipe_name) => {
                        actor.from_julia_pipe = Some(pipe_name);
                        // Check if both pipes are connected
                        if actor.to_julia_pipe.is_some() && actor.from_julia_pipe.is_some() {
                            actor.is_connected = true;
                            debug!("CommunicationActor: Both pipes connected successfully");
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            })
        )
    }
}

impl Handler<DisconnectFromPipes> for CommunicationActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: DisconnectFromPipes, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("CommunicationActor: Received DisconnectFromPipes message");
        
        // Use async spawn for disconnect
        let state = self.state.clone();
        let event_manager = self.event_manager.clone();
        // Spawn async task to disconnect - we don't need to update actor state here
        // so we use tokio::spawn instead of ctx.spawn to avoid lifetime issues
        tokio::spawn(async move {
            debug!("CommunicationActor: Disconnecting from pipes in async task");
            match connection::disconnect_from_pipes(&state).await {
                Ok(_) => {
                    debug!("CommunicationActor: Successfully disconnected from pipes");
                    let _ = event_manager.emit_communication_event(
                        "communication_disconnected",
                        crate::services::events::CommunicationEventPayload { status: Some("disconnected".to_string()), connected: Some(false), request_id: None, message: None, error: None }
                    ).await;
                }
                Err(e) => {
                    error!("CommunicationActor: Failed to disconnect from pipes: {}", e);
                }
            }
        });
        Ok(())
    }
}

impl Handler<ExecuteCode> for CommunicationActor {
    type Result = ResponseActFuture<Self, Result<JuliaMessage, String>>;
    
    fn handle(&mut self, msg: ExecuteCode, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("CommunicationActor: Received ExecuteCode message");
        
        let state = self.state.clone();
        let is_connected = self.is_connected;
        Box::pin(
            async move {
                debug!("CommunicationActor: Executing code");
                
                // Check if connected
                if !is_connected {
                    return Err("Not connected to Julia process".to_string());
                }
                
                execution::execute_code(&state, msg.code, msg.execution_type, msg.file_path, msg.suppress_busy_events).await
            }
            .into_actor(self)
        )
    }
}

impl Handler<SendDebugMessage> for CommunicationActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: SendDebugMessage, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("CommunicationActor: Received SendDebugMessage");
        let state = self.state.clone();
        Box::pin(
            async move {
                let sender_guard = state.message_sender.lock().await;
                if let Some(ref sender) = *sender_guard {
                    sender.send(msg.message).await
                        .map_err(|e| format!("Failed to send debug message: {}", e))
                } else {
                    Err("Not connected to Julia process".to_string())
                }
            }
            .into_actor(self)
        )
    }
}

impl Handler<IsConnected> for CommunicationActor {
    type Result = Result<bool, String>;
    
    fn handle(&mut self, _msg: IsConnected, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(self.is_connected)
    }
}

impl Handler<GetBackendBusyStatus> for CommunicationActor {
    type Result = ResponseActFuture<Self, Result<bool, String>>;
    
    fn handle(&mut self, _msg: GetBackendBusyStatus, _ctx: &mut Context<Self>) -> Self::Result {
        // Removed debug logging to avoid cluttering logs with periodic sync requests
        
        let state = self.state.clone();
        
        Box::pin(async move {
            let is_busy = execution::is_busy(&state).await;
            Ok(is_busy)
        }.into_actor(self))
    }
}

impl Handler<SetOrchestratorActor> for CommunicationActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: SetOrchestratorActor, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("CommunicationActor: Received SetOrchestratorActor message");
        
        self.orchestrator_actor = Some(msg.orchestrator_actor.clone());
        
        // Restart handler removed - stuck-state detection has been removed
        //debug!("CommunicationActor: Restart handler removed (stuck-state detection removed)");
        
        Ok(())
    }
}


// Implement CommunicationServiceTrait for backward compatibility if needed
#[async_trait::async_trait]
impl CommunicationServiceTrait for CommunicationActor {
    async fn connect_to_pipes(&self, to_julia_pipe: String, from_julia_pipe: String) -> Result<(), String> {
        let message_sender = connection::connect_to_pipes(&self.state, to_julia_pipe, from_julia_pipe).await?;
        
        // Store the message sender for use in execution
        {
            let mut sender_guard = self.state.message_sender.lock().await;
            *sender_guard = Some(message_sender);
        }
        
        Ok(())
    }
    
    async fn disconnect_from_pipes(&self) -> Result<(), String> {
        connection::disconnect_from_pipes(&self.state).await
    }
    
    async fn execute_code(
        &self,
        code: String,
        execution_type: ExecutionType,
        file_path: Option<String>,
    ) -> Result<JuliaMessage, String> {
        execution::execute_code(&self.state, code, execution_type, file_path, false).await
    }
    
    async fn send_debug_message(&self, message: JuliaMessage) -> Result<(), String> {
        self.send_debug_message(message).await
    }
    
    async fn is_connected(&self) -> bool {
        self.is_connected_internal().await
    }
    
    async fn is_busy(&self) -> bool {
        self.is_busy_internal().await
    }
    
    
    async fn set_plot_actor(&self, plot_actor: actix::Addr<crate::actors::PlotActor>) {
        self.state.set_plot_actor(plot_actor).await;
    }
}
