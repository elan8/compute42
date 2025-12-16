use crate::service_traits::*;
use crate::messages::*;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

// ============================================================================
// Mock ProcessService
// ============================================================================

pub struct MockProcessService {
    is_running: Arc<TokioMutex<bool>>,
    pipe_names: Arc<TokioMutex<(String, String)>>,
}

impl Default for MockProcessService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockProcessService {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(TokioMutex::new(false)),
            pipe_names: Arc::new(TokioMutex::new(("test-code-pipe".to_string(), "test-plot-pipe".to_string()))),
        }
    }

    pub async fn set_running(&self, running: bool) {
        *self.is_running.lock().await = running;
    }

    pub async fn set_pipe_names(&self, to_julia_pipe: String, from_julia_pipe: String) {
        *self.pipe_names.lock().await = (to_julia_pipe, from_julia_pipe);
    }
}

#[async_trait]
impl ProcessService for MockProcessService {
    async fn start_julia_process(&self) -> Result<(), String> {
        *self.is_running.lock().await = true;
        Ok(())
    }

    async fn stop_julia_process(&self) -> Result<(), String> {
        *self.is_running.lock().await = false;
        Ok(())
    }

    async fn is_julia_running(&self) -> bool {
        *self.is_running.lock().await
    }

    async fn get_pipe_names(&self) -> Result<(String, String), String> {
        Ok(self.pipe_names.lock().await.clone())
    }

    async fn restart_julia(&self) -> Result<(), String> {
        self.stop_julia_process().await?;
        self.start_julia_process().await
    }

    async fn set_julia_executable_path(&self, _path: std::path::PathBuf) {
        // Mock implementation - do nothing
    }
}

// ============================================================================
// Mock CommunicationService
// ============================================================================

pub struct MockCommunicationService {
    is_connected: Arc<TokioMutex<bool>>,
    messages: Arc<TokioMutex<Vec<JuliaMessage>>>,
}

impl Default for MockCommunicationService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCommunicationService {
    pub fn new() -> Self {
        Self {
            is_connected: Arc::new(TokioMutex::new(false)),
            messages: Arc::new(TokioMutex::new(Vec::new())),
        }
    }

    pub async fn set_connected(&self, connected: bool) {
        *self.is_connected.lock().await = connected;
    }

    pub async fn get_messages(&self) -> Vec<JuliaMessage> {
        self.messages.lock().await.clone()
    }

    pub async fn add_message(&self, message: JuliaMessage) {
        self.messages.lock().await.push(message);
    }
}

#[async_trait]
impl CommunicationService for MockCommunicationService {
    async fn connect_to_pipes(&self, _to_julia_pipe: String, _from_julia_pipe: String) -> Result<(), String> {
        *self.is_connected.lock().await = true;
        Ok(())
    }

    async fn disconnect_from_pipes(&self) -> Result<(), String> {
        *self.is_connected.lock().await = false;
        Ok(())
    }

    async fn execute_code(
        &self,
        _code: String,
        execution_type: ExecutionType,
        _file_path: Option<String>,
    ) -> Result<JuliaMessage, String> {
        let message = JuliaMessage::ExecutionComplete {
            id: uuid::Uuid::new_v4().to_string(),
            execution_type,
            result: Some("Mock execution result".to_string()),
            error: None,
            success: true,
            duration_ms: Some(100),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
        };
        self.messages.lock().await.push(message.clone());
        Ok(message)
    }

    async fn send_debug_message(&self, message: JuliaMessage) -> Result<(), String> {
        self.messages.lock().await.push(message);
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        *self.is_connected.lock().await
    }
    
    async fn is_busy(&self) -> bool {
        false // Mock is never busy
    }
}

