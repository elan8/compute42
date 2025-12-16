use crate::pipeline::types::{SourceItem, FileMetadata};
use crate::types::LspError;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Source that discovers Base, Core, and stdlib files from Julia installation
pub struct BaseSource {
    julia_base_dir: PathBuf,
}

impl BaseSource {
    pub fn new(julia_executable_path: &Path) -> Result<Self, LspError> {
        let julia_base_dir = julia_executable_path
            .parent()
            .and_then(|p| p.parent())
            .ok_or_else(|| LspError::InternalError(
                "Failed to determine Julia installation directory from executable path".to_string()
            ))?;

        Ok(Self {
            julia_base_dir: julia_base_dir.to_path_buf(),
        })
    }

    /// Get the path to exports.jl file
    pub fn get_exports_path(&self) -> Option<PathBuf> {
        let base_dir = self.julia_base_dir.join("share").join("julia").join("base");
        let base_dir = if base_dir.exists() {
            base_dir
        } else {
            self.julia_base_dir.join("base")
        };

        let exports_path = base_dir.join("exports.jl");
        if exports_path.exists() {
            Some(exports_path)
        } else {
            None
        }
    }
    
    /// Get the path to basedocs.jl file
    pub fn get_basedocs_path(&self) -> Option<PathBuf> {
        let docs_dir = self.julia_base_dir.join("share").join("julia").join("base").join("docs");
        let docs_dir = if docs_dir.exists() {
            docs_dir
        } else {
            // Try alternative location
            let alt_docs_dir = self.julia_base_dir.join("base").join("docs");
            if alt_docs_dir.exists() {
                alt_docs_dir
            } else {
                return None;
            }
        };

        let basedocs_path = docs_dir.join("basedocs.jl");
        if basedocs_path.exists() {
            Some(basedocs_path)
        } else {
            None
        }
    }

    /// Discover Base files
    pub fn discover_base(&self) -> Result<Vec<SourceItem>, LspError> {
        let base_dir = self.julia_base_dir.join("share").join("julia").join("base");
        let base_dir = if base_dir.exists() {
            base_dir
        } else {
            self.julia_base_dir.join("base")
        };

        if !base_dir.exists() {
            return Ok(Vec::new());
        }

        Self::discover_directory(&base_dir, "Base")
    }

    /// Discover stdlib files
    pub fn discover_stdlib(&self) -> Result<Vec<SourceItem>, LspError> {
        let stdlib_dir = self.julia_base_dir.join("share").join("julia").join("stdlib");
        let stdlib_dir = if stdlib_dir.exists() {
            stdlib_dir
        } else {
            self.julia_base_dir.join("stdlib")
        };

        if !stdlib_dir.exists() {
            return Ok(Vec::new());
        }

        let mut all_items = Vec::new();

        for entry in WalkDir::new(&stdlib_dir)
            .max_depth(2)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.')
            })
        {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("Error walking stdlib directory: {}", e);
                    continue;
                }
            };

            if entry.file_type().is_dir() {
                let module_src = entry.path().join("src");
                if module_src.exists() {
                    match Self::discover_directory(&module_src, &entry.file_name().to_string_lossy()) {
                        Ok(mut items) => all_items.append(&mut items),
                        Err(e) => {
                            log::warn!("Failed to discover stdlib module {:?}: {}", entry.path(), e);
                        }
                    }
                }
            }
        }

        Ok(all_items)
    }

    fn discover_directory(dir: &Path, _module_name: &str) -> Result<Vec<SourceItem>, LspError> {
        let mut items = Vec::new();

        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.')
            })
        {
            let entry = entry.map_err(|e| {
                LspError::InternalError(format!("Failed to walk directory: {}", e))
            })?;

            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "jl" {
                        match Self::load_file(entry.path()) {
                            Ok(item) => items.push(item),
                            Err(e) => {
                                log::warn!("Failed to load file {:?}: {}", entry.path(), e);
                            }
                        }
                    }
                }
            }
        }

        Ok(items)
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

    fn create_test_julia_installation() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let julia_dir = temp_dir.path().join("julia");
        let julia_exe = julia_dir.join("bin").join("julia.exe");

        // Create Julia installation structure
        let base_dir = julia_dir.join("share").join("julia").join("base");
        fs::create_dir_all(&base_dir).unwrap();

        // Create a test Base file
        fs::write(
            base_dir.join("array.jl"),
            "module Base\nfunction length(x) end\nend",
        )
        .unwrap();

        // Create stdlib structure
        let stdlib_dir = julia_dir.join("share").join("julia").join("stdlib");
        let testlib_dir = stdlib_dir.join("Test").join("src");
        fs::create_dir_all(&testlib_dir).unwrap();
        fs::write(
            testlib_dir.join("Test.jl"),
            "module Test\nfunction @test() end\nend",
        )
        .unwrap();

        // Create bin directory and executable
        fs::create_dir_all(julia_dir.join("bin")).unwrap();
        fs::write(&julia_exe, "#!/bin/bash\necho julia").unwrap();

        (temp_dir, julia_exe)
    }

    #[test]
    fn test_new_with_valid_path() {
        let (_temp_dir, julia_exe) = create_test_julia_installation();
        let source = BaseSource::new(&julia_exe);
        assert!(source.is_ok());
    }

    #[test]
    fn test_new_with_invalid_path() {
        let invalid_path = PathBuf::from("/nonexistent/path/julia");
        let source = BaseSource::new(&invalid_path);
        // Should handle gracefully - might succeed if path structure is valid
        // or fail if it can't determine base directory
        let _ = source; // Just check it doesn't panic
    }

    #[test]
    fn test_discover_base() {
        let (_temp_dir, julia_exe) = create_test_julia_installation();
        let source = BaseSource::new(&julia_exe).unwrap();
        let items = source.discover_base().unwrap();

        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.path.ends_with("array.jl")));
    }

    #[test]
    fn test_discover_base_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let julia_dir = temp_dir.path().join("julia");
        let julia_exe = julia_dir.join("bin").join("julia.exe");
        fs::create_dir_all(julia_dir.join("bin")).unwrap();
        fs::write(&julia_exe, "#!/bin/bash").unwrap();

        let source = BaseSource::new(&julia_exe).unwrap();
        let items = source.discover_base().unwrap();

        // Should return empty list, not error
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_discover_stdlib() {
        let (_temp_dir, julia_exe) = create_test_julia_installation();
        let source = BaseSource::new(&julia_exe).unwrap();
        let items = source.discover_stdlib().unwrap();

        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.path.ends_with("Test.jl")));
    }

    #[test]
    fn test_discover_stdlib_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let julia_dir = temp_dir.path().join("julia");
        let julia_exe = julia_dir.join("bin").join("julia.exe");
        fs::create_dir_all(julia_dir.join("bin")).unwrap();
        fs::write(&julia_exe, "#!/bin/bash").unwrap();

        let source = BaseSource::new(&julia_exe).unwrap();
        let items = source.discover_stdlib().unwrap();

        // Should return empty list, not error
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_discover_skips_hidden_files() {
        let (_temp_dir, julia_exe) = create_test_julia_installation();
        let base_dir = julia_exe
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("share")
            .join("julia")
            .join("base");

        // Add a hidden file
        fs::write(base_dir.join(".hidden.jl"), "hidden content").unwrap();

        let source = BaseSource::new(&julia_exe).unwrap();
        let items = source.discover_base().unwrap();

        // Should skip hidden files
        assert!(items.iter().all(|i| !i.path.file_name().unwrap().to_string_lossy().starts_with('.')));
    }
}

