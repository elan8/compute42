// Actor state management and initialization

use actix::prelude::*;
use std::path::PathBuf;

use crate::services::events::EventService;
use crate::types::LspServerInfo;
use crate::actors::{ConfigurationActor, InstallationActor, OrchestratorActor};
use super::service_impl::LspService;

/// LspActor state
pub struct LspActorState {
    // Actor state (no mutexes needed)
    pub is_running: bool,
    pub server_info: Option<LspServerInfo>,
    pub current_project: Option<String>,
    
    // Service owned by this actor
    pub lsp_service: LspService,
    pub event_manager: EventService,
    
    // Actor addresses for message passing
    pub config_actor: Option<Addr<ConfigurationActor>>,
    pub installation_actor: Option<Addr<InstallationActor>>,
    pub orchestrator_actor: Option<Addr<OrchestratorActor>>,
}

impl LspActorState {
    /// Create a new LspActor state
    pub fn new(
        event_manager: EventService,
        config_actor: Option<Addr<ConfigurationActor>>,
        installation_actor: Option<Addr<InstallationActor>>,
    ) -> Self {
        // Construct LspService internally
        // Default Julia path - will be updated when Julia path is available from InstallationActor
        let default_julia_path = {
            #[cfg(target_os = "windows")]
            {
                PathBuf::from("julia.exe")
            }
            #[cfg(not(target_os = "windows"))]
            {
                PathBuf::from("julia")
            }
        };
        
        // Calculate depot path (same logic as in generic.rs)
        let depot_path = dirs::data_local_dir()
            .map(|app_data_dir| {
                let compute42_dir = app_data_dir.join("com.compute42.dev");
                compute42_dir.join("depot")
            });
        
        let lsp_service = LspService::new(default_julia_path, None, depot_path);
        
        Self {
            is_running: false,
            server_info: None,
            current_project: None,
            lsp_service,
            event_manager,
            config_actor,
            installation_actor,
            orchestrator_actor: None,
        }
    }
    
    /// Set orchestrator actor address for coordination
    pub fn set_orchestrator_actor(&mut self, orchestrator_actor: Addr<OrchestratorActor>) {
        self.orchestrator_actor = Some(orchestrator_actor);
    }
}


