use tauri::State;

use crate::error::AppError;
use crate::state::AppState;



#[tauri::command]
pub async fn start_orchestrator(app_state: State<'_, AppState>) -> Result<(), AppError> {
    use internals::messages::orchestrator::FrontendReady;
    app_state.actor_system
        .orchestrator_actor
        .send(FrontendReady)
        .await
        .map_err(|_| AppError::InternalError("Actor communication failed".to_string()))??;
    Ok(())
}

/// Continue the orchestrator startup after authentication is complete
#[tauri::command]
pub async fn continue_orchestrator_startup(app_state: State<'_, AppState>) -> Result<(), AppError> {
    use internals::messages::orchestrator::ContinueOrchestratorStartup;
    app_state.actor_system
        .orchestrator_actor
        .send(ContinueOrchestratorStartup)
        .await
        .map_err(|_| AppError::InternalError("Actor communication failed".to_string()))??;
    Ok(())
}
