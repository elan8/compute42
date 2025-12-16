// Cleanup of old Julia versions

use log::{debug, error};
use std::path::Path;
use tokio::fs;

/// Clean up old Julia versions, keeping only the current version
pub async fn cleanup_old_julia_versions(installation_dir: &Path, current_version: &str) -> Result<(), String> {
    debug!("InstallationActor: Cleaning up old Julia versions, keeping version: {}", current_version);
    
    if !installation_dir.exists() {
        debug!("InstallationActor: Installation directory does not exist, nothing to clean up");
        return Ok(());
    }

    let mut entries = fs::read_dir(installation_dir)
        .await
        .map_err(|e| format!("Failed to read installation directory: {}", e))?;

    let mut removed_versions = Vec::new();
    let mut errors = Vec::new();

    while let Some(entry) = entries.next_entry().await
        .map_err(|e| format!("Failed to read directory entry: {}", e))? {
        
        let path = entry.path();
        if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                // Check if this is a Julia version directory (starts with "julia-")
                if let Some(version) = dir_name.strip_prefix("julia-") {
                    // Remove "julia-" prefix
                    
                    // Skip the current version
                    if version == current_version {
                        debug!("InstallationActor: Keeping current version directory: {}", dir_name);
                        continue;
                    }
                    
                    // Remove old version directory
                    debug!("InstallationActor: Removing old Julia version directory: {}", dir_name);
                    match fs::remove_dir_all(&path).await {
                        Ok(_) => {
                            debug!("InstallationActor: Successfully removed old Julia version: {}", version);
                            removed_versions.push(version.to_string());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to remove old Julia version {}: {}", version, e);
                            error!("InstallationActor: {}", error_msg);
                            errors.push(error_msg);
                        }
                    }
                }
            }
        }
    }

    if !removed_versions.is_empty() {
        debug!("InstallationActor: Cleaned up {} old Julia versions: {:?}", removed_versions.len(), removed_versions);
    }

    if !errors.is_empty() {
        return Err(format!("Failed to clean up some old Julia versions: {}", errors.join("; ")));
    }

    Ok(())
}






















