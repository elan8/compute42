// Message handlers for language features

use actix::prelude::*;

use crate::messages::lsp::*;
use super::state::LspActorState;

impl Handler<GetHover> for LspActorState {
    type Result = ResponseActFuture<Self, Result<Option<crate::types::LspHover>, String>>;
    
    fn handle(&mut self, msg: GetHover, _ctx: &mut Context<Self>) -> Self::Result {
        let lsp_service = self.lsp_service.clone();
        Box::pin(
            async move {
                lsp_service.get_hover(msg.uri, msg.position).await
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<GetCompletions> for LspActorState {
    type Result = ResponseActFuture<Self, Result<Vec<crate::types::LspCompletionItem>, String>>;

    fn handle(&mut self, msg: GetCompletions, _ctx: &mut Context<Self>) -> Self::Result {
        let lsp_service = self.lsp_service.clone();
        Box::pin(async move {
            lsp_service.get_completions(msg.uri, msg.position).await
        }
        .into_actor(self)
        .map(|res, _actor, _| res))
    }
}

impl Handler<GetCompletionsWithContent> for LspActorState {
    type Result = ResponseActFuture<Self, Result<Vec<crate::types::LspCompletionItem>, String>>;
    fn handle(&mut self, msg: GetCompletionsWithContent, _ctx: &mut Context<Self>) -> Self::Result {
        let lsp_service = self.lsp_service.clone();
        Box::pin(async move {
            lsp_service.get_completions_with_content(msg.uri, msg.position, msg.content).await
        }
        .into_actor(self)
        .map(|res, _actor, _| res))
    }
}

impl Handler<GetSignatureHelp> for LspActorState {
    type Result = ResponseActFuture<Self, Result<Option<crate::types::LspSignatureHelp>, String>>;
    
    fn handle(&mut self, msg: GetSignatureHelp, _ctx: &mut Context<Self>) -> Self::Result {
        let lsp_service = self.lsp_service.clone();
        Box::pin(
            async move {
                lsp_service.get_signature_help(msg.uri, msg.position).await
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<GetDefinition> for LspActorState {
    type Result = ResponseActFuture<Self, Result<Vec<crate::types::LspLocation>, String>>;
    
    fn handle(&mut self, msg: GetDefinition, _ctx: &mut Context<Self>) -> Self::Result {
        let lsp_service = self.lsp_service.clone();
        Box::pin(
            async move {
                lsp_service.get_definition(msg.uri, msg.position).await
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<GetReferences> for LspActorState {
    type Result = ResponseActFuture<Self, Result<Vec<crate::types::LspLocation>, String>>;
    
    fn handle(&mut self, msg: GetReferences, _ctx: &mut Context<Self>) -> Self::Result {
        let lsp_service = self.lsp_service.clone();
        Box::pin(
            async move {
                lsp_service.get_references(msg.uri, msg.position).await
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<GetDocumentSymbols> for LspActorState {
    type Result = ResponseActFuture<Self, Result<Vec<crate::types::LspDocumentSymbol>, String>>;
    
    fn handle(&mut self, msg: GetDocumentSymbols, _ctx: &mut Context<Self>) -> Self::Result {
        let lsp_service = self.lsp_service.clone();
        Box::pin(
            async move {
                lsp_service.get_document_symbols(msg.uri).await
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<GetDiagnostics> for LspActorState {
    type Result = ResponseActFuture<Self, Result<Vec<crate::types::LspDiagnostic>, String>>;
    
    fn handle(&mut self, msg: GetDiagnostics, _ctx: &mut Context<Self>) -> Self::Result {
        let lsp_service = self.lsp_service.clone();
        Box::pin(
            async move {
                lsp_service.get_diagnostics(msg.uri).await
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}





















