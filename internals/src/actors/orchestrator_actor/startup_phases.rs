use actix::prelude::*;
use log::{debug, info, warn};
use std::time::Duration;
use std::path::PathBuf;
use crate::messages::configuration::GetRootFolder;
use crate::messages::execution::ActivateProject;
use crate::messages::lsp::RestartLspServer;
use crate::messages::process::StartJuliaProcess;
use crate::messages::plot::StartPlotServer;
use crate::messages::file_server::StartFileServer;
use crate::messages::communication::DisconnectFromPipes;
use crate::messages::installation::{CheckJuliaInstallation, InstallJulia};
use crate::services::events::EventService;
use super::startup_state::StartupPhase;

/// Timeout durations for each phase
/// Note: Some phases (like Julia installation) can legitimately take a very long time,
/// so we use generous timeouts or no timeout for those phases
pub struct PhaseTimeouts;

impl PhaseTimeouts {
    /// Get timeout for a specific phase, or None if no timeout should be applied
    /// 
    /// All phases now have timeouts to prevent infinite hangs. Long-running phases
    /// have generous timeouts, but they will still timeout eventually to allow
    /// recovery or user notification.
    pub fn timeout_for(phase: &StartupPhase) -> Option<Duration> {
        match phase {
            // Very long timeout for installation - can take 10+ minutes, but we still want a limit
            StartupPhase::InstallingJulia => Some(Duration::from_secs(20 * 60)), // 20 minutes
            // Long timeout for activation - can take several minutes for large projects
            StartupPhase::ActivatingProject => Some(Duration::from_secs(10 * 60)), // 10 minutes
            // Long timeout for LSP ready - pipeline can take a while, especially on first run
            StartupPhase::WaitingForLspReady => Some(Duration::from_secs(5 * 60)), // 5 minutes
            // Medium timeouts for operations that should complete reasonably quickly
            StartupPhase::CheckingJulia => Some(Duration::from_secs(30)),
            StartupPhase::StartingJuliaProcess => Some(Duration::from_secs(60)),
            StartupPhase::StartingLsp => Some(Duration::from_secs(120)), // Increased from 60
            // Short timeouts for quick operations
            _ => Some(Duration::from_secs(10)),
        }
    }
}

/// Context for executing startup phases
pub struct StartupPhaseContext<'a> {
    pub event_manager: &'a EventService,
    pub config_actor: Option<&'a Addr<crate::actors::ConfigurationActor>>,
    pub installation_actor: Option<&'a Addr<crate::actors::InstallationActor>>,
    pub process_actor: Option<&'a Addr<crate::actors::ProcessActor>>,
    pub communication_actor: Option<&'a Addr<crate::actors::CommunicationActor>>,
    pub execution_actor: Option<&'a Addr<crate::actors::ExecutionActor>>,
    pub lsp_actor: Option<&'a Addr<crate::actors::LspActor>>,
    pub plot_actor: Option<&'a Addr<crate::actors::PlotActor>>,
    pub file_server_actor: Option<&'a Addr<crate::actors::FileServerActor>>,
}

impl<'a> StartupPhaseContext<'a> {
    /// Execute phase: Check and install Julia if needed
    pub async fn check_and_install_julia(&self) -> Result<StartupPhase, String> {
        debug!("StartupPhase: Checking Julia installation");
        
        let installation_actor = self.installation_actor
            .ok_or_else(|| "Installation actor not available".to_string())?;
        
        let julia_installed = installation_actor.send(CheckJuliaInstallation).await
            .map_err(|e| format!("Failed to send CheckJuliaInstallation: {:?}", e))?
            .map_err(|e| format!("Failed to check Julia installation: {}", e))?;
        
        if julia_installed {
            debug!("StartupPhase: Julia already installed");
            Ok(StartupPhase::StartingJuliaProcess)
        } else {
            debug!("StartupPhase: Julia not installed, starting installation");
            
            installation_actor.send(InstallJulia { julia_version: None }).await
                .map_err(|e| format!("Failed to send InstallJulia: {:?}", e))?
                .map_err(|e| format!("Failed to install Julia: {}", e))?;
            
            // Return InstallingJulia phase - actual transition will happen when installation completes
            Ok(StartupPhase::InstallingJulia)
        }
    }
    
    /// Execute phase: Start Julia-dependent services
    pub async fn start_julia_dependent_services(&self) -> Result<(), String> {
        debug!("StartupPhase: Starting Julia-dependent services");
        
        // Reset connection state on startup to ensure clean state
        if let Some(communication_actor) = self.communication_actor {
            debug!("StartupPhase: Resetting connection state on startup");
            let _ = communication_actor.send(DisconnectFromPipes).await;
        }
        
        // Start Julia process
        debug!("StartupPhase: Starting Julia process");
        
        let process_actor = self.process_actor
            .ok_or_else(|| "Process actor not available".to_string())?;
        
        process_actor.send(StartJuliaProcess { orchestrator_addr: None }).await
            .map_err(|e| format!("Failed to send StartJuliaProcess: {:?}", e))?
            .map_err(|e| format!("Failed to start Julia process: {}", e))?;
        
        debug!("StartupPhase: Julia process start initiated");
        
        // Start plot server
        debug!("StartupPhase: Starting plot server");
        
        if let Some(plot_actor) = self.plot_actor {
            let _ = plot_actor.send(StartPlotServer { orchestrator_addr: None }).await
                .map_err(|e| format!("Failed to start plot server: {:?}", e))?;
            debug!("StartupPhase: Plot server start initiated");
        }
        
        // Start file server
        debug!("StartupPhase: Starting file server");
        
        if let Some(file_server_actor) = self.file_server_actor {
            let _root_folder = if let Some(config_actor) = self.config_actor {
                match config_actor.send(GetRootFolder).await {
                    Ok(Ok(folder)) => folder.unwrap_or_default(),
                    Ok(Err(e)) => {
                        warn!("StartupPhase: Failed to get root folder: {}, using default", e);
                        String::new()
                    }
                    Err(e) => {
                        warn!("StartupPhase: Failed to send GetRootFolder: {:?}, using default", e);
                        String::new()
                    }
                }
            } else {
                String::new()
            };
            
            let _ = file_server_actor.send(StartFileServer { orchestrator_addr: None }).await
                .map_err(|e| format!("Failed to start file server: {:?}", e))?;
            debug!("StartupPhase: File server start initiated");
        }
        
        Ok(())
    }
    
    /// Execute phase: Activate saved or demo project
    pub async fn activate_saved_or_demo_project(&self) -> Result<Option<String>, String> {
        debug!("StartupPhase: Checking for saved or demo project");
        
        let config_actor = self.config_actor
            .ok_or_else(|| "Config actor not available".to_string())?;
        
        // Check for saved project
        if let Ok(Ok(Some(folder))) = config_actor.send(GetRootFolder).await {
            let folder_path = std::path::Path::new(&folder);
            if folder_path.exists() && folder_path.is_dir() {
                let project_toml = folder_path.join("Project.toml");
                if project_toml.exists() {
                    info!("StartupPhase: Found saved Julia project: {}", folder);
                    return Ok(Some(folder));
                } else {
                    info!("StartupPhase: Saved directory is not a Julia project: {}", folder);
                }
            } else {
                warn!("StartupPhase: Saved directory no longer exists: {}", folder);
            }
        }
        
        // Fallback to demo directory
        if let Some(demo_path) = get_demo_folder_path() {
            if demo_path.exists() && demo_path.is_dir() {
                let project_toml = demo_path.join("Project.toml");
                if project_toml.exists() {
                    let demo_path_str = demo_path.to_string_lossy().to_string();
                    info!("StartupPhase: Using demo project: {}", demo_path_str);
                    return Ok(Some(demo_path_str));
                }
            }
        }
        
        debug!("StartupPhase: No Julia project found");
        Ok(None)
    }
    
    /// Execute phase: Activate project in Julia
    pub async fn activate_project(&self, project_path: String) -> Result<(), String> {
        debug!("StartupPhase: Activating project: {}", project_path);
        
        let execution_actor = self.execution_actor
            .ok_or_else(|| "Execution actor not available".to_string())?;
        
        match execution_actor.send(ActivateProject { project_path: project_path.clone() }).await {
            Ok(Ok(())) => {
                debug!("StartupPhase: Project activation initiated: {}", project_path);
            }
            Ok(Err(e)) => {
                if e.contains("Communication service not connected") {
                    debug!("StartupPhase: Communication not ready yet, activation will happen when ready");
                    // This is OK - activation will happen when message loop is ready
                } else {
                    return Err(format!("Failed to activate project: {}", e));
                }
            }
            Err(e) => {
                return Err(format!("Failed to send ActivateProject: {:?}", e));
            }
        }
        
        // Removed duplicate log - already logged above
        Ok(())
    }
    
    /// Execute phase: Start LSP server
    pub async fn start_lsp_server(&self, project_path: String) -> Result<(), String> {
        debug!("StartupPhase: Starting LSP server for project: {}", project_path);
        
        let lsp_actor = self.lsp_actor
            .ok_or_else(|| "LSP actor not available".to_string())?;
        
        lsp_actor.send(RestartLspServer { project_path }).await
            .map_err(|e| format!("Failed to send RestartLspServer: {:?}", e))?
            .map_err(|e| format!("Failed to restart LSP server: {}", e))?;
        
        debug!("StartupPhase: LSP server start initiated");
        Ok(())
    }
}

/// Get demo folder path from resource directory
/// The demo folder is bundled as a Tauri resource at compile time
fn get_demo_folder_path() -> Option<PathBuf> {
    // Try to find the demo folder relative to the executable
    // In Tauri, resources are typically in the same directory as the executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // In development, resources might be in target/debug/resources or target/release/resources
            // In production, resources are in the same directory as the executable
            let mut possible_paths = vec![
                exe_dir.join("resources").join("demo"),  // Production path
                exe_dir.join("demo"),  // Alternative production path
            ];
            
            // Add development path if parent exists
            if let Some(parent) = exe_dir.parent() {
                possible_paths.push(parent.join("resources").join("demo"));  // Development path (target/debug/resources/demo)
            }
            
            for path in possible_paths {
                if path.exists() && path.is_dir() {
                    let project_toml = path.join("Project.toml");
                    if project_toml.exists() {
                        return Some(path);
                    }
                }
            }
            
            // Also try workspace root for development
            if let Ok(cwd) = std::env::current_dir() {
                let workspace_demo = cwd.join("demo");
                if workspace_demo.exists() && workspace_demo.is_dir() {
                    let project_toml = workspace_demo.join("Project.toml");
                    if project_toml.exists() {
                        return Some(workspace_demo);
                    }
                }
            }
        }
    }
    
    None
}

