// Main Julia installation orchestration

use crate::services::events::EventService;
use crate::types::JuliaInstallation;
use crate::version;
use log::{debug, error, info};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::fs;

use super::cleanup::cleanup_old_julia_versions;
use super::discovery::find_julia_executable;
use super::download::download_julia_binary;
#[cfg(target_os = "windows")]
use super::extraction::extract_zip_with_progress;
#[cfg(target_os = "linux")]
use super::extraction::extract_targz_with_progress;
use super::version_info::{get_julia_version_info, get_julia_installation_dir};

/// Install Julia binary
pub async fn install_julia_binary(
    download_path: &Path,
    installation_dir: &Path,
    event_service: Option<&EventService>,
) -> Result<(), String> {
    debug!("[InstallationActor] Starting Julia binary installation from: {:?}", download_path);
    
    // Verify download file exists
    if !download_path.exists() {
        return Err(format!("Download file does not exist: {:?}", download_path));
    }

    let download_metadata = fs::metadata(download_path)
        .await
        .map_err(|e| format!("Failed to get download file metadata: {}", e))?;
    debug!(
        "[InstallationActor] Download file size: {} bytes ({:.2} MB)",
        download_metadata.len(),
        download_metadata.len() as f64 / 1024.0 / 1024.0
    );

    #[cfg(target_os = "windows")]
    {
        debug!("[InstallationActor] Extracting ZIP file on Windows");
        extract_zip_with_progress(download_path, installation_dir, event_service)
            .await?;
    }

    #[cfg(target_os = "macos")]
    {
        debug!("[InstallationActor] macOS DMG installation not yet implemented");
        return Err("macOS DMG installation not yet implemented".to_string());
    }

    #[cfg(target_os = "linux")]
    {
        debug!("[InstallationActor] Extracting tar.gz file on Linux");
        extract_targz_with_progress(download_path, installation_dir, event_service)
            .await?;
    }
    
    debug!("[InstallationActor] Julia binary installation completed successfully");
    Ok(())
}

/// Main installation function
pub async fn install_julia(
    julia_version: Option<&str>,
    installation_in_progress: Arc<Mutex<bool>>,
    event_service: Option<&EventService>,
) -> Result<(), String> {
    // Set installation in progress
    {
        let mut in_progress = installation_in_progress.lock().await;
        if *in_progress {
            return Err("Julia installation already in progress".to_string());
        }
        *in_progress = true;
    }

    // Ensure we clean up the flag on exit
    let in_progress_guard = installation_in_progress.clone();

    let result = async {
        let version = julia_version.unwrap_or(version::get_julia_version()); // Default version if none provided
        debug!("[InstallationActor] Starting Julia installation for version: {}", version);
        
        let julia_info = get_julia_version_info(version);
        info!("[InstallationActor] Julia download URL: {}", julia_info.download_url);
        info!("[InstallationActor] Julia filename: {}", julia_info.filename);
        
        let installation_dir = get_julia_installation_dir();
        debug!("[InstallationActor] Installation directory: {:?}", installation_dir);
        
        // Emit installation started event
        if let Some(event_service) = event_service {
            debug!("[InstallationActor] Emitting julia:installation-started event");
            let _ = event_service.emit_julia_installation_started(version.to_string()).await;
        } else {
            debug!("[InstallationActor] No event service available for installation started event");
        }
        
        // Clean up old Julia versions before installing new one
        debug!("[InstallationActor] Cleaning up old Julia versions...");
        if let Some(event_service) = event_service {
            debug!("[InstallationActor] Emitting julia:installation-progress event - Cleaning up old Julia versions...");
            let _ = event_service.emit_julia_installation_progress("Cleaning up old Julia versions...", 10).await;
        }
        cleanup_old_julia_versions(&installation_dir, version).await
            .map_err(|e| {
                error!("[InstallationActor] Failed to clean up old Julia versions: {}", e);
                format!("Failed to clean up old Julia versions: {}", e)
            })?;
        debug!("[InstallationActor] Old Julia versions cleaned up successfully");
        
        // Create installation directory
        debug!("[InstallationActor] Creating installation directory...");
        if let Some(event_service) = event_service {
            let _ = event_service.emit_julia_installation_progress("Creating installation directory...", 20).await;
        }
        match fs::create_dir_all(&installation_dir).await {
            Ok(_) => {
                debug!("[InstallationActor] Installation directory created successfully");
            }
            Err(e) => {
                error!("[InstallationActor] Failed to create Julia installation directory: {}", e);
                if let Some(event_service) = event_service {
                    let _ = event_service.emit_julia_installation_error(
                        "Failed to create Julia installation directory",
                        &e.to_string()
                    ).await;
                }
                return Err(format!("Failed to create Julia installation directory: {}", e));
            }
        }

        // Download Julia
        debug!("[InstallationActor] Starting Julia download...");
        if let Some(event_service) = event_service {
            let _ = event_service.emit_julia_installation_progress(&format!("Downloading Julia {}...", version), 30).await;
        }
        let download_path = installation_dir.join(&julia_info.filename);
        debug!("[InstallationActor] Download path: {:?}", download_path);
        
        // Validate URL before attempting download
        debug!("[InstallationActor] Validating download URL: {}", julia_info.download_url);
        if !julia_info.download_url.starts_with("https://") {
            return Err(format!("Invalid download URL: {}", julia_info.download_url));
        }
        
        match download_julia_binary(&julia_info.download_url, &download_path, event_service).await {
            Ok(_) => {
                debug!("[InstallationActor] Julia download completed successfully");
            }
            Err(e) => {
                error!("[InstallationActor] Failed to download Julia: {}", e);
                if let Some(event_service) = event_service {
                    let _ = event_service.emit_julia_installation_error(
                        "Failed to download Julia",
                        &e
                    ).await;
                }
                return Err(format!("Failed to download Julia: {}", e));
            }
        }

        // Install Julia based on platform
        debug!("[InstallationActor] Starting Julia extraction...");
        if let Some(event_service) = event_service {
            let _ = event_service.emit_julia_installation_progress("Extracting Julia installation...", 70).await;
        }
        match install_julia_binary(&download_path, &installation_dir, event_service).await {
            Ok(_) => {
                debug!("[InstallationActor] Julia extraction completed successfully");
            }
            Err(e) => {
                error!("[InstallationActor] Failed to install Julia binary: {}", e);
                if let Some(event_service) = event_service {
                    let _ = event_service.emit_julia_installation_error(
                        "Failed to install Julia binary",
                        &e
                    ).await;
                }
                return Err(format!("Failed to install Julia binary: {}", e));
            }
        }

        // Clean up download file
        debug!("[InstallationActor] Cleaning up download file...");
        if let Some(event_service) = event_service {
            let _ = event_service.emit_julia_installation_progress("Cleaning up installation files...", 90).await;
        }
        if let Err(e) = fs::remove_file(&download_path).await {
            debug!("[InstallationActor] Failed to remove download file: {}", e);
            // Don't fail the installation for this
        } else {
            debug!("[InstallationActor] Download file cleaned up successfully");
        }
        
        // Emit installation completed event
        debug!("[InstallationActor] Julia installation completed successfully");
        if let Some(event_service) = event_service {
            let _ = event_service.emit_julia_installation_progress("Julia installation completed successfully", 100).await;
            
            // Get the Julia installation info and emit completion event
            if let Ok(julia_path) = find_julia_executable(None).await {
                let installation = JuliaInstallation {
                    path: julia_path.clone(),
                    version: version.to_string(),
                    is_valid: true,
                };
                let _ = event_service.emit_julia_installation_completed(installation).await;
            }
        }
        
        Ok(())
    }
    .await;

    // Clean up installation in progress flag
    {
        let mut in_progress = in_progress_guard.lock().await;
        *in_progress = false;
    }

    // Log the final result
    match &result {
        Ok(_) => {
            debug!("[InstallationActor] Julia installation completed successfully");
        }
        Err(e) => {
            error!("[InstallationActor] Julia installation failed: {}", e);
        }
    }

    result
}

