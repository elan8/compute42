// LspActor - manages LSP server lifecycle using EmbeddedLspService

use actix::prelude::*;
use log::debug;

mod state;
mod lifecycle;
mod service_impl;
mod utils;
mod type_conversions;
mod handlers_lifecycle;
mod handlers_documents;
mod handlers_features;

pub use state::LspActorState;
pub use handlers_lifecycle::SetOrchestratorActor;

// Re-export for external use
pub use state::LspActorState as LspActor;

// Implement Actor trait
impl Actor for LspActorState {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // debug!("LspActor: Actor started");
        ctx.set_mailbox_capacity(128);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("LspActor: Actor stopped");
    }
}

// Clone implementation for async operations
impl Clone for LspActorState {
    fn clone(&self) -> Self {
        Self {
            is_running: self.is_running,
            server_info: self.server_info.clone(),
            current_project: self.current_project.clone(),
            lsp_service: self.lsp_service.clone(),
            event_manager: self.event_manager.clone(),
            config_actor: self.config_actor.clone(),
            installation_actor: self.installation_actor.clone(),
            orchestrator_actor: self.orchestrator_actor.clone(),
        }
    }
}
