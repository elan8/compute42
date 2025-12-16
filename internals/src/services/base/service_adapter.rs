// Service adapter for actor-service communication
// This provides a clean interface between actors and service implementations

use std::sync::Arc;
use async_trait::async_trait;
use crate::services::base::{BaseService, ServiceResult, ServiceError};

/// Service adapter trait for actor-service communication
/// This provides a standardized way for actors to interact with services
#[async_trait]
pub trait ServiceAdapter<S: BaseService>: Send + Sync {
    /// Get a reference to the underlying service
    fn service(&self) -> &Arc<S>;
    
    /// Get a mutable reference to the underlying service
    fn service_mut(&mut self) -> &mut Arc<S>;
    
    /// Check if the service is available
    async fn is_available(&self) -> bool {
        self.service().health_check().await.unwrap_or(false)
    }
    
    /// Execute a service operation with error handling
    async fn execute_operation<F, R>(&self, operation: F) -> ServiceResult<R>
    where
        F: FnOnce(&S) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, String>> + Send>> + Send + Sync,
        R: Send + Sync,
    {
        let service = self.service();
        match operation(service.as_ref()).await {
            Ok(result) => Ok(result),
            Err(e) => Err(ServiceError::Internal(e)),
        }
    }
}

/// Generic service adapter implementation
pub struct GenericServiceAdapter<S: BaseService> {
    service: Arc<S>,
}

impl<S: BaseService> GenericServiceAdapter<S> {
    /// Create a new service adapter
    pub fn new(service: Arc<S>) -> Self {
        Self { service }
    }
    
    /// Get the service name
    pub fn service_name(&self) -> &'static str {
        self.service.service_name()
    }
}

#[async_trait]
impl<S: BaseService> ServiceAdapter<S> for GenericServiceAdapter<S> {
    fn service(&self) -> &Arc<S> {
        &self.service
    }
    
    fn service_mut(&mut self) -> &mut Arc<S> {
        &mut self.service
    }
}

