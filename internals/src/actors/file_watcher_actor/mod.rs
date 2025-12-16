use actix::prelude::*;
use log::{debug, error, info};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::messages::filesystem::{StartFileWatcher, StopFileWatcher, StopAllFileWatchers};
use crate::services::events::EventService;

/// File change event that will be sent to the frontend
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileChangeEvent {
    pub path: String,
    pub change_type: FileChangeType,
    pub timestamp: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum FileChangeType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// File watcher actor that manages file system watching
pub struct FileWatcherActor {
    watchers: HashMap<String, RecommendedWatcher>,
    _event_service: Arc<EventService>,
    event_tx: mpsc::UnboundedSender<FileChangeEvent>,
}

impl FileWatcherActor {
    pub fn new(event_service: Arc<EventService>) -> Self {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<FileChangeEvent>();
        
        // Spawn a task to handle file change events
        let event_service_clone = event_service.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                debug!("File change event received: {:?}", event);
                
                // Emit the event to the frontend
                let payload = serde_json::to_value(&event)
                    .unwrap_or_else(|e| {
                        error!("Failed to serialize file change event: {}", e);
                        serde_json::Value::Null
                    });
                
                if let Err(e) = event_service_clone.emit("file:changed", payload).await {
                    error!("Failed to emit file change event: {}", e);
                }
            }
        });
        
        Self {
            watchers: HashMap::new(),
            _event_service: event_service,
            event_tx,
        }
    }
}

impl Actor for FileWatcherActor {
    type Context = Context<Self>;
}

impl Handler<StartFileWatcher> for FileWatcherActor {
    type Result = Result<String, String>;
    
    fn handle(&mut self, msg: StartFileWatcher, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Starting file watcher for path: {}", msg.path);
        
        // Check if path exists
        if !Path::new(&msg.path).exists() {
            return Err(format!("Path does not exist: {}", msg.path));
        }
        
        // Generate a unique watcher ID
        let watcher_id = Uuid::new_v4().to_string();
        
        // Create the watcher
        let (tx, mut rx) = mpsc::unbounded_channel::<Result<Event, notify::Error>>();
        
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                if let Err(e) = tx.send(res) {
                    error!("Failed to send file event: {}", e);
                }
            },
            notify::Config::default(),
        ).map_err(|e| format!("Failed to create file watcher: {}", e))?;
        
        // Watch the path
        let recursive_mode = if msg.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        
        watcher.watch(Path::new(&msg.path), recursive_mode)
            .map_err(|e| format!("Failed to watch path: {}", e))?;
        
        // Store the watcher
        self.watchers.insert(watcher_id.clone(), watcher);
        
        // Spawn a task to handle events from this watcher
        let event_tx = self.event_tx.clone();
        let _watched_path = msg.path.clone();
        tokio::spawn(async move {
            while let Some(event_result) = rx.recv().await {
                match event_result {
                    Ok(event) => {
                        debug!("File system event: {:?}", event);
                        
                        for path in event.paths {
                            let change_type = match event.kind {
                                EventKind::Create(_) => FileChangeType::Created,
                                EventKind::Modify(_) => FileChangeType::Modified,
                                EventKind::Remove(_) => FileChangeType::Deleted,
                                EventKind::Other => continue, // Skip unknown events
                                _ => continue, // Skip other event types for now
                            };
                            
                            let file_event = FileChangeEvent {
                                path: path.to_string_lossy().to_string(),
                                change_type,
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs(),
                            };
                            
                            if let Err(e) = event_tx.send(file_event) {
                                error!("Failed to send file change event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("File watcher error: {}", e);
                    }
                }
            }
        });
        
        info!("File watcher started for path: {} (ID: {})", msg.path, watcher_id);
        Ok(watcher_id)
    }
}

impl Handler<StopFileWatcher> for FileWatcherActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: StopFileWatcher, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Stopping file watcher: {}", msg.watcher_id);
        
        if let Some(_watcher) = self.watchers.remove(&msg.watcher_id) {
            // The watcher will be dropped and automatically stop watching
            info!("File watcher stopped: {}", msg.watcher_id);
            Ok(())
        } else {
            Err(format!("Watcher not found: {}", msg.watcher_id))
        }
    }
}

impl Handler<StopAllFileWatchers> for FileWatcherActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: StopAllFileWatchers, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Stopping all file watchers");
        
        let count = self.watchers.len();
        self.watchers.clear();
        
        info!("Stopped {} file watchers", count);
        Ok(())
    }
}
