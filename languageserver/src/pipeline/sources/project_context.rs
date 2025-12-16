use crate::types::LspError;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a Julia project's context parsed from Project.toml
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectToml {
    pub name: Option<String>,
    pub uuid: Option<String>,
    pub version: Option<String>,
    pub authors: Option<Vec<String>>,
    #[serde(rename = "deps")]
    pub dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "compat")]
    pub compat: Option<HashMap<String, String>>,
}

/// Represents dependency information from Manifest.toml
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestEntry {
    pub uuid: Option<String>,
    pub version: Option<String>,
    pub path: Option<String>,
    pub repo: Option<String>,
    #[serde(rename = "git-tree-sha1")]
    pub git_tree_sha1: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestToml {
    #[serde(flatten)]
    pub packages: HashMap<String, Vec<ManifestEntry>>,
}

/// Project context that understands Julia project structure
#[derive(Debug, Clone)]
pub struct ProjectContext {
    /// Project root directory
    pub root_path: PathBuf,
    
    /// Parsed Project.toml
    pub project_toml: Option<ProjectToml>,
    
    /// Parsed Manifest.toml
    pub manifest_toml: Option<ManifestToml>,
    
    /// Resolved package paths for dependencies
    pub package_paths: HashMap<String, PathBuf>,
    
    /// Optional Julia depot path for resolving packages
    pub depot_path: Option<PathBuf>,
}

impl ProjectContext {
    /// Create a new project context from a project root directory
    pub fn new(root_path: PathBuf) -> Result<Self, LspError> {
        Self::with_depot_path(root_path, None)
    }
    
    /// Create a new project context with an optional depot path
    pub fn with_depot_path(root_path: PathBuf, depot_path: Option<PathBuf>) -> Result<Self, LspError> {
        log::trace!("ProjectContext: Creating context for project at {:?}", root_path);
        
        if !root_path.exists() {
            return Err(LspError::InternalError(format!(
                "Project root does not exist: {:?}",
                root_path
            )));
        }
        
        let mut context = Self {
            root_path: root_path.clone(),
            project_toml: None,
            manifest_toml: None,
            package_paths: HashMap::new(),
            depot_path,
        };
        
        // Try to parse Project.toml
        context.load_project_toml()?;
        
        // Try to parse Manifest.toml (optional)
        let _ = context.load_manifest_toml();
        
        // Resolve package paths
        context.resolve_package_paths();
        
        log::trace!(
            "ProjectContext: Initialized with {} dependencies",
            context.package_paths.len()
        );
        
        Ok(context)
    }
    
    /// Load and parse Project.toml
    fn load_project_toml(&mut self) -> Result<(), LspError> {
        let project_toml_path = self.root_path.join("Project.toml");
        
        if !project_toml_path.exists() {
            log::trace!("ProjectContext: No Project.toml found at {:?}", project_toml_path);
            return Ok(());
        }
        
        let content = fs::read_to_string(&project_toml_path)
            .map_err(|e| LspError::InternalError(format!("Failed to read Project.toml: {}", e)))?;
        
        let project_toml: ProjectToml = toml::from_str(&content)
            .map_err(|e| LspError::ParseError(format!("Failed to parse Project.toml: {}", e)))?;
        
        log::trace!(
            "ProjectContext: Loaded Project.toml - name: {:?}, {} dependencies",
            project_toml.name,
            project_toml.dependencies.as_ref().map(|d| d.len()).unwrap_or(0)
        );
        
        self.project_toml = Some(project_toml);
        Ok(())
    }
    
    /// Load and parse Manifest.toml
    fn load_manifest_toml(&mut self) -> Result<(), LspError> {
        let manifest_toml_path = self.root_path.join("Manifest.toml");
        
        if !manifest_toml_path.exists() {
            log::trace!("ProjectContext: No Manifest.toml found at {:?}", manifest_toml_path);
            return Ok(());
        }
        
        let content = fs::read_to_string(&manifest_toml_path)
            .map_err(|e| LspError::InternalError(format!("Failed to read Manifest.toml: {}", e)))?;
        
        // Parse Manifest.toml manually to handle [[deps.PackageName]] format
        // The TOML format uses array of tables: [[deps.DataFrames]]
        let value: toml::Value = toml::from_str(&content)
            .map_err(|e| LspError::ParseError(format!("Failed to parse Manifest.toml: {}", e)))?;
        
        let mut packages: HashMap<String, Vec<ManifestEntry>> = HashMap::new();
        
        // Extract all deps.* entries
        if let Some(deps_value) = value.get("deps") {
            if let Some(root) = deps_value.as_table() {
                for (package_name, package_value) in root {
                    if let Some(entries) = package_value.as_array() {
                        let mut manifest_entries = Vec::new();
                        for entry_value in entries {
                            if let Some(entry_table) = entry_value.as_table() {
                                let entry = ManifestEntry {
                                    uuid: entry_table.get("uuid")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    version: entry_table.get("version")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    path: entry_table.get("path")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    repo: entry_table.get("repo")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    git_tree_sha1: entry_table.get("git-tree-sha1")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                };
                                manifest_entries.push(entry);
                            }
                        }
                        if !manifest_entries.is_empty() {
                            packages.insert(package_name.clone(), manifest_entries);
                        }
                    }
                }
            }
        }
        
        let manifest_toml = ManifestToml { packages };
        log::trace!("ProjectContext: Loaded Manifest.toml with {} package entries", manifest_toml.packages.len());
        
        self.manifest_toml = Some(manifest_toml);
        Ok(())
    }
    
    /// Resolve paths to package dependencies
    fn resolve_package_paths(&mut self) {
        // Check if we have dependencies in Project.toml
        let Some(ref project_toml) = self.project_toml else {
            return;
        };
        
        let Some(ref dependencies) = project_toml.dependencies else {
            return;
        };
        
        // Try to resolve paths for each dependency
        for package_name in dependencies.keys() {
            if let Some(path) = self.try_resolve_package_path(package_name) {
                self.package_paths.insert(package_name.clone(), path);
            }
        }
    }
    
    /// Try to resolve the path for a specific package
    fn try_resolve_package_path(&self, package_name: &str) -> Option<PathBuf> {
        // First check if manifest has path information
        if let Some(ref manifest) = self.manifest_toml {
            if let Some(entries) = manifest.packages.get(package_name) {
                for entry in entries {
                    if let Some(ref path) = entry.path {
                        let resolved_path = if Path::new(path).is_absolute() {
                            PathBuf::from(path)
                        } else {
                            self.root_path.join(path)
                        };
                        
                        if resolved_path.exists() {
                            log::trace!("ProjectContext: Resolved package {} from manifest path: {:?}", package_name, resolved_path);
                            return Some(resolved_path);
                        }
                    }
                }
            }
        }
        
        // Try common Julia package locations
        // 1. Check local project deps folder
        let local_path = self.root_path.join("deps").join(package_name);
        if local_path.exists() {
            log::trace!("ProjectContext: Resolved package {} from local deps: {:?}", package_name, local_path);
            return Some(local_path);
        }
        
        // 2. Try Julia depot using package_resolver
        if let Some(ref depot_path) = self.depot_path {
            use crate::pipeline::sources::indexing::resolve_package_path;
            if let Some(package_path) = resolve_package_path(depot_path, package_name, self.manifest_toml.as_ref()) {
                log::trace!("ProjectContext: Resolved package {} from depot: {:?}", package_name, package_path);
                return Some(package_path);
            }
        }
        
        log::trace!("ProjectContext: Could not resolve path for package: {}", package_name);
        
        None
    }
    
    /// Get project name
    pub fn project_name(&self) -> Option<&str> {
        self.project_toml.as_ref()?.name.as_deref()
    }
    
    /// Get project version
    pub fn project_version(&self) -> Option<&str> {
        self.project_toml.as_ref()?.version.as_deref()
    }
    
    /// Get all dependencies
    pub fn dependencies(&self) -> Option<&HashMap<String, String>> {
        self.project_toml.as_ref()?.dependencies.as_ref()
    }
    
    /// Get resolved path for a specific package
    pub fn get_package_path(&self, package_name: &str) -> Option<&PathBuf> {
        self.package_paths.get(package_name)
    }
    
    /// Check if a path is within the project
    pub fn is_project_file(&self, path: &Path) -> bool {
        path.starts_with(&self.root_path)
    }
    
    /// Check if project has a valid Project.toml
    pub fn has_project_toml(&self) -> bool {
        self.project_toml.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    fn create_test_project() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();
        
        let project_toml = r#"
name = "TestProject"
uuid = "12345678-1234-1234-1234-123456789012"
version = "0.1.0"

[deps]
JSON = "682c06a0-de6a-54ab-a142-c8b1cf79cde6"
DataFrames = "a93c6f00-e57d-5684-b7b6-d8193f3e46c0"
"#;
        
        fs::write(project_path.join("Project.toml"), project_toml).unwrap();
        
        (temp_dir, project_path)
    }
    
    #[test]
    fn test_project_context_creation() {
        let (_temp_dir, project_path) = create_test_project();
        
        let context = ProjectContext::new(project_path.clone()).unwrap();
        
        assert!(context.has_project_toml());
        assert_eq!(context.project_name(), Some("TestProject"));
        assert_eq!(context.project_version(), Some("0.1.0"));
    }
    
    #[test]
    fn test_dependencies_parsing() {
        let (_temp_dir, project_path) = create_test_project();
        
        let context = ProjectContext::new(project_path).unwrap();
        
        let deps = context.dependencies().unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains_key("JSON"));
        assert!(deps.contains_key("DataFrames"));
    }
    
    #[test]
    fn test_project_without_toml() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();
        
        // Create context without Project.toml
        let context = ProjectContext::new(project_path).unwrap();
        
        assert!(!context.has_project_toml());
        assert_eq!(context.project_name(), None);
        assert_eq!(context.dependencies(), None);
    }
    
    #[test]
    fn test_is_project_file() {
        let (_temp_dir, project_path) = create_test_project();
        let context = ProjectContext::new(project_path.clone()).unwrap();
        
        let file_in_project = project_path.join("src").join("main.jl");
        assert!(context.is_project_file(&file_in_project));
        
        let file_outside = PathBuf::from("/tmp/other.jl");
        assert!(!context.is_project_file(&file_outside));
    }
}

