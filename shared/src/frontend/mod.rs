pub mod plots;
pub mod lsp;
pub mod orchestrator;
pub mod notebook;

// Re-export for stable paths like shared::frontend::PlotData
pub use plots::*;
pub use lsp::*;
pub use orchestrator::*;
pub use notebook::*;


