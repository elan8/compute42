use log::{debug, error};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tauri::State;
use tokio::fs as async_fs;
use toml;

use internals::services::base::file_utils::build_tree;
use crate::state::AppState;
use crate::error::AppError;
// Import specific messages as needed
use internals::messages::execution::{ExecuteFile, ExecuteApiRequest, ActivateProject, DeactivateProject};
use internals::messages::communication::{IsConnected, ExecuteCode};
use internals::messages::orchestrator::ChangeProjectDirectory;
use internals::messages::installation::GetJuliaPathFromInstallation;
use internals::messages::process::RestartJulia;

// Project.toml configuration structures
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ProjectTomlConfig {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub uuid: Option<String>,
    pub license: Option<String>,
    pub deps: Option<toml::Table>,
    pub compat: Option<toml::Table>,
    pub sources: Option<toml::Table>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ProjectTomlWriteConfig {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub uuid: Option<String>,
    pub license: Option<String>,
    pub project_path: String,
}

// ============================================================================
// File Operations
// ============================================================================

/// Get file tree for a given root path
// moved: files::get_file_tree
#[allow(dead_code)]
async fn get_file_tree(
    root_path: String,
    _app_state: State<'_, AppState>,
) -> Result<Value, String> {
    debug!(
        "[OrchestratorCommands] Getting file tree for: {}",
        root_path
    );

    let path = PathBuf::from(root_path);
    if !path.is_dir() {
        debug!("[OrchestratorCommands] Provided path is not a directory");
        return Err("Provided path is not a directory".to_string());
    }

    match build_tree(&path).await {
        Ok(file_node) => {
            debug!("[OrchestratorCommands] Successfully built file tree");
            Ok(serde_json::to_value(file_node)
                .map_err(|e| format!("Failed to serialize file tree: {}", e))?)
        }
        Err(e) => {
            error!("[OrchestratorCommands] Failed to build file tree: {}", e);
            Err(e)
        }
    }
}

/// Read file content
// moved: files::read_file_content
#[allow(dead_code)]
async fn read_file_content(
    path: String,
    _app_state: State<'_, AppState>,
) -> Result<String, String> {
    debug!("[OrchestratorCommands] Reading file content: {}", path);

    async_fs::read_to_string(path.clone()).await.map_err(|e| {
        error!("[OrchestratorCommands] Failed to read file content");
        e.to_string()
    })
}

/// Write file content
// moved: files::write_file_content
#[allow(dead_code)]
async fn write_file_content(
    path: String,
    content: String,
    _app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("[OrchestratorCommands] Writing file content: {}", path);

    async_fs::write(path.clone(), content).await.map_err(|e| {
        error!("[OrchestratorCommands] Failed to write file content");
        e.to_string()
    })
}

/// Create a new file
// moved: files::create_file_item
#[allow(dead_code)]
async fn create_file_item(
    path: String,
    _app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("[OrchestratorCommands] Creating file: {}", path);

    if Path::new(&path).exists() {
        debug!("[OrchestratorCommands] Attempted to create file that already exists");
        return Err("File already exists".to_string());
    }
    async_fs::File::create(path.clone()).await.map_err(|e| {
        error!("[OrchestratorCommands] Failed to create file");
        e.to_string()
    })?;
    Ok(())
}

/// Create a new folder
// moved: files::create_folder_item
#[allow(dead_code)]
async fn create_folder_item(
    path: String,
    _app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("[OrchestratorCommands] Creating folder: {}", path);

    if Path::new(&path).exists() {
        debug!("[OrchestratorCommands] Attempted to create folder that already exists");
        return Err("Folder already exists".to_string());
    }
    async_fs::create_dir_all(path.clone()).await.map_err(|e| {
        error!("[OrchestratorCommands] Failed to create folder");
        e.to_string()
    })
}

/// Delete an item (file or folder)
// moved: files::delete_item
#[allow(dead_code)]
async fn delete_item(
    path: String,
    _app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("[OrchestratorCommands] Deleting item: {}", path);

    let path_obj = PathBuf::from(path.clone());
    if path_obj.is_dir() {
        async_fs::remove_dir_all(path_obj).await.map_err(|e| {
            error!("[OrchestratorCommands] Failed to remove directory");
            e.to_string()
        })
    } else if path_obj.is_file() {
        async_fs::remove_file(path_obj).await.map_err(|e| {
            error!("[OrchestratorCommands] Failed to remove file");
            e.to_string()
        })
    } else {
        debug!("[OrchestratorCommands] Attempted to delete item that does not exist or is not a file/directory");
        Err("Item does not exist or is not a file/directory".to_string())
    }
}

/// Rename an item
// moved: files::rename_item
#[allow(dead_code)]
async fn rename_item(
    old_path: String,
    new_path: String,
    _app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!(
        "[OrchestratorCommands] Renaming item from {} to {}",
        old_path, new_path
    );

    // Check if old path exists
    if !Path::new(&old_path).exists() {
        debug!(
            "[OrchestratorCommands] Attempted to rename item that does not exist: {}",
            old_path
        );
        return Err("Item does not exist".to_string());
    }

    // Check if new path already exists
    if Path::new(&new_path).exists() {
        debug!(
            "[OrchestratorCommands] Attempted to rename to path that already exists: {}",
            new_path
        );
        return Err("Target path already exists".to_string());
    }

    async_fs::rename(old_path.clone(), new_path.clone())
        .await
        .map_err(|e| {
            error!(
                "[OrchestratorCommands] Failed to rename item from {} to {}: {}",
                old_path, new_path, e
            );
            e.to_string()
        })
}

/// Check if a path exists
// moved: files::check_path_exists
#[allow(dead_code)]
async fn check_path_exists(
    path: String,
    _app_state: State<'_, AppState>,
) -> Result<bool, String> {
    debug!("[OrchestratorCommands] Checking if path exists: {}", path);

    let exists = Path::new(&path).exists();
    debug!("[OrchestratorCommands] Path exists: {}", exists);
    Ok(exists)
}

/// Start file watcher for a path
#[tauri::command]
pub async fn start_file_watcher(
    path: String,
    _app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("[OrchestratorCommands] Starting file watcher for: {}", path);

    // TODO: Implement using orchestrator state service
    // For now, just log the operation
    debug!("Would start file watcher for: {}", path);
    Ok(())
}

/// Stop file watcher
#[tauri::command]
pub async fn stop_file_watcher(_app_state: State<'_, AppState>) -> Result<(), String> {
    debug!("[OrchestratorCommands] Stopping file watcher");

    // TODO: Implement using orchestrator state service
    // For now, just log the operation
    debug!("Would stop file watcher");
    Ok(())
}

/// Read project.toml file
// moved: projects::read_project_toml
#[allow(dead_code)]
async fn read_project_toml(
    project_path: String,
    _app_state: State<'_, AppState>,
) -> Result<Value, String> {
    debug!(
        "[OrchestratorCommands] Reading project.toml: {}",
        project_path
    );

    let project_toml_path = std::path::Path::new(&project_path).join("Project.toml");

    if !project_toml_path.exists() {
        return Err("Project.toml file does not exist".to_string());
    }

    let content = async_fs::read_to_string(&project_toml_path)
        .await
        .map_err(|e| format!("Failed to read Project.toml: {}", e))?;

    let parsed: toml::Value = content
        .parse()
        .map_err(|e| format!("Failed to parse Project.toml: {}", e))?;

    let mut config = ProjectTomlConfig {
        name: None,
        version: None,
        description: None,
        authors: None,
        uuid: None,
        license: None,
        deps: None,
        compat: None,
        sources: None,
    };

    // Extract values from the parsed TOML
    if let Some(name) = parsed.get("name") {
        config.name = name.as_str().map(|s| s.to_string());
    }

    if let Some(version) = parsed.get("version") {
        config.version = version.as_str().map(|s| s.to_string());
    }

    if let Some(description) = parsed.get("description") {
        config.description = description.as_str().map(|s| s.to_string());
    }

    if let Some(authors) = parsed.get("authors") {
        if let Some(authors_array) = authors.as_array() {
            let authors_str = authors_array
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            config.authors = Some(authors_str);
        }
    }

    if let Some(uuid) = parsed.get("uuid") {
        config.uuid = uuid.as_str().map(|s| s.to_string());
    }

    if let Some(license) = parsed.get("license") {
        config.license = license.as_str().map(|s| s.to_string());
    }

    // Handle sections
    if let Some(deps) = parsed.get("deps") {
        if let Some(deps_table) = deps.as_table() {
            config.deps = Some(deps_table.clone());
        }
    }

    if let Some(compat) = parsed.get("compat") {
        if let Some(compat_table) = compat.as_table() {
            config.compat = Some(compat_table.clone());
        }
    }

    if let Some(sources) = parsed.get("sources") {
        if let Some(sources_table) = sources.as_table() {
            config.sources = Some(sources_table.clone());
        }
    }

    // Note: Project.toml fields are not mandatory, so we don't validate them as required

    debug!(
        "[OrchestratorCommands] Successfully read Project.toml: {:?}",
        config
    );
    serde_json::to_value(config)
        .map_err(|e| format!("Failed to serialize project.toml: {}", e))
}

/// Write project.toml file
// moved: projects::write_project_toml
#[allow(dead_code)]
async fn write_project_toml(
    config: Value,
    _app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("[OrchestratorCommands] Writing project.toml");

    // Parse the config from JSON
    let config: ProjectTomlWriteConfig =
        serde_json::from_value(config).map_err(|e| format!("Failed to parse config: {}", e))?;

    let project_toml_path = std::path::Path::new(&config.project_path).join("Project.toml");

    // Check if Project.toml exists
    let project_exists = project_toml_path.exists();

    if project_exists {
        debug!("[OrchestratorCommands] Updating existing Project.toml...");
    } else {
        debug!("[OrchestratorCommands] Creating new Project.toml");
    }

    // Read existing TOML content if file exists
    let mut toml_table = if project_exists {
        let content = async_fs::read_to_string(&project_toml_path)
            .await
            .map_err(|e| format!("Failed to read Project.toml: {}", e))?;

        content
            .parse::<toml::Table>()
            .map_err(|e| format!("Failed to parse existing Project.toml: {}", e))?
    } else {
        toml::Table::new()
    };

    // Update the TOML table with new values
    if let Some(name) = &config.name {
        toml_table.insert("name".to_string(), toml::Value::String(name.clone()));
    }

    if let Some(version) = &config.version {
        toml_table.insert("version".to_string(), toml::Value::String(version.clone()));
    }

    if let Some(description) = &config.description {
        if !description.trim().is_empty() {
            toml_table.insert(
                "description".to_string(),
                toml::Value::String(description.clone()),
            );
        }
    }

    if let Some(authors) = &config.authors {
        if !authors.trim().is_empty() {
            let authors_array: Vec<toml::Value> = authors
                .split('\n')
                .filter(|s| !s.trim().is_empty())
                .map(|s| toml::Value::String(s.trim().to_string()))
                .collect();
            toml_table.insert("authors".to_string(), toml::Value::Array(authors_array));
        }
    }

    if let Some(uuid) = &config.uuid {
        toml_table.insert("uuid".to_string(), toml::Value::String(uuid.clone()));
    }

    if let Some(license) = &config.license {
        if !license.trim().is_empty() {
            toml_table.insert("license".to_string(), toml::Value::String(license.clone()));
        }
    }

    // Note: We preserve existing [deps], [compat], and [sources] sections
    // by reading the existing file and not overwriting them

    // Convert to TOML string
    let toml_string = toml::to_string(&toml_table)
        .map_err(|e| format!("Failed to serialize Project.toml: {}", e))?;

    // Write to file
    async_fs::write(&project_toml_path, toml_string)
        .await
        .map_err(|e| format!("Failed to write Project.toml: {}", e))?;

    debug!(
        "[OrchestratorCommands] Successfully wrote Project.toml to: {:?}",
        project_toml_path
    );
    Ok(())
}

// ============================================================================
// Julia Operations
// ============================================================================

/// Execute Julia code
#[tauri::command]
pub async fn execute_julia_code(
    code: String,
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    debug!("[OrchestratorCommands] Executing Julia code");

    use internals::messages::execution::ExecuteReplRequest;
    app_state.actor_system.execution_actor.send(ExecuteReplRequest { code }).await.map_err(|_| "Actor comm failed".to_string())?
}

/// Execute notebook cell with proper event routing
#[tauri::command]
pub async fn execute_notebook_cell(
    cell_id: String,
    code: String,
    notebook_path: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    debug!("[OrchestratorCommands] Executing notebook cell: {}", cell_id);

    use internals::messages::execution::ExecuteNotebookCell;
    use internals::messages::process::SetOutputSuppression;

    // Suppress terminal output before executing notebook cell
    let _ = app_state.actor_system.process_actor
        .send(SetOutputSuppression { suppressed: true })
        .await;
    debug!("[OrchestratorCommands] Suppressed terminal output for notebook cell execution");

    // Execute the notebook cell
    let result = app_state.actor_system.execution_actor
        .send(ExecuteNotebookCell { cell_id, code, notebook_path })
        .await
        .map_err(|_| "Actor comm failed".to_string())?;

    // Restore terminal output after execution completes
    let _ = app_state.actor_system.process_actor
        .send(SetOutputSuppression { suppressed: false })
        .await;
    debug!("[OrchestratorCommands] Restored terminal output after notebook cell execution");

    result
}

/// Execute multiple notebook cells in batch (emits busy/done only at start/end)
#[tauri::command]
pub async fn execute_notebook_cells_batch(
    cells: Vec<(String, String)>, // Vec of (cell_id, code) tuples
    notebook_path: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<Vec<(String, Result<String, String>)>, String> {
    debug!("[OrchestratorCommands] Executing notebook cells batch: {} cells", cells.len());

    use internals::messages::execution::{ExecuteNotebookCellsBatch, NotebookCellBatchItem};
    use internals::messages::process::SetOutputSuppression;

    // Suppress terminal output before executing notebook cells
    let _ = app_state.actor_system.process_actor
        .send(SetOutputSuppression { suppressed: true })
        .await;
    debug!("[OrchestratorCommands] Suppressed terminal output for notebook cells batch execution");

    // Convert to batch items
    let batch_items: Vec<NotebookCellBatchItem> = cells
        .into_iter()
        .map(|(cell_id, code)| NotebookCellBatchItem { cell_id, code, notebook_path: notebook_path.clone() })
        .collect();

    // Execute the notebook cells batch
    let results = app_state.actor_system.execution_actor
        .send(ExecuteNotebookCellsBatch { cells: batch_items })
        .await
        .map_err(|_| "Actor comm failed".to_string())?
        .map_err(|e| format!("Batch execution failed: {}", e))?;

    // Restore terminal output after execution completes
    let _ = app_state.actor_system.process_actor
        .send(SetOutputSuppression { suppressed: false })
        .await;
    debug!("[OrchestratorCommands] Restored terminal output after notebook cells batch execution");

    Ok(results)
}

/// Execute Julia file
#[tauri::command]
pub async fn execute_julia_file(
    file_path: String,
    _file_content: String,
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    debug!("[OrchestratorCommands] Executing Julia file: {}", file_path);

    app_state.actor_system
        .execution_actor
        .send(ExecuteFile { file_path })
        .await
        .map_err(|_| "Actor comm failed".to_string())?
}

/// Trigger workspace variables refresh
#[tauri::command]
pub async fn refresh_workspace_variables(
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("[OrchestratorCommands] Triggering workspace variables refresh");

    // Use CommunicationActor to trigger workspace variables update
    use internals::messages::ExecutionType;
    
    // Send ExecuteCode message to CommunicationActor
    let result = app_state.actor_system.communication_actor.send(ExecuteCode {
        code: "get_workspace_variables()".to_string(),
        execution_type: ExecutionType::ApiCall,
        file_path: None,
        suppress_busy_events: false,
    }).await
        .map_err(|e| format!("Failed to send execute code message: {}", e))?
        .map_err(|e| format!("Failed to execute workspace variables request: {}", e))?;
    
    // Result is a JuliaMessage, we just need to ensure it executed
    debug!("[OrchestratorCommands] Workspace variables refresh triggered, result: {:?}", result);
    
    Ok(())
}


/// Get full value of a specific variable
#[tauri::command]
pub async fn get_variable_value(
    variable_name: String,
    app_state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    debug!("[OrchestratorCommands] Getting value for variable: {}", variable_name);

    // Send GetVariableValue message to Julia via CommunicationService
    let code = format!(r#"
        try
            using JSON
            value = get_variable_value("{}")
            if value !== nothing
                JSON.json(Dict("variable_name" => "{}", "value" => value))
            else
                JSON.json(Dict("variable_name" => "{}", "value" => nothing))
            end
        catch e
            println(stderr, "Error getting variable value: ", e)
            JSON.json(Dict("variable_name" => "{}", "value" => nothing))
        end
    "#, variable_name, variable_name, variable_name, variable_name);
    
    use internals::messages::execution::ExecuteApiRequest;
    match app_state.actor_system.execution_actor.send(ExecuteApiRequest { code }).await {
        Ok(Ok(result_str)) => {
            // Parse the JSON response
            match serde_json::from_str::<Value>(&result_str) {
                Ok(json) => {
                    if let Some(value) = json.get("value") {
                        if let Some(value_str) = value.as_str() {
                            // Check if this is a DataFrame by looking for DataFrame pattern
                            if value_str.contains("DataFrame") && value_str.contains("Row │") {
                                // This is a DataFrame, parse it into structured data
                                use internals::services::base::variable_utils::process_variable_data;
                                let var_data = serde_json::json!({
                                    "name": "df",
                                    "type": "DataFrame",
                                    "value": value_str,
                                    "is_dataframe": true
                                });
                                let processed = process_variable_data(var_data);
                                
                                // Return the processed DataFrame data
                                if let Some(parsed_data) = processed.get("parsed_data") {
                                    Ok(Some(parsed_data.to_string()))
                                } else {
                                    // Fallback to raw value if parsing failed
                                    Ok(Some(value_str.to_string()))
                                }
                            } else {
                                // Clean the value to remove type prefixes for arrays
                                use internals::services::base::variable_utils::clean_array_string;
                                let cleaned_value = clean_array_string(value_str);
                                Ok(Some(cleaned_value))
                            }
                        } else {
                            Ok(None)
                        }
                    } else {
                        Ok(None)
                    }
                }
                Err(_) => Ok(None)
            }
        }
        _ => Err("Failed to get variable value".to_string())
    }
}

/// Get session status
// moved: process::get_session_status
#[allow(dead_code)]
async fn get_session_status(
    app_state: State<'_, AppState>,
) -> Result<bool, String> {
    debug!("[OrchestratorCommands] Getting session status");

    // Use CommunicationActor connectivity
    app_state.actor_system
        .communication_actor
        .send(IsConnected)
        .await
        .map_err(|_| "Actor comm failed".to_string())?
}



// ============================================================================
// Orchestrator Lifecycle Management
// ============================================================================

/// Signal that the frontend is ready to receive events
/// This is part of the backend-frontend handshake mechanism
#[tauri::command]
pub async fn frontend_ready_handshake(
    app_state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), AppError> {
    use tauri::Manager;
    debug!("[OrchestratorCommands] Frontend ready handshake signal received");
    
    // Spawn update check thread now that UI is confirmed ready
    // Then wait for it to complete before continuing startup
    if let (Some(update_tx), Some(update_rx)) = (
        app_handle.try_state::<std::sync::mpsc::Sender<()>>(),
        app_handle.try_state::<std::sync::Mutex<std::sync::mpsc::Receiver<()>>>()
    ) {
        debug!("[OrchestratorCommands] UI is ready, spawning update check thread...");
        
        // Get updater service from app state
        let updater_service = app_handle.state::<crate::updater_service::UpdaterService>();
        let updater_service_clone = updater_service.inner().clone();
        let update_tx_clone = update_tx.inner().clone();
        
        // Spawn update check thread - it will signal when done
        std::thread::spawn(move || {
            debug!("[OrchestratorCommands] Update check thread started");
            
            debug!("[OrchestratorCommands] Creating Tokio runtime for update check...");
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime for update check");
            
            debug!("[OrchestratorCommands] Running update check in Tokio runtime...");
            rt.block_on(async {
                debug!("[OrchestratorCommands] Calling check_for_updates_blocking...");
                match updater_service_clone.check_for_updates_blocking().await {
                    Ok(_) => {
                        debug!("[OrchestratorCommands] Update check completed successfully");
                    }
                    Err(e) => {
                        debug!("[OrchestratorCommands] Update check completed with error: {}", e);
                    }
                }
                debug!("[OrchestratorCommands] Update check thread finishing, sending continue signal...");
            });
            
            // Send continue signal to main thread
            let _ = update_tx_clone.send(());
            debug!("[OrchestratorCommands] Update check thread exited");
        });
        
        debug!("[OrchestratorCommands] Update check thread spawned, waiting for completion...");
        // Wait for the update check thread to signal completion
        // This will block until: no update found, user skips, or user installs (app will restart)
        let rx = update_rx.lock().unwrap();
        match rx.recv() {
            Ok(_) => {
                debug!("[OrchestratorCommands] Update check completed, transitioning orchestrator to WaitingForAuth");
            }
            Err(e) => {
                debug!("[OrchestratorCommands] Error waiting for update check: {:?}, continuing anyway", e);
            }
        }
    } else {
        debug!("[OrchestratorCommands] Update check coordination not found, skipping update check");
    }
    
    // Don't initialize actors here - the startup state machine will handle initialization
    // AccountActor will be initialized when entering WaitingForAuth state
    // InstallationActor will be initialized when entering CheckingJulia state
    
    // Check if orchestrator is already completed before sending FrontendReady
    use internals::messages::orchestrator::GetStartupPhase;
    let current_phase = app_state.actor_system
        .orchestrator_actor
        .send(GetStartupPhase)
        .await
        .map_err(|_| AppError::InternalError("Actor communication failed".to_string()))??;
    
    if current_phase == "Completed" {
        debug!("[OrchestratorCommands] Orchestrator already completed (phase: {}), skipping FrontendReady", current_phase);
        return Ok(());
    }
    
    // Start the orchestrator - it will be in CheckingForUpdates phase
    use internals::messages::orchestrator::FrontendReady;
    app_state.actor_system
        .orchestrator_actor
        .send(FrontendReady)
        .await
        .map_err(|_| AppError::InternalError("Actor communication failed".to_string()))??;
    
    // After update check completes, transition orchestrator from CheckingForUpdates to WaitingForAuth
    use internals::messages::orchestrator::ContinueOrchestratorStartup;
    app_state.actor_system
        .orchestrator_actor
        .send(ContinueOrchestratorStartup)
        .await
        .map_err(|_| AppError::InternalError("Actor communication failed".to_string()))??;
    Ok(())
}

/// Handle project changed event - manages project transition
#[tauri::command]
pub async fn project_changed(
    project_path: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!(
        "[OrchestratorCommands] Project changed event received: {}",
        project_path
    );

    app_state.actor_system
        .orchestrator_actor
        .send(ChangeProjectDirectory { project_path })
        .await
        .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??;
    Ok(())
}

/// Check if backend is ready
// moved: process::is_backend_ready
#[allow(dead_code)]
async fn is_backend_ready(app_state: State<'_, AppState>) -> Result<bool, String> {
    debug!("[OrchestratorCommands] Checking if backend is ready");

    app_state.actor_system
        .communication_actor
        .send(IsConnected)
        .await
        .map_err(|_| "Actor comm failed".to_string())?
}

/// Get orchestrator startup phase
#[tauri::command]
pub async fn get_orchestrator_startup_phase(
    app_state: State<'_, AppState>,
) -> Result<String, AppError> {
    debug!("[OrchestratorCommands] Getting orchestrator startup phase");
    
    use internals::messages::orchestrator::GetStartupPhase;
    let phase = app_state.actor_system
        .orchestrator_actor
        .send(GetStartupPhase)
        .await
        .map_err(|_| AppError::InternalError("Actor communication failed".to_string()))??;
    
    Ok(phase)
}

/// Activate Julia project
#[tauri::command]
pub async fn activate_julia_project_process(
    project_path: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!(
        "[OrchestratorCommands] Activating Julia project: {}",
        project_path
    );

    app_state.actor_system
        .execution_actor
        .send(ActivateProject { project_path })
        .await
        .map_err(|_| "Actor comm failed".to_string())?
}

/// Close terminal session
#[tauri::command]
pub async fn close_terminal_session(
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("[OrchestratorCommands] Closing terminal session");

    app_state.actor_system
        .execution_actor
        .send(DeactivateProject)
        .await
        .map_err(|_| "Actor comm failed".to_string())?
}

/// Initialize terminal session
// moved: process::init_terminal_session
#[allow(dead_code)]
async fn init_terminal_session(
    _app_state: State<'_, AppState>,
) -> Result<String, String> {
    debug!("[OrchestratorCommands] Initializing terminal session");

    // For now, return a session ID - in a real implementation this would create a new session
    Ok("session_1".to_string())
}

// ============================================================================
// Project Management
// ============================================================================

/// Get Julia project data
#[tauri::command]
pub async fn get_julia_project_data(
    project_path: String,
    _app_state: State<'_, AppState>,
) -> Result<Value, String> {
    debug!(
        "[OrchestratorCommands] Getting Julia project data: {}",
        project_path
    );

    // TODO: Implement using orchestrator file service
    // For now, return a placeholder
    Ok(serde_json::json!({
        "name": "placeholder_project",
        "version": "0.1.0",
        "dependencies": []
    }))
}

/// Create new Julia project
#[tauri::command]
pub async fn create_new_julia_project(
    project_path: String,
    project_name: String,
    authors: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!(
        "[OrchestratorCommands] Creating new Julia project: {} at {}",
        project_name, project_path
    );

    // Convert Windows paths to Julia-compatible format
    let julia_project_path = project_path.replace('\\', "/");
    
    // Create the Julia code to generate the project with custom Project.toml
    let authors_field = match &authors {
        Some(authors) if !authors.trim().is_empty() => format!("authors = [\"{}\"]", authors.replace("\"", "\\\"")),
        _ => "authors = []".to_string(),
    };
    
    let julia_code = format!(
        "using Pkg\n\n# Change to the target directory first\ncd(\"{}\")\n\n# Create project directory\nproject_dir = \"{}\"\nmkpath(project_dir)\nmkpath(joinpath(project_dir, \"src\"))\n\n# Create custom Project.toml\nproject_toml_content = \"\"\"\nname = \"{}\"\nuuid = \"$(uuid4())\"\nversion = \"0.1.0\"\n{}\n\n[deps]\n\"\"\"\n\nproject_toml_file = joinpath(project_dir, \"Project.toml\")\nopen(project_toml_file, \"w\") do file\n    write(file, project_toml_content)\nend\n\n# Create the main source file with Hello World example\nproject_src_file = joinpath(project_dir, \"src\", \"{}.jl\")\nhello_world_code = \"\"\"\n# Welcome to your new Julia project!\n# This is a simple example to get you started.\n\nprintln(\"Hello World!\")\n\n# You can add more code here to build your Julia application.\n# For example:\n# - Define functions\n# - Import packages\n# - Process data\n# - Create visualizations\n\n# Happy coding with Julia! 🚀\n\"\"\"\n\n# Write the Hello World example to the source file\nopen(project_src_file, \"w\") do file\n    write(file, hello_world_code)\nend\n\nprintln(\"Julia project '{}' created successfully at '{}'\")\nprintln(\"Added Hello World example to src/{}.jl\")\n\n# Clear workspace variables created during project creation\n# Get all names in Main module and remove temporary variables\nall_names = names(Main, all=true, imported=false)\nbuiltin_names = Set([:Base, :Core, :Main, :InteractiveUtils, :REPL, :Pkg, :ans, :err, :stdout, :stderr, :stdin])\nfor name in all_names\n    if !(name in builtin_names) && !startswith(string(name), \"JJ_\")\n        try\n            eval(Meta.parse(\"$name = nothing\"))\n        catch\n            # Ignore errors when clearing variables\n        end\n    end\nend\nprintln(\"Workspace cleared\")",
        julia_project_path, project_name, project_name, authors_field, project_name, project_name, julia_project_path, project_name
    );

    debug!("[OrchestratorCommands] Executing Julia code: {}", julia_code);

    // Execute the Julia code using the execution actor
    let result = app_state.actor_system
        .execution_actor
        .send(ExecuteApiRequest { 
            code: julia_code
        })
        .await
        .map_err(|_| "Failed to send execution request".to_string())?;

    match result {
        Ok(output) => {
            debug!("[OrchestratorCommands] Julia project creation output: {}", output);
            Ok(())
        }
        Err(e) => {
            error!("[OrchestratorCommands] Failed to create Julia project: {}", e);
            Err(format!("Failed to create Julia project: {}", e))
        }
    }
}

// ============================================================================





/// Install project dependencies using Pkg.instantiate()
#[tauri::command]
pub async fn instantiate_julia_project(
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    debug!("[OrchestratorCommands] Installing project dependencies with Pkg.instantiate()");

    // Verify that a project is currently open
    let _project_path = app_state
        .actor_system
        .config_actor
        .send(internals::messages::configuration::GetRootFolder)
        .await
        .map_err(|_| "Failed to get project path".to_string())?
        .map_err(|e| format!("Failed to get project path: {}", e))?;

    if _project_path.is_none() {
        return Err("No project is currently open".to_string());
    }

    // Execute Pkg.instantiate() in the project environment
    // The project environment is already activated via Pkg.activate()
    // Catch and ignore precompilation errors (they're not critical for installation)
    let code = r#"
        try
            using Pkg
            try
                Pkg.instantiate()
                println("✅ Project dependencies installed successfully!")
            catch precompile_error
                # Check if it's a precompilation error
                error_msg = sprint(showerror, precompile_error)
                if occursin("Creating a new global in closed module", error_msg) || 
                   occursin("precompil", lowercase(error_msg))
                    # Precompilation errors are not critical - packages are still installed
                    println("✅ Project dependencies installed!")
                    println("⚠️  Some packages had precompilation warnings (packages are still usable)")
                else
                    # Re-throw non-precompilation errors
                    rethrow(precompile_error)
                end
            end
        catch e
            println("❌ Failed to install dependencies: ", e)
            rethrow(e)
        end
    "#;

    app_state
        .actor_system
        .execution_actor
        .send(internals::messages::execution::ExecuteReplRequest { code: code.to_string() })
        .await
        .map_err(|_| "Failed to execute Pkg.instantiate()".to_string())?
        .map_err(|e| format!("Failed to install dependencies: {}", e))
}

/// Get default Julia environment path
#[tauri::command]
pub async fn get_default_julia_environment_path(
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    debug!("[OrchestratorCommands] Getting default Julia environment path");

    // Execute Julia code to get the default environment path
    let env_path_code = r#"
        try
            using Pkg
            
            # Get the default environment path
            default_env = Pkg.env()
            default_env_path = Pkg.env().path
            
            println(default_env_path)
            true
        catch e
            println("Failed to get default environment path: ", e)
            false
        end
    "#;

    match app_state.actor_system
        .execution_actor
        .send(ExecuteApiRequest { code: env_path_code.to_string() })
        .await
        .map_err(|_| "Actor comm failed".to_string())? {
        Ok(result) => {
            let result = result.to_string();
            let trimmed_result = result.trim();
            if !trimmed_result.is_empty() && !trimmed_result.contains("Failed") {
                Ok(trimmed_result.to_string())
            } else {
                // Fallback to a reasonable default
                Ok(std::env::var("JULIA_DEPOT_PATH").unwrap_or_else(|_| {
                    let home = std::env::var("HOME")
                        .or_else(|_| std::env::var("USERPROFILE"))
                        .unwrap_or_else(|_| ".".to_string());
                    format!("{}/.julia", home)
                }))
            }
        }
        Err(e) => {
            error!(
                "[OrchestratorCommands] Failed to get default environment path: {}",
                e
            );
            // Fallback to a reasonable default
            Ok(std::env::var("JULIA_DEPOT_PATH").unwrap_or_else(|_| {
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_else(|_| ".".to_string());
                format!("{}/.julia", home)
            }))
        }
    }
}

// ============================================================================
// Storage Management
// ============================================================================

/// Get Julia storage paths
#[tauri::command]
pub async fn get_julia_storage_paths(
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    debug!("[OrchestratorCommands] Getting Julia storage paths");

    let app_data_dir = dirs::data_local_dir()
        .ok_or_else(|| "Failed to get app data directory".to_string())?;
    
    let compute42_dir = app_data_dir.join("com.compute42.dev");
    let julia_depot_path = compute42_dir.join("depot");
    let lsp_env_path = compute42_dir.join("lsp-env");
    
    // Get Julia installation path from installation actor
    let julia_path = app_state.actor_system
        .installation_actor
        .send(GetJuliaPathFromInstallation)
        .await
        .map_err(|_| "Actor comm failed".to_string())??
        .unwrap_or_else(|| "Not configured".to_string());
    
    // Get default Julia depot path
    let default_depot_path = std::env::var("JULIA_DEPOT_PATH").unwrap_or_else(|_| {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        format!("{}/.julia", home)
    });

    let paths = serde_json::json!({
        "julia_installation": {
            "path": julia_path,
            "exists": std::path::Path::new(&julia_path).exists()
        },
        "compute42_depot": {
            "path": julia_depot_path.to_string_lossy(),
            "exists": julia_depot_path.exists()
        },
        "compute42_env": {
            "path": lsp_env_path.to_string_lossy(),
            "exists": lsp_env_path.exists()
        },
        "default_depot": {
            "path": default_depot_path,
            "exists": std::path::Path::new(&default_depot_path).exists()
        },
        "lsp_env": {
            "path": lsp_env_path.to_string_lossy(),
            "exists": lsp_env_path.exists()
        }
    });

    Ok(paths)
}

/// Get Julia version
#[tauri::command]
pub async fn get_julia_version(
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    debug!("[OrchestratorCommands] Getting Julia version from JuliaJunction installation");

    use std::process::Command;
    use internals::messages::installation::GetJuliaPathFromInstallation;

    // Get the  Julia executable path
    let julia_path = match app_state.actor_system
        .installation_actor
        .send(GetJuliaPathFromInstallation)
        .await
        .map_err(|_| "Actor comm failed".to_string())? {
        Ok(Some(path)) => path,
        Ok(None) => {
            debug!("[OrchestratorCommands] No Julia installation found");
            return Ok("Not installed".to_string());
        }
        Err(e) => {
            debug!("[OrchestratorCommands] Failed to get Julia path: {}", e);
            return Ok("Unknown".to_string());
        }
    };

    debug!("[OrchestratorCommands] Using Julia path: {}", julia_path);

    // Run julia --version command directly
    match Command::new(&julia_path).args(["--version"]).output() {
        Ok(output) => {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stdout);
                let version = version_output.trim();
                debug!("[OrchestratorCommands] Julia version output: {}", version);
                
                // Extract version number from output like "julia version 1.12.0"
                if let Some(version_part) = version.split_whitespace().nth(2) {
                    Ok(version_part.to_string())
                } else {
                    Ok(version.to_string())
                }
            } else {
                let error_output = String::from_utf8_lossy(&output.stderr);
                debug!("[OrchestratorCommands] Julia version command failed: {}", error_output);
                Ok("Unknown".to_string())
            }
        }
        Err(e) => {
            debug!("[OrchestratorCommands] Failed to execute Julia version command: {}", e);
            Ok("Unknown".to_string())
        }
    }
}

/// Get depot size info
#[tauri::command]
pub async fn get_depot_size_info(
    _app_state: State<'_, AppState>,
) -> Result<Value, String> {
    debug!("[OrchestratorCommands] Getting depot size info");

    let app_data_dir = dirs::data_local_dir()
        .ok_or_else(|| "Failed to get app data directory".to_string())?;
    
    let compute42_dir = app_data_dir.join("com.compute42.dev");
    let julia_depot_path = compute42_dir.join("depot");
    
    // Get default Julia depot path
    let default_depot_path = std::env::var("JULIA_DEPOT_PATH").unwrap_or_else(|_| {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        format!("{}/.julia", home)
    });

    // Calculate directory sizes
    let juliajunction_size = calculate_directory_size(&julia_depot_path).await;
    let default_depot_size = calculate_directory_size(std::path::Path::new(&default_depot_path)).await;

    let sizes = serde_json::json!({
        "compute42_depot": {
            "path": julia_depot_path.to_string_lossy(),
            "size_bytes": juliajunction_size,
            "size_human": format_size(juliajunction_size)
        },
        "default_depot": {
            "path": default_depot_path,
            "size_bytes": default_depot_size,
            "size_human": format_size(default_depot_size)
        }
    });

    Ok(sizes)
}

/// Clear JuliaJunction depot
#[tauri::command]
pub async fn clear_compute42_depot(
    app_state: State<'_, AppState>,
) -> Result<Value, String> {
    debug!("[OrchestratorCommands] Clearing JuliaJunction depot");

    // Get current project path before stopping Julia
    let current_project_path = match app_state.actor_system.config_actor.send(internals::messages::configuration::GetRootFolder).await {
        Ok(Ok(Some(path))) => {
            debug!("[OrchestratorCommands] Found current project path: {}", path);
            Some(path)
        }
        Ok(Ok(None)) => {
            debug!("[OrchestratorCommands] No current project path found");
            None
        }
        Ok(Err(e)) => {
            debug!("[OrchestratorCommands] Failed to get current project path: {}", e);
            None
        }
        Err(e) => {
            debug!("[OrchestratorCommands] Failed to send GetRootFolder message: {}", e);
            None
        }
    };

    // First, stop any running Julia processes to avoid file locking issues
    debug!("[OrchestratorCommands] Stopping Julia processes before clearing depot");
    use internals::messages::process::StopJuliaProcess;
    if let Err(e) = app_state.actor_system.process_actor.send(StopJuliaProcess).await {
        debug!("[OrchestratorCommands] Failed to send stop message to Julia process: {:?}", e);
    }
    
    // Give processes a moment to stop
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let app_data_dir = dirs::data_local_dir()
        .ok_or_else(|| "Failed to get app data directory".to_string())?;
    
    let compute42_dir = app_data_dir.join("com.compute42.dev");
    let julia_depot_path = compute42_dir.join("depot");
    let lsp_env_path = compute42_dir.join("lsp-env");

    let mut cleared_items = Vec::new();
    let mut errors = Vec::new();

    // Clear depot directory with retry logic for Windows file locking
    if julia_depot_path.exists() {
        let mut retry_count = 0;
        let max_retries = 3;
        let mut depot_cleared = false;
        
        while retry_count < max_retries && !depot_cleared {
            match tokio::fs::remove_dir_all(&julia_depot_path).await {
                Ok(_) => {
                    debug!("Successfully cleared JuliaJunction depot");
                    cleared_items.push("depot".to_string());
                    depot_cleared = true;
                }
                Err(e) => {
                    retry_count += 1;
                    error!("Failed to clear depot (attempt {}): {}", retry_count, e);
                    
                    if retry_count < max_retries {
                        // Wait a bit longer between retries for file handles to be released
                        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                    } else {
                        let error_msg = if e.raw_os_error() == Some(5) {
                            "Access denied - files may be in use by Julia processes. Please close all Julia sessions and try again."
                        } else {
                            &format!("Failed to clear depot: {}", e)
                        };
                        errors.push(error_msg.to_string());
                    }
                }
            }
        }
    } else {
        debug!("Depot directory does not exist, nothing to clear");
    }

    // Clear LSP environment directory with retry logic
    if lsp_env_path.exists() {
        let mut retry_count = 0;
        let max_retries = 3;
        let mut lsp_cleared = false;
        
        while retry_count < max_retries && !lsp_cleared {
            match tokio::fs::remove_dir_all(&lsp_env_path).await {
                Ok(_) => {
                    debug!("Successfully cleared LSP environment");
                    cleared_items.push("lsp_environment".to_string());
                    lsp_cleared = true;
                }
                Err(e) => {
                    retry_count += 1;
                    error!("Failed to clear LSP environment (attempt {}): {}", retry_count, e);
                    
                    if retry_count < max_retries {
                        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                    } else {
                        let error_msg = if e.raw_os_error() == Some(5) {
                            "Access denied - LSP files may be in use. Please close all Julia sessions and try again."
                        } else {
                            &format!("Failed to clear LSP environment: {}", e)
                        };
                        errors.push(error_msg.to_string());
                    }
                }
            }
        }
    } else {
        debug!("LSP environment directory does not exist, nothing to clear");
    }

    // If clearing was successful, restart Julia and reactivate project
    if errors.is_empty() {
        debug!("[OrchestratorCommands] Depot cleared successfully, restarting Julia");
        
        // Restart Julia process
        use internals::messages::process::RestartJulia;
        if let Err(e) = app_state.actor_system.process_actor.send(RestartJulia).await {
            debug!("[OrchestratorCommands] Failed to restart Julia process: {:?}", e);
            errors.push(format!("Failed to restart Julia process: {}", e));
        } else {
            debug!("[OrchestratorCommands] Julia process restart initiated");
            
            // Give Julia a moment to start up
            tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
            
            // Reactivate the project if there was one
            if let Some(project_path) = current_project_path {
                debug!("[OrchestratorCommands] Reactivating project: {}", project_path);
                use internals::messages::execution::ActivateProject;
                if let Err(e) = app_state.actor_system.execution_actor.send(ActivateProject { project_path: project_path.clone() }).await {
                    debug!("[OrchestratorCommands] Failed to reactivate project: {:?}", e);
                    errors.push(format!("Failed to reactivate project: {}", e));
                } else {
                    debug!("[OrchestratorCommands] Project reactivation initiated: {}", project_path);
                }
            }
        }
    }

    let result = serde_json::json!({
        "success": errors.is_empty(),
        "cleared_items": cleared_items,
        "errors": errors,
        "message": if errors.is_empty() {
            "Successfully cleared JuliaJunction depot and environment, Julia restarted"
        } else {
            "Cleared depot but encountered errors during restart"
        }
    });

    Ok(result)
}



/// Restart Julia process
// moved: process::restart_julia
#[allow(dead_code)]
async fn restart_julia(app_state: State<'_, AppState>) -> Result<(), String> {
    debug!("[OrchestratorCommands] Restarting Julia process");

    app_state.actor_system.process_actor.send(RestartJulia).await.map_err(|_| "Actor comm failed".to_string())?
}



// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate the total size of a directory in bytes
async fn calculate_directory_size(path: &std::path::Path) -> u64 {
    if !path.exists() || !path.is_dir() {
        return 0;
    }

    let mut total_size = 0u64;
    
    if let Ok(entries) = tokio::fs::read_dir(path).await {
        let mut entries = entries;
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    total_size += metadata.len();
                } else if metadata.is_dir() {
                    total_size += Box::pin(calculate_directory_size(&entry.path())).await;
                }
            }
        }
    }
    
    total_size
}

/// Format bytes into human readable format
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    match bytes {
        0..KB => format!("{} B", bytes),
        KB..MB => format!("{:.1} KB", bytes as f64 / KB as f64),
        MB..GB => format!("{:.1} MB", bytes as f64 / MB as f64),
        GB..TB => format!("{:.1} GB", bytes as f64 / GB as f64),
        _ => format!("{:.1} TB", bytes as f64 / TB as f64),
    }
}
