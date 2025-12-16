// Platform-specific Julia binary extraction

use crate::services::events::EventService;
use log::debug;
use std::path::Path;
#[cfg(target_os = "linux")]
use tokio::fs;
#[cfg(target_os = "windows")]
use zip::ZipArchive;

/// Extract ZIP file with progress
#[cfg(target_os = "windows")]
pub async fn extract_zip_with_progress(
    zip_path: &Path,
    extract_to: &Path,
    event_service: Option<&EventService>,
) -> Result<(), String> {
    debug!("[InstallationActor] Starting ZIP extraction from: {:?} to: {:?}", zip_path, extract_to);
    
    // Verify ZIP file exists and is readable
    if !zip_path.exists() {
        if let Some(event_service) = event_service {
            let _ = event_service.emit_julia_installation_error(
                "Julia extraction failed",
                &format!("ZIP file does not exist: {:?}", zip_path)
            ).await;
        }
        return Err(format!("ZIP file does not exist: {:?}", zip_path));
    }

    let zip_metadata = std::fs::metadata(zip_path)
        .map_err(|e| format!("Failed to get ZIP file metadata: {}", e))?;
    debug!(
        "[InstallationActor] ZIP file size: {} bytes ({:.2} MB)",
        zip_metadata.len(),
        zip_metadata.len() as f64 / 1024.0 / 1024.0
    );

    let file =
        std::fs::File::open(zip_path).map_err(|e| format!("Failed to open ZIP file: {}", e))?;

    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP archive: {}", e))?;

    let total_files = archive.len();
    debug!("[InstallationActor] ZIP archive contains {} files", total_files);
    let mut extracted_files = 0;
    let mut total_extracted_size: u64 = 0;
    let mut last_reported_percentage = -1.0;

    for i in 0..archive.len() {
        let file_name = {
            let file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to access ZIP file {}: {}", i, e))?;
            let name = file.name().to_string();
            let size = file.size();
            (name, size)
        };

        let outpath = extract_to.join(&file_name.0);

        if file_name.0.ends_with('/') {
            std::fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)
                        .map_err(|e| format!("Failed to create parent directory: {}", e))?;
                }
            }

            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;

            {
                let mut file = archive
                    .by_index(i)
                    .map_err(|e| format!("Failed to access ZIP file {}: {}", i, e))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to extract file: {}", e))?;
            }

            total_extracted_size += file_name.1;
        }

        extracted_files += 1;

        // Report progress every 5% of total extraction
        let progress_percentage = if total_files > 0 {
            (extracted_files as f64 / total_files as f64) * 100.0
        } else {
            0.0
        };
        
        // Check if we've crossed a 5% threshold
        // Also report if we're at or near completion (>= 99.5%) to ensure final update
        let current_threshold = (progress_percentage / 5.0).floor() * 5.0;
        let is_complete = extracted_files >= total_files || progress_percentage >= 99.5;
        
        if current_threshold > last_reported_percentage || is_complete {
            if is_complete {
                last_reported_percentage = 100.0;
            } else {
                last_reported_percentage = current_threshold;
            }
            
            let extracted_mb = total_extracted_size as f64 / 1024.0 / 1024.0;
            
            debug!(
                "[InstallationActor] Extraction progress: {}/{} files, {:.2} MB extracted",
                extracted_files,
                total_files,
                extracted_mb
            );
            
            // Emit progress event to frontend
            if let Some(event_service) = event_service {
                let display_percentage = if is_complete { 100.0 } else { progress_percentage };
                let _ = event_service.emit_julia_installation_progress(
                    &format!("Extracting Julia... ({:.1}% - {}/{} files)", display_percentage, extracted_files, total_files),
                    70 + (display_percentage * 0.2) as u8 // Scale to 70-90% range
                ).await;
            }
        }
    }
    
    debug!(
        "[InstallationActor] Total size extracted: {:.2} MB",
        total_extracted_size as f64 / 1024.0 / 1024.0
    );
    
    // Emit extraction completed event
    if let Some(event_service) = event_service {
        let _ = event_service.emit_julia_installation_progress(
            "Julia extraction completed",
            90
        ).await;
    }
    
    debug!("[InstallationActor] ZIP extraction completed successfully - {} files extracted", extracted_files);
    Ok(())
}

/// Extract TAR.GZ file with progress
#[cfg(target_os = "linux")]
pub async fn extract_targz_with_progress(
    targz_path: &Path,
    extract_to: &Path,
    _event_service: Option<&EventService>,
) -> Result<(), String> {
    // Verify TAR.GZ file exists
    if !targz_path.exists() {
        return Err(format!("TAR.GZ file does not exist: {:?}", targz_path));
    }

    let targz_metadata = std::fs::metadata(targz_path)
        .map_err(|e| format!("Failed to get TAR.GZ file metadata: {}", e))?;
    debug!(
        "[InstallationActor] TAR.GZ file size: {} bytes ({:.2} MB)",
        targz_metadata.len(),
        targz_metadata.len() as f64 / 1024.0 / 1024.0
    );

    // For now, use system tar command
    let mut command = std::process::Command::new("tar");
    
    // On Windows, prevent the console window from appearing
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    let status = command
        .args(["-xzf", targz_path.to_str().unwrap()])
        .current_dir(extract_to)
        .status()
        .map_err(|e| format!("Failed to extract TAR.GZ: {}", e))?;

    if !status.success() {
        return Err("Failed to extract TAR.GZ file".to_string());
    }
    
    // List extracted contents
    if let Ok(mut entries) = fs::read_dir(extract_to).await {
        let mut entry_count = 0;
        while let Some(entry) = entries.next_entry().await.ok().flatten() {
            entry_count += 1;
            if entry_count <= 10 {
                // Log first 10 entries
                debug!("[InstallationActor] Extracted: {:?}", entry.path());
            }
        }
        debug!(
            "[InstallationActor] Total files/directories extracted: {}",
            entry_count
        );
    }

    Ok(())
}

