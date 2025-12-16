use log::debug;
use tauri::State;
use crate::state::AppState;
use internals::types::Tab;

// ============================================================================
// Tab Management Commands
// ============================================================================

/// Add a new tab to the state manager
#[tauri::command]
pub async fn add_tab(tab: Tab, app_state: State<'_, AppState>) -> Result<(), String> {
    debug!("[TabCommands] Adding tab: {}", tab.id);

    use internals::messages::state::AddTab;
    app_state.actor_system.state_actor.send(AddTab { tab }).await.map_err(|_| "Actor comm failed".to_string())?
}

/// Remove a tab from the state manager
#[tauri::command]
pub async fn remove_tab(
    tab_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("[TabCommands] Removing tab: {}", tab_id);

    use internals::messages::state::RemoveTab;
    app_state.actor_system.state_actor.send(RemoveTab { tab_id }).await.map_err(|_| "Actor comm failed".to_string())?
}

/// Get all tabs from the state manager
#[tauri::command]
pub async fn get_tabs(app_state: State<'_, AppState>) -> Result<Vec<Tab>, String> {
    use internals::messages::state::GetTabs;
    app_state.actor_system.state_actor.send(GetTabs).await.map_err(|_| "Actor comm failed".to_string())?
}

/// Update a tab in the state manager
#[tauri::command]
pub async fn update_tab(
    tab_id: String,
    updated_tab: Tab,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    use internals::messages::state::UpdateTab;
    app_state.actor_system.state_actor.send(UpdateTab { tab_id, updated_tab }).await.map_err(|_| "Actor comm failed".to_string())?
}

/// Clear all tabs from the state manager
#[tauri::command]
pub async fn clear_tabs(app_state: State<'_, AppState>) -> Result<(), String> {
    debug!("[TabCommands] Clearing all tabs");

    use internals::messages::state::ClearTabs;
    app_state.actor_system.state_actor.send(ClearTabs).await.map_err(|_| "Actor comm failed".to_string())?
}

/// Update tab content
#[tauri::command]
pub async fn update_tab_content(
    tab_id: String,
    content: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    use internals::messages::state::UpdateTabContent;
    app_state.actor_system.state_actor.send(UpdateTabContent { tab_id, content }).await.map_err(|_| "Actor comm failed".to_string())?
}

/// Save tab content to file
#[tauri::command]
pub async fn save_tab_to_file(
    tab_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("[TabCommands] Saving tab to file: {}", tab_id);

    use internals::messages::state::SaveTabToFile;
    app_state.actor_system.state_actor.send(SaveTabToFile { tab_id }).await.map_err(|_| "Actor comm failed".to_string())?
}

/// Get dirty tabs
#[tauri::command]
pub async fn get_dirty_tabs(
    app_state: State<'_, AppState>,
) -> Result<Vec<Tab>, String> {
    debug!("[TabCommands] Getting dirty tabs");

    use internals::messages::state::GetDirtyTabs;
    app_state.actor_system.state_actor.send(GetDirtyTabs).await.map_err(|_| "Actor comm failed".to_string())?
}
