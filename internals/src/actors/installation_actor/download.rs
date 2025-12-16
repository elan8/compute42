// Julia binary download functionality

use crate::services::events::EventService;
use futures::StreamExt;
use log::{debug, error};
use std::path::Path;

/// Download Julia binary
pub async fn download_julia_binary(
    url: &str,
    download_path: &Path,
    event_service: Option<&EventService>,
) -> Result<(), String> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = download_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create download directory: {}", e))?;
    }

    // Test URL accessibility with HEAD request first
    debug!("[InstallationActor] Testing URL accessibility with HEAD request: {}", url);
    let client = reqwest::Client::builder()
        .user_agent("Compute42/0.15.0")
        .build()
        .map_err(|e| {
            error!("[InstallationActor] Failed to create HTTP client: {:?}", e);
            format!("Failed to create HTTP client: {}", e)
        })?;
    
    debug!("[InstallationActor] HTTP client created successfully");
        
    let head_response = client.head(url)
        .send()
        .await
        .map_err(|e| format!("Failed to test URL accessibility: {}", e))?;
    
    debug!("[InstallationActor] HEAD request status: {}", head_response.status());
    if !head_response.status().is_success() {
        error!("[InstallationActor] URL is not accessible: {}", head_response.status());
        return Err(format!("URL is not accessible: {}", head_response.status()));
    }
    
    // Get expected content length from HEAD request
    let expected_size = head_response.headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
        
    debug!("[InstallationActor] Expected content length from HEAD request: {} bytes ({:.2} MB)", 
           expected_size, expected_size as f64 / 1024.0 / 1024.0);
    
    // Download with progress tracking
    debug!("[InstallationActor] Starting HTTP GET request to: {}", url);
    let response = client.get(url)
        .send()
        .await
        .map_err(|e| {
            error!("[InstallationActor] HTTP request failed: {:?}", e);
            error!("[InstallationActor] Error type: {}", std::any::type_name::<reqwest::Error>());
            format!("Failed to download Julia: {}", e)
        })?;

    // Check HTTP status
    let status = response.status();
    debug!("[InstallationActor] HTTP response status: {}", status);
    debug!("[InstallationActor] Response headers: {:?}", response.headers());
    
    if !status.is_success() {
        error!("[InstallationActor] HTTP request failed with status: {}", status);
        if let Some(event_service) = event_service {
            let _ = event_service.emit_julia_installation_error(
                "Julia download failed",
                &format!("HTTP request failed with status: {}", status)
            ).await;
        }
        return Err(format!("Download failed with HTTP status: {}", status));
    }

    let total_size = response.content_length().unwrap_or(0);
    debug!(
        "[InstallationActor] Total download size: {} bytes ({:.2} MB)",
        total_size,
        total_size as f64 / 1024.0 / 1024.0
    );
    
    // Log response content type
    if let Some(content_type) = response.headers().get("content-type") {
        debug!("[InstallationActor] Response content-type: {:?}", content_type);
    }

    // Validate expected download size
    if total_size > 0 && total_size < 50 * 1024 * 1024 { // Less than 50MB is suspicious
        error!("[InstallationActor] Expected download size is suspiciously small: {} bytes ({:.2} MB)", 
               total_size, total_size as f64 / 1024.0 / 1024.0);
        if let Some(event_service) = event_service {
            let _ = event_service.emit_julia_installation_error(
                "Julia download failed",
                &format!("Expected download size too small ({} bytes) - Julia binary should be at least 50MB", total_size)
            ).await;
        }
        return Err(format!("Expected download size too small ({} bytes) - Julia binary should be at least 50MB", total_size));
    }

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(download_path)
        .await
        .map_err(|e| format!("Failed to create download file: {}", e))?;
    
    use tokio::io::AsyncWriteExt;

    debug!("[InstallationActor] Starting to process response stream...");
    let mut chunk_count = 0;
    let mut last_reported_percentage = -1.0;
    
    while let Some(chunk_result) = stream.next().await {
        chunk_count += 1;
        
        let chunk = match chunk_result {
            Ok(chunk) => chunk,
            Err(e) => {
                error!("[InstallationActor] Failed to read chunk #{}: {:?}", chunk_count, e);
                error!("[InstallationActor] Error type: {}", std::any::type_name::<std::io::Error>());
                return Err(format!("Failed to read download chunk #{}: {}", chunk_count, e));
            }
        };
        
        file.write_all(&chunk)
            .await
            .map_err(|e| {
                error!("[InstallationActor] Failed to write chunk #{} to file: {:?}", chunk_count, e);
                format!("Failed to write download chunk #{}: {}", chunk_count, e)
            })?;

        downloaded += chunk.len() as u64;

        // Report progress every 5% of total download
        let progress_percentage = if total_size > 0 {
            (downloaded as f64 / total_size as f64) * 100.0
        } else {
            0.0
        };
        
        // Check if we've crossed a 5% threshold
        // Also report if we're at or near completion (>= 99.5%) to ensure final update
        let current_threshold = (progress_percentage / 5.0).floor() * 5.0;
        let is_complete = total_size > 0 && (downloaded >= total_size || progress_percentage >= 99.5);
        
        if current_threshold > last_reported_percentage || is_complete {
            // If complete, use 100% for the threshold
            if is_complete {
                last_reported_percentage = 100.0;
            } else {
                last_reported_percentage = current_threshold;
            }
            
            debug!(
                "[InstallationActor] Download progress: {} / {} bytes ({:.1}%)",
                downloaded,
                total_size,
                progress_percentage
            );
            
            // Emit progress event to frontend (will appear in Detailed Information)
            if let Some(event_service) = event_service {
                let display_percentage = if is_complete { 100.0 } else { progress_percentage };
                let _ = event_service.emit_julia_installation_progress(
                    &format!("Downloading Julia... ({:.1}%)", display_percentage),
                    30 + (display_percentage * 0.4) as u8 // Scale to 30-70% range
                ).await;
            }
        }
    }

    debug!("[InstallationActor] Stream processing completed. Total chunks processed: {}", chunk_count);
    
    file.flush()
        .await
        .map_err(|e| {
            error!("[InstallationActor] Failed to flush download file: {:?}", e);
            format!("Failed to flush download file: {}", e)
        })?;
    
    debug!(
        "[InstallationActor] Final download size: {} bytes ({:.2} MB)",
        downloaded,
        downloaded as f64 / 1024.0 / 1024.0
    );

    // Verify file exists and has content
    let metadata = tokio::fs::metadata(download_path)
        .await
        .map_err(|e| format!("Failed to get download file metadata: {}", e))?;
    debug!("[InstallationActor] Downloaded file size: {} bytes", metadata.len());

    // Validate download size - Julia binaries should be much larger than 1KB
    if metadata.len() < 50 * 1024 * 1024 { // Less than 50MB is suspicious
        error!("[InstallationActor] Downloaded file is suspiciously small: {} bytes ({:.2} MB)", 
               metadata.len(), metadata.len() as f64 / 1024.0 / 1024.0);
        
        // Read the first few bytes to see if it's an HTML error page
        let mut file = tokio::fs::File::open(download_path)
            .await
            .map_err(|e| format!("Failed to open downloaded file for validation: {}", e))?;
        
        let mut buffer = vec![0; 1024];
        let bytes_read = tokio::io::AsyncReadExt::read(&mut file, &mut buffer)
            .await
            .map_err(|e| format!("Failed to read downloaded file: {}", e))?;
        
        let content_preview = String::from_utf8_lossy(&buffer[..bytes_read]);
        debug!("[InstallationActor] Downloaded file content preview: {}", content_preview);
        
        // Check if it looks like an HTML error page
        if content_preview.contains("<html") || content_preview.contains("<!DOCTYPE") || 
           content_preview.contains("Error") || content_preview.contains("error") ||
           content_preview.contains("404") || content_preview.contains("403") ||
           content_preview.contains("500") {
            error!("[InstallationActor] Downloaded file appears to be an HTML error page, not a Julia binary");
            if let Some(event_service) = event_service {
                let _ = event_service.emit_julia_installation_error(
                    "Julia download failed",
                    &format!("Download failed - received HTML error page instead of Julia binary. Content preview: {}", 
                            content_preview.chars().take(200).collect::<String>())
                ).await;
            }
            return Err(format!("Download failed - received HTML error page instead of Julia binary. Content preview: {}", 
                             content_preview.chars().take(200).collect::<String>()));
        }
        
        if let Some(event_service) = event_service {
            let _ = event_service.emit_julia_installation_error(
                "Julia download failed",
                &format!("Downloaded file is too small ({} bytes) - expected Julia binary to be at least 50MB", metadata.len())
            ).await;
        }
        return Err(format!("Downloaded file is too small ({} bytes) - expected Julia binary to be at least 50MB", metadata.len()));
    }

    debug!("[InstallationActor] Download validation passed - file size looks correct");
    Ok(())
}






















