// LSP utility functions for the actor

/// LSP utility functions
#[derive(Clone)]
pub struct LspUtils;

impl Default for LspUtils {
    fn default() -> Self {
        Self::new()
    }
}

impl LspUtils {
    pub fn new() -> Self {
        Self
    }

    /// Convert a file URI (file://) to a local filesystem path string when needed
    /// If input is already a path, returns it unchanged
    pub fn uri_to_path(&self, input: &str) -> String {
        if input.starts_with("file://") {
            if let Ok(url) = url::Url::parse(input) {
                if let Ok(path_buf) = url.to_file_path() {
                    return path_buf.to_string_lossy().to_string();
                }
            }
        }
        input.to_string()
    }

    /// Ensure we produce a canonical file URI (file:///C:/...) regardless of input
    #[allow(dead_code)]
    pub fn ensure_file_uri(&self, input: &str) -> String {
        // First, convert any existing file URI to a local path
        let path = self.uri_to_path(input);
        // Then, build a proper file URI from the path
        if let Ok(url) = url::Url::from_file_path(&path) {
            url.to_string()
        } else {
            // Fallback to original input
            input.to_string()
        }
    }
}

