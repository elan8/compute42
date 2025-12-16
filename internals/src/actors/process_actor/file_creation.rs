use log::{debug, error};

use super::state::ProcessState;

/// Create Julia files from embedded sources
/// Always overwrites existing files to ensure we use the most recent version
pub async fn create_julia_files(state: &ProcessState) -> Result<(), String> {
    use std::fs;

    // Get the Compute42 data directory
    let data_dir = state.get_julia_data_directory();
    debug!("Creating Julia files in data directory: {:?}", data_dir);
    
    // Create the scripts directory if it doesn't exist
    let scripts_dir = data_dir.join("scripts");
    if !scripts_dir.exists() {
        fs::create_dir_all(&scripts_dir)
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to create scripts directory at {:?}: {} (os error {})",
                    scripts_dir,
                    e,
                    e.raw_os_error().unwrap_or(0)
                );
                error!("{}", error_msg);
                error_msg
            })?;
        debug!("Created scripts directory: {:?}", scripts_dir);
    }

    // Always overwrite main.jl to ensure we use the most recent version
    // Note: These are placeholder values that will be replaced with actual pipe names when the setup code is executed
    let setup_code = get_julia_setup_code(state, "to_julia_pipe", "from_julia_pipe");
    let setup_path = scripts_dir.join("main.jl");
    if setup_path.exists() {
        //debug!("Overwriting existing main.jl with latest version from executable");
    }
    fs::write(&setup_path, setup_code)
        .map_err(|e| {
            let error_msg = format!(
                "Failed to write main.jl to {:?}: {} (os error {})",
                setup_path,
                e,
                e.raw_os_error().unwrap_or(0)
            );
            error!("{}", error_msg);
            error_msg
        })?;

    // Create core subdirectory if it doesn't exist
    let core_dir = scripts_dir.join("core");
    if !core_dir.exists() {
        fs::create_dir_all(&core_dir)
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to create core directory at {:?}: {} (os error {})",
                    core_dir,
                    e,
                    e.raw_os_error().unwrap_or(0)
                );
                error!("{}", error_msg);
                error_msg
            })?;
        debug!("Created core directory: {:?}", core_dir);
    }

    // Create submodules subdirectory if it doesn't exist
    let submodules_dir = core_dir.join("submodules");
    if !submodules_dir.exists() {
        fs::create_dir_all(&submodules_dir)
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to create directory at {:?}: {} (os error {})",
                    submodules_dir,
                    e,
                    e.raw_os_error().unwrap_or(0)
                );
                error!("{}", error_msg);
                error_msg
            })?;
        debug!("Created submodules directory: {:?}", submodules_dir);
    }

    // Write all core module files
    let core_modules = vec![
        ("packages.jl", include_str!("../../../scripts/core/packages.jl")),
        ("display.jl", include_str!("../../../scripts/core/display.jl")),
        ("core.jl", include_str!("../../../scripts/core/core.jl")),
    ];

    for (module_name, module_code) in core_modules {
        let module_path = core_dir.join(module_name);
        if module_path.exists() {
            //debug!("Overwriting existing {} with latest version from executable", module_name);
        }
        fs::write(&module_path, module_code)
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to write {} to {:?}: {} (os error {})",
                    module_name,
                    module_path,
                    e,
                    e.raw_os_error().unwrap_or(0)
                );
                error!("{}", error_msg);
                error_msg
            })?;
    }

    // Write all submodule files
    let submodules = vec![
        ("communication.jl", include_str!("../../../scripts/core/submodules/communication.jl")),
        ("json_utils.jl", include_str!("../../../scripts/core/submodules/json_utils.jl")),
        ("execution.jl", include_str!("../../../scripts/core/submodules/execution.jl")),
        ("notebook.jl", include_str!("../../../scripts/core/submodules/notebook.jl")),
        ("workspace.jl", include_str!("../../../scripts/core/submodules/workspace.jl")),
        ("handlers.jl", include_str!("../../../scripts/core/submodules/handlers.jl")),
    ];

    for (module_name, module_code) in submodules {
        let module_path = submodules_dir.join(module_name);
        if module_path.exists() {
            //debug!("Overwriting existing {} with latest version from executable", module_name);
        }
        fs::write(&module_path, module_code)
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to write {} to {:?}: {} (os error {})",
                    module_name,
                    module_path,
                    e,
                    e.raw_os_error().unwrap_or(0)
                );
                error!("{}", error_msg);
                error_msg
            })?;
    }

    // Create debugger subdirectory if it doesn't exist
    let debugger_dir = scripts_dir.join("debugger");
    if !debugger_dir.exists() {
        fs::create_dir_all(&debugger_dir)
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to create debugger directory at {:?}: {} (os error {})",
                    debugger_dir,
                    e,
                    e.raw_os_error().unwrap_or(0)
                );
                error!("{}", error_msg);
                error_msg
            })?;
        debug!("Created debugger directory: {:?}", debugger_dir);
    }

    // Always overwrite debugger.jl to ensure we use the most recent version
    let debugger_code = include_str!("../../../scripts/debugger.jl");
    let debugger_path = scripts_dir.join("debugger.jl");
    if debugger_path.exists() {
        //debug!("Overwriting existing debugger.jl with latest version from executable");
    }
    fs::write(&debugger_path, debugger_code)
        .map_err(|e| {
            let error_msg = format!(
                "Failed to write debugger.jl to {:?}: {} (os error {})",
                debugger_path,
                e,
                e.raw_os_error().unwrap_or(0)
            );
            error!("{}", error_msg);
            error_msg
        })?;

    // Write all debugger module files
    let debugger_modules = vec![
        ("state.jl", include_str!("../../../scripts/debugger/state.jl")),
        ("display.jl", include_str!("../../../scripts/debugger/display.jl")),
        ("communication.jl", include_str!("../../../scripts/debugger/communication.jl")),
        ("execution.jl", include_str!("../../../scripts/debugger/execution.jl")),
        ("inspection.jl", include_str!("../../../scripts/debugger/inspection.jl")),
        ("handlers.jl", include_str!("../../../scripts/debugger/handlers.jl")),
    ];

    for (module_name, module_code) in debugger_modules {
        let module_path = debugger_dir.join(module_name);
        if module_path.exists() {
            //debug!("Overwriting existing {} with latest version from executable", module_name);
        }
        fs::write(&module_path, module_code)
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to write {} to {:?}: {} (os error {})",
                    module_name,
                    module_path,
                    e,
                    e.raw_os_error().unwrap_or(0)
                );
                error!("{}", error_msg);
                error_msg
            })?;
    }

    debug!("Created/updated Julia files at: {:?}", scripts_dir);
    Ok(())
}

/// Get the Julia setup code with the correct scripts directory
pub fn get_julia_setup_code(state: &ProcessState, to_julia_pipe: &str, from_julia_pipe: &str) -> String {
    // Get the scripts directory where julia_debug.jl is located
    let scripts_dir = state.get_julia_data_directory().join("scripts");
    let scripts_dir_str = scripts_dir.to_string_lossy().replace("\\", "/");
    
    // Load the actual setup script from the embedded source
    let setup_script = include_str!("../../../scripts/main.jl");
    setup_script
        .replace("{to_julia_pipe_name}", to_julia_pipe)
        .replace("{from_julia_pipe_name}", from_julia_pipe)
        .replace("{scripts_directory}", &scripts_dir_str)
}

