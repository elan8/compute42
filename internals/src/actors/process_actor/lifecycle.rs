use log::debug;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;

use super::state::ProcessState;
use super::session::PersistentJuliaSession;
use super::file_creation;
use super::setup;
use super::output_monitoring;
use crate::service_traits::EventEmitter;

/// Start Julia process with communication setup
pub async fn start_julia_with_communication(
    state: Arc<ProcessState>,
    event_emitter: Arc<dyn EventEmitter>,
    julia_session: Arc<Mutex<Option<PersistentJuliaSession>>>,
) -> Result<(), String> {
    // Create Julia files from embedded sources
    file_creation::create_julia_files(state.as_ref()).await?;

    // Generate pipe names
    let (to_julia_pipe, from_julia_pipe) = state.generate_pipe_names();

    // Start Julia process without sysimage for now
    try_start_julia_without_sysimage(state, event_emitter, julia_session, &to_julia_pipe, &from_julia_pipe).await
}

/// Try to start Julia process without sysimage
async fn try_start_julia_without_sysimage(
    state: Arc<ProcessState>,
    event_emitter: Arc<dyn EventEmitter>,
    julia_session: Arc<Mutex<Option<PersistentJuliaSession>>>,
    to_julia_pipe: &str,
    from_julia_pipe: &str,
) -> Result<(), String> {
    // Get the Julia path
    let julia_path = state.get_julia_executable_path().await;

    // Build Julia command without sysimage
    let mut command = Command::new(&julia_path);

    // On Windows, prevent the console window from appearing
    #[cfg(target_os = "windows")]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW

    // Note: Environment variables are inherited by default from the parent process
    // The fix_path_env crate (called in main.rs) ensures PATH is properly set
    // for GUI apps, especially in AppImage environments

    // Set environment variable for Julia data directory
    let data_dir = state.get_julia_data_directory();
    command.env("COMPUTE42_DATA_DIR", &data_dir);
    
    // Configure GR backend to prevent GKS QtTerm window from appearing
    // "nul" = null device (no output), prevents interactive window popup
    command.env("GKSwstype", "nul");

    // Set Julia environment variables to use specific environment
    let app_data_dir = dirs::data_local_dir().expect("Failed to get app data directory");
    let julia_depot_path = app_data_dir.join("com.compute42.dev").join("depot");
    let julia_project_path = app_data_dir.join("com.compute42.dev").join("julia-env");
    
    // Ensure the Julia environment directory exists
    if !julia_project_path.exists() {
        let _ = std::fs::create_dir_all(&julia_project_path);
    }
    
    command.env("JULIA_DEPOT_PATH", julia_depot_path.to_string_lossy().to_string());
    command.env("JULIA_PROJECT", julia_project_path.to_string_lossy().to_string());

    // Try to find and activate Julia project environment
    if let Some(project_path) = state.find_julia_project() {
        command.arg(format!("--project={}", project_path.to_string_lossy()));
    }

    // Add basic Julia arguments (no sysimage)
    command
        .arg("--startup-file=no")
        .arg("-t1")
        .arg("--history-file=no");

    // Set up stdin/stdout/stderr
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Start the Julia process
    let julia_process = command
        .spawn()
        .map_err(|e| format!("Failed to start Julia process without sysimage: {}", e))?;

    debug!("ProcessActor: Julia process started successfully without sysimage");

    // Reset output suppression flag for new Julia process
    state.set_output_suppression(true).await; // Suppress output during initialization

    // Create persistent session
    let mut session = PersistentJuliaSession::new(julia_process, event_emitter.clone());
    session.to_julia_pipe_name = Some(to_julia_pipe.to_string());
    session.from_julia_pipe_name = Some(from_julia_pipe.to_string());

    // Start stdout/stderr monitoring BEFORE storing the session
    let stdout = session.julia_process.stdout.take();
    let stderr = session.julia_process.stderr.take();
    
    if let Some(stdout) = stdout {
        output_monitoring::start_stdout_monitoring(
            stdout,
            event_emitter.clone(),
            state.output_suppressed.clone(),
            state.notebook_cell_output_buffer.clone(),
            state.current_notebook_cell.clone(),
        );
    }
    
    if let Some(stderr) = stderr {
        output_monitoring::start_stderr_monitoring(
            stderr,
            event_emitter.clone(),
            state.clone(),
            julia_session.clone(),
        );
    }

    // Execute the Julia setup code
    setup::execute_julia_setup(state.as_ref(), &mut session, to_julia_pipe, from_julia_pipe).await?;

    // Store the session AFTER starting monitoring and executing setup code
    {
        let mut session_guard = julia_session.lock().await;
        *session_guard = Some(session);
    }

    Ok(())
}

/// Stop Julia process
pub async fn stop_julia_process(
    julia_session: Arc<Mutex<Option<PersistentJuliaSession>>>,
) -> Result<(), String> {
    let mut session_guard = julia_session.lock().await;
    if let Some(mut session) = session_guard.take() {
        // Kill the Julia process
        let _ = session.julia_process.kill().await;

        session.is_active = false;
        session.emit_session_status("stopped".to_string()).await;
    }

    Ok(())
}

/// Get pipe names from the session
pub async fn get_pipe_names(
    julia_session: Arc<Mutex<Option<PersistentJuliaSession>>>,
) -> Result<(String, String), String> {
    let session_guard = julia_session.lock().await;

    if let Some(session) = &*session_guard {
        let to_julia_pipe = session
            .to_julia_pipe_name
            .clone()
            .ok_or("To Julia pipe name not set")?;
        let from_julia_pipe = session
            .from_julia_pipe_name
            .clone()
            .ok_or("From Julia pipe name not set")?;
        Ok((to_julia_pipe, from_julia_pipe))
    } else {
        Err("Julia session not active".to_string())
    }
}

/// Check if Julia is running
#[allow(dead_code)]
pub async fn is_julia_running(
    julia_session: Arc<Mutex<Option<PersistentJuliaSession>>>,
) -> bool {
    let session_guard = julia_session.lock().await;
    if let Some(session) = &*session_guard {
        session.is_active
    } else {
        false
    }
}

