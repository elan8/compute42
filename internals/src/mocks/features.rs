use crate::service_traits::*;
use crate::types::*;
use crate::messages::PlotData;
use async_trait::async_trait;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

// ============================================================================
// Mock PlotService
// ============================================================================

pub struct MockPlotService {
    is_running: Arc<TokioMutex<bool>>,
    port: Arc<TokioMutex<Option<u16>>>,
    plots: Arc<TokioMutex<Vec<PlotData>>>,
}

impl Default for MockPlotService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPlotService {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(TokioMutex::new(false)),
            port: Arc::new(TokioMutex::new(None)),
            plots: Arc::new(TokioMutex::new(Vec::new())),
        }
    }

    pub async fn set_running(&self, running: bool) {
        *self.is_running.lock().await = running;
    }

    pub async fn set_port(&self, port: Option<u16>) {
        *self.port.lock().await = port;
    }

    pub async fn get_plots_internal(&self) -> Vec<PlotData> {
        self.plots.lock().await.clone()
    }
}

#[async_trait]
impl PlotService for MockPlotService {
    async fn start_plot_server(&self) -> Result<u16, String> {
        *self.is_running.lock().await = true;
        let port = 8080;
        *self.port.lock().await = Some(port);
        Ok(port)
    }

    async fn stop_plot_server(&self) -> Result<(), String> {
        *self.is_running.lock().await = false;
        *self.port.lock().await = None;
        Ok(())
    }

    async fn get_plot_port(&self) -> Option<u16> {
        *self.port.lock().await
    }

    async fn add_plot(&self, plot_data: PlotData) -> Result<(), String> {
        self.plots.lock().await.push(plot_data);
        Ok(())
    }

    async fn get_plots(&self) -> Result<Vec<PlotData>, String> {
        Ok(self.plots.lock().await.clone())
    }

    async fn delete_plot(&self, plot_id: String) -> Result<(), String> {
        self.plots.lock().await.retain(|plot| plot.id != plot_id);
        Ok(())
    }

    async fn handle_plot_data_received(&self, plot_data_json: serde_json::Value) -> Result<(), String> {
        // For mock, just add a basic plot data
        if let Some(id) = plot_data_json["id"].as_str() {
            let plot_data = PlotData {
                id: id.to_string(),
                mime_type: plot_data_json["mime_type"].as_str().unwrap_or("image/svg+xml").to_string(),
                data: plot_data_json["data"].as_str().unwrap_or("").to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64,
                title: plot_data_json["title"].as_str().map(|s| s.to_string()),
                description: plot_data_json["description"].as_str().map(|s| s.to_string()),
                source_file: plot_data_json["source_file"].as_str().map(|s| s.to_string()),
                line_number: plot_data_json["line_number"].as_u64().map(|n| n as u32),
                code_context: plot_data_json["code_context"].as_str().map(|s| s.to_string()),
                session_id: plot_data_json["session_id"].as_str().map(|s| s.to_string()),
            };
            self.plots.lock().await.push(plot_data);
        }
        Ok(())
    }

    async fn is_plot_server_running(&self) -> bool { *self.is_running.lock().await }
    async fn clear_plots(&self) -> Result<(), String> { self.plots.lock().await.clear(); Ok(()) }
    async fn save_plot_to_file(&self, _plot: &PlotData, _file_path: &str) -> Result<(), String> { Ok(()) }
    async fn update_plot(&self, _plot: &PlotData) -> Result<(), String> { Ok(()) }
    async fn update_plot_urls_for_new_port(&self, _new_port: u16) -> Result<(), String> { Ok(()) }
}

// ============================================================================
// Mock FileService
// ============================================================================

pub struct MockFileService {
    is_running: Arc<TokioMutex<bool>>,
    port: Arc<TokioMutex<Option<u16>>>,
    root_path: Arc<TokioMutex<Option<String>>>,
}

impl Default for MockFileService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockFileService {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(TokioMutex::new(false)),
            port: Arc::new(TokioMutex::new(None)),
            root_path: Arc::new(TokioMutex::new(None)),
        }
    }

    pub async fn set_running(&self, running: bool) {
        *self.is_running.lock().await = running;
    }

    pub async fn set_port(&self, port: Option<u16>) {
        *self.port.lock().await = port;
    }

    pub async fn set_root_path(&self, path: Option<String>) {
        *self.root_path.lock().await = path;
    }
}

#[async_trait]
impl FileService for MockFileService {
    async fn start_file_server(&self, root_path: String) -> Result<u16, String> {
        *self.is_running.lock().await = true;
        *self.root_path.lock().await = Some(root_path);
        let port = 8081;
        *self.port.lock().await = Some(port);
        Ok(port)
    }

    async fn stop_file_server(&self) -> Result<(), String> {
        *self.is_running.lock().await = false;
        *self.port.lock().await = None;
        *self.root_path.lock().await = None;
        Ok(())
    }

    async fn get_file_server_port(&self) -> Option<u16> {
        *self.port.lock().await
    }

    async fn is_file_server_running(&self) -> bool {
        *self.is_running.lock().await
    }

    async fn get_file_server_url(&self) -> Option<String> {
        (*self.port.lock().await).map(|port| format!("http://localhost:{}", port))
    }

    async fn check_file_server_health(&self) -> Result<bool, String> { Ok(*self.is_running.lock().await) }
    async fn get_file_server_stats(&self) -> Result<serde_json::Value, String> { Ok(serde_json::json!({})) }
    async fn serve_file(&self, _file_path: &str) -> Result<Vec<u8>, String> { Ok(vec![]) }
    async fn list_directory(&self, _directory_path: &str) -> Result<Vec<String>, String> { Ok(vec![]) }
}

// ============================================================================
// Mock LspService
// ============================================================================

pub struct MockLspService {
    is_running: Arc<TokioMutex<bool>>,
    project_path: Arc<TokioMutex<Option<String>>>,
    documents: Arc<TokioMutex<HashMap<String, String>>>,
}

impl Default for MockLspService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockLspService {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(TokioMutex::new(false)),
            project_path: Arc::new(TokioMutex::new(None)),
            documents: Arc::new(TokioMutex::new(HashMap::new())),
        }
    }

    pub async fn set_running(&self, running: bool) {
        *self.is_running.lock().await = running;
    }

    pub async fn set_project_path(&self, path: Option<String>) {
        *self.project_path.lock().await = path;
    }

    pub async fn get_documents(&self) -> HashMap<String, String> {
        self.documents.lock().await.clone()
    }
}

#[async_trait]
impl LspService for MockLspService {
    async fn start_lsp_server(&self, project_path: String) -> Result<(), String> {
        *self.is_running.lock().await = true;
        *self.project_path.lock().await = Some(project_path);
        Ok(())
    }

    async fn stop_lsp_server(&self) -> Result<(), String> {
        *self.is_running.lock().await = false;
        *self.project_path.lock().await = None;
        Ok(())
    }

    async fn is_lsp_running(&self) -> bool {
        *self.is_running.lock().await
    }

    async fn initialize_lsp(&self, project_path: String) -> Result<(), String> {
        *self.project_path.lock().await = Some(project_path);
        Ok(())
    }

    async fn initialize_julia_lsp(&self) -> Result<(), String> {
        // Mock implementation - just return success
        Ok(())
    }

    async fn set_julia_lsp_stderr_sender(
        &self,
        _sender: tokio::sync::mpsc::UnboundedSender<String>,
    ) -> Result<(), String> {
        // Mock implementation - just return success
        Ok(())
    }

    async fn shutdown_lsp(&self) -> Result<(), String> {
        *self.is_running.lock().await = false;
        Ok(())
    }

    async fn notify_did_open(
        &self,
        uri: String,
        content: String,
        _language: String,
    ) -> Result<(), String> {
        self.documents.lock().await.insert(uri, content);
        Ok(())
    }

    async fn notify_did_change(&self, uri: String, content: String) -> Result<(), String> {
        self.documents.lock().await.insert(uri, content);
        Ok(())
    }

    async fn notify_did_close(&self, uri: String) -> Result<(), String> {
        self.documents.lock().await.remove(&uri);
        Ok(())
    }

    async fn notify_did_save(&self, _uri: String) -> Result<(), String> {
        Ok(())
    }

    async fn resolve_completion(
        &self,
        completion_item: LspCompletionItem,
    ) -> Result<LspCompletionItem, String> {
        Ok(completion_item)
    }

    async fn get_declaration(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Vec<LspLocation>, String> {
        Ok(vec![])
    }

    async fn get_type_definition(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Vec<LspLocation>, String> {
        Ok(vec![])
    }

    async fn get_implementation(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Vec<LspLocation>, String> {
        Ok(vec![])
    }

    async fn get_document_highlights(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Vec<LspRange>, String> {
        Ok(vec![])
    }

    async fn execute_command(
        &self,
        _command: String,
        _arguments: Option<Vec<serde_json::Value>>,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::Value::Null)
    }

    async fn prepare_rename(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Option<LspRange>, String> {
        Ok(None)
    }

    async fn rename(
        &self,
        _uri: String,
        _position: LspPosition,
        _new_name: String,
    ) -> Result<Option<LspWorkspaceEdit>, String> {
        Ok(None)
    }

    async fn format_document(
        &self,
        _uri: String,
        _options: serde_json::Value,
    ) -> Result<Vec<LspTextEdit>, String> {
        Ok(vec![])
    }

    async fn format_range(
        &self,
        _uri: String,
        _range: LspRange,
        _options: serde_json::Value,
    ) -> Result<Vec<LspTextEdit>, String> {
        Ok(vec![])
    }

    async fn format_on_type(
        &self,
        _uri: String,
        _position: LspPosition,
        _ch: String,
        _options: serde_json::Value,
    ) -> Result<Vec<LspTextEdit>, String> {
        Ok(vec![])
    }

    async fn get_folding_ranges(&self, _uri: String) -> Result<Vec<LspRange>, String> {
        Ok(vec![])
    }

    async fn get_selection_ranges(
        &self,
        _uri: String,
        _positions: Vec<LspPosition>,
    ) -> Result<Vec<LspRange>, String> {
        Ok(vec![])
    }



    async fn get_semantic_tokens_delta(
        &self,
        _uri: String,
        _previous_result_id: String,
    ) -> Result<Vec<LspSemanticToken>, String> {
        Ok(vec![])
    }

    async fn resolve_inlay_hint(&self, hint: LspInlayHint) -> Result<LspInlayHint, String> {
        Ok(hint)
    }

    async fn get_workspace_folders(&self) -> Result<Vec<String>, String> {
        Ok(vec![])
    }

    async fn get_workspace_configuration(
        &self,
        _section: String,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::Value::Null)
    }

    async fn get_document_links(&self, _uri: String) -> Result<Vec<LspLocation>, String> {
        Ok(vec![])
    }

    async fn resolve_document_link(&self, link: LspLocation) -> Result<LspLocation, String> {
        Ok(link)
    }

    async fn get_document_colors(&self, _uri: String) -> Result<Vec<serde_json::Value>, String> {
        Ok(vec![])
    }

    async fn get_color_presentations(
        &self,
        _uri: String,
        _color: serde_json::Value,
        _range: LspRange,
    ) -> Result<Vec<serde_json::Value>, String> {
        Ok(vec![])
    }

    async fn prepare_type_hierarchy(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Vec<LspCallHierarchyItem>, String> {
        Ok(vec![])
    }

    async fn get_type_hierarchy_supertypes(
        &self,
        _item: LspCallHierarchyItem,
    ) -> Result<Vec<LspCallHierarchyItem>, String> {
        Ok(vec![])
    }

    async fn get_type_hierarchy_subtypes(
        &self,
        _item: LspCallHierarchyItem,
    ) -> Result<Vec<LspCallHierarchyItem>, String> {
        Ok(vec![])
    }

    async fn get_inline_values(
        &self,
        _uri: String,
        _range: LspRange,
        _context: serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, String> {
        Ok(vec![])
    }

    async fn create_progress(
        &self,
        _token: String,
        _title: String,
        _message: Option<String>,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn report_progress(
        &self,
        _token: String,
        _message: String,
        _percentage: Option<u32>,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn end_progress(
        &self,
        _token: String,
        _message: Option<String>,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn get_julia_symbols(&self, _uri: String, _query: String) -> Result<Vec<String>, String> {
        Ok(vec![])
    }

    async fn get_julia_diagnostics(&self, _uri: String) -> Result<Vec<LspDiagnostic>, String> {
        Ok(vec![])
    }

    async fn get_julia_workspace_symbols(
        &self,
        _query: String,
    ) -> Result<Vec<LspDocumentSymbol>, String> {
        Ok(vec![])
    }

    async fn get_julia_references(
        &self,
        _uri: String,
        _position: LspPosition,
        _include_declaration: bool,
    ) -> Result<Vec<LspLocation>, String> {
        Ok(vec![])
    }

    async fn get_completions(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Vec<LspCompletionItem>, String> {
        Ok(vec![
            LspCompletionItem {
                label: "mock_completion".to_string(),
                kind: Some(1),
                detail: Some("Mock completion".to_string()),
                documentation: Some("Mock documentation".to_string()),
                sort_text: Some("1".to_string()),
                filter_text: Some("mock".to_string()),
                insert_text: Some("mock_completion".to_string()),
                insert_text_format: Some(1),
                text_edit: None,
                additional_text_edits: None,
                command: None,
                data: None,
            }
        ])
    }

    async fn get_completions_with_content(
        &self,
        uri: String,
        position: crate::types::LspPosition,
        _content: String,
    ) -> Result<Vec<crate::types::LspCompletionItem>, String> {
        self.get_completions(uri, position).await
    }

    async fn get_hover(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Option<LspHover>, String> {
        Ok(Some(LspHover {
            contents: vec![
                crate::types::LspMarkedString {
                    language: Some("julia".to_string()),
                    value: "Mock hover content".to_string(),
                }
            ],
            range: None,
        }))
    }

    async fn get_signature_help(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Option<LspSignatureHelp>, String> {
        Ok(Some(LspSignatureHelp {
            signatures: vec![
                crate::types::LspSignatureInformation {
                    label: "mock_function(arg1, arg2)".to_string(),
                    documentation: Some("Mock function documentation".to_string()),
                    parameters: None,

                }
            ],
            active_signature: Some(0),
            active_parameter: Some(0),
        }))
    }

    async fn get_definition(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Vec<LspLocation>, String> {
        Ok(vec![LspLocation {
            uri: "file:///mock/definition.jl".to_string(),
            range: LspRange {
                start: LspPosition { line: 0, character: 0 },
                end: LspPosition { line: 0, character: 10 },
            },
        }])
    }

    async fn get_references(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Vec<LspLocation>, String> {
        Ok(vec![LspLocation {
            uri: "file:///mock/reference.jl".to_string(),
            range: LspRange {
                start: LspPosition { line: 0, character: 0 },
                end: LspPosition { line: 0, character: 10 },
            },
        }])
    }

    async fn get_document_symbols(&self, _uri: String) -> Result<Vec<LspDocumentSymbol>, String> {
        Ok(vec![LspDocumentSymbol {
            name: "mock_symbol".to_string(),
            detail: Some("Mock symbol".to_string()),
            kind: 1,
            deprecated: Some(false),
            range: LspRange {
                start: LspPosition { line: 0, character: 0 },
                end: LspPosition { line: 0, character: 10 },
            },
            selection_range: LspRange {
                start: LspPosition { line: 0, character: 0 },
                end: LspPosition { line: 0, character: 10 },
            },
            children: None,
        }])
    }

    async fn get_code_actions(
        &self,
        _uri: String,
        _range: LspRange,
        _context: Vec<LspDiagnostic>,
    ) -> Result<Vec<LspCodeAction>, String> {
        Ok(vec![LspCodeAction {
            title: "Mock action".to_string(),
            kind: Some("quickfix".to_string()),
            diagnostics: None,
            is_preferred: Some(true),
            data: None,
            edit: None,
            command: None,
        }])
    }

    async fn get_diagnostics(&self, _uri: String) -> Result<Vec<LspDiagnostic>, String> {
        Ok(vec![LspDiagnostic {
            range: LspRange {
                start: LspPosition { line: 0, character: 0 },
                end: LspPosition { line: 0, character: 10 },
            },
            severity: Some(1),
            code: Some("MOCK001".to_string()),
            source: Some("mock".to_string()),
            message: "Mock diagnostic".to_string(),
            tags: None,
            related_information: None,
            data: None,
        }])
    }

    async fn update_document(&self, _uri: String, _content: String) -> Result<(), String> {
        Ok(())
    }

    async fn invalidate_cache(&self, _uri: String) -> Result<(), String> {
        Ok(())
    }

    async fn get_semantic_tokens(
        &self,
        _uri: String,
        _range: Option<LspRange>,
    ) -> Result<Vec<LspSemanticToken>, String> {
        Ok(vec![LspSemanticToken {
            delta_line: 0,
            delta_start: 0,
            length: 10,
            token_type: 0,
            token_modifiers_bitset: 0,
        }])
    }

    async fn get_inlay_hints(&self, _uri: String, _range: LspRange) -> Result<Vec<LspInlayHint>, String> {
        Ok(vec![LspInlayHint {
            position: LspPosition { line: 0, character: 5 },
            label: vec![crate::types::LspInlayHintLabelPart {
                value: "Mock hint".to_string(),
                tooltip: None,
                location: None,
                command: None,
            }],
            kind: Some(1),
            text_edits: None,
            tooltip: None,
            padding_left: Some(true),
            padding_right: Some(false),
            data: None,
        }])
    }

    async fn prepare_call_hierarchy(
        &self,
        _uri: String,
        _position: LspPosition,
    ) -> Result<Vec<LspCallHierarchyItem>, String> {
        Ok(vec![LspCallHierarchyItem {
            name: "mock_function".to_string(),
            kind: 1,
            tags: None,
            detail: Some("Mock function".to_string()),
            uri: "file:///mock/function.jl".to_string(),
            range: LspRange {
                start: LspPosition { line: 0, character: 0 },
                end: LspPosition { line: 0, character: 10 },
            },
            selection_range: LspRange {
                start: LspPosition { line: 0, character: 0 },
                end: LspPosition { line: 0, character: 10 },
            },
            data: None,
        }])
    }

    async fn get_call_hierarchy_incoming_calls(
        &self,
        _item: LspCallHierarchyItem,
    ) -> Result<Vec<LspCallHierarchyItem>, String> {
        Ok(vec![])
    }

    async fn get_call_hierarchy_outgoing_calls(
        &self,
        _item: LspCallHierarchyItem,
    ) -> Result<Vec<LspCallHierarchyItem>, String> {
        Ok(vec![])
    }

    async fn get_workspace_symbols(&self, _query: String) -> Result<Vec<LspDocumentSymbol>, String> {
        Ok(vec![])
    }

    async fn check_lsp_health(&self) -> Result<bool, String> { Ok(*self.is_running.lock().await) }
    async fn send_request(&self, _request: serde_json::Value) -> Result<serde_json::Value, String> { Ok(serde_json::json!({"ok":true})) }
    async fn get_capabilities(&self) -> Result<serde_json::Value, String> { Ok(serde_json::json!({})) }
}
