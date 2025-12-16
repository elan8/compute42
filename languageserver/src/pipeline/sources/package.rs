use crate::pipeline::types::{SourceItem, FileMetadata};
use crate::pipeline::sources::ProjectContext;
use crate::pipeline::sources::indexing::{resolve_package_path, should_skip_entry, compute_package_slug, extract_package_slug};
use crate::pipeline::sources::base_docs::BaseDocsRegistry;
use crate::types::LspError;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use walkdir::WalkDir;

enum CacheValidity {
    Valid,
    Invalid(String),
}

/// Result of package discovery with caching
pub struct DiscoverResult {
    /// Cached package docstring registries (cache hit) - maps package name to registry
    pub cached_registries: HashMap<String, BaseDocsRegistry>,
    /// Metadata for packages that need processing (for saving cache later)
    pub packages_to_process: Vec<PackageMetadata>,
}

/// Metadata about a package that needs processing
pub struct PackageMetadata {
    pub name: String,
    pub slug: Option<String>,
    pub path: PathBuf,
}

/// Source that discovers package files from depot based on project dependencies
pub struct PackageSource {
    depot_path: PathBuf,
    project_context: ProjectContext,
}

impl PackageSource {
    pub fn new(depot_path: PathBuf, project_context: ProjectContext) -> Self {
        Self {
            depot_path,
            project_context,
        }
    }

    /// Discover all package files from project dependencies with caching support
    /// 
    /// Note: We only index direct dependencies because:
    /// 1. Symbol resolution and type inference only need symbols from packages the user directly imports
    /// 2. Transitive dependencies are typically re-exported by direct dependencies if needed
    /// 3. Scanning transitive dependencies can be very expensive for large dependency trees
    pub fn discover_with_cache(&self) -> Result<DiscoverResult, LspError> {
        let mut cached_registries = HashMap::new();
        let mut package_stats = Vec::new();

        let Some(dependencies) = self.project_context.dependencies() else {
            log::info!("PackageSource: No dependencies found in project");
            return Ok(DiscoverResult {
                cached_registries,
                packages_to_process: Vec::new(),
            });
        };

        log::info!("PackageSource: Found {} direct dependencies to index (skipping transitive dependencies)", dependencies.len());
        let manifest = self.project_context.manifest_toml.as_ref();

        // Only process direct dependencies (skip transitive dependencies)
        for package_name in dependencies.keys() {
            // Try to get UUID and git-tree-sha1 from manifest to compute slug
            let slug = if let Some(manifest) = manifest {
                if let Some(entries) = manifest.packages.get(package_name) {
                    // Find entry with UUID and git-tree-sha1
                    entries.iter().find_map(|entry| {
                        if let (Some(uuid), Some(git_tree_sha1)) = (&entry.uuid, &entry.git_tree_sha1) {
                            match compute_package_slug(uuid, git_tree_sha1) {
                                Ok(s) => Some(s),
                                Err(e) => {
                                    log::warn!("PackageSource: Failed to compute slug for {}: {}", package_name, e);
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            } else {
                None
            };

            // Check cache if we have a slug
            let mut cache_hit = false;
            if let Some(ref slug) = slug {
                let cache_path = Self::get_package_cache_path(package_name, slug);
                
                // Try to resolve package path to validate cache
                if let Some(package_path) = resolve_package_path(
                    &self.depot_path,
                    package_name,
                    manifest,
                ) {
                    if let Some(cached_registry) = Self::load_package_cache(&cache_path, &package_path) {
                        log::info!("PackageSource: Loaded cached docstrings for package '{}' (slug: {}, {} entries)", 
                            package_name, slug, cached_registry.len());
                        cached_registries.insert(package_name.clone(), cached_registry);
                        cache_hit = true;
                        package_stats.push((package_name.clone(), 0, package_path.clone(), true));
                    }
                }
            }

            // If cache miss, add to packages to process
            if !cache_hit {
                if let Some(package_path) = resolve_package_path(
                    &self.depot_path,
                    package_name,
                    manifest,
                ) {
                    // Validate slug if we computed one
                    if let Some(ref computed_slug) = slug {
                        if let Some(extracted_slug) = extract_package_slug(&package_path) {
                            if computed_slug != &extracted_slug {
                                log::warn!("PackageSource: Computed slug '{}' does not match extracted slug '{}' for package '{}'", 
                                    computed_slug, extracted_slug, package_name);
                            }
                        }
                    }

                    log::info!("PackageSource: Will process package '{}' from {:?}", package_name, package_path);
                    package_stats.push((package_name.clone(), 0, package_path.clone(), false));
                } else {
                    log::warn!("PackageSource: Could not resolve path for package '{}'", package_name);
                    package_stats.push((package_name.clone(), 0, PathBuf::new(), false));
                }
            }
        }

        let cached_count = cached_registries.len();
        let processed_count = package_stats.iter().filter(|(_, _, _, cached)| !cached).count();
        log::info!("PackageSource: {} packages from cache, {} packages to process", 
            cached_count, processed_count);

        // Collect packages that were processed (not cached)
        let packages_to_process: Vec<PackageMetadata> = package_stats
            .iter()
            .filter(|(_, _, _, cached)| !cached)
            .filter_map(|(name, _, path, _)| {
                if path.exists() {
                    // Try to get slug from manifest
                    let slug = if let Some(manifest) = manifest {
                        manifest.packages.get(name).and_then(|entries| {
                            entries.iter().find_map(|entry| {
                                if let (Some(uuid), Some(git_tree_sha1)) = (&entry.uuid, &entry.git_tree_sha1) {
                                    compute_package_slug(uuid, git_tree_sha1).ok()
                                } else {
                                    None
                                }
                            })
                        })
                    } else {
                        None
                    };
                    
                    Some(PackageMetadata {
                        name: name.clone(),
                        slug,
                        path: path.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(DiscoverResult {
            cached_registries,
            packages_to_process,
        })
    }

    /// Discover all package files from project dependencies (only direct dependencies, not transitive)
    /// 
    /// This is the legacy method that doesn't use caching. Use `discover_with_cache()` for better performance.
    /// 
    /// Note: We only index direct dependencies because:
    /// 1. Symbol resolution and type inference only need symbols from packages the user directly imports
    /// 2. Transitive dependencies are typically re-exported by direct dependencies if needed
    /// 3. Scanning transitive dependencies can be very expensive for large dependency trees
    pub fn discover(&self) -> Result<Vec<SourceItem>, LspError> {
        // Legacy method - discover files for processing
        let Some(dependencies) = self.project_context.dependencies() else {
            return Ok(Vec::new());
        };
        
        let mut source_items = Vec::new();
        let manifest = self.project_context.manifest_toml.as_ref();
        
        for package_name in dependencies.keys() {
            if let Some(package_path) = resolve_package_path(
                &self.depot_path,
                package_name,
                manifest,
            ) {
                match Self::discover_package_files(&package_path, package_name) {
                    Ok(mut package_items) => {
                        source_items.append(&mut package_items);
                    }
                    Err(e) => {
                        log::warn!("PackageSource: Failed to discover files for package {}: {}", package_name, e);
                    }
                }
            }
        }
        
        Ok(source_items)
    }
    

    fn discover_package_files(
        package_path: &Path,
        package_name: &str,
    ) -> Result<Vec<SourceItem>, LspError> {
        let mut items = Vec::new();
        let src_dir = package_path.join("src");

        if !src_dir.exists() {
            return Ok(items);
        }

        for entry in WalkDir::new(&src_dir)
            .into_iter()
            .filter_entry(|e| !should_skip_entry(e.path()))
        {
            let entry = entry.map_err(|e| {
                LspError::InternalError(format!("Failed to walk directory: {}", e))
            })?;

            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "jl" {
                        match Self::load_file(entry.path()) {
                            Ok(item) => {
                                items.push(item);
                            }
                            Err(e) => {
                                log::warn!("PackageSource: Failed to load file {:?} for package '{}': {}", entry.path(), package_name, e);
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

    /// Get package cache directory path
    fn get_package_cache_dir() -> PathBuf {
        dirs::data_local_dir()
            .map(|dir| dir.join("com.compute42.dev").join("package_cache"))
            .unwrap_or_else(|| {
                log::warn!("Failed to get user data directory, falling back to current directory");
                PathBuf::from(".").join("package_cache")
            })
    }

    /// Generate cache file path for a package
    pub fn get_package_cache_path(package_name: &str, slug: &str) -> PathBuf {
        let cache_dir = Self::get_package_cache_dir();
        // Sanitize package name for filesystem (replace invalid chars)
        let sanitized_name = package_name.replace(std::path::MAIN_SEPARATOR, "_");
        cache_dir.join(format!("package_{}_{}.json", sanitized_name, slug))
    }

    /// Load package docstrings from cache if valid
    /// Uses the same format as base_index.json (array of DocEntry objects)
    fn load_package_cache(cache_path: &Path, package_path: &Path) -> Option<BaseDocsRegistry> {
        if !cache_path.exists() {
            return None;
        }

        // Check if cache is valid (package directory hasn't changed)
        match Self::check_cache_validity(cache_path, package_path) {
            CacheValidity::Valid => {
                // Load docstrings from cache (same format as base_index.json)
                match BaseDocsRegistry::from_file(cache_path) {
                    Ok(registry) => {
                        log::info!("PackageSource: Loaded {} docstrings from cache: {:?}", registry.len(), cache_path);
                        Some(registry)
                    }
                    Err(e) => {
                        log::warn!("PackageSource: Failed to load cache from {:?}: {}", cache_path, e);
                        None
                    }
                }
            }
            CacheValidity::Invalid(_reason) => {
                None
            }
        }
    }
    
    fn check_cache_validity(cache_path: &Path, package_path: &Path) -> CacheValidity {
        let cache_metadata = match std::fs::metadata(cache_path) {
            Ok(m) => m,
            Err(e) => return CacheValidity::Invalid(format!("Cannot read cache metadata: {}", e)),
        };

        let cache_modified = match cache_metadata.modified() {
            Ok(t) => t,
            Err(e) => return CacheValidity::Invalid(format!("Cannot get cache modification time: {}", e)),
        };

        // Check modification time of source files in the package, not the directory itself
        // Package directories can have their modification times updated even when source files don't change
        let src_dir = package_path.join("src");
        if !src_dir.exists() {
            // If src directory doesn't exist, fall back to checking package directory
            let package_metadata = match std::fs::metadata(package_path) {
                Ok(m) => m,
                Err(e) => return CacheValidity::Invalid(format!("Cannot read package metadata: {}", e)),
            };
            let package_modified = match package_metadata.modified() {
                Ok(t) => t,
                Err(e) => return CacheValidity::Invalid(format!("Cannot get package modification time: {}", e)),
            };
            if cache_modified >= package_modified {
                return CacheValidity::Valid;
            } else {
                return CacheValidity::Invalid(format!(
                    "Cache is older than package directory (cache: {:?}, package: {:?})",
                    cache_modified, package_modified
                ));
            }
        }

        // Find the most recent modification time of any .jl file in src/ (recursively)
        let mut most_recent_source: Option<std::time::SystemTime> = None;
        if let Ok(walker) = WalkDir::new(&src_dir).into_iter().collect::<Result<Vec<_>, _>>() {
            for entry in walker {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("jl") {
                    if let Ok(metadata) = std::fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            most_recent_source = Some(most_recent_source
                                .map(|prev| prev.max(modified))
                                .unwrap_or(modified));
                        }
                    }
                }
            }
        }

        // If we found source files, check if cache is newer than the most recent source file
        if let Some(most_recent) = most_recent_source {
            if cache_modified >= most_recent {
                CacheValidity::Valid
            } else {
                CacheValidity::Invalid(format!(
                    "Cache is older than source files (cache: {:?}, most recent source: {:?})",
                    cache_modified, most_recent
                ))
            }
        } else {
            // No source files found, consider cache valid if it exists
            CacheValidity::Valid
        }
    }

    /// Process a single package and extract docstrings directly from source files
    /// Uses the same docstring-first approach as base docs extraction
    /// Returns a BaseDocsRegistry with docstrings extracted from source files
    pub fn process_package(
        package_path: &Path,
        package_name: &str,
    ) -> Result<BaseDocsRegistry, LspError> {
        // Discover all Julia files in the package
        let mut source_files = Vec::new();
        for entry in WalkDir::new(package_path)
            .into_iter()
            .filter_entry(|e| !should_skip_entry(e.path()))
        {
            let entry = entry.map_err(|e| LspError::InternalError(format!("Failed to walk directory: {}", e)))?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("jl") {
                source_files.push(path.to_path_buf());
            }
        }
        
        log::info!("PackageSource: Processing {} files from package '{}'", source_files.len(), package_name);
        
        // Extract docstrings directly from source files (same approach as base docs)
        let registry = BaseDocsRegistry::from_source_files(&source_files)?;
        
        log::info!("PackageSource: Extracted {} docstrings from package '{}'", registry.len(), package_name);
        
        Ok(registry)
    }
    
    /// Save package docstrings to cache using the same format as base_index.json
    pub fn save_package_cache(registry: &BaseDocsRegistry, cache_path: &Path) -> Result<(), LspError> {
        // Create cache directory if it doesn't exist
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| LspError::InternalError(format!("Failed to create cache directory: {}", e)))?;
        }

        // Save using BaseDocsRegistry's to_file method (same format as base_index.json)
        registry.to_file(cache_path)?;
        
        log::info!("PackageSource: Saved {} docstrings to cache: {:?}", registry.len(), cache_path);
        Ok(())
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::sources::ProjectContext;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_project_with_deps() -> (TempDir, PathBuf, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let depot_path = temp_dir.path().join("depot");

        // Create Project.toml with dependencies
        let project_toml = r#"
name = "TestProject"
uuid = "12345678-1234-1234-1234-123456789012"
[deps]
TestPackage = "87654321-4321-4321-4321-210987654321"
"#;
        fs::write(project_root.join("Project.toml"), project_toml).unwrap();

        // Create a test package in depot
        let package_path = depot_path
            .join("packages")
            .join("TestPackage")
            .join("87654321-4321-4321-4321-210987654321")
            .join("src");
        fs::create_dir_all(&package_path).unwrap();

        fs::write(
            package_path.join("TestPackage.jl"),
            "module TestPackage\nfunction test_func() end\nend",
        )
        .unwrap();

        (temp_dir, project_root, depot_path)
    }

    #[test]
    fn test_discover_with_dependencies() {
        let (_temp_dir, project_root, depot_path) = create_test_project_with_deps();
        let context = ProjectContext::new(project_root).unwrap();
        let source = PackageSource::new(depot_path, context);
        let items = source.discover().unwrap();

        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.path.ends_with("TestPackage.jl")));
    }

    #[test]
    fn test_discover_no_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let depot_path = temp_dir.path().join("depot");

        // Create Project.toml without dependencies
        let project_toml = r#"
name = "TestProject"
uuid = "12345678-1234-1234-1234-123456789012"
"#;
        fs::write(project_root.join("Project.toml"), project_toml).unwrap();

        let context = ProjectContext::new(project_root).unwrap();
        let source = PackageSource::new(depot_path, context);
        let items = source.discover().unwrap();

        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_discover_no_project_toml() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let depot_path = temp_dir.path().join("depot");

        let context = ProjectContext::new(project_root).unwrap();
        let source = PackageSource::new(depot_path, context);
        let items = source.discover().unwrap();

        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_discover_package_without_src() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        let depot_path = temp_dir.path().join("depot");

        // Create Project.toml with dependencies
        let project_toml = r#"
name = "TestProject"
uuid = "12345678-1234-1234-1234-123456789012"
[deps]
TestPackage = "87654321-4321-4321-4321-210987654321"
"#;
        fs::write(project_root.join("Project.toml"), project_toml).unwrap();

        // Create package directory but without src/
        let package_path = depot_path
            .join("packages")
            .join("TestPackage")
            .join("87654321-4321-4321-4321-210987654321");
        fs::create_dir_all(&package_path).unwrap();

        let context = ProjectContext::new(project_root).unwrap();
        let source = PackageSource::new(depot_path, context);
        let items = source.discover().unwrap();

        // Should return empty list, not error
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_discover_skips_non_julia_files() {
        let (_temp_dir, project_root, depot_path) = create_test_project_with_deps();
        let package_path = depot_path
            .join("packages")
            .join("TestPackage")
            .join("87654321-4321-4321-4321-210987654321")
            .join("src");

        // Add a non-Julia file
        fs::write(package_path.join("readme.txt"), "This is a readme").unwrap();

        let context = ProjectContext::new(project_root).unwrap();
        let source = PackageSource::new(depot_path, context);
        let items = source.discover().unwrap();

        // Should only find .jl files
        assert!(items.iter().all(|i| i.path.extension().unwrap() == "jl"));
    }
}

