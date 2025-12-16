use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::service_traits::{EventEmitter, FilePersistenceService};

// ============================================================================
// Mock EventEmitter
// ============================================================================

pub struct MockEventEmitter {
    events: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
}

impl Default for MockEventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEventEmitter {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn get_emitted_events(&self) -> Vec<(String, serde_json::Value)> {
        let events_guard = self.events.lock().await;
        events_guard.clone()
    }
    
    pub async fn get_events(&self) -> Vec<serde_json::Value> {
        let events_guard = self.events.lock().await;
        events_guard.clone().into_iter().map(|(event_type, payload)| {
            let mut event_obj = serde_json::Map::new();
            event_obj.insert(event_type, payload);
            serde_json::Value::Object(event_obj)
        }).collect()
    }
    
    pub async fn clear_events(&self) {
        let mut events_guard = self.events.lock().await;
        events_guard.clear();
    }
    
    pub async fn find_event(&self, event_type: &str) -> Option<serde_json::Value> {
        let events_guard = self.events.lock().await;
        events_guard
            .iter()
            .find(|(event, _)| event == event_type)
            .map(|(_, payload)| payload.clone())
    }
}

#[async_trait]
impl EventEmitter for MockEventEmitter {
    async fn emit(&self, event: &str, payload: serde_json::Value) -> Result<(), String> {
        let mut events_guard = self.events.lock().await;
        events_guard.push((event.to_string(), payload));
        Ok(())
    }

    async fn emit_all(&self, event: &str, payload: serde_json::Value) -> Result<(), String> {
        let mut events_guard = self.events.lock().await;
        events_guard.push((event.to_string(), payload));
        Ok(())
    }
}



// ============================================================================
// Mock PersistenceService
// ============================================================================

pub struct MockPersistenceService {
    data: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for MockPersistenceService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPersistenceService {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl FilePersistenceService for MockPersistenceService {
    async fn load_json_value(&self, key: &str) -> Result<serde_json::Value, String> {
        let data_guard = self.data.lock().await;
        if let Some(json_str) = data_guard.get(key) {
            serde_json::from_str(json_str)
                .map_err(|e| format!("Failed to deserialize data for {}: {}", key, e))
        } else {
            Ok(serde_json::Value::Null)
        }
    }
    
    async fn save_json_value(&self, key: &str, data: &serde_json::Value) -> Result<(), String> {
        let json_str = serde_json::to_string(data)
            .map_err(|e| format!("Failed to serialize data for {}: {}", key, e))?;
        
        let mut data_guard = self.data.lock().await;
        data_guard.insert(key.to_string(), json_str);
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<(), String> {
        let mut data_guard = self.data.lock().await;
        data_guard.remove(key);
        Ok(())
    }
    
    async fn exists(&self, key: &str) -> bool {
        let data_guard = self.data.lock().await;
        data_guard.contains_key(key)
    }
}


