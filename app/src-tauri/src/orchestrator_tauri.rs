use log::debug;
use tauri::State;
use crate::state::AppState;
use internals::messages::orchestrator::RestartJuliaOrchestrator;

#[tauri::command]
pub async fn restart_julia_orchestrator(app_state: State<'_, AppState>) -> Result<(), String> {
    debug!("[OrchestratorTauri] Restarting Julia orchestrator");
    
    // Send restart message to orchestrator actor
    app_state
        .actor_system
        .orchestrator_actor
        .send(RestartJuliaOrchestrator)
        .await
        .map_err(|_| "Failed to send restart message to orchestrator")??;
    
    debug!("[OrchestratorTauri] Julia orchestrator restart message sent successfully");
    Ok(())
}


