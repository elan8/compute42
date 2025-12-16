use crate::messages::{ExecutionType, PlotData};
use crate::types::{
    UserPreferences, LspCallHierarchyItem, LspCodeAction, LspCompletionItem, LspDiagnostic,
    LspDocumentSymbol, LspHover, LspInlayHint, LspLocation, LspPosition, LspRange,
    LspSemanticToken, LspSignatureHelp, LspTextEdit, LspWorkspaceEdit,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Generic event data that can be emitted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    pub event_type: String,
    pub payload: serde_json::Value,
}

/// Generic event emitter interface to replace Tauri's event system
#[async_trait]
pub trait EventEmitter: Send + Sync {
    async fn emit(&self, event: &str, payload: serde_json::Value) -> Result<(), String>;
    async fn emit_all(&self, event: &str, payload: serde_json::Value) -> Result<(), String>;
}


/// Generic logging interface to replace Tauri logging
#[async_trait]
pub trait LoggingService: Send + Sync {
    async fn configure_logging(&self, config: LoggingConfig) -> Result<(), String>;
    async fn log(&self, level: LogLevel, message: &str) -> Result<(), String>;
    async fn log_with_context(&self, level: LogLevel, message: &str, context: serde_json::Value) -> Result<(), String>;
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub targets: Vec<LogTarget>,
    pub max_file_size: Option<usize>,
    pub rotation_strategy: Option<RotationStrategy>,
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Log targets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogTarget {
    Stdout,
    File { path: String, filename: Option<String> },
    LogDir { filename: Option<String> },
}

/// Rotation strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStrategy {
    KeepSome(usize),
    KeepAll,
}

#[async_trait]
pub trait ConfigurationService: Send + Sync {
    async fn load_config(&self) -> Result<UserPreferences, String>;
    async fn save_config(&self, config: &UserPreferences) -> Result<(), String>;
    async fn get_root_folder(&self) -> Result<Option<String>, String>;
    async fn set_root_folder(&self, folder: Option<String>) -> Result<(), String>;
    async fn get_user_email(&self) -> Result<Option<String>, String>;
    async fn set_user_email(&self, email: Option<String>) -> Result<(), String>;
}

/// Persistence service for file I/O operations (renamed from PersistenceService)
#[async_trait]
pub trait FilePersistenceService: Send + Sync {
    async fn load_json_value(&self, key: &str) -> Result<serde_json::Value, String>;
    async fn save_json_value(&self, key: &str, data: &serde_json::Value) -> Result<(), String>;
    
    async fn delete(&self, key: &str) -> Result<(), String>;
    
    async fn exists(&self, key: &str) -> bool;
}

/// Dyn-compatible version of FilePersistenceService for trait objects
#[async_trait]
pub trait DynFilePersistenceService: Send + Sync {
    async fn delete(&self, key: &str) -> Result<(), String>;
    async fn exists(&self, key: &str) -> bool;
    async fn load_json_value(&self, key: &str) -> Result<serde_json::Value, String>;
    async fn save_json_value(&self, key: &str, data: &serde_json::Value) -> Result<(), String>;
}

// Implement DynFilePersistenceService for any type that implements FilePersistenceService
#[async_trait]
impl<T: FilePersistenceService + Send + Sync> DynFilePersistenceService for T {
    async fn delete(&self, key: &str) -> Result<(), String> {
        self.delete(key).await
    }
    
    async fn exists(&self, key: &str) -> bool {
        self.exists(key).await
    }
    async fn load_json_value(&self, key: &str) -> Result<serde_json::Value, String> {
        self.load_json_value(key).await
    }
    async fn save_json_value(&self, key: &str, data: &serde_json::Value) -> Result<(), String> {
        self.save_json_value(key, data).await
    }
}


#[async_trait]
pub trait InstallationService: Send + Sync {
    async fn check_julia_installation(&self) -> Result<bool, String>;
    async fn install_julia(&self, julia_version: Option<&str>) -> Result<(), String>;
    async fn get_julia_version(&self) -> Result<Option<String>, String>;
    async fn is_installation_in_progress(&self) -> bool;
    async fn get_julia_executable_path(&self) -> Result<Option<String>, String>;
    async fn cleanup_old_julia_versions(&self, current_version: &str) -> Result<(), String>;
    
    // Additional methods needed by actors
    async fn get_detected_installations(&self) -> Result<Vec<crate::types::JuliaInstallation>, String>;
    async fn detect_installations(&self) -> Result<Vec<crate::types::JuliaInstallation>, String>;
    async fn validate_installation(&self, installation: crate::types::JuliaInstallation) -> Result<bool, String>;
    async fn repair_installation(&self, installation: &crate::types::JuliaInstallation) -> Result<crate::types::JuliaInstallation, String>;
}

#[async_trait]
pub trait SysimageService: Send + Sync {
    async fn check_sysimage_available(&self) -> Result<bool, String>;
    async fn download_sysimage(&self) -> Result<(), String>;
    async fn get_sysimage_path(&self) -> Result<Option<std::path::PathBuf>, String>;
    async fn build_sysimage(&self) -> Result<(), String>;
    async fn is_sysimage_download_in_progress(&self) -> bool;
    
    // Additional methods needed by actors
    async fn get_available_sysimages(&self) -> Result<Vec<crate::types::SysimageInfo>, String>;
    async fn load_sysimage(&self, sysimage_path: &str) -> Result<crate::types::SysimageInfo, String>;
    async fn unload_sysimage(&self, sysimage_path: &str) -> Result<(), String>;
    async fn validate_sysimage(&self, sysimage: crate::types::SysimageInfo) -> Result<bool, String>;
    async fn optimize_sysimage(&self, sysimage: &crate::types::SysimageInfo) -> Result<crate::types::SysimageInfo, String>;
    async fn clear_sysimages(&self) -> Result<(), String>;
}

#[async_trait]
pub trait ProcessService: Send + Sync {
    async fn start_julia_process(&self) -> Result<(), String>;
    async fn stop_julia_process(&self) -> Result<(), String>;
    async fn is_julia_running(&self) -> bool;
    async fn get_pipe_names(&self) -> Result<(String, String), String>;
    async fn restart_julia(&self) -> Result<(), String>;
    async fn set_julia_executable_path(&self, path: std::path::PathBuf);
    
    /// Set channel sender to notify when Julia pipes are ready
    /// Default implementation does nothing (for mock services)
    async fn set_julia_pipes_ready_sender(&self, _sender: tokio::sync::mpsc::UnboundedSender<()>) {
        // Default: no-op for trait objects that don't implement this
    }

    /// Set channel sender to notify when Julia message loop is ready
    /// Default implementation does nothing (for mock services)
    async fn set_julia_message_loop_ready_sender(&self, _sender: tokio::sync::mpsc::UnboundedSender<()>) {
        // Default: no-op for trait objects that don't implement this
    }

    /// Set channel sender to notify when project activation is complete
    /// Default implementation does nothing (for mock services)
    async fn set_project_activation_complete_sender(&self, _sender: tokio::sync::mpsc::UnboundedSender<()>) {
        // Default: no-op for trait objects that don't implement this
    }

    /// Set output suppression state
    /// Default implementation does nothing (for mock services)
    async fn set_output_suppression(&self, _suppressed: bool) {
        // Default: no-op for trait objects that don't implement this
    }
}

#[async_trait]
pub trait CommunicationService: Send + Sync {
    async fn connect_to_pipes(&self, to_julia_pipe: String, from_julia_pipe: String) -> Result<(), String>;
    async fn disconnect_from_pipes(&self) -> Result<(), String>;
    async fn execute_code(
        &self,
        code: String,
        execution_type: ExecutionType,
        file_path: Option<String>,
    ) -> Result<crate::messages::JuliaMessage, String>;
    async fn send_debug_message(&self, message: crate::messages::JuliaMessage) -> Result<(), String>;
    async fn is_connected(&self) -> bool;
    async fn is_busy(&self) -> bool;
    
    /// Set a restart handler that will be called when stuck state is detected
    /// Default implementation does nothing (for mock services)
    async fn set_restart_handler(&self, _handler: Box<dyn Fn() + Send + Sync>) {
        // Default: no-op for trait objects that don't implement this
    }
    
    /// Set PlotActor address for routing plot data through actor
    /// Default implementation does nothing (for mock services)
    async fn set_plot_actor(&self, _plot_actor: actix::Addr<crate::actors::PlotActor>) {
        // Default: no-op for trait objects that don't implement this
    }
}

// Forward declarations for AI types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OllamaStatus {
    pub installed: bool,
    pub running: bool,
    pub version: Option<String>,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiContext {
    pub current_file: Option<String>,
    pub current_file_content: Option<String>,
    pub project_path: Option<String>,
    pub workspace_variables: std::collections::HashMap<String, serde_json::Value>,
    pub lsp_symbols: Vec<LspSymbol>,
    pub dependencies: Vec<String>,
    pub recent_execution_results: Vec<String>,
    pub open_files: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LspSymbol {
    pub name: String,
    pub kind: String,
    pub location: String,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyntaxValidationResult {
    pub is_valid: bool,
    pub diagnostics: Vec<crate::types::LspDiagnostic>,
    pub error_count: usize,
    pub warning_count: usize,
    pub suggestions: Vec<String>,
}

// AiService trait removed - functionality moved to AiActor
// The actor pattern is now used instead of the service trait pattern

#[async_trait]
pub trait PlotService: Send + Sync {
    async fn start_plot_server(&self) -> Result<u16, String>;
    async fn stop_plot_server(&self) -> Result<(), String>;
    async fn get_plot_port(&self) -> Option<u16>;
    async fn is_plot_server_running(&self) -> bool;
    async fn add_plot(&self, plot_data: PlotData) -> Result<(), String>;
    async fn get_plots(&self) -> Result<Vec<PlotData>, String>;
    async fn delete_plot(&self, plot_id: String) -> Result<(), String>;
    async fn handle_plot_data_received(&self, plot_data_json: serde_json::Value) -> Result<(), String>;
    async fn clear_plots(&self) -> Result<(), String>;
    async fn save_plot_to_file(&self, plot: &PlotData, file_path: &str) -> Result<(), String>;
    async fn update_plot(&self, plot: &PlotData) -> Result<(), String>;
    async fn update_plot_urls_for_new_port(&self, new_port: u16) -> Result<(), String>;
}

#[async_trait]
pub trait FileService: Send + Sync {
    async fn start_file_server(&self, root_path: String) -> Result<u16, String>;
    async fn stop_file_server(&self) -> Result<(), String>;
    async fn get_file_server_port(&self) -> Option<u16>;
    async fn is_file_server_running(&self) -> bool;
    async fn get_file_server_url(&self) -> Option<String>;
    async fn check_file_server_health(&self) -> Result<bool, String>;
    async fn get_file_server_stats(&self) -> Result<serde_json::Value, String>;
    async fn serve_file(&self, file_path: &str) -> Result<Vec<u8>, String>;
    async fn list_directory(&self, directory_path: &str) -> Result<Vec<String>, String>;
}

// Alias removed in refactor

#[async_trait]
pub trait LspService: Send + Sync {
    // Basic LSP functionality
    async fn start_lsp_server(&self, project_path: String) -> Result<(), String>;
    async fn stop_lsp_server(&self) -> Result<(), String>;
    async fn is_lsp_running(&self) -> bool;
    async fn initialize_lsp(&self, project_path: String) -> Result<(), String>;
    async fn initialize_julia_lsp(&self) -> Result<(), String>;
    async fn shutdown_lsp(&self) -> Result<(), String>;
    
    // Julia LSP stderr output setup - uses a channel sender for message-based communication
    async fn set_julia_lsp_stderr_sender(
        &self,
        sender: tokio::sync::mpsc::UnboundedSender<String>,
    ) -> Result<(), String>;
    
    // Text document synchronization
    async fn notify_did_open(
        &self,
        uri: String,
        content: String,
        language: String,
    ) -> Result<(), String>;
    async fn notify_did_change(&self, uri: String, content: String) -> Result<(), String>;
    async fn notify_did_close(&self, uri: String) -> Result<(), String>;
    async fn notify_did_save(&self, uri: String) -> Result<(), String>;

    // Completion and IntelliSense
    async fn get_completions(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspCompletionItem>, String>;
    async fn resolve_completion(
        &self,
        completion_item: LspCompletionItem,
    ) -> Result<LspCompletionItem, String>;
    async fn get_completions_with_content(
        &self,
        uri: String,
        position: LspPosition,
        content: String,
    ) -> Result<Vec<LspCompletionItem>, String>;

    // Hover and documentation
    async fn get_hover(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Option<LspHover>, String>;

    // Signature help
    async fn get_signature_help(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Option<LspSignatureHelp>, String>;

    // Navigation and references
    async fn get_definition(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspLocation>, String>;
    async fn get_declaration(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspLocation>, String>;
    async fn get_type_definition(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspLocation>, String>;
    async fn get_implementation(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspLocation>, String>;
    async fn get_references(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspLocation>, String>;
    async fn get_document_highlights(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspRange>, String>;

    // Document symbols and workspace symbols
    async fn get_document_symbols(&self, uri: String) -> Result<Vec<LspDocumentSymbol>, String>;
    async fn get_workspace_symbols(&self, query: String) -> Result<Vec<LspDocumentSymbol>, String>;

    // Code actions and refactoring
    async fn get_code_actions(
        &self,
        uri: String,
        range: LspRange,
        context: Vec<LspDiagnostic>,
    ) -> Result<Vec<LspCodeAction>, String>;
    async fn execute_command(
        &self,
        command: String,
        arguments: Option<Vec<serde_json::Value>>,
    ) -> Result<serde_json::Value, String>;

    // Rename and refactoring
    async fn prepare_rename(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Option<LspRange>, String>;
    async fn rename(
        &self,
        uri: String,
        position: LspPosition,
        new_name: String,
    ) -> Result<Option<LspWorkspaceEdit>, String>;

    // Formatting
    async fn format_document(
        &self,
        uri: String,
        options: serde_json::Value,
    ) -> Result<Vec<LspTextEdit>, String>;
    async fn format_range(
        &self,
        uri: String,
        range: LspRange,
        options: serde_json::Value,
    ) -> Result<Vec<LspTextEdit>, String>;
    async fn format_on_type(
        &self,
        uri: String,
        position: LspPosition,
        ch: String,
        options: serde_json::Value,
    ) -> Result<Vec<LspTextEdit>, String>;

    // Folding ranges
    async fn get_folding_ranges(&self, uri: String) -> Result<Vec<LspRange>, String>;

    // Selection ranges
    async fn get_selection_ranges(
        &self,
        uri: String,
        positions: Vec<LspPosition>,
    ) -> Result<Vec<LspRange>, String>;

    // Call hierarchy
    async fn prepare_call_hierarchy(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspCallHierarchyItem>, String>;
    async fn get_call_hierarchy_incoming_calls(
        &self,
        item: LspCallHierarchyItem,
    ) -> Result<Vec<LspCallHierarchyItem>, String>;
    async fn get_call_hierarchy_outgoing_calls(
        &self,
        item: LspCallHierarchyItem,
    ) -> Result<Vec<LspCallHierarchyItem>, String>;

    // Semantic tokens
    async fn get_semantic_tokens(
        &self,
        uri: String,
        range: Option<LspRange>,
    ) -> Result<Vec<LspSemanticToken>, String>;
    async fn get_semantic_tokens_delta(
        &self,
        uri: String,
        previous_result_id: String,
    ) -> Result<Vec<LspSemanticToken>, String>;

    // Inlay hints
    async fn get_inlay_hints(
        &self,
        uri: String,
        range: LspRange,
    ) -> Result<Vec<LspInlayHint>, String>;
    async fn resolve_inlay_hint(&self, hint: LspInlayHint) -> Result<LspInlayHint, String>;

    // Diagnostics
    async fn get_diagnostics(&self, uri: String) -> Result<Vec<LspDiagnostic>, String>;

    // Document management
    async fn update_document(&self, uri: String, content: String) -> Result<(), String>;
    async fn invalidate_cache(&self, uri: String) -> Result<(), String>;

    // Workspace operations
    async fn get_workspace_folders(&self) -> Result<Vec<String>, String>;
    async fn get_workspace_configuration(
        &self,
        section: String,
    ) -> Result<serde_json::Value, String>;

    // Document links
    async fn get_document_links(&self, uri: String) -> Result<Vec<LspLocation>, String>;
    async fn resolve_document_link(&self, link: LspLocation) -> Result<LspLocation, String>;

    // Color provider
    async fn get_document_colors(&self, uri: String) -> Result<Vec<serde_json::Value>, String>;
    async fn get_color_presentations(
        &self,
        uri: String,
        color: serde_json::Value,
        range: LspRange,
    ) -> Result<Vec<serde_json::Value>, String>;

    // Type hierarchy
    async fn prepare_type_hierarchy(
        &self,
        uri: String,
        position: LspPosition,
    ) -> Result<Vec<LspCallHierarchyItem>, String>;
    async fn get_type_hierarchy_supertypes(
        &self,
        item: LspCallHierarchyItem,
    ) -> Result<Vec<LspCallHierarchyItem>, String>;
    async fn get_type_hierarchy_subtypes(
        &self,
        item: LspCallHierarchyItem,
    ) -> Result<Vec<LspCallHierarchyItem>, String>;

    // Inline values
    async fn get_inline_values(
        &self,
        uri: String,
        range: LspRange,
        context: serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, String>;

    // Progress reporting
    async fn create_progress(
        &self,
        token: String,
        title: String,
        message: Option<String>,
    ) -> Result<(), String>;
    async fn report_progress(
        &self,
        token: String,
        message: String,
        percentage: Option<u32>,
    ) -> Result<(), String>;
    async fn end_progress(&self, token: String, message: Option<String>) -> Result<(), String>;

    // Custom Julia-specific features
    async fn get_julia_symbols(&self, uri: String, query: String) -> Result<Vec<String>, String>;
    async fn get_julia_diagnostics(&self, uri: String) -> Result<Vec<LspDiagnostic>, String>;
    async fn get_julia_workspace_symbols(
        &self,
        query: String,
    ) -> Result<Vec<LspDocumentSymbol>, String>;
    async fn get_julia_references(
        &self,
        uri: String,
        position: LspPosition,
        include_declaration: bool,
    ) -> Result<Vec<LspLocation>, String>;

    // Additional methods that are missing
    async fn check_lsp_health(&self) -> Result<bool, String>;
    async fn send_request(&self, request: serde_json::Value) -> Result<serde_json::Value, String>;
    async fn get_capabilities(&self) -> Result<serde_json::Value, String>;
}

/// Debug service interface for Julia debugging functionality
#[async_trait]
pub trait DebugService: Send + Sync {
    // Type casting support
    fn as_any(&self) -> &dyn std::any::Any;
    
    // Initialization
    async fn initialize(&self) -> Result<(), String>;
    
    // Julia executable management
    async fn set_julia_executable(&self, julia_path: std::path::PathBuf);
    
    // Debug functionality removed in open-source version
}


// SyntaxService removed; use languageserver::features::DiagnosticsProvider instead

