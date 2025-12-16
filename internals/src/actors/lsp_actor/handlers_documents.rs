// Message handlers for document operations

use actix::prelude::*;
use log::debug;

use crate::messages::lsp::*;
use super::state::LspActorState;

impl Handler<NotifyDidOpen> for LspActorState {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: NotifyDidOpen, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("LspActor: Document opened - URI: {}, Language: {}", msg.uri, msg.language);
        let lsp_service = self.lsp_service.clone();
        Box::pin(
            async move {
                lsp_service.notify_did_open(msg.uri, msg.content, msg.language).await
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<NotifyDidClose> for LspActorState {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: NotifyDidClose, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("LspActor: Document closed - URI: {}", msg.uri);
        let lsp_service = self.lsp_service.clone();
        Box::pin(
            async move {
                lsp_service.notify_did_close(msg.uri).await
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<NotifyDidChange> for LspActorState {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: NotifyDidChange, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("LspActor: Document changed - URI: {}", msg.uri);
        let lsp_service = self.lsp_service.clone();
        Box::pin(
            async move {
                lsp_service.notify_did_change(msg.uri, msg.content).await
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<NotifyDidSave> for LspActorState {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: NotifyDidSave, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("LspActor: Document saved - URI: {}", msg.uri);
        let lsp_service = self.lsp_service.clone();
        Box::pin(
            async move {
                lsp_service.notify_did_save(msg.uri).await
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<UpdateDocument> for LspActorState {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: UpdateDocument, _ctx: &mut Context<Self>) -> Self::Result {
        let lsp_service = self.lsp_service.clone();
        Box::pin(
            async move {
                lsp_service.update_document(msg.uri, msg.content).await
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<InvalidateCache> for LspActorState {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: InvalidateCache, _ctx: &mut Context<Self>) -> Self::Result {
        let lsp_service = self.lsp_service.clone();
        Box::pin(
            async move {
                lsp_service.invalidate_cache(msg.uri).await
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}





















