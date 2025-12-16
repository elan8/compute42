// Logging service for centralized logging
// This provides consistent logging across all services

use log::{Level, LevelFilter, Metadata, Record};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::services::base::BaseService;

/// Log entry structure
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: Level,
    pub target: String,
    pub message: String,
    pub timestamp: std::time::SystemTime,
}

/// Logging service implementation
#[derive(Debug)]
pub struct LoggingService {
    logs: Arc<Mutex<Vec<LogEntry>>>,
    max_logs: usize,
}

impl LoggingService {
    /// Create a new LoggingService instance
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
            max_logs,
        }
    }
    
    /// Get all stored logs
    pub async fn get_logs(&self) -> Vec<LogEntry> {
        let logs = self.logs.lock().await;
        logs.clone()
    }
    
    /// Clear all stored logs
    pub async fn clear_logs(&self) -> Result<(), String> {
        let mut logs = self.logs.lock().await;
        logs.clear();
        Ok(())
    }
    
    /// Get logs by level
    pub async fn get_logs_by_level(&self, level: Level) -> Vec<LogEntry> {
        let logs = self.logs.lock().await;
        logs.iter()
            .filter(|entry| entry.level == level)
            .cloned()
            .collect()
    }
}

impl Default for LoggingService {
    fn default() -> Self {
        Self::new(1000) // Default to 1000 log entries
    }
}

#[async_trait::async_trait]
impl BaseService for LoggingService {
    fn service_name(&self) -> &'static str {
        "LoggingService"
    }
    
    async fn initialize(&self) -> Result<(), String> {
        // Initialize logging system
        log::set_max_level(LevelFilter::Debug);
        Ok(())
    }
    
    async fn health_check(&self) -> Result<bool, String> {
        // Logging service is always healthy
        Ok(true)
    }
    
    async fn shutdown(&self) -> Result<(), String> {
        // Flush any pending logs
        Ok(())
    }
}

/// Custom logger implementation
pub struct CustomLogger {
    service: Arc<LoggingService>,
}

impl CustomLogger {
    /// Create a new CustomLogger instance
    pub fn new(service: Arc<LoggingService>) -> Self {
        Self { service }
    }
}

impl log::Log for CustomLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let entry = LogEntry {
                level: record.level(),
                target: record.target().to_string(),
                message: record.args().to_string(),
                timestamp: std::time::SystemTime::now(),
            };
            
            // Store log entry asynchronously
            let service = self.service.clone();
            let entry_clone = entry.clone();
            tokio::spawn(async move {
                let mut logs = service.logs.lock().await;
                logs.push(entry_clone);
                
                // Maintain max log count
                if logs.len() > service.max_logs {
                    logs.remove(0);
                }
            });
            
            // Also output to console for immediate visibility
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
            println!("[{}] {} {}: {}", timestamp, record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {
        // Flush any pending operations
    }
}
