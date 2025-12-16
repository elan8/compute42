// InstallationActor - manages Julia installation detection and management
// This replaces the mutex-based InstallationManager with a clean actor model

mod types;
mod discovery;
mod version_info;
mod download;
mod extraction;
mod cleanup;
mod installation;
mod handlers;

use actix::prelude::*;
use log::debug;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::services::events::EventService;
use crate::service_traits::InstallationService as InstallationServiceTrait;
use crate::types::JuliaInstallation;
use crate::actors::ConfigurationActor;

use discovery::{find_julia_executable, verify_julia_executable, get_julia_version_from_executable};
use cleanup::cleanup_old_julia_versions;
use version_info::get_julia_installation_dir;
use installation::install_julia;

/// InstallationActor - manages Julia installation detection and management
/// This replaces the mutex-based InstallationManager with a clean actor model
pub struct InstallationActor {
    // Actor state (no mutexes needed)
    installations: Vec<JuliaInstallation>,
    current_installation: Option<JuliaInstallation>,
    installation_in_progress: Arc<Mutex<bool>>,
    julia_path: Option<String>, // Store Julia path locally
    
    event_manager: EventService,
    
    // Actor addresses for message passing
    config_actor: Option<Addr<ConfigurationActor>>,
    orchestrator_actor: Option<Addr<crate::actors::orchestrator_actor::OrchestratorActor>>,
}

impl InstallationActor {
    /// Create a new InstallationActor instance
    pub fn new(
        event_manager: EventService,
        config_actor: Option<Addr<ConfigurationActor>>,
        orchestrator_actor: Option<Addr<crate::actors::orchestrator_actor::OrchestratorActor>>,
    ) -> Self {
        Self {
            installations: Vec::new(),
            current_installation: None,
            installation_in_progress: Arc::new(Mutex::new(false)),
            julia_path: None,
            event_manager,
            config_actor,
            orchestrator_actor,
        }
    }
    
    /// Set the orchestrator actor address
    pub fn set_orchestrator_actor(&mut self, orchestrator_actor: Addr<crate::actors::orchestrator_actor::OrchestratorActor>) {
        self.orchestrator_actor = Some(orchestrator_actor);
        debug!("InstallationActor: Orchestrator actor address set");
    }
}

impl Actor for InstallationActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // debug!("InstallationActor: Actor started");
        ctx.set_mailbox_capacity(64);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("InstallationActor: Actor stopped");
    }
}

// Implementation of InstallationService trait for the actor
// This allows the actor to be used where the trait is expected
#[async_trait::async_trait]
impl InstallationServiceTrait for InstallationActor {
    async fn check_julia_installation(&self) -> Result<bool, String> {
        debug!("InstallationActor: Checking Julia installation");
        
        match find_julia_executable(None).await {
            Ok(julia_path) => {
                debug!("InstallationActor: Found Julia executable at: {}", julia_path);
                
                // Verify the executable actually works by running julia --version
                if verify_julia_executable(&julia_path).await {
                    debug!("InstallationActor: Julia installation verified successfully");
                    Ok(true)
                } else {
                    debug!("InstallationActor: Julia executable found but verification failed");
                    Ok(false)
                }
            }
            Err(e) => {
                debug!("InstallationActor: Julia installation not found: {}", e);
                Ok(false)
            }
        }
    }
    
    async fn install_julia(&self, julia_version: Option<&str>) -> Result<(), String> {
        install_julia(julia_version, self.installation_in_progress.clone(), Some(&self.event_manager)).await
    }
    
    async fn get_julia_version(&self) -> Result<Option<String>, String> {
        debug!("InstallationActor: Getting Julia version");
        
        match find_julia_executable(None).await {
            Ok(julia_path) => {
                if let Ok(version) = get_julia_version_from_executable(&julia_path).await {
                    debug!("InstallationActor: Found Julia version: {}", version);
                    Ok(Some(version))
                } else {
                    debug!("InstallationActor: Failed to get Julia version");
                    Ok(None)
                }
            }
            Err(_) => {
                debug!("InstallationActor: Julia not found, cannot get version");
                Ok(None)
            }
        }
    }
    
    async fn is_installation_in_progress(&self) -> bool {
        *self.installation_in_progress.lock().await
    }
    
    async fn get_julia_executable_path(&self) -> Result<Option<String>, String> {
        debug!("InstallationActor: Getting Julia executable path");
        
        match find_julia_executable(None).await {
            Ok(julia_path) => {
                debug!("InstallationActor: Found Julia executable path: {}", julia_path);
                Ok(Some(julia_path))
            }
            Err(_) => {
                debug!("InstallationActor: Julia executable not found");
                Ok(None)
            }
        }
    }

    async fn cleanup_old_julia_versions(&self, current_version: &str) -> Result<(), String> {
        let installation_dir = get_julia_installation_dir();
        cleanup_old_julia_versions(&installation_dir, current_version).await
    }
    
    async fn get_detected_installations(&self) -> Result<Vec<JuliaInstallation>, String> {
        debug!("InstallationActor: Getting detected Julia installations");
        
        match find_julia_executable(None).await {
            Ok(julia_path) => {
                if let Ok(version) = get_julia_version_from_executable(&julia_path).await {
                    let installation = JuliaInstallation {
                        path: julia_path,
                        version,
                        is_valid: true,
                    };
                    debug!("InstallationActor: Found Julia installation: {}", installation.version);
                    Ok(vec![installation])
                } else {
                    debug!("InstallationActor: Found Julia but failed to get version");
                    Ok(Vec::new())
                }
            }
            Err(_) => {
                debug!("InstallationActor: No Julia installations found");
                Ok(Vec::new())
            }
        }
    }
    
    async fn detect_installations(&self) -> Result<Vec<JuliaInstallation>, String> {
        // Same as get_detected_installations for now
        self.get_detected_installations().await
    }
    
    async fn validate_installation(&self, installation: JuliaInstallation) -> Result<bool, String> {
        debug!("InstallationActor: Validating Julia installation: {}", installation.version);
        
        // Check if the path exists
        if !std::path::Path::new(&installation.path).exists() {
            debug!("InstallationActor: Installation path does not exist: {}", installation.path);
            return Ok(false);
        }
        
        // Verify the executable works
        if verify_julia_executable(&installation.path).await {
            debug!("InstallationActor: Installation validation successful");
            Ok(true)
        } else {
            debug!("InstallationActor: Installation validation failed");
            Ok(false)
        }
    }
    
    async fn repair_installation(&self, _installation: &JuliaInstallation) -> Result<JuliaInstallation, String> {
        // Repair not implemented yet
        Err("Installation repair not implemented yet".to_string())
    }
}

// Clone implementation for async operations
impl Clone for InstallationActor {
    fn clone(&self) -> Self {
        Self {
            installations: self.installations.clone(),
            current_installation: self.current_installation.clone(),
            installation_in_progress: self.installation_in_progress.clone(),
            julia_path: self.julia_path.clone(),
            event_manager: self.event_manager.clone(),
            config_actor: self.config_actor.clone(),
            orchestrator_actor: self.orchestrator_actor.clone(),
        }
    }
}
