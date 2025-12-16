use actix::prelude::*;
use serde::{Deserialize, Serialize};
use shared::auth::User;

// ============================================================================
// Inter-Actor Communication Messages
// ============================================================================

/// Messages for coordination between actors during startup/shutdown
#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct ActorStartupComplete {
    pub actor_name: String,
}

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct ActorShutdownComplete {
    pub actor_name: String,
}

/// Messages for project lifecycle coordination
#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct ProjectActivated {
    pub project_path: String,
}

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct ProjectDeactivated;

/// Messages for Julia process coordination
#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct JuliaProcessStarted {
    pub pipe_names: (String, String),
}

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct JuliaProcessStopped;

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct JuliaProcessRestarted {
    pub pipe_names: (String, String),
}

/// Messages for LSP coordination
#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct LspServerStarted {
    pub project_path: String,
    pub port: Option<u16>,
}

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct LspServerStopped;

/// LSP is fully ready (packages loaded, initialization complete)
#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct LspReady;

/// Messages for plot coordination
#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct PlotServerStarted {
    pub port: u16,
}

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct PlotServerStopped;

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct PlotDataReceived {
    pub plot_data_json: serde_json::Value,
}

/// Messages for file server coordination
#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct FileServerStarted {
    pub root_path: String,
    pub port: u16,
}

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct FileServerStopped;

/// Messages for installation coordination
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct JuliaInstallationDetected {
    pub installation_path: String,
    pub version: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct JuliaInstallationCompleted {
    pub installation_path: String,
    pub version: String,
}

/// Messages for sysimage coordination
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SysimageAvailable {
    pub sysimage_path: std::path::PathBuf,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SysimageCompilationStarted;

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SysimageCompilationCompleted {
    pub sysimage_path: std::path::PathBuf,
}

/// Messages for configuration coordination
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ConfigurationLoaded {
    pub config: serde_json::Value,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ConfigurationChanged {
    pub config: serde_json::Value,
}

/// Messages for state coordination
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct TabStateChanged {
    pub tab_id: String,
    pub is_dirty: bool,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ProjectStateChanged {
    pub project_path: Option<String>,
}

/// Messages for execution coordination
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ExecutionStarted {
    pub execution_id: String,
    pub code: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ExecutionCompleted {
    pub execution_id: String,
    pub result: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ExecutionFailed {
    pub execution_id: String,
    pub error: String,
}

/// Messages for communication coordination
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct CommunicationConnected {
    pub code_pipe: String,
    pub plot_pipe: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct CommunicationDisconnected;

/// Messages for account coordination
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct UserAuthenticated {
    pub user: User,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct UserLoggedOut;

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetupCompleted;

/// Messages for error propagation between actors
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ActorError {
    pub actor_name: String,
    pub error: String,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
}

/// Messages for health checks between actors
#[derive(Message)]
#[rtype(result = "Result<ActorHealth, String>")]
pub struct HealthCheck;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorHealth {
    pub actor_name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Messages for system-wide coordination
#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct SystemReady;

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct SystemShutdown;

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct EmergencyShutdown {
    pub reason: String,
}

/// Messages for resource management coordination
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ResourceAcquired {
    pub resource_type: String,
    pub resource_id: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ResourceReleased {
    pub resource_type: String,
    pub resource_id: String,
}

/// Messages for dependency coordination
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct DependencyReady {
    pub dependency_name: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct DependencyFailed {
    pub dependency_name: String,
    pub error: String,
}

/// Messages for performance monitoring
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct PerformanceMetric {
    pub metric_name: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: i64,
}

/// Messages for debugging and logging coordination
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct DebugLog {
    pub level: String,
    pub message: String,
    pub actor_name: String,
    pub timestamp: i64,
}