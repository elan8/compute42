use crate::pipeline::types::{SourceItem, FileMetadata};
use crate::types::LspError;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Source that discovers Julia files in a workspace
pub struct WorkspaceSource {
    root_path: PathBuf,
}

impl WorkspaceSource {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }

    /// Discover all Julia files in the workspace
    pub fn discover(&self) -> Result<Vec<SourceItem>, LspError> {
        let mut items = Vec::new();

        for entry in WalkDir::new(&self.root_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !Self::should_skip_directory(e.path()))
        {
            let entry = entry.map_err(|e| {
                LspError::InternalError(format!("Failed to walk directory: {}", e))
            })?;

            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "jl" {
                        let path = entry.path().to_path_buf();
                        match Self::load_file(&path) {
                            Ok(item) => items.push(item),
                            Err(e) => {
                                log::warn!("Failed to load file {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(items)
    }

    fn should_skip_directory(path: &Path) -> bool {
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            matches!(
                name_str.as_ref(),
                ".git" | "node_modules" | "target" | ".vscode" | ".idea" | "__pycache__"
            )
        } else {
            false
        }
    }

    fn load_file(path: &Path) -> Result<SourceItem, LspError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| LspError::InternalError(format!("Failed to read file: {}", e)))?;

        let metadata = std::fs::metadata(path)
            .map_err(|e| LspError::InternalError(format!("Failed to get file metadata: {}", e)))?;

        let last_modified = metadata
            .modified()
            .map_err(|e| LspError::InternalError(format!("Failed to get modified time: {}", e)))?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(SourceItem {
            path: path.to_path_buf(),
            content,
            metadata: FileMetadata::new(last_modified, metadata.len()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_workspace() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();

        let src_dir = workspace_path.join("src");
        fs::create_dir(&src_dir).unwrap();

        fs::write(
            src_dir.join("main.jl"),
            "function main() println(\"Hello\") end",
        )
        .unwrap();

        fs::write(
            src_dir.join("utils.jl"),
            "function helper(x) return x * 2 end",
        )
        .unwrap();

        (temp_dir, workspace_path)
    }

    #[test]
    fn test_discover_julia_files() {
        let (_temp_dir, workspace_path) = create_test_workspace();
        let source = WorkspaceSource::new(workspace_path);
        let items = source.discover().unwrap();

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|i| i.path.file_name().unwrap() == "main.jl"));
        assert!(items.iter().any(|i| i.path.file_name().unwrap() == "utils.jl"));
    }

    #[test]
    fn test_skip_directories() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();

        fs::create_dir(workspace_path.join(".git")).unwrap();
        fs::write(workspace_path.join(".git").join("config"), "test").unwrap();

        let source = WorkspaceSource::new(workspace_path);
        let items = source.discover().unwrap();

        // Should not find files in .git directory
        assert_eq!(items.len(), 0);
    }
}


















