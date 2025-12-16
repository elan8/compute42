use log::{debug, error};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind, MessageDialogButtons};

#[derive(Debug, Serialize, Deserialize)]
struct UpdateCheck {
    last_check: u64,
}

#[derive(Clone)]
pub struct UpdaterService {
    app_handle: AppHandle,
    config_path: PathBuf,
}

impl UpdaterService {
    pub fn new(app_handle: AppHandle) -> Self {
        let config_dir = app_handle
            .path()
            .app_config_dir()
            .expect("Failed to get config directory");
        
        let config_path = config_dir.join("updater_check.json");
        
        Self {
            app_handle,
            config_path,
        }
    }

    pub async fn check_for_updates(&self) {
        // Check if we should run the update check (once per day)
        if !self.should_check_for_updates() {
            debug!("[UpdaterService] Skipping update check - already checked today");
            return;
        }

        debug!("[UpdaterService] Checking for updates...");
        
        match self.perform_update_check().await {
            Ok(_) => {
                debug!("[UpdaterService] Update check completed successfully");
                self.save_last_check_time();
            }
            Err(e) => {
                debug!("[UpdaterService] Update check failed: {}", e);
                // Continue running the app even if update check fails
            }
        }
    }

    // Blocking version for startup - always checks and blocks if update found
    pub async fn check_for_updates_blocking(&self) -> Result<(), String> {
        debug!("[UpdaterService] Checking for updates (blocking mode)...");
        
        debug!("[UpdaterService] About to call perform_update_check...");
        match self.perform_update_check().await {
            Ok(_) => {
                debug!("[UpdaterService] perform_update_check returned Ok");
                debug!("[UpdaterService] Saving last check time...");
                self.save_last_check_time();
                debug!("[UpdaterService] Update check completed successfully, returning Ok");
                Ok(())
            }
            Err(e) => {
                debug!("[UpdaterService] Update check failed: {}", e);
                // Return error but don't fail startup
                Err(e)
            }
        }
    }

    // Manual update check (bypasses daily check restriction)
    pub async fn manual_check_for_updates(&self) -> Result<(), String> {
        debug!("[UpdaterService] Manual update check requested");
        self.perform_update_check().await
    }

    // Get current app version
    pub fn get_app_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    // Get update information
    pub async fn get_update_info(&self) -> Result<serde_json::Value, String> {
        debug!("[UpdaterService] Getting update info");
        
        // Use platform-specific endpoints from tauri.conf.json
        let updater = self.app_handle.updater_builder()
            .build()
            .map_err(|e| format!("Failed to build updater: {}", e))?;
        
        match updater.check().await {
            Ok(update) => {
                if let Some(update) = update {
                    Ok(json!({
                        "available": true,
                        "version": update.version,
                        "body": update.body,
                        "date": update.date.map(|d| d.to_string())
                    }))
                } else {
                    Ok(json!({
                        "available": false,
                        "message": "No updates available"
                    }))
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                
                // Check if this is a deserialization error indicating no update available
                if error_msg.contains("missing field `version`") 
                    || error_msg.contains("failed to deserialize update response") {
                    debug!("[UpdaterService] Server indicates no updates available: {}", error_msg);
                    // Return a user-friendly "no update available" response
                    Ok(json!({
                        "available": false,
                        "message": "No updates available"
                    }))
                } else {
                    debug!("[UpdaterService] Failed to get update info: {}", error_msg);
                    Err(format!("Failed to get update info: {}", error_msg))
                }
            }
        }
    }

    // Download and install update
    pub async fn download_and_install_update(&self) -> Result<(), String> {
        debug!("[UpdaterService] Download and install update requested");
        
        // Use platform-specific endpoints from tauri.conf.json
        let updater = self.app_handle.updater_builder()
            .build()
            .map_err(|e| format!("Failed to build updater: {}", e))?;
        
        match updater.check().await {
            Ok(update) => {
                if let Some(update) = update {
                    debug!("[UpdaterService] Downloading and installing update: version {}", update.version);
                    update.download_and_install(|_, _| {}, || {}).await
                        .map_err(|e| format!("Failed to download and install update: {}", e))?;
                    Ok(())
                } else {
                    debug!("[UpdaterService] No update available to install");
                    Err("No update available".to_string())
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                
                // Check if this is a deserialization error indicating no update available
                if error_msg.contains("missing field `version`") 
                    || error_msg.contains("failed to deserialize update response") {
                    debug!("[UpdaterService] Server indicates no updates available: {}", error_msg);
                    Err("No update available".to_string())
                } else {
                    debug!("[UpdaterService] Update check failed: {}", error_msg);
                    Err(format!("Update check failed: {}", error_msg))
                }
            }
        }
    }

    fn should_check_for_updates(&self) -> bool {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let last_check = self.get_last_check_time();
        let one_day_in_seconds = 24 * 60 * 60;
        
        current_time - last_check >= one_day_in_seconds
    }

    fn get_last_check_time(&self) -> u64 {
        if let Ok(contents) = fs::read_to_string(&self.config_path) {
            if let Ok(update_check) = serde_json::from_str::<UpdateCheck>(&contents) {
                return update_check.last_check;
            }
        }
        0 // Return 0 if file doesn't exist or is invalid
    }

    fn save_last_check_time(&self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let update_check = UpdateCheck {
            last_check: current_time,
        };
        
        // Ensure config directory exists
        if let Some(parent) = self.config_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                error!("[UpdaterService] Failed to create config directory: {}", e);
                return;
            }
        }
        
        if let Ok(json) = serde_json::to_string(&update_check) {
            if let Err(e) = fs::write(&self.config_path, json) {
                error!("[UpdaterService] Failed to save last check time: {}", e);
            }
        }
    }

    async fn perform_update_check(&self) -> Result<(), String> {
        // Use the updater builder to check for updates with platform-specific endpoints
        let updater = self.app_handle.updater_builder()
            .build()
            .map_err(|e| format!("Failed to build updater: {}", e))?;
        
        match updater.check().await {
            Ok(update) => {
                if let Some(update) = update {
                    debug!("[UpdaterService] Update available: version {}", update.version);
                    
                    // Show a confirmation dialog asking the user if they want to install the update
                    let version_str = update.version.to_string();
                    let notes = update.body.as_deref().unwrap_or("No release notes available.");
                    
                    debug!("[UpdaterService] Showing update confirmation dialog...");
                    
                    // Show a non-blocking confirmation dialog asking the user if they want to install
                    // We use a channel to convert the callback-based API to an awaitable future
                    let (tx, rx) = std::sync::mpsc::channel();
                    let tx_clone = tx.clone();
                    
                    debug!("[UpdaterService] Creating dialog and setting up callback...");
                    self.app_handle.dialog()
                        .message(format!(
                            "A new version is available!\n\nVersion: {}\n\n{}\n\nWould you like to install it now?",
                            version_str, notes
                        ))
                        .title("Update Available")
                        .kind(MessageDialogKind::Info)
                        .buttons(MessageDialogButtons::OkCancelCustom("Install Now".to_string(), "Skip".to_string()))
                        .show(move |result| {
                            debug!("[UpdaterService] Dialog callback fired with result: {}", result);
                            let send_result = tx_clone.send(result);
                            if let Err(e) = send_result {
                                debug!("[UpdaterService] Failed to send dialog result to channel: {:?}", e);
                            } else {
                                debug!("[UpdaterService] Dialog result sent to channel successfully");
                            }
                        });
                    
                    debug!("[UpdaterService] Dialog shown, waiting for user response...");
                    // Wait for the user's response without blocking the UI thread
                    // Use spawn_blocking to move the blocking receive to a blocking thread
                    let user_wants_install = tokio::task::spawn_blocking(move || {
                        debug!("[UpdaterService] Entered spawn_blocking, waiting for channel receive...");
                        let result = rx.recv();
                        debug!("[UpdaterService] Channel receive completed, result: {:?}", result);
                        result.unwrap_or(false)
                    }).await.unwrap_or(false);
                    debug!("[UpdaterService] Got user response from dialog: {}", user_wants_install);
                    
                    if user_wants_install {
                        // User chose to install - proceed with download and installation
                        debug!("[UpdaterService] User chose to install update, starting download...");
                        
                        let install_result = update.download_and_install(
                            |chunk_length, content_length| {
                                debug!("[UpdaterService] Download progress: {}/{}", chunk_length, content_length.unwrap_or(0));
                            },
                            || {
                                debug!("[UpdaterService] Update download complete, installing...");
                            }
                        ).await;
                        
                        match install_result {
                            Ok(_) => {
                                debug!("[UpdaterService] Update installation initiated, app will restart");
                                // The app will exit and restart automatically after installation
                                // We return Ok here, but the app won't continue past this point
                                Ok(())
                            }
                            Err(e) => {
                                debug!("[UpdaterService] Update installation failed: {}", e);
                                // Installation failed - continue with startup
                                Ok(())
                            }
                        }
                    } else {
                        // User chose to skip
                        debug!("[UpdaterService] User chose to skip update");
                        debug!("[UpdaterService] Returning Ok from perform_update_check after skip");
                        Ok(())
                    }
                } else {
                    debug!("[UpdaterService] No updates available");
                    Ok(())
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                
                // Check if this is a deserialization error indicating no update available
                // This happens when the server returns {"available": false, "message": "..."}
                // instead of a proper Tauri update manifest
                if error_msg.contains("missing field `version`") 
                    || error_msg.contains("failed to deserialize update response") {
                    debug!("[UpdaterService] Server indicates no updates available (response format issue): {}", error_msg);
                    // Treat this as "no update available" rather than a failure
                    Ok(())
                } else {
                    // Log other errors but don't fail the application
                    debug!("[UpdaterService] Update check failed: {}", error_msg);
                    Ok(()) // Return Ok to continue app execution
                }
            }
        }
    }
}
