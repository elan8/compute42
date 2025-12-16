use serde::{Deserialize, Serialize};
use ts_rs::TS;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspLocation {
    pub uri: String,
    pub range: LspRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspTextEdit {
    pub range: LspRange,
    pub new_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspCommand {
    pub title: String,
    pub command: String,
    #[ts(type = "unknown[] | null")]
    pub arguments: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
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
    #[ts(type = "unknown | null")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspMarkedString {
    pub language: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspHover {
    pub contents: Vec<LspMarkedString>,
    pub range: Option<LspRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspSignatureInformation {
    pub label: String,
    pub documentation: Option<String>,
    pub parameters: Option<Vec<LspParameterInformation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspParameterInformation {
    pub label: String,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspSignatureHelp {
    pub signatures: Vec<LspSignatureInformation>,
    pub active_signature: Option<u32>,
    pub active_parameter: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspDocumentSymbol {
    pub name: String,
    pub detail: Option<String>,
    pub kind: u32,
    pub deprecated: Option<bool>,
    pub range: LspRange,
    pub selection_range: LspRange,
    pub children: Option<Vec<LspDocumentSymbol>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspDiagnosticRelatedInformation {
    pub location: LspLocation,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: Option<u32>,
    pub code: Option<String>,
    pub source: Option<String>,
    pub message: String,
    pub tags: Option<Vec<u32>>,
    pub related_information: Option<Vec<LspDiagnosticRelatedInformation>>,
    #[ts(type = "unknown | null")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspWorkspaceEdit {
    pub changes: Option<HashMap<String, Vec<LspTextEdit>>>,
    pub document_changes: Option<Vec<LspDocumentChange>>,
    pub change_annotations: Option<HashMap<String, LspChangeAnnotation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspVersionedTextDocumentIdentifier {
    pub uri: String,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspDocumentChange {
    pub text_document: LspVersionedTextDocumentIdentifier,
    pub edits: Vec<LspTextEdit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspChangeAnnotation {
    pub label: String,
    pub needs_confirmation: Option<bool>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspCallHierarchyItem {
    pub name: String,
    pub kind: u32,
    pub tags: Option<Vec<u32>>,
    pub detail: Option<String>,
    pub uri: String,
    pub range: LspRange,
    pub selection_range: LspRange,
    #[ts(type = "unknown | null")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspSemanticToken {
    pub delta_line: u32,
    pub delta_start: u32,
    pub length: u32,
    pub token_type: u32,
    pub token_modifiers_bitset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspInlayHintLabelPart {
    pub value: String,
    pub tooltip: Option<String>,
    pub location: Option<LspLocation>,
    pub command: Option<LspCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspInlayHint {
    pub position: LspPosition,
    pub label: Vec<LspInlayHintLabelPart>,
    pub kind: Option<u32>,
    pub text_edits: Option<Vec<LspTextEdit>>,
    pub tooltip: Option<String>,
    pub padding_left: Option<bool>,
    pub padding_right: Option<bool>,
    #[ts(type = "unknown | null")]
    pub data: Option<serde_json::Value>,
}


