use log::debug;
use tauri::State;
use uuid::Uuid;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn read_project_toml(
    project_path: String,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, AppError> {
    debug!("[Projects] Reading Project.toml for: {}", project_path);
    app_state
            .actor_system
            .read_project_toml(project_path)
            .await
            .map_err(AppError::InternalError)
}

#[tauri::command]
pub async fn write_project_toml(
    config: serde_json::Value,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("[Projects] Writing Project.toml");
    app_state
        .actor_system
        .write_project_toml(config)
        .await
        .map_err(AppError::InternalError)
}

#[tauri::command]
pub async fn generate_uuid() -> Result<String, AppError> {
    debug!("[Projects] Generating new UUID");
    let uuid = Uuid::new_v4();
    Ok(uuid.to_string())
}


