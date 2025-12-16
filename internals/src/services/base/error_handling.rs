// Standardized error handling and logging for services
// This module provides consistent error handling patterns across all services

use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Standardized error types for services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceError {
    /// Configuration error
    Configuration(String),
    /// Network/communication error
    Network(String),
    /// Authentication/authorization error
    Authentication(String),
    /// Resource not found
    NotFound(String),
    /// Invalid input/parameters
    InvalidInput(String),
    /// Internal service error
    Internal(String),
    /// External service error (e.g., API errors)
    External(String),
    /// Timeout error
    Timeout(String),
    /// Resource already exists
    AlreadyExists(String),
    /// Service unavailable
    Unavailable(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            ServiceError::Network(msg) => write!(f, "Network error: {}", msg),
            ServiceError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            ServiceError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ServiceError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            ServiceError::Internal(msg) => write!(f, "Internal error: {}", msg),
            ServiceError::External(msg) => write!(f, "External service error: {}", msg),
            ServiceError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            ServiceError::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            ServiceError::Unavailable(msg) => write!(f, "Service unavailable: {}", msg),
        }
    }
}

impl std::error::Error for ServiceError {}

/// Standardized logging macros for services
#[derive(Clone)]
pub struct ServiceLogger {
    service_name: &'static str,
}

impl ServiceLogger {
    /// Create a new service logger
    pub fn new(service_name: &'static str) -> Self {
        Self { service_name }
    }

    /// Log debug message with service prefix
    pub fn debug(&self, message: &str) {
        debug!("[{}] {}", self.service_name, message);
    }

    /// Log debug message with service prefix and context
    pub fn debug_with_context(&self, message: &str, context: &str) {
        debug!("[{}] {} - Context: {}", self.service_name, message, context);
    }

    /// Log info message with service prefix
    pub fn info(&self, message: &str) {
        info!("[{}] {}", self.service_name, message);
    }

    /// Log info message with service prefix and context
    pub fn info_with_context(&self, message: &str, context: &str) {
        info!("[{}] {} - Context: {}", self.service_name, message, context);
    }

    /// Log warning message with service prefix
    pub fn warn(&self, message: &str) {
        warn!("[{}] {}", self.service_name, message);
    }

    /// Log warning message with service prefix and context
    pub fn warn_with_context(&self, message: &str, context: &str) {
        warn!("[{}] {} - Context: {}", self.service_name, message, context);
    }

    /// Log error message with service prefix
    pub fn error(&self, message: &str) {
        error!("[{}] {}", self.service_name, message);
    }

    /// Log error message with service prefix and context
    pub fn error_with_context(&self, message: &str, context: &str) {
        error!("[{}] {} - Context: {}", self.service_name, message, context);
    }

    /// Log error with service prefix and error details
    pub fn error_with_details(&self, message: &str, error: &dyn std::error::Error) {
        error!("[{}] {} - Error: {}", self.service_name, message, error);
    }

    /// Log error with service prefix, context, and error details
    pub fn error_with_context_and_details(&self, message: &str, context: &str, error: &dyn std::error::Error) {
        error!("[{}] {} - Context: {} - Error: {}", self.service_name, message, context, error);
    }
}

/// Standardized result type for services
pub type ServiceResult<T> = Result<T, ServiceError>;

/// Helper functions for common error handling patterns
pub struct ErrorHandler;

impl ErrorHandler {
    /// Convert a generic error to a ServiceError
    pub fn from_error<E: std::error::Error>(error: E, error_type: ServiceErrorType) -> ServiceError {
        match error_type {
            ServiceErrorType::Configuration => ServiceError::Configuration(error.to_string()),
            ServiceErrorType::Network => ServiceError::Network(error.to_string()),
            ServiceErrorType::Authentication => ServiceError::Authentication(error.to_string()),
            ServiceErrorType::NotFound => ServiceError::NotFound(error.to_string()),
            ServiceErrorType::InvalidInput => ServiceError::InvalidInput(error.to_string()),
            ServiceErrorType::Internal => ServiceError::Internal(error.to_string()),
            ServiceErrorType::External => ServiceError::External(error.to_string()),
            ServiceErrorType::Timeout => ServiceError::Timeout(error.to_string()),
            ServiceErrorType::AlreadyExists => ServiceError::AlreadyExists(error.to_string()),
            ServiceErrorType::Unavailable => ServiceError::Unavailable(error.to_string()),
        }
    }

    /// Log and convert error
    pub fn log_and_convert<E: std::error::Error>(
        logger: &ServiceLogger,
        error: E,
        error_type: ServiceErrorType,
        context: &str,
    ) -> ServiceError {
        let service_error = Self::from_error(error, error_type);
        logger.error_with_context_and_details(
            "Operation failed",
            context,
            &service_error,
        );
        service_error
    }

    /// Handle operation with logging
    pub async fn handle_operation<F, T>(
        logger: &ServiceLogger,
        operation_name: &str,
        operation: F,
    ) -> ServiceResult<T>
    where
        F: std::future::Future<Output = Result<T, String>>,
    {
        logger.debug(&format!("Starting operation: {}", operation_name));
        
        match operation.await {
            Ok(result) => {
                logger.debug(&format!("Operation completed successfully: {}", operation_name));
                Ok(result)
            }
            Err(error) => {
                let service_error = ServiceError::Internal(error);
                logger.error_with_context_and_details(
                    "Operation failed",
                    operation_name,
                    &service_error,
                );
                Err(service_error)
            }
        }
    }

    /// Handle operation with custom error type
    pub async fn handle_operation_with_error_type<F, T>(
        logger: &ServiceLogger,
        operation_name: &str,
        error_type: ServiceErrorType,
        operation: F,
    ) -> ServiceResult<T>
    where
        F: std::future::Future<Output = Result<T, String>>,
    {
        logger.debug(&format!("Starting operation: {}", operation_name));
        
        match operation.await {
            Ok(result) => {
                logger.debug(&format!("Operation completed successfully: {}", operation_name));
                Ok(result)
            }
            Err(error) => {
                let service_error = Self::from_error(
                    std::io::Error::other(error),
                    error_type,
                );
                logger.error_with_context_and_details(
                    "Operation failed",
                    operation_name,
                    &service_error,
                );
                Err(service_error)
            }
        }
    }
}

/// Error type classification for better error handling
#[derive(Debug, Clone, Copy)]
pub enum ServiceErrorType {
    Configuration,
    Network,
    Authentication,
    NotFound,
    InvalidInput,
    Internal,
    External,
    Timeout,
    AlreadyExists,
    Unavailable,
}

/// Trait for services to implement standardized error handling
pub trait ServiceErrorHandler {
    /// Get the service logger
    fn logger(&self) -> &ServiceLogger;

    /// Handle a generic error with logging
    fn handle_error(&self, error: String, context: &str) -> ServiceError {
        let service_error = ServiceError::Internal(error);
        self.logger().error_with_context_and_details(
            "Service operation failed",
            context,
            &service_error,
        );
        service_error
    }

    /// Handle a network error with logging
    fn handle_network_error(&self, error: String, context: &str) -> ServiceError {
        let service_error = ServiceError::Network(error);
        self.logger().error_with_context_and_details(
            "Network operation failed",
            context,
            &service_error,
        );
        service_error
    }

    /// Handle an authentication error with logging
    fn handle_auth_error(&self, error: String, context: &str) -> ServiceError {
        let service_error = ServiceError::Authentication(error);
        self.logger().error_with_context_and_details(
            "Authentication failed",
            context,
            &service_error,
        );
        service_error
    }

    /// Handle a configuration error with logging
    fn handle_config_error(&self, error: String, context: &str) -> ServiceError {
        let service_error = ServiceError::Configuration(error);
        self.logger().error_with_context_and_details(
            "Configuration error",
            context,
            &service_error,
        );
        service_error
    }

    /// Handle an external service error with logging
    fn handle_external_error(&self, error: String, context: &str) -> ServiceError {
        let service_error = ServiceError::External(error);
        self.logger().error_with_context_and_details(
            "External service error",
            context,
            &service_error,
        );
        service_error
    }
}

/// Macro for consistent service logging
#[macro_export]
macro_rules! service_log {
    ($logger:expr, $level:ident, $msg:expr) => {
        $logger.$level($msg);
    };
    ($logger:expr, $level:ident, $msg:expr, $context:expr) => {
        $logger.$level($msg, $context);
    };
}

/// Macro for consistent error handling
#[macro_export]
macro_rules! service_error {
    ($logger:expr, $msg:expr, $error:expr) => {
        $logger.error_with_details($msg, $error);
    };
    ($logger:expr, $msg:expr, $context:expr, $error:expr) => {
        $logger.error_with_context_and_details($msg, $context, $error);
    };
}
