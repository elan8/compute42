use crate::state::AppState;
use crate::error::AppError;
use internals::types::LspDiagnostic;
use log::{debug, error, warn};
use tauri::State;

/// Parse Julia code and return syntax diagnostics
/// Now uses the languageserver's DiagnosticsProvider instead of the deprecated TreeSitterSyntaxService
#[tauri::command]
pub async fn parse_julia_syntax(
    uri: String,
    content: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<LspDiagnostic>, AppError> {
    debug!(
        "Syntax parse request for {} ({} chars)",
        uri, content.len()
    );

    // First, update the document in the LSP
    use internals::messages::lsp::{UpdateDocument, GetDiagnostics};
    
    // Update document content (this will also trigger re-parsing)
    match app_state.actor_system.lsp_actor.send(UpdateDocument { 
        uri: uri.clone(), 
        content 
    }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(_) => {
            debug!("Document updated successfully, fetching diagnostics");
        }
        Err(e) => {
            warn!("Document update failed: {}, continuing to get diagnostics anyway", e);
        }
    }

    // Then get diagnostics from the languageserver
    match app_state.actor_system.lsp_actor.send(GetDiagnostics { uri }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(diagnostics) => {
            debug!("Syntax parse response: {} diagnostics", diagnostics.len());
            Ok(diagnostics)
        }
        Err(e) => {
            error!("Syntax parse error: {}", e);
            Err(AppError::InternalError(e))
        }
    }
}

/// Get cached syntax diagnostics for a file
/// Now uses the languageserver's DiagnosticsProvider instead of the deprecated TreeSitterSyntaxService
#[tauri::command]
pub async fn get_syntax_diagnostics(
    uri: String,
    app_state: State<'_, AppState>,
) -> Result<Vec<LspDiagnostic>, AppError> {
    debug!("Syntax diagnostics request for {}", uri);

    use internals::messages::lsp::GetDiagnostics;
    match app_state.actor_system.lsp_actor.send(GetDiagnostics { uri }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(diagnostics) => {
            debug!("Syntax diagnostics response: {} diagnostics", diagnostics.len());
            Ok(diagnostics)
        }
        Err(e) => {
            error!("Syntax diagnostics error: {}", e);
            Err(AppError::InternalError(e))
        }
    }
}

/// Clear cached syntax diagnostics for a file
/// With the new languageserver integration, this invalidates the cache for the specified file
#[tauri::command]
pub async fn clear_syntax_cache(
    uri: String,
    app_state: State<'_, AppState>,
) -> Result<(), AppError> {
    debug!("Clear syntax cache request for {}", uri);

    // The languageserver automatically invalidates cache when documents are updated,
    // but we can still provide this command for explicit cache invalidation if needed
    use internals::messages::lsp::InvalidateCache;
    match app_state.actor_system.lsp_actor.send(InvalidateCache { uri }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(()) => {
            debug!("Syntax cache cleared successfully");
            Ok(())
        }
        Err(e) => {
            // If the message doesn't exist yet, just return success
            warn!("Clear syntax cache warning (may not be implemented yet): {}", e);
            Ok(())
        }
    }
}

/// Check if syntax service is available
/// Now checks if the LSP service with diagnostics is available
#[tauri::command]
pub async fn is_syntax_service_available(
    app_state: State<'_, AppState>,
) -> Result<bool, AppError> {
    debug!("Check syntax service availability");

    // Check if LSP is running
    use internals::messages::lsp::IsLspRunning;
    match app_state.actor_system.lsp_actor.send(IsLspRunning).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(available) => {
            debug!("Syntax service (LSP) available: {}", available);
            Ok(available)
        }
        Err(e) => {
            error!("Check syntax service availability error: {}", e);
            Err(AppError::InternalError(e))
        }
    }
}

