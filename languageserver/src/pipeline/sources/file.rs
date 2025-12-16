use crate::pipeline::types::{SourceItem, FileMetadata};
use crate::types::LspError;
use std::path::PathBuf;

/// Source for a single file
pub struct FileSource {
    path: PathBuf,
}

impl FileSource {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Load the file as a source item
    pub fn load(&self) -> Result<SourceItem, LspError> {
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| LspError::InternalError(format!("Failed to read file: {}", e)))?;

        let metadata = std::fs::metadata(&self.path)
            .map_err(|e| LspError::InternalError(format!("Failed to get file metadata: {}", e)))?;

        let last_modified = metadata
            .modified()
            .map_err(|e| LspError::InternalError(format!("Failed to get modified time: {}", e)))?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(SourceItem {
            path: self.path.clone(),
            content,
            metadata: FileMetadata::new(last_modified, metadata.len()),
        })
    }

    /// Create a source item from content (for in-memory files)
    pub fn from_content(path: PathBuf, content: String) -> SourceItem {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        SourceItem {
            path,
            content,
            metadata: FileMetadata::new(now, 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.jl");
        fs::write(&file_path, "function test() end").unwrap();

        let source = FileSource::new(file_path.clone());
        let item = source.load().unwrap();

        assert_eq!(item.path, file_path);
        assert_eq!(item.content, "function test() end");
        assert!(item.metadata.size > 0);
    }

    #[test]
    fn test_from_content() {
        let path = PathBuf::from("test.jl");
        let content = "function test() end".to_string();

        let item = FileSource::from_content(path.clone(), content.clone());

        assert_eq!(item.path, path);
        assert_eq!(item.content, content);
    }
}


















