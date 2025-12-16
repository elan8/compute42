use tauri::{command, State};
use crate::state::AppState;
use crate::error::AppError;
use internals::messages::file_server::{StartFileServer, StopFileServer, GetFileServerUrl, IsFileServerRunning};

#[command]
pub async fn start_file_server(app_state: State<'_, AppState>, _base_path: String) -> Result<u16, AppError> {
    Ok(
        app_state
            .actor_system
            .file_server_actor
            .send(StartFileServer { orchestrator_addr: None })
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}

#[command]
pub async fn stop_file_server(app_state: State<'_, AppState>) -> Result<(), AppError> {
    app_state.actor_system.file_server_actor.send(StopFileServer).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??;
    Ok(())
}

#[command]
pub async fn get_file_server_url(app_state: State<'_, AppState>) -> Result<String, AppError> {
    match app_state.actor_system.file_server_actor.send(GetFileServerUrl).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(Some(url)) => Ok(url),
        Ok(None) => Err(AppError::ValidationError("File server is not running".to_string())),
        Err(e) => Err(AppError::InternalError(e)),
    }
}

#[command]
pub async fn is_file_server_running(app_state: State<'_, AppState>) -> Result<bool, AppError> {
    Ok(
        app_state
            .actor_system
            .file_server_actor
            .send(IsFileServerRunning)
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}
