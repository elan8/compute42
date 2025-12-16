use actix::prelude::*;

// ============================================================================
// FileServerActor Messages
// ============================================================================

/// Start file server
/// FileServerActor will get the root path from ConfigurationActor itself
#[derive(Message)]
#[rtype(result = "Result<u16, String>")]
pub struct StartFileServer {
    pub orchestrator_addr: Option<actix::Addr<crate::actors::orchestrator_actor::OrchestratorActor>>,
}

/// Stop file server
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct StopFileServer;

/// Get file server port
#[derive(Message)]
#[rtype(result = "Result<Option<u16>, String>")]
pub struct GetFileServerPort;

/// Check if file server is running
#[derive(Message)]
#[rtype(result = "Result<bool, String>")]
pub struct IsFileServerRunning;

/// Get file server URL
#[derive(Message)]
#[rtype(result = "Result<Option<String>, String>")]
pub struct GetFileServerUrl;