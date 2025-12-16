// State management for CommunicationActor
// Contains all state fields needed for communication with Julia processes

use crate::services::events::EventService;
use actix::prelude::*;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

// Platform-specific stream type
// On Unix: use standard library UnixStream
// On Windows: use interprocess LocalSocketStream for named pipes
#[cfg(unix)]
pub type LocalSocketStream = std::os::unix::net::UnixStream;

#[cfg(not(unix))]
pub use interprocess::local_socket::prelude::LocalSocketStream;

/// State for CommunicationActor
/// Fields that are only accessed within actor message handlers don't need mutexes
/// Fields accessed from spawned tasks (like streams) still need Arc<Mutex<>>
pub struct State {
    // Connection state - accessed from spawned tasks, need mutexes
    pub to_julia_pipe_name: Arc<Mutex<String>>,
    pub from_julia_pipe_name: Arc<Mutex<String>>,
    pub code_connection: Arc<Mutex<Option<LocalSocketStream>>>,
    pub plot_connection: Arc<Mutex<Option<LocalSocketStream>>>,
    pub is_connecting: Arc<Mutex<bool>>,
    pub is_connected: Arc<Mutex<bool>>,
    pub code_stream: Arc<Mutex<Option<LocalSocketStream>>>,
    pub from_julia_read_stream: Arc<Mutex<Option<LocalSocketStream>>>,
    
    // Services - EventService is already thread-safe
    pub event_manager: EventService,
    
    // Actor references - accessed from spawned tasks, need mutexes
    pub plot_actor: Arc<Mutex<Option<Addr<crate::actors::PlotActor>>>>,
    #[allow(dead_code)]
    pub process_actor: Arc<Mutex<Option<Addr<crate::actors::ProcessActor>>>>,
    
    // Communication state - accessed from spawned tasks, need mutexes
    #[allow(clippy::type_complexity)]
    pub current_request: Arc<Mutex<Option<(String, tokio::sync::oneshot::Sender<crate::messages::JuliaMessage>)>>>,
    pub message_sender: Arc<Mutex<Option<mpsc::Sender<crate::messages::JuliaMessage>>>>,
    
}

impl State {
    pub fn new(
        event_manager: EventService,
        plot_actor: Addr<crate::actors::PlotActor>,
        process_actor: Addr<crate::actors::ProcessActor>,
    ) -> Self {
        Self {
            to_julia_pipe_name: Arc::new(Mutex::new(String::new())),
            from_julia_pipe_name: Arc::new(Mutex::new(String::new())),
            code_connection: Arc::new(Mutex::new(None)),
            plot_connection: Arc::new(Mutex::new(None)),
            is_connecting: Arc::new(Mutex::new(false)),
            is_connected: Arc::new(Mutex::new(false)),
            code_stream: Arc::new(Mutex::new(None)),
            from_julia_read_stream: Arc::new(Mutex::new(None)),
            event_manager,
            plot_actor: Arc::new(Mutex::new(Some(plot_actor))),
            process_actor: Arc::new(Mutex::new(Some(process_actor))),
            current_request: Arc::new(Mutex::new(None)),
            message_sender: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Set PlotActor address for routing plot data through actor
    pub async fn set_plot_actor(&self, plot_actor: Addr<crate::actors::PlotActor>) {
        let mut plot_actor_guard = self.plot_actor.lock().await;
        *plot_actor_guard = Some(plot_actor);
    }
    
}

