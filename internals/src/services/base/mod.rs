// Base service traits and adapters
// This module provides the foundation for all service implementations

pub mod service_trait;
pub mod service_adapter;
pub mod file_utils;
pub mod logging;
pub mod error_handling;
pub mod variable_utils;

// Re-export specific items to avoid conflicts
pub use service_trait::{BaseService};
pub use service_adapter::*;
pub use file_utils::*;
pub use logging::*;
pub use error_handling::{
    ServiceLogger, ServiceError, ServiceResult, ServiceErrorHandler, 
    ServiceErrorType, ErrorHandler
};
pub use variable_utils::{clean_array_string, process_variable_data, process_variables_map};

