// Message handlers for lifecycle operations

use actix::prelude::*;
use log::{debug, error};

use crate::messages::lsp::*;
use super::state::LspActorState;

// Message to set orchestrator actor address
#[derive(Message)]
#[rtype(result = "()")]
pub struct SetOrchestratorActor {
    pub orchestrator_actor: Addr<crate::actors::OrchestratorActor>,
}

impl Handler<SetOrchestratorActor> for LspActorState {
    type Result = ();
    
    fn handle(&mut self, msg: SetOrchestratorActor, _ctx: &mut Context<Self>) -> Self::Result {
        self.set_orchestrator_actor(msg.orchestrator_actor);
    }
}

impl Handler<StartLspServer> for LspActorState {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: StartLspServer, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("LspActor: Received StartLspServer message for project: {:?}", msg.project_path);
        let mut actor = self.clone();
        async move {
            debug!("LspActor: Starting LSP server in async task...");
            let result = actor.start_lsp_server(msg.project_path).await;
            match result {
                Ok(_) => debug!("LspActor: LSP server started successfully"),
                Err(e) => error!("LspActor: Failed to start LSP server: {}", e),
            }
        }.into_actor(self).spawn(_ctx);
        debug!("LspActor: StartLspServer message handled, async task spawned");
        Ok(())
    }
}

impl Handler<StopLspServer> for LspActorState {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: StopLspServer, _ctx: &mut Context<Self>) -> Self::Result {
        let mut actor = self.clone();
        async move {
            let _ = actor.stop_lsp_server().await;
        }.into_actor(self).spawn(_ctx);
        Ok(())
    }
}

impl Handler<RestartLspServer> for LspActorState {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: RestartLspServer, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("LspActor: Received RestartLspServer message for project: {:?}", msg.project_path);
        let mut actor = self.clone();
        async move {
            debug!("LspActor: Restarting LSP server in async task...");
            
            // First stop the existing server if running
            if actor.is_running {
                debug!("LspActor: Stopping existing LSP server before restart");
                if let Err(e) = actor.stop_lsp_server().await {
                    debug!("LspActor: Warning - failed to stop existing LSP server: {}", e);
                }
                // Add delay to ensure cleanup
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }
            
            // Now start the new server
            let result = actor.start_lsp_server(msg.project_path).await;
            match result {
                Ok(_) => debug!("LspActor: LSP server restarted successfully"),
                Err(e) => error!("LspActor: Failed to restart LSP server: {}", e),
            }
        }.into_actor(self).spawn(_ctx);
        debug!("LspActor: RestartLspServer message handled, async task spawned");
        Ok(())
    }
}

impl Handler<IsLspRunning> for LspActorState {
    type Result = Result<bool, String>;
    
    fn handle(&mut self, _msg: IsLspRunning, _ctx: &mut Context<Self>) -> Self::Result {
        // Return the current running state directly
        Ok(self.is_running)
    }
}

impl Handler<InitializeLsp> for LspActorState {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: InitializeLsp, ctx: &mut Context<Self>) -> Self::Result {
        // InitializeLsp is a legacy message - use StartLspServer instead
        // For backward compatibility, we route to StartLspServer
        let mut actor = self.clone();
        async move {
            let _ = actor.start_lsp_server(msg.project_path).await;
        }.into_actor(self).spawn(ctx);
        Ok(())
    }
}

impl Handler<ShutdownLsp> for LspActorState {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: ShutdownLsp, ctx: &mut Context<Self>) -> Self::Result {
        // ShutdownLsp is a legacy message - use StopLspServer instead
        // For backward compatibility, we route to stop_lsp_server
        let mut actor = self.clone();
        async move {
            let _ = actor.stop_lsp_server().await;
        }.into_actor(self).spawn(ctx);
        Ok(())
    }
}













