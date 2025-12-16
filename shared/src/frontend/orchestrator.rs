use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct Tab {
    pub id: String,
    pub title: String,
    pub path: Option<String>,
    pub content: String,
    pub is_dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub enum TabState {
    Active,
    Inactive,
    Dirty,
    Clean,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct Configuration {
    pub julia_path: Option<String>,
    pub root_folder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub enum OrchestratorState {
    Initializing,
    Ready,
    Running,
    Stopped,
    Error,
    WaitingForAuth,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub julia_version: Option<String>,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct JuliaInstallation {
    pub path: String,
    pub version: String,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct LspServerInfo {
    pub project_path: String,
    pub port: Option<u16>,
    pub is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct FileServerInfo {
    pub root_path: String,
    pub port: u16,
    pub is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct SysimageInfo {
    pub path: std::path::PathBuf,
    pub is_available: bool,
    pub compilation_state: SysimageCompilationState,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub enum SysimageCompilationState {
    Idle,
    Compiling,
    Completed,
    Failed,
}


