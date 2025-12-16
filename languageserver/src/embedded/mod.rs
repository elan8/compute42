pub mod service;
// Settings facade for toggles (headless; UI owns persistence)
#[derive(Debug, Clone, Default)]
pub struct EmbeddedSettings {
    pub enhanced_hover: bool,
    pub augment_with_julia: bool,
}

pub use service::{EmbeddedLspService, LspConfig};
