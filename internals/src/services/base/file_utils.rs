// File utilities service
// This provides file system operations and path conversions

use futures::future::BoxFuture;
use futures::FutureExt; // For .boxed()
use log::error;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs as async_fs;
use crate::services::base::BaseService;

/// FileUtils service - provides file utility functions
/// This service handles file system operations and path conversions
#[derive(Debug)]
pub struct FileUtilsService;

impl FileUtilsService {
    /// Create a new FileUtilsService instance
    pub fn new() -> Self {
        Self
    }
    
    /// Convert a Windows path to Julia-compatible format
    pub fn convert_path_for_julia(&self, path: &str) -> String {
        convert_path_for_julia(path)
    }
    
    /// Build a file tree from a path
    pub fn build_tree<'a>(&self, path: &'a Path) -> BoxFuture<'a, Result<FileNode, String>> {
        build_tree(path)
    }
    
    /// Load directory contents for lazy loading
    pub fn load_directory_contents<'a>(&self, path: &'a Path) -> BoxFuture<'a, Result<Vec<FileNode>, String>> {
        load_directory_contents(path)
    }
}

impl Default for FileUtilsService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl BaseService for FileUtilsService {
    fn service_name(&self) -> &'static str {
        "FileUtilsService"
    }
    
    async fn initialize(&self) -> Result<(), String> {
        // File utils service doesn't need initialization
        Ok(())
    }
    
    async fn health_check(&self) -> Result<bool, String> {
        // File utils service is always healthy
        Ok(true)
    }
    
    async fn shutdown(&self) -> Result<(), String> {
        // File utils service doesn't need shutdown
        Ok(())
    }
}

/// Convert a Windows path to Julia-compatible format
/// Julia expects forward slashes and proper drive letter format
pub fn convert_path_for_julia(path: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        // Convert backslashes to forward slashes
        let normalized = path.replace('\\', "/");

        // Handle drive letter format for Julia
        // Julia expects C:/path format, not C:\path
        if normalized.len() >= 2 && normalized.chars().nth(1) == Some(':') {
            // Already in correct format, just ensure forward slashes
            normalized
        } else {
            // Fallback to simple replacement
            normalized
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On non-Windows systems, just return as-is
        path.to_string()
    }
}

// Data structure for file tree nodes
#[derive(Serialize, Deserialize, Debug)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub children: Option<Vec<FileNode>>,
}

// Use BoxFuture for recursive async function
pub fn build_tree(path: &Path) -> BoxFuture<'_, Result<FileNode, String>> {
    build_tree_lazy(path) // Use lazy loading approach
}

// Lazy loading function - loads top level contents, subdirectories get placeholders
fn build_tree_lazy(path: &Path) -> BoxFuture<'_, Result<FileNode, String>> {
    async move {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let path_str = path.to_string_lossy().to_string();

        log::debug!("[FileUtils] build_tree_lazy: Reading directory: {:?}", path);

        if !path.is_dir() {
            log::debug!("[FileUtils] build_tree_lazy: Path is not a directory: {:?}", path);
            return Ok(FileNode {
                name,
                path: path_str,
                is_directory: false,
                children: None,
            });
        }

        // For the root directory, we want to show the actual top-level contents
        // but subdirectories will have placeholders until expanded
        let mut children = Vec::new();
        let mut entries = async_fs::read_dir(path).await.map_err(|e| {
            error!("Failed to read directory");
            e.to_string()
        })?;

        log::debug!("[FileUtils] build_tree_lazy: Reading entries from: {:?}", path);

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            error!("Failed to get next directory entry");
            e.to_string()
        })? {
            let child_path = entry.path();
            let name = child_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let path_str = child_path.to_string_lossy().to_string();

            let is_dir = child_path.is_dir();
            log::debug!(
                "[FileUtils] build_tree_lazy: Found entry: {} (is_dir: {}) at {:?}",
                name,
                is_dir,
                child_path
            );

            if is_dir {
                // For subdirectories, create a lazy-loaded node with no children
                children.push(FileNode {
                    name: name.clone(),
                    path: path_str.clone(),
                    is_directory: true,
                    children: None, // No placeholder - let frontend handle lazy loading
                });
                log::debug!(
                    "[FileUtils] build_tree_lazy: Added directory node: {} -> {}",
                    name,
                    path_str
                );
            } else {
                // For files, create a regular node
                children.push(FileNode {
                    name: name.clone(),
                    path: path_str.clone(),
                    is_directory: false,
                    children: None,
                });
                log::debug!(
                    "[FileUtils] build_tree_lazy: Added file node: {} -> {}",
                    name,
                    path_str
                );
            }
        }

        log::debug!(
            "[FileUtils] build_tree_lazy: Found {} total entries ({} directories, {} files)",
            children.len(),
            children.iter().filter(|c| c.is_directory).count(),
            children.iter().filter(|c| !c.is_directory).count()
        );

        // Log all directory names found
        let dir_names: Vec<&String> = children
            .iter()
            .filter(|c| c.is_directory)
            .map(|c| &c.name)
            .collect();
        log::debug!(
            "[FileUtils] build_tree_lazy: Directory names found: {:?}",
            dir_names
        );

        // Check specifically for demo folder
        let has_demo = children.iter().any(|c| c.is_directory && c.name == "demo");
        log::debug!(
            "[FileUtils] build_tree_lazy: Demo folder found: {}",
            has_demo
        );

        // Sort: directories first, then files, then alphabetically
        children.sort_by(|a, b| {
            if a.is_directory != b.is_directory {
                b.is_directory.cmp(&a.is_directory)
            } else {
                a.name.cmp(&b.name)
            }
        });

        log::debug!(
            "[FileUtils] build_tree_lazy: Returning {} children for path: {:?}",
            children.len(),
            path
        );

        Ok(FileNode {
            name,
            path: path_str,
            is_directory: true,
            children: if children.is_empty() { None } else { Some(children) },
        })
    }
    .boxed()
}

// Load directory contents for lazy loading
pub fn load_directory_contents(path: &Path) -> BoxFuture<'_, Result<Vec<FileNode>, String>> {
    async move {
        if !path.is_dir() {
            return Err("Path is not a directory".to_string());
        }

        let mut children = Vec::new();
        let mut entries = async_fs::read_dir(path).await.map_err(|e| {
            error!("Failed to read directory");
            e.to_string()
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            error!("Failed to get next directory entry");
            e.to_string()
        })? {
            let child_path = entry.path();
            let name = child_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let path_str = child_path.to_string_lossy().to_string();

            if child_path.is_dir() {
                // For directories, create a lazy-loaded node with no children
                children.push(FileNode {
                    name,
                    path: path_str,
                    is_directory: true,
                    children: None, // No placeholder - let frontend handle lazy loading
                });
            } else {
                // For files, create a regular node
                children.push(FileNode {
                    name,
                    path: path_str,
                    is_directory: false,
                    children: None,
                });
            }
        }

        // Sort: directories first, then files, then alphabetically
        children.sort_by(|a, b| {
            if a.is_directory != b.is_directory {
                b.is_directory.cmp(&a.is_directory)
            } else {
                a.name.cmp(&b.name)
            }
        });

        Ok(children)
    }
    .boxed()
}

