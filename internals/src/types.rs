use serde::{Deserialize, Serialize};

/// User preferences structure
/// Contains only user preferences that need to persist
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserPreferences {
    /// Last opened folder/workspace path
    pub last_opened_folder: Option<String>,
    /// Email of the currently logged in user
    pub user_email: Option<String>,
    /// Editor font family
    pub editor_font_family: Option<String>,
    /// Editor font size
    pub editor_font_size: Option<u16>,
    /// Terminal font family
    pub terminal_font_family: Option<String>,
    /// Terminal font size
    pub terminal_font_size: Option<u16>,
    /// Editor word wrap setting
    pub editor_word_wrap: Option<bool>,
    /// Editor tab size
    pub editor_tab_size: Option<u16>,
    /// Editor line numbers visibility
    pub editor_line_numbers: Option<bool>,
    /// Editor minimap visibility
    pub editor_minimap: Option<bool>,
    /// Editor color scheme/theme
    pub editor_color_scheme: Option<String>,
}

impl UserPreferences {
    /// Create a new UserPreferences with default values
    pub fn new() -> Self {
        Self {
            last_opened_folder: None,
            user_email: None,
            editor_font_family: None,
            editor_font_size: None,
            terminal_font_family: None,
            terminal_font_size: None,
            editor_word_wrap: None,
            editor_tab_size: None,
            editor_line_numbers: None,
            editor_minimap: None,
            editor_color_scheme: None,
        }
    }
    
    /// Deserialize from JSON value with null handling
    pub fn from_json_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        // Handle null values gracefully
        if value.is_null() {
            return Ok(UserPreferences::new());
        }
        
        // Handle object with null fields
        if let Some(obj) = value.as_object() {
            let mut prefs = UserPreferences::new();
            
            if let Some(folder) = obj.get("last_opened_folder") {
                if !folder.is_null() {
                    prefs.last_opened_folder = folder.as_str().map(|s| s.to_string());
                }
            }
            
            if let Some(email) = obj.get("user_email") {
                if !email.is_null() {
                    prefs.user_email = email.as_str().map(|s| s.to_string());
                }
            }
            
            if let Some(font_family) = obj.get("editor_font_family") {
                if !font_family.is_null() {
                    prefs.editor_font_family = font_family.as_str().map(|s| s.to_string());
                }
            }
            
            if let Some(font_size) = obj.get("editor_font_size") {
                if !font_size.is_null() {
                    prefs.editor_font_size = font_size.as_u64().map(|v| v as u16);
                }
            }
            
            if let Some(font_family) = obj.get("terminal_font_family") {
                if !font_family.is_null() {
                    prefs.terminal_font_family = font_family.as_str().map(|s| s.to_string());
                }
            }
            
            if let Some(font_size) = obj.get("terminal_font_size") {
                if !font_size.is_null() {
                    prefs.terminal_font_size = font_size.as_u64().map(|v| v as u16);
                }
            }
            
            if let Some(word_wrap) = obj.get("editor_word_wrap") {
                if !word_wrap.is_null() {
                    prefs.editor_word_wrap = word_wrap.as_bool();
                }
            }
            
            if let Some(tab_size) = obj.get("editor_tab_size") {
                if !tab_size.is_null() {
                    prefs.editor_tab_size = tab_size.as_u64().map(|v| v as u16);
                }
            }
            
            if let Some(line_numbers) = obj.get("editor_line_numbers") {
                if !line_numbers.is_null() {
                    prefs.editor_line_numbers = line_numbers.as_bool();
                }
            }
            
            if let Some(minimap) = obj.get("editor_minimap") {
                if !minimap.is_null() {
                    prefs.editor_minimap = minimap.as_bool();
                }
            }
            
            if let Some(color_scheme) = obj.get("editor_color_scheme") {
                if !color_scheme.is_null() {
                    prefs.editor_color_scheme = color_scheme.as_str().map(|s| s.to_string());
                }
            }
            
            return Ok(prefs);
        }
        
        // Fallback to standard deserialization
        serde_json::from_value(value)
    }
}


// ============================================================================
// LSP Data Structures
// ============================================================================

/// LSP Position structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

/// LSP Range structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP Location structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspLocation {
    pub uri: String,
    pub range: LspRange,
}

/// LSP Completion Item structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspCompletionItem {
    pub label: String,
    pub kind: Option<u32>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub sort_text: Option<String>,
    pub filter_text: Option<String>,
    pub insert_text: Option<String>,
    pub insert_text_format: Option<u32>,
    pub text_edit: Option<LspTextEdit>,
    pub additional_text_edits: Option<Vec<LspTextEdit>>,
    pub command: Option<LspCommand>,
    pub data: Option<serde_json::Value>,
}

/// LSP Text Edit structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspTextEdit {
    pub range: LspRange,
    pub new_text: String,
}

/// LSP Command structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspCommand {
    pub title: String,
    pub command: String,
    pub arguments: Option<Vec<serde_json::Value>>,
}

/// LSP Hover structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspHover {
    pub contents: Vec<LspMarkedString>,
    pub range: Option<LspRange>,
}

/// LSP Marked String structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspMarkedString {
    pub language: Option<String>,
    pub value: String,
}

/// LSP Signature Help structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspSignatureHelp {
    pub signatures: Vec<LspSignatureInformation>,
    pub active_signature: Option<u32>,
    pub active_parameter: Option<u32>,
}

/// LSP Signature Information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspSignatureInformation {
    pub label: String,
    pub documentation: Option<String>,
    pub parameters: Option<Vec<LspParameterInformation>>,
}

/// LSP Parameter Information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspParameterInformation {
    pub label: String,
    pub documentation: Option<String>,
}

/// LSP Document Symbol structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDocumentSymbol {
    pub name: String,
    pub detail: Option<String>,
    pub kind: u32,
    pub deprecated: Option<bool>,
    pub range: LspRange,
    pub selection_range: LspRange,
    pub children: Option<Vec<LspDocumentSymbol>>,
}

/// LSP Code Action structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspCodeAction {
    pub title: String,
    pub kind: Option<String>,
    pub diagnostics: Option<Vec<LspDiagnostic>>,
    pub is_preferred: Option<bool>,
    pub edit: Option<LspWorkspaceEdit>,
    pub command: Option<LspCommand>,
    pub data: Option<serde_json::Value>,
}

/// LSP Diagnostic structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: Option<u32>,
    pub code: Option<String>,
    pub source: Option<String>,
    pub message: String,
    pub tags: Option<Vec<u32>>,
    pub related_information: Option<Vec<LspDiagnosticRelatedInformation>>,
    pub data: Option<serde_json::Value>,
}

/// LSP Diagnostic Related Information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnosticRelatedInformation {
    pub location: LspLocation,
    pub message: String,
}

/// LSP Workspace Edit structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspWorkspaceEdit {
    pub changes: Option<std::collections::HashMap<String, Vec<LspTextEdit>>>,
    pub document_changes: Option<Vec<LspDocumentChange>>,
    pub change_annotations: Option<std::collections::HashMap<String, LspChangeAnnotation>>,
}

/// LSP Document Change structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDocumentChange {
    pub text_document: LspVersionedTextDocumentIdentifier,
    pub edits: Vec<LspTextEdit>,
}

/// LSP Versioned Text Document Identifier structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspVersionedTextDocumentIdentifier {
    pub uri: String,
    pub version: i32,
}

/// LSP Change Annotation structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspChangeAnnotation {
    pub label: String,
    pub needs_confirmation: Option<bool>,
    pub description: Option<String>,
}

/// LSP Call Hierarchy Item structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspCallHierarchyItem {
    pub name: String,
    pub kind: u32,
    pub tags: Option<Vec<u32>>,
    pub detail: Option<String>,
    pub uri: String,
    pub range: LspRange,
    pub selection_range: LspRange,
    pub data: Option<serde_json::Value>,
}

/// LSP Semantic Token structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspSemanticToken {
    pub delta_line: u32,
    pub delta_start: u32,
    pub length: u32,
    pub token_type: u32,
    pub token_modifiers_bitset: u32,
}

/// LSP Inlay Hint structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspInlayHint {
    pub position: LspPosition,
    pub label: Vec<LspInlayHintLabelPart>,
    pub kind: Option<u32>,
    pub text_edits: Option<Vec<LspTextEdit>>,
    pub tooltip: Option<String>,
    pub padding_left: Option<bool>,
    pub padding_right: Option<bool>,
    pub data: Option<serde_json::Value>,
}

/// LSP Inlay Hint Label Part structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspInlayHintLabelPart {
    pub value: String,
    pub tooltip: Option<String>,
    pub location: Option<LspLocation>,
    pub command: Option<LspCommand>,
}

// ============================================================================
// Frontend State Structures
// ============================================================================

/// Tab structure for frontend state management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: String,
    pub title: String,
    pub path: Option<String>,
    pub content: String,
    pub is_dirty: bool,
}

/// Tab state enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TabState {
    Active,
    Inactive,
    Dirty,
    Clean,
}

/// Configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Configuration {
    pub julia_path: Option<String>,
    pub root_folder: Option<String>,
}

/// Orchestrator state enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestratorState {
    Initializing,
    Ready,
    Running,
    Stopped,
    Error,
    WaitingForAuth,
}

/// Project information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub julia_version: Option<String>,
    pub packages: Vec<String>,
}

/// Julia installation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JuliaInstallation {
    pub path: String,
    pub version: String,
    pub is_valid: bool,
}

/// LSP server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspServerInfo {
    pub project_path: String,
    pub port: Option<u16>,
    pub is_running: bool,
    pub error_message: Option<String>,
    pub status: LspServerStatus,
}

/// LSP server status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LspServerStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

/// Plot server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotServerInfo {
    pub port: u16,
    pub is_running: bool,
    pub plots: Vec<crate::messages::PlotData>,
}

/// File server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileServerInfo {
    pub root_path: String,
    pub port: u16,
    pub is_running: bool,
}

/// Sysimage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysimageInfo {
    pub path: std::path::PathBuf,
    pub is_available: bool,
    pub compilation_state: SysimageCompilationState,
}

/// Sysimage compilation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SysimageCompilationState {
    Idle,
    Compiling,
    Completed,
    Failed,
}

// ============================================================================
// Account Management Types
// ============================================================================

/// Account state enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountState {
    Checking,
    NoUser,
    Unauthenticated,
    Authenticated,
    SetupIncomplete,
    Error { message: String },
}
