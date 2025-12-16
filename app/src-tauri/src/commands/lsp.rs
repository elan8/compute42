use crate::state::AppState;
use crate::error::AppError;
use internals::types::{
    LspCompletionItem, LspDiagnostic, LspDocumentSymbol, LspHover, LspLocation, LspPosition,
    LspSignatureHelp,
};
use log::{debug, error};
use tauri::State;

/// Get hover information for a position in a document
#[tauri::command]
pub async fn lsp_hover(
    uri: String,
    line: u32,
    character: u32,
    app_state: State<'_, AppState>,
) -> Result<Option<LspHover>, AppError> {
    debug!(
        "LSP hover request for {} at line {}, character {}",
        uri, line, character
    );

    let position = LspPosition { line, character };
    use internals::messages::lsp::GetHover;
    match app_state.actor_system.lsp_actor.send(GetHover { uri, position }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(hover) => Ok(hover),
        Err(e) => {
            error!("LSP hover error: {}", e);
            Err(AppError::InternalError(e))
        }
    }
}

/// Notify about document open
#[tauri::command]
pub async fn lsp_notify_did_open(
    uri: String,
    content: String,
    language: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("LSP notify did open: {} (language: {})", uri, language);

    use internals::messages::lsp::NotifyDidOpen;
    Ok(app_state.actor_system.lsp_actor.send(NotifyDidOpen { uri, content, language }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??)
}

/// Notify about document close
#[tauri::command]
pub async fn lsp_notify_did_close(
    uri: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("LSP notify did close: {}", uri);

    use internals::messages::lsp::NotifyDidClose;
    Ok(app_state.actor_system.lsp_actor.send(NotifyDidClose { uri }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??)
}

/// Notify about document change
#[tauri::command]
pub async fn lsp_notify_did_change(
    uri: String,
    content: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("LSP notify did change: {}", uri);

    use internals::messages::lsp::NotifyDidChange;
    Ok(app_state.actor_system.lsp_actor.send(NotifyDidChange { uri, content }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??)
}

/// Notify about document save
#[tauri::command]
pub async fn lsp_notify_did_save(
    uri: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("LSP notify did save: {}", uri);

    use internals::messages::lsp::NotifyDidSave;
    Ok(app_state.actor_system.lsp_actor.send(NotifyDidSave { uri }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??)
}

/// Get completions for a position
#[tauri::command]
pub async fn lsp_get_completions(
    uri: String,
    line: u32,
    character: u32,
    file_content: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<Vec<LspCompletionItem>, AppError> {
    debug!(
        "LSP completions request for {} at line {}, character {} (has file_content: {})",
        uri, line, character, file_content.is_some()
    );
    let position = LspPosition { line, character };
    use internals::messages::lsp::{GetCompletions, GetCompletionsWithContent};
    if let Some(content) = file_content {
        let msg = GetCompletionsWithContent { uri, position, content };
        match app_state.actor_system.lsp_actor.send(msg).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
            Ok(completions) => {
                debug!("LSP completions response: {} items", completions.len());
                Ok(completions)
            }
            Err(e) => {
                error!("LSP completions error: {}", e);
                Err(AppError::InternalError(e))
            }
        }
    } else {
        let msg = GetCompletions { uri, position };
        match app_state.actor_system.lsp_actor.send(msg).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
            Ok(completions) => {
                debug!("LSP completions response: {} items", completions.len());
                Ok(completions)
            }
            Err(e) => {
                error!("LSP completions error: {}", e);
                Err(AppError::InternalError(e))
            }
        }
    }
}

/// Get signature help for a position
#[tauri::command]
pub async fn lsp_get_signature_help(
    uri: String,
    line: u32,
    character: u32,
    _app_state: State<'_, AppState>,
) -> Result<Option<LspSignatureHelp>, AppError> {
    debug!(
        "LSP signature help request for {} at line {}, character {}",
        uri, line, character
    );

    // Note: Signature help is not yet implemented in the Rust LSP
    debug!("LSP signature help: Feature not yet implemented in Rust LSP");
    Ok(None)
}

/// Get definition for a position
#[tauri::command]
pub async fn lsp_get_definition(
    uri: String,
    line: u32,
    character: u32,
    app_state: State<'_, AppState>,
) -> Result<Vec<LspLocation>, AppError> {
    debug!(
        "LSP definition request for {} at line {}, character {}",
        uri, line, character
    );

    let position = LspPosition { line, character };
    use internals::messages::lsp::GetDefinition;
    match app_state.actor_system.lsp_actor.send(GetDefinition { uri, position }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(definition) => {
            debug!("LSP definition response: {} locations", definition.len());
            Ok(definition)
        }
        Err(e) => {
            error!("LSP definition error: {}", e);
            Err(AppError::InternalError(e))
        }
    }
}

/// Get references for a position
#[tauri::command]
pub async fn lsp_get_references(
    uri: String,
    line: u32,
    character: u32,
    app_state: State<'_, AppState>,
) -> Result<Vec<LspLocation>, AppError> {
    debug!(
        "LSP references request for {} at line {}, character {}",
        uri, line, character
    );

    let position = LspPosition { line, character };
    use internals::messages::lsp::GetReferences;
    match app_state.actor_system.lsp_actor.send(GetReferences { uri, position }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(references) => {
            debug!("LSP references response: {} locations", references.len());
            Ok(references)
        }
        Err(e) => {
            error!("LSP references error: {}", e);
            Err(AppError::InternalError(e))
        }
    }
}

/// Get document symbols
#[tauri::command]
pub async fn lsp_get_document_symbols(
    uri: String,
    _app_state: State<'_, AppState>,
) -> Result<Vec<LspDocumentSymbol>, AppError> {
    debug!("LSP document symbols request for {}", uri);

    // Note: Document symbols is not yet implemented in the Rust LSP
    debug!("LSP document symbols: Feature not yet implemented in Rust LSP");
    Ok(vec![])
}

/// Get diagnostics for a document
#[tauri::command]
pub async fn lsp_get_diagnostics(
    uri: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<LspDiagnostic>, AppError> {
    debug!("LSP diagnostics request for {}", uri);
    use internals::messages::lsp::GetDiagnostics;
    match app_state
        .actor_system
        .lsp_actor
        .send(GetDiagnostics { uri })
        .await
        .map_err(|_| AppError::InternalError("Actor comm failed".to_string()))?
    {
        Ok(diags) => Ok(diags),
        Err(e) => {
            error!("LSP diagnostics error: {}", e);
            Err(AppError::InternalError(e))
        }
    }
}

/// Check if LSP is running
#[tauri::command]
pub async fn lsp_is_running(app_state: State<'_, AppState>) -> Result<bool, AppError> {
    use internals::messages::lsp::IsLspRunning;
    Ok(app_state.actor_system.lsp_actor.send(IsLspRunning).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??)
}

/// Initialize LSP for a project
#[tauri::command]
pub async fn lsp_initialize(
    project_path: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("LSP initialize request for project: {}", project_path);

    use internals::messages::lsp::InitializeLsp;
    Ok(app_state.actor_system.lsp_actor.send(InitializeLsp { project_path }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??)
}

/// Shutdown LSP
#[tauri::command]
pub async fn lsp_shutdown(app_state: State<'_, AppState>) -> Result<(), AppError> {
    debug!("LSP shutdown request");

    use internals::messages::lsp::ShutdownLsp;
    Ok(app_state.actor_system.lsp_actor.send(ShutdownLsp).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))??)
}

/// Restart LSP server for a project
#[tauri::command]
pub async fn lsp_restart(
    project_path: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("LSP restart request for project: {}", project_path);
    
    use internals::messages::lsp::RestartLspServer;
    match app_state.actor_system.lsp_actor.send(RestartLspServer { project_path }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(()) => {
            debug!("LSP restart completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("LSP restart error: {}", e);
            Err(AppError::InternalError(e))
        }
    }
}
