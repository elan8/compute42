use log::debug;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn get_file_tree(
    root_path: String,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, AppError> {
    debug!("[Files] Getting file tree for: {}", root_path);
    use internals::messages::filesystem::BuildFileTree;
    Ok(
        app_state
            .actor_system
            .filesystem_actor
            .send(BuildFileTree { root_path })
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}

#[tauri::command]
pub async fn read_file_content(
    path: String,
    app_state: State<'_, AppState>,
) -> Result<String, AppError> {
    debug!("[Files] Reading file: {}", path);
    use internals::messages::filesystem::ReadFileContent;
    Ok(
        app_state
            .actor_system
            .filesystem_actor
            .send(ReadFileContent { path })
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}

#[tauri::command]
pub async fn write_file_content(
    path: String,
    content: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("[Files] Writing file: {}", path);
    use internals::messages::filesystem::WriteFileContent;
    Ok(
        app_state
            .actor_system
            .filesystem_actor
            .send(WriteFileContent { path, content })
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}

#[tauri::command]
pub async fn create_file_item(
    path: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("[Files] Create file: {}", path);
    use internals::messages::filesystem::CreateFile;
    Ok(
        app_state
            .actor_system
            .filesystem_actor
            .send(CreateFile { path })
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}

#[tauri::command]
pub async fn create_folder_item(
    path: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("[Files] Create folder: {}", path);
    use internals::messages::filesystem::CreateDirectory;
    Ok(
        app_state
            .actor_system
            .filesystem_actor
            .send(CreateDirectory { path })
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}

#[tauri::command]
pub async fn delete_item(
    path: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("[Files] Delete entry: {}", path);
    use internals::messages::filesystem::DeleteEntry;
    Ok(
        app_state
            .actor_system
            .filesystem_actor
            .send(DeleteEntry { path })
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}

#[tauri::command]
pub async fn rename_item(
    old_path: String,
    new_path: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("[Files] Rename entry: {} -> {}", old_path, new_path);
    use internals::messages::filesystem::RenameEntry;
    Ok(
        app_state
            .actor_system
            .filesystem_actor
            .send(RenameEntry { old_path, new_path })
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}

#[tauri::command]
pub async fn check_path_exists(
    path: String,
    app_state: State<'_, AppState>,
) -> Result<bool, AppError> {
    debug!("[Files] Path exists?: {}", path);
    use internals::messages::filesystem::PathExists;
    Ok(
        app_state
            .actor_system
            .filesystem_actor
            .send(PathExists { path })
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}

#[tauri::command]
pub async fn load_directory_contents(
    path: String,
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, AppError> {
    debug!("[Files] Loading directory contents for: {}", path);
    use internals::messages::filesystem::LoadDirectoryContents;
    Ok(
        app_state
            .actor_system
            .filesystem_actor
            .send(LoadDirectoryContents { path })
            .await
            .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??,
    )
}


