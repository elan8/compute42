use log::debug;
use tauri::State;
use crate::updater_service::UpdaterService;
use crate::error::AppError;

#[tauri::command]
pub async fn check_for_updates(updater_service: State<'_, UpdaterService>) -> Result<(), AppError> {
    debug!("[UpdaterCommands] Manual update check requested");
    updater_service.manual_check_for_updates().await.map_err(AppError::InternalError)
}

#[tauri::command]
pub async fn get_app_version() -> Result<String, AppError> {
    debug!("[UpdaterCommands] Getting app version");
    // We don't need the service for this, just return the version directly
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

#[tauri::command]
pub async fn get_update_info(updater_service: State<'_, UpdaterService>) -> Result<serde_json::Value, AppError> {
    debug!("[UpdaterCommands] Getting update info");
    updater_service.get_update_info().await.map_err(AppError::InternalError)
}

#[tauri::command]
pub async fn download_and_install_update(updater_service: State<'_, UpdaterService>) -> Result<(), AppError> {
    debug!("[UpdaterCommands] Download and install update requested");
    updater_service.download_and_install_update().await.map_err(AppError::InternalError)
}
