// Julia executable discovery and verification

use crate::version;
use log::{debug, error};
use tokio::process::Command as TokioCommand;

/// Find the Julia executable
pub async fn find_julia_executable(julia_version: Option<&str>) -> Result<String, String> {
    // Only use bundled Julia (embedded Julia for JuliaJunction)
    if let Some(bundled_path) = find_bundled_julia(julia_version) {
        Ok(bundled_path)
    } else {
        Err("Bundled Julia not found. Please ensure Compute42 is properly installed.".to_string())
    }
}

/// Find bundled Julia executable
fn find_bundled_julia(julia_version: Option<&str>) -> Option<String> {
    // Get the Julia installation directory
    let app_data_dir = dirs::data_local_dir().expect("Failed to get app data directory");
    let julia_dir = app_data_dir.join("com.compute42.dev").join("julia");

    // Use provided Julia version or default from centralized version
    let version = julia_version.unwrap_or(version::get_julia_version());
    let version_dir = julia_dir.join(format!("julia-{}", version));

    #[cfg(target_os = "windows")]
    {
        let executable = version_dir.join("bin").join("julia.exe");
        if executable.exists() {
            return Some(executable.to_string_lossy().to_string());
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let executable = version_dir.join("bin").join("julia");
        if executable.exists() {
            return Some(executable.to_string_lossy().to_string());
        }
    }

    None
}

/// Verify that a Julia executable works by running julia --version
pub async fn verify_julia_executable(julia_path: &str) -> bool {
    debug!("InstallationActor: Verifying Julia executable: {}", julia_path);
    
    let mut command = TokioCommand::new(julia_path);
    
    // On Windows, prevent the console window from appearing
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    let output = command
        .arg("--version")
        .output()
        .await;
        
    match output {
        Ok(output) => {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stdout);
                debug!("InstallationActor: Julia version output: {}", version_output);
                true
            } else {
                error!("InstallationActor: Julia --version failed with status: {:?}", output.status);
                false
            }
        }
        Err(e) => {
            error!("InstallationActor: Failed to run Julia executable: {}", e);
            false
        }
    }
}

/// Get Julia version from executable by running julia --version
pub async fn get_julia_version_from_executable(julia_path: &str) -> Result<String, String> {
    debug!("InstallationActor: Getting Julia version from executable: {}", julia_path);
    
    let mut command = TokioCommand::new(julia_path);
    
    // On Windows, prevent the console window from appearing
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    let output = command
        .arg("--version")
        .output()
        .await
        .map_err(|e| format!("Failed to run Julia executable: {}", e))?;
        
    if output.status.success() {
        let version_output = String::from_utf8_lossy(&output.stdout);
        // Parse version from output like "julia version 1.12.1"
        if let Some(version_line) = version_output.lines().next() {
            if let Some(version) = version_line.split_whitespace().nth(2) {
                debug!("InstallationActor: Parsed Julia version: {}", version);
                return Ok(version.to_string());
            }
        }
        Err("Failed to parse Julia version from output".to_string())
    } else {
        Err(format!("Julia --version failed with status: {:?}", output.status))
    }
}

