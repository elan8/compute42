// Base service trait for all services
// This provides common functionality and ensures consistent service behavior

use async_trait::async_trait;
use std::fmt::Debug;

/// Base trait for all services
/// This ensures consistent behavior across all service implementations
#[async_trait]
pub trait BaseService: Send + Sync + Debug {
    /// Service name for identification and logging
    fn service_name(&self) -> &'static str;
    
    /// Initialize the service
    async fn initialize(&self) -> Result<(), String>;
    
    /// Check if the service is healthy
    async fn health_check(&self) -> Result<bool, String>;
    
    /// Shutdown the service gracefully
    async fn shutdown(&self) -> Result<(), String>;
}

/// Service error types
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Service initialization failed: {0}")]
    InitializationFailed(String),
    
    #[error("Service operation failed: {0}")]
    OperationFailed(String),
    
    #[error("Service not available: {0}")]
    NotAvailable(String),
    
    #[error("Service timeout: {0}")]
    Timeout(String),
}

/// Service result type
pub type ServiceResult<T> = Result<T, ServiceError>;

