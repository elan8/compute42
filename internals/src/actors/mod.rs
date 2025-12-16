// Actors module - contains all business logic actors
// This module organizes all actors in a clean, hierarchical structure

pub mod communication_actor;
pub mod configuration_actor;
pub mod execution_actor;
pub mod file_server_actor;
pub mod filesystem_actor;
pub mod file_watcher_actor;
pub mod installation_actor;
pub mod lsp_actor;
pub mod orchestrator_actor;
pub mod plot_actor;
pub mod process_actor;
pub mod project_actor;
pub mod state_actor;

// Re-export all actors for convenience
pub use communication_actor::*;
pub use configuration_actor::*;
pub use execution_actor::*;
pub use file_server_actor::*;
pub use filesystem_actor::*;
pub use file_watcher_actor::*;
pub use installation_actor::*;
pub use lsp_actor::*;
pub use orchestrator_actor::*;
pub use plot_actor::*;
pub use process_actor::*;
pub use project_actor::*;
pub use state_actor::*;
