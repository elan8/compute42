use actix::prelude::*;
use crate::types::{LspHover, LspPosition, LspCompletionItem, LspSignatureHelp, LspLocation, LspDocumentSymbol, LspDiagnostic};

// ============================================================================
// LspActor Messages
// ============================================================================

/// Start LSP server
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct StartLspServer {
    pub project_path: String,
}

/// Stop LSP server
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct StopLspServer;

/// Restart LSP server (graceful stop + start)
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct RestartLspServer {
    pub project_path: String,
}

/// Check if LSP is running
#[derive(Message)]
#[rtype(result = "Result<bool, String>")]
pub struct IsLspRunning;

/// Initialize LSP
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct InitializeLsp {
    pub project_path: String,
}

/// Shutdown LSP
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ShutdownLsp;

/// Get hover information
#[derive(Message)]
#[rtype(result = "Result<Option<LspHover>, String>")]
pub struct GetHover {
    pub uri: String,
    pub position: LspPosition,
}

/// Notify document did open
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct NotifyDidOpen {
    pub uri: String,
    pub content: String,
    pub language: String,
}

/// Notify document did close
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct NotifyDidClose {
    pub uri: String,
}

/// Notify document did change
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct NotifyDidChange {
    pub uri: String,
    pub content: String,
}

/// Notify document did save
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct NotifyDidSave {
    pub uri: String,
}

/// Get completions
#[derive(Message)]
#[rtype(result = "Result<Vec<LspCompletionItem>, String>")]
pub struct GetCompletions {
    pub uri: String,
    pub position: LspPosition,
}

/// Get signature help
#[derive(Message)]
#[rtype(result = "Result<Option<LspSignatureHelp>, String>")]
pub struct GetSignatureHelp {
    pub uri: String,
    pub position: LspPosition,
}

/// Get definition
#[derive(Message)]
#[rtype(result = "Result<Vec<LspLocation>, String>")]
pub struct GetDefinition {
    pub uri: String,
    pub position: LspPosition,
}

/// Get references
#[derive(Message)]
#[rtype(result = "Result<Vec<LspLocation>, String>")]
pub struct GetReferences {
    pub uri: String,
    pub position: LspPosition,
}

/// Get document symbols
#[derive(Message)]
#[rtype(result = "Result<Vec<LspDocumentSymbol>, String>")]
pub struct GetDocumentSymbols {
    pub uri: String,
}

/// Get diagnostics
#[derive(Message)]
#[rtype(result = "Result<Vec<LspDiagnostic>, String>")]
pub struct GetDiagnostics {
    pub uri: String,
}

/// Update document content
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct UpdateDocument {
    pub uri: String,
    pub content: String,
}

/// Invalidate cache for a document
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct InvalidateCache {
    pub uri: String,
}

/// Update Julia executable path
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct UpdateJuliaExecutable {
    pub julia_path: std::path::PathBuf,
}

#[derive(Debug, Clone)]
pub struct GetCompletionsWithContent {
    pub uri: String,
    pub position: crate::types::LspPosition,
    pub content: String,
}

impl actix::Message for GetCompletionsWithContent {
    type Result = Result<Vec<crate::types::LspCompletionItem>, String>;
}

/// Julia LSP stderr output line
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct JuliaLspStderrLine {
    pub line: String,
}