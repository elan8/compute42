use async_trait::async_trait;
use internals::service_traits::EventEmitter;
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub struct TauriEventEmitter {
    app: AppHandle,
}

impl TauriEventEmitter {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait]
impl EventEmitter for TauriEventEmitter {
    async fn emit(&self, event: &str, payload: serde_json::Value) -> Result<(), String> {
        // The new unified event system uses "category:event_type" format
        // We emit the event with the full name and the payload contains the unified event structure
        self.app.emit(event, payload).map_err(|e| e.to_string())
    }

    async fn emit_all(&self, event: &str, payload: serde_json::Value) -> Result<(), String> {
        // For Tauri, emit and emit_all are the same since we're dealing with a single window
        self.app.emit(event, payload).map_err(|e| e.to_string())
    }
}


