use super::state::ProcessState;
use super::session::PersistentJuliaSession;

/// Execute the Julia setup code
pub async fn execute_julia_setup(
    state: &ProcessState,
    session: &mut PersistentJuliaSession,
    to_julia_pipe: &str,
    from_julia_pipe: &str,
) -> Result<(), String> {
    let setup_code = get_julia_setup_code(state, to_julia_pipe, from_julia_pipe);
    
    // Write setup code to a temporary file
    let data_dir = state.get_julia_data_directory();
    let scripts_dir = data_dir.join("scripts");
    let setup_path = scripts_dir.join("main.jl");
    
    // Write the setup code to the file
    if let Err(e) = std::fs::write(&setup_path, setup_code) {
        log::error!("ProcessActor: Failed to write main.jl: {}", e);
        return Err(format!("Failed to write main.jl: {}", e));
    }
    
    // Use include() to load the setup file instead of sending via stdin
    let include_command = format!("include(\"{}\")", setup_path.to_string_lossy().replace("\\", "/"));
    match session.execute_code(include_command).await {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            log::error!("ProcessActor: Failed to execute Julia setup code: {}", e);
            Err(format!("Failed to execute Julia setup code: {}", e))
        }
    }
}

/// Get the Julia setup code with the correct scripts directory
fn get_julia_setup_code(state: &ProcessState, to_julia_pipe: &str, from_julia_pipe: &str) -> String {
    super::file_creation::get_julia_setup_code(state, to_julia_pipe, from_julia_pipe)
}


