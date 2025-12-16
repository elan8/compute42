use log::error;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Child;
use crate::service_traits::EventEmitter;

/// Single persistent Julia session that stays alive for the entire application
pub struct PersistentJuliaSession {
    pub julia_process: Child,
    pub to_julia_pipe_name: Option<String>,
    pub from_julia_pipe_name: Option<String>,
    pub is_active: bool,
    pub event_emitter: Arc<dyn EventEmitter>,
}

impl PersistentJuliaSession {
    pub fn new(julia_process: Child, event_emitter: Arc<dyn EventEmitter>) -> Self {
        Self {
            julia_process,
            to_julia_pipe_name: None,
            from_julia_pipe_name: None,
            is_active: true,
            event_emitter,
        }
    }

    /// Execute code by writing to Julia's stdin
    pub async fn execute_code(&mut self, code: String) -> Result<String, String> {
        // Write the code to Julia's stdin
        if let Some(stdin) = &mut self.julia_process.stdin {
            let code_with_newline = format!("{}\n", code);
            stdin
                .write_all(code_with_newline.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to Julia stdin: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("Failed to flush Julia stdin: {}", e))?;

            // Small delay to allow Julia to process the input
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Return empty string since output will come via stdout stream
            Ok("".to_string())
        } else {
            Err("Julia process stdin not available".to_string())
        }
    }

    pub async fn emit_session_status(&self, status: String) {
        if let Err(e) = self.event_emitter.emit("communication:session-status", serde_json::json!(status)).await {
            error!("Failed to emit session status: {}", e);
        }
    }
}


