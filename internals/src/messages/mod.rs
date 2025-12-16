// Actor messages module
// This module organizes all actor communication messages by domain

pub mod orchestrator;
pub mod configuration;
pub mod state;
pub mod execution;
pub mod communication;
pub mod process;
pub mod lsp;
pub mod plot;
pub mod file_server;
pub mod installation;
pub mod coordination;
pub mod filesystem;

// Re-export commonly used types for convenience
pub use execution::ExecutionType;
pub use plot::PlotData;
pub use communication::{
    JuliaMessage, SessionStatus, ErrorInfo, StreamOutput, StreamType, MessageHandler
};