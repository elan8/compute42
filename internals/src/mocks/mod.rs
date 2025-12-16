//! Mock implementations for testing the internals library
//! 
//! This module provides comprehensive mock implementations of all service traits
//! for testing purposes. The mocks are organized into logical groups:
//! 
//! - **core**: Core event and command mocks (EventEmitter, EventListener, CommandHandler)
//! - **services**: Service-related mocks (Logging, Configuration, State, Installation, Sysimage)
//! - **process**: Process and execution mocks (Process, Communication, Execution)
//! - **features**: Feature-specific mocks (Plot, File, LSP)

pub mod core;
pub mod services;
pub mod process;
pub mod features;

// Re-export all mock types for convenience
pub use core::*;
pub use services::*;
pub use process::*;
pub use features::*;

use std::sync::Arc;

/// Create a complete set of mock services for testing
#[allow(clippy::type_complexity)]
pub fn create_mock_services() -> (
    Arc<MockEventEmitter>,
    Arc<MockLoggingService>,
    Arc<MockConfigurationService>,
    Arc<MockInstallationService>,
    // Arc<MockSysimageService>,
    Arc<MockProcessService>,
    Arc<MockCommunicationService>,
    Arc<MockPlotService>,
    Arc<MockFileService>,
    Arc<MockLspService>,
) {
    (
        Arc::new(MockEventEmitter::new()),
        Arc::new(MockLoggingService::new()),
        Arc::new(MockConfigurationService::new()),
        Arc::new(MockInstallationService::new()),
       // Arc::new(MockSysimageService::new()),
        Arc::new(MockProcessService::new()),
        Arc::new(MockCommunicationService::new()),
        Arc::new(MockPlotService::new()),
        Arc::new(MockFileService::new()),
        Arc::new(MockLspService::new()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service_traits::{EventEmitter, LoggingService, ProcessService, ConfigurationService, LogLevel};

    #[tokio::test]
    async fn test_mock_event_emitter() {
        let emitter = MockEventEmitter::new();
        
        emitter.emit("test_event", serde_json::json!({"key": "value"})).await.unwrap();
        
        let events = emitter.get_emitted_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "test_event");
        assert_eq!(events[0].1["key"], "value");
    }

    #[tokio::test]
    async fn test_mock_logging_service() {
        let logger = MockLoggingService::new();
        
        logger.log(LogLevel::Info, "test message").await.unwrap();
        
        let logs = logger.get_logs().await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].0, LogLevel::Info);
        assert_eq!(logs[0].1, "test message");
    }

    #[tokio::test]
    async fn test_mock_process_service() {
        let process = MockProcessService::new();
        
        assert!(!process.is_julia_running().await);
        
        process.start_julia_process().await.unwrap();
        assert!(process.is_julia_running().await);
        
        process.stop_julia_process().await.unwrap();
        assert!(!process.is_julia_running().await);
    }

    #[tokio::test]
    async fn test_mock_services_factory() {
        let services = create_mock_services();
        
        // Test that all services are created successfully
        assert_eq!(services.1.get_logs().await.len(), 0);
        assert_eq!(services.2.load_config().await.unwrap().last_opened_folder, None);
    }
}
