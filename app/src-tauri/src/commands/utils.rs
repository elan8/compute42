//! Utility functions and helpers for command handlers

use log::{debug, error, info};
use tauri::{Emitter, State};
use std::sync::Arc;
use internals::actor_system::ActorSystem;
use internals::messages::configuration::SetRootFolder;
use internals::messages::orchestrator::ChangeProjectDirectory;
use crate::error::AppError;

// use crate::setup::FinalSetupStatus;
use std::process::Command as StdCommand;

/// Open a URL in the default web browser
#[tauri::command]
pub async fn open_url(url: String) -> Result<(), AppError> {
    let output = if cfg!(target_os = "windows") {
        StdCommand::new("cmd").args(["/C", "start", &url]).output()
    } else if cfg!(target_os = "macos") {
        StdCommand::new("open").arg(&url).output()
    } else {
        StdCommand::new("xdg-open").arg(&url).output()
    };

    match output {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::InternalError(format!("Failed to open URL: {}", e))),
    }
}

/// Query the current setup status
// #[tauri::command]
// pub async fn query_setup_status<'a>(
//     state: State<'a, Mutex<FinalSetupStatus>>,
// ) -> Result<[Option<crate::setup::JuliaInstallationStatusPayload>; 1], String> {
//     let guard = state.lock().await;
//     Ok([guard.0.clone()])
// }

/// Set the last opened folder in orchestrator configuration
#[tauri::command]
pub async fn set_last_opened_folder<'a>(
    path: String,
    actor_system: State<'a, Arc<ActorSystem>>,
    app_handle: tauri::AppHandle,
) -> Result<(), AppError> {
    info!("[Utils] Setting last opened folder to: {}", path);
    
    // Check if this is the same path as the current project to prevent duplicate project switching
    // This prevents the "Switching project" modal from appearing multiple times during startup
    match actor_system.config_actor.send(internals::messages::configuration::GetRootFolder).await {
        Ok(Ok(Some(current_path))) => {
            if current_path == path {
                debug!("[Utils] Project path unchanged ({}), skipping project switching", path);
                return Ok(());
            }
        }
        Ok(Ok(None)) => {
            debug!("[Utils] No current project path set, proceeding with project switching");
        }
        Ok(Err(e)) => {
            debug!("[Utils] Failed to get current project path: {}, proceeding with project switching", e);
        }
        Err(e) => {
            debug!("[Utils] Failed to send GetRootFolder message: {}, proceeding with project switching", e);
        }
    }
    
    actor_system.config_actor.send(SetRootFolder { folder: Some(path.clone()) }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??;
    info!("[Utils] Successfully updated configuration with new project folder: {}", path);

    // Check if it's a valid Julia project
    let project_toml_path = std::path::Path::new(&path).join("Project.toml");
    let is_julia_project = project_toml_path.exists();

    // Emit selected-directory event (unified)
    let payload = serde_json::json!({
        "path": path,
        "is_julia_project": is_julia_project
    });

    app_handle
        .emit("orchestrator:selected-directory", payload)
        .map_err(|e| AppError::InternalError(format!("Failed to emit selected-directory event: {}", e)))?;

    debug!(
        "[Utils] Emitted selected-directory event: {} (Julia project: {})",
        path, is_julia_project
    );

    // Emit project change status events (unified)
    let project_change_status_payload = serde_json::json!({
        "message": "Switching project...",
        "progress_percentage": 10
    });
    
    app_handle
        .emit("orchestrator:project-change-status", project_change_status_payload)
        .map_err(|e| AppError::InternalError(format!("Failed to emit project-change-status event: {}", e)))?;
        
    debug!("[Utils] Emitted project-change-status: Switching project... (10%)");

    // Emit LSP status based on whether it's a Julia project
    if is_julia_project {
        // Update project change status
        let project_change_status_payload = serde_json::json!({
            "message": "Starting Language Server...",
            "progress_percentage": 50
        });
        
        app_handle
            .emit("orchestrator:project-change-status", project_change_status_payload)
            .map_err(|e| format!("Failed to emit project-change-status event: {}", e))?;
            
        debug!("[Utils] Emitted project-change-status: Starting Language Server... (50%)");
        
        // If it's a Julia project, emit LSP starting status and start the LSP server
        let lsp_status_payload = serde_json::json!({
            "status": "starting",
            "message": "Starting Language Server...",
            "error": null,
            "project_path": path
        });
        
        app_handle
            .emit("lsp:status", lsp_status_payload)
            .map_err(|e| AppError::InternalError(format!("Failed to emit LSP status event: {}", e)))?;
            
        debug!("[Utils] Emitted LSP status: starting - Starting Language Server for Julia project");
        
        // Start the LSP server for the Julia project
        // LSP server will be managed by LspActor in the background; just emit UI events here
        if false {
            let e = "LSP start not invoked";
            error!("[Utils] Failed to start LSP server: {}", e);
            
            // Emit LSP failed status
            let lsp_failed_payload = serde_json::json!({
                "status": "failed",
                "message": "Failed to start Language Server",
                "error": e,
                "project_path": path
            });
            
            app_handle
                .emit("lsp:status", lsp_failed_payload)
                .map_err(|e| AppError::InternalError(format!("Failed to emit LSP status event: {}", e)))?;
                
            debug!("[Utils] Emitted LSP status: failed - Failed to start Language Server");
        }
    } else {
        // Update project change status for non-Julia projects
        let project_change_status_payload = serde_json::json!({
            "message": "Project switched (no Language Server needed)",
            "progress_percentage": 80
        });
        
        app_handle
            .emit("orchestrator:project-change-status", project_change_status_payload)
            .map_err(|e| AppError::InternalError(format!("Failed to emit project-change-status event: {}", e)))?;
            
        debug!("[Utils] Emitted project-change-status: Project switched (no Language Server needed) (80%)");
        
        // If it's not a Julia project, emit LSP stopped status
        let lsp_status_payload = serde_json::json!({
            "status": "stopped",
            "message": "No active Julia project",
            "error": null,
            "project_path": null
        });
        
        app_handle
            .emit("lsp:status", lsp_status_payload)
            .map_err(|e| AppError::InternalError(format!("Failed to emit LSP status event: {}", e)))?;
            
        debug!("[Utils] Emitted LSP status: stopped - No active Julia project");
    }
    
    // If it's a Julia project, notify orchestrator to change project (which activates and starts LSP)
    // The orchestrator will emit project-change-complete after activation completes
    if is_julia_project {
        if let Err(e) = actor_system
            .orchestrator_actor
            .send(ChangeProjectDirectory { project_path: path.clone() })
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))
        {
            error!("[Utils] Failed to request project change in orchestrator: {}", e);
        }
        // For Julia projects, project-change-complete will be emitted by orchestrator
        // after project activation completes (when transitioning to StartingLsp phase)
    } else {
        // For non-Julia projects, emit project-change-complete immediately since no activation is needed
        let project_change_complete_payload = serde_json::json!({
            "project_path": path
        });
        
        app_handle
            .emit("orchestrator:project-change-complete", project_change_complete_payload)
            .map_err(|e| AppError::InternalError(format!("Failed to emit project-change-complete event: {}", e)))?;
            
        debug!("[Utils] Emitted project-change-complete for non-Julia project: {}", path);
    }

    Ok(())
}

/// Get system information for the About page
#[tauri::command]
pub async fn get_system_info() -> Result<serde_json::Value, AppError> {
    use std::process::Command;

    let mut info = serde_json::json!({
        "platform": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "julia_version": "Unknown",
        "node_version": "Unknown"
    });

    // Try to get Julia version from embedded installation
    // if let Some(julia_path) = crate::julia_manager::get_julia_executable_path() {
    //     if let Ok(output) = Command::new(julia_path).args(["--version"]).output() {
    //         if output.status.success() {
    //             let version = String::from_utf8_lossy(&output.stdout);
    //             info["julia_version"] = serde_json::Value::String(version.trim().to_string());
    //         }
    //     }
    // }

    // Try to get Node.js version
    if let Ok(output) = Command::new("node").args(["--version"]).output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            info["node_version"] = serde_json::Value::String(version.trim().to_string());
        }
    }

    Ok(info)
}

/// Check if subscription feature is enabled via JJ_SUBSCRIPTION environment variable
#[tauri::command]
pub fn is_subscription_enabled() -> bool {
    match std::env::var("JJ_SUBSCRIPTION") {
        Ok(val) => {
            let val_lower = val.to_lowercase();
            // Enabled if value is "1", "true", or any value other than disabled values
            val_lower != "0" && val_lower != "false" && val_lower != "disabled"
        }
        Err(_) => {
            // Not set - default to disabled
            false
        }
    }
}

/// Check if AI feature is enabled
/// AI feature removed in open-source version
#[tauri::command]
pub fn is_ai_enabled() -> bool {
    // AI feature removed in open-source version
    false
}

/// Centralized function to check if user has Pro subscription
/// In open-source version, all features are available (always returns true)
pub async fn check_pro_subscription(_app_state: &crate::state::AppState) -> Result<bool, AppError> {
    // Open-source version: all features are available
    Ok(true)
}

/// Get application settings (fonts and editor preferences)
#[tauri::command]
pub async fn get_app_settings(
    actor_system: State<'_, Arc<ActorSystem>>,
) -> Result<serde_json::Value, AppError> {
    debug!("[Utils] Getting application settings");
    
    let result = actor_system.config_actor.send(internals::messages::configuration::GetFontSettings).await
        .map_err(|_| AppError::InternalError("Actor communication failed".to_string()))?;
    
    match result {
        Ok(settings) => {
            debug!("[Utils] Successfully retrieved application settings");
            Ok(settings)
        }
        Err(e) => {
            error!("[Utils] Failed to get application settings: {}", e);
            Err(AppError::InternalError(format!("Failed to get settings: {}", e)))
        }
    }
}

/// Get list of available monospace fonts on the system
#[tauri::command]
pub fn get_available_fonts() -> Result<Vec<serde_json::Value>, AppError> {
    use font_kit::source::SystemSource;
    use std::collections::HashSet;

    debug!("[Utils] Enumerating available monospace fonts");
    
    let source = SystemSource::new();
    let mut fonts: Vec<serde_json::Value> = Vec::new();
    let mut seen_names = HashSet::new();

    // Known monospace font names to prioritize
    let preferred_monospace = vec![
        "Fira Code",
        "FiraCode",
        "Consolas",
        "Monaco",
        "Courier New",
        "CourierNew",
        "JetBrains Mono",
        "JetBrainsMono",
        "Source Code Pro",
        "SourceCodePro",
        "Inconsolata",
        "Roboto Mono",
        "RobotoMono",
        "SF Mono",
        "SFMono",
        "DejaVu Sans Mono",
        "DejaVuSansMono",
        "Liberation Mono",
        "LiberationMono",
        "Menlo",
        "Ubuntu Mono",
        "UbuntuMono",
        "Cascadia Code",
        "CascadiaCode",
        "Hack",
        "Anonymous Pro",
        "AnonymousPro",
    ];

    // First, try to find preferred monospace fonts
    for font_name in &preferred_monospace {
        match source.select_family_by_name(font_name) {
            Ok(family_handle) => {
                let family_fonts = family_handle.fonts();

                // For known monospace fonts, we can assume they are monospace
                // Just verify the font family exists and can be loaded
                if !family_fonts.is_empty() && seen_names.insert(font_name.to_string()) {
                    fonts.push(serde_json::json!({
                        "label": font_name.to_string(),
                        "value": format!("\"{}\", monospace", font_name)
                    }));
                }
            }
            Err(_) => {
                // Font not found, skip
                continue;
            }
        }
    }

    // Also enumerate all system fonts and filter for monospace
    // This is done by checking font properties
    match source.all_families() {
        Ok(all_families) => {
            for family_name_str in all_families {
                // Skip if we already added this font
                if seen_names.contains(&family_name_str) {
                    continue;
                }

                // Check if font name suggests monospace (common patterns)
                let lower_name = family_name_str.to_lowercase();
                let is_likely_monospace = lower_name.contains("mono")
                    || lower_name.contains("code")
                    || lower_name.contains("console")
                    || lower_name == "courier"
                    || lower_name == "consolas"
                    || lower_name == "monaco"
                    || lower_name == "menlo";

                if is_likely_monospace {
                    match source.select_family_by_name(&family_name_str) {
                        Ok(_) => {
                            if seen_names.insert(family_name_str.clone()) {
                                fonts.push(serde_json::json!({
                                    "label": family_name_str.clone(),
                                    "value": format!("\"{}\", monospace", family_name_str)
                                }));
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
        Err(e) => {
            error!("[Utils] Failed to enumerate system fonts: {}", e);
            // Return what we have so far (preferred fonts)
        }
    }

    // Sort fonts: preferred first, then alphabetically
    fonts.sort_by(|a, b| {
        let a_label = a["label"].as_str().unwrap_or("");
        let b_label = b["label"].as_str().unwrap_or("");
        
        let a_priority = preferred_monospace.iter().position(|n| a_label == *n).unwrap_or(usize::MAX);
        let b_priority = preferred_monospace.iter().position(|n| b_label == *n).unwrap_or(usize::MAX);
        
        a_priority.cmp(&b_priority).then_with(|| a_label.cmp(b_label))
    });

    debug!("[Utils] Found {} available monospace fonts", fonts.len());
    Ok(fonts)
}

/// Set application settings (fonts and editor preferences)
#[tauri::command]
pub async fn set_app_settings(
    settings: serde_json::Value,
    actor_system: State<'_, Arc<ActorSystem>>,
) -> Result<(), AppError> {
    debug!("[Utils] Setting application settings");
    
    // Parse the settings JSON into SetFontSettings message
    let font_settings = internals::messages::configuration::SetFontSettings {
        editor_font_family: settings.get("editor_font_family")
            .and_then(|v| if v.is_null() { None } else { v.as_str().map(|s| s.to_string()) }),
        editor_font_size: settings.get("editor_font_size")
            .and_then(|v| if v.is_null() { None } else { v.as_u64().map(|n| n as u16) }),
        terminal_font_family: settings.get("terminal_font_family")
            .and_then(|v| if v.is_null() { None } else { v.as_str().map(|s| s.to_string()) }),
        terminal_font_size: settings.get("terminal_font_size")
            .and_then(|v| if v.is_null() { None } else { v.as_u64().map(|n| n as u16) }),
        editor_word_wrap: settings.get("editor_word_wrap")
            .and_then(|v| if v.is_null() { None } else { v.as_bool() }),
        editor_tab_size: settings.get("editor_tab_size")
            .and_then(|v| if v.is_null() { None } else { v.as_u64().map(|n| n as u16) }),
        editor_line_numbers: settings.get("editor_line_numbers")
            .and_then(|v| if v.is_null() { None } else { v.as_bool() }),
        editor_minimap: settings.get("editor_minimap")
            .and_then(|v| if v.is_null() { None } else { v.as_bool() }),
        editor_color_scheme: settings.get("editor_color_scheme")
            .and_then(|v| if v.is_null() { None } else { v.as_str().map(|s| s.to_string()) }),
    };
    
    let result = actor_system.config_actor.send(font_settings).await
        .map_err(|_| AppError::InternalError("Actor communication failed".to_string()))?;
    
    match result {
        Ok(()) => {
            debug!("[Utils] Successfully saved application settings");
            Ok(())
        }
        Err(e) => {
            error!("[Utils] Failed to save application settings: {}", e);
            Err(AppError::InternalError(format!("Failed to save settings: {}", e)))
        }
    }
}
