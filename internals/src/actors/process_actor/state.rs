use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use actix::prelude::*;

/// Output buffer for notebook cell execution
#[derive(Clone, Debug)]
pub struct NotebookCellOutputBuffer {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub plots: Vec<(String, String)>, // (mime_type, data) pairs
}

/// Internal state for ProcessActor
pub struct ProcessState {
    pub julia_path: Arc<Mutex<PathBuf>>,
    #[allow(dead_code)]
    pub julia_pipes_ready_sender: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<()>>>>,
    pub julia_message_loop_ready_sender: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<()>>>>,
    pub project_activation_complete_sender: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<()>>>>,
    pub output_suppressed: Arc<Mutex<bool>>,
    pub message_loop_ready_received: Arc<Mutex<bool>>,
    pub communication_actor: Arc<Mutex<Option<Addr<crate::actors::CommunicationActor>>>>,
    pub orchestrator_actor: Arc<Mutex<Option<Addr<crate::actors::OrchestratorActor>>>>,
    // Notebook cell output buffering
    pub current_notebook_cell: Arc<Mutex<Option<String>>>, // Current cell ID being executed
    pub notebook_cell_output_buffer: Arc<Mutex<Option<NotebookCellOutputBuffer>>>, // Buffered output for current cell
}

impl ProcessState {
    pub fn new() -> Self {
        // Default Julia path based on OS
        #[cfg(target_os = "windows")]
        let default_path = PathBuf::from("julia.exe");
        #[cfg(not(target_os = "windows"))]
        let default_path = PathBuf::from("julia");

        Self {
            julia_path: Arc::new(Mutex::new(default_path)),
            julia_pipes_ready_sender: Arc::new(Mutex::new(None)),
            julia_message_loop_ready_sender: Arc::new(Mutex::new(None)),
            project_activation_complete_sender: Arc::new(Mutex::new(None)),
            output_suppressed: Arc::new(Mutex::new(true)), // Suppress output during initialization
            message_loop_ready_received: Arc::new(Mutex::new(false)),
            communication_actor: Arc::new(Mutex::new(None)),
            orchestrator_actor: Arc::new(Mutex::new(None)),
            current_notebook_cell: Arc::new(Mutex::new(None)),
            notebook_cell_output_buffer: Arc::new(Mutex::new(None)),
        }
    }

    /// Get Julia executable path
    pub async fn get_julia_executable_path(&self) -> PathBuf {
        let path_guard = self.julia_path.lock().await;
        path_guard.clone()
    }

    /// Set Julia executable path
    pub async fn set_julia_executable_path(&self, path: PathBuf) {
        let mut path_guard = self.julia_path.lock().await;
        *path_guard = path;
    }

    /// Set Julia pipes ready sender
    #[allow(dead_code)]
    pub async fn set_julia_pipes_ready_sender(&self, sender: tokio::sync::mpsc::UnboundedSender<()>) {
        let mut sender_guard = self.julia_pipes_ready_sender.lock().await;
        *sender_guard = Some(sender);
    }

    /// Set Julia message loop ready sender
    pub async fn set_julia_message_loop_ready_sender(&self, sender: tokio::sync::mpsc::UnboundedSender<()>) {
        let mut sender_guard = self.julia_message_loop_ready_sender.lock().await;
        *sender_guard = Some(sender);
    }

    /// Set project activation complete sender
    pub async fn set_project_activation_complete_sender(&self, sender: tokio::sync::mpsc::UnboundedSender<()>) {
        let mut sender_guard = self.project_activation_complete_sender.lock().await;
        *sender_guard = Some(sender);
    }

    /// Set output suppression
    pub async fn set_output_suppression(&self, suppressed: bool) {
        let mut suppressed_guard = self.output_suppressed.lock().await;
        *suppressed_guard = suppressed;
    }

    /// Get output suppression state
    #[allow(dead_code)]
    pub async fn is_output_suppressed(&self) -> bool {
        let suppressed_guard = self.output_suppressed.lock().await;
        *suppressed_guard
    }

    /// Generate unique pipe names for communication
    pub fn generate_pipe_names(&self) -> (String, String) {
        let pid = std::process::id();
        // Add timestamp to ensure uniqueness even if PID is reused
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let to_julia_pipe = format!("compute42_to_julia_{}_{}", pid, timestamp);
        let from_julia_pipe = format!("compute42_from_julia_{}_{}", pid, timestamp);
        (to_julia_pipe, from_julia_pipe)
    }

    /// Get the JuliaJunction data directory
    /// Always uses AppData\Local\com.compute42.dev to avoid write permission errors
    pub fn get_julia_data_directory(&self) -> PathBuf {
        self.get_user_data_directory()
    }
    
    /// Get user data directory for JuliaJunction
    /// Uses AppData\Local\com.compute42.dev for consistent access
    pub fn get_user_data_directory(&self) -> PathBuf {
        dirs::data_local_dir()
            .map(|dir| dir.join("com.compute42.dev"))
            .unwrap_or_else(|| {
                log::warn!("Failed to get user data directory, falling back to current directory");
                PathBuf::from(".")
            })
    }

    /// Set communication actor address
    pub async fn set_communication_actor(&self, actor: Addr<crate::actors::CommunicationActor>) {
        let mut guard = self.communication_actor.lock().await;
        *guard = Some(actor);
    }

    /// Set orchestrator actor address
    pub async fn set_orchestrator_actor(&self, actor: Addr<crate::actors::OrchestratorActor>) {
        let mut guard = self.orchestrator_actor.lock().await;
        *guard = Some(actor);
    }

    /// Find Julia project by looking for Project.toml in current directory and parent directories
    pub fn find_julia_project(&self) -> Option<PathBuf> {
        // Start from current working directory
        let mut current_dir = std::env::current_dir().ok()?;
        
        // Walk up the directory tree looking for Project.toml
        loop {
            let project_toml = current_dir.join("Project.toml");
            if project_toml.exists() {
                log::debug!("Found Julia project at: {:?}", current_dir);
                return Some(current_dir);
            }
            
            // Move to parent directory
            match current_dir.parent() {
                Some(parent) => current_dir = parent.to_path_buf(),
                None => break, // Reached root directory
            }
        }
        
        log::debug!("No Julia project found in directory tree");
        None
    }
}

impl Default for ProcessState {
    fn default() -> Self {
        Self::new()
    }
}

