use log::debug;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn get_session_status(app_state: State<'_, AppState>) -> Result<bool, AppError> {
    use internals::messages::communication::IsConnected;
    Ok(
        app_state
            .actor_system
            .communication_actor
            .send(IsConnected)
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}

#[tauri::command]
pub async fn init_terminal_session(_app_state: State<'_, AppState>) -> Result<String, AppError> {
    debug!("[Process] Initialize terminal session");
    Ok("session_1".to_string())
}

#[tauri::command]
pub async fn restart_julia(app_state: State<'_, AppState>) -> Result<(), AppError> {
    debug!("[Process] Restart Julia");
    use internals::messages::process::RestartJulia;
    Ok(
        app_state
            .actor_system
            .process_actor
            .send(RestartJulia)
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}

#[tauri::command]
pub async fn is_backend_ready(app_state: State<'_, AppState>) -> Result<bool, AppError> {
    use internals::messages::communication::IsConnected;
    Ok(
        app_state
            .actor_system
            .communication_actor
            .send(IsConnected)
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}

#[tauri::command]
pub async fn get_backend_busy_status(app_state: State<'_, AppState>) -> Result<bool, AppError> {
    use internals::messages::communication::GetBackendBusyStatus;
    Ok(
        app_state
            .actor_system
            .communication_actor
            .send(GetBackendBusyStatus)
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}


