use actix::prelude::*;

// ============================================================================
// OrchestratorActor Messages
// ============================================================================

/// Set actor addresses for coordination
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetActorAddresses {
    pub config_actor: actix::Addr<crate::actors::ConfigurationActor>,
    pub state_actor: actix::Addr<crate::actors::StateActor>,
    pub execution_actor: actix::Addr<crate::actors::ExecutionActor>,
    pub communication_actor: actix::Addr<crate::actors::CommunicationActor>,
    pub process_actor: actix::Addr<crate::actors::ProcessActor>,
    pub lsp_actor: actix::Addr<crate::actors::LspActor>,
    pub plot_actor: actix::Addr<crate::actors::PlotActor>,
    pub file_server_actor: actix::Addr<crate::actors::FileServerActor>,
    pub installation_actor: actix::Addr<crate::actors::InstallationActor>,
}

/// Get orchestrator state
#[derive(Message)]
#[rtype(result = "Result<String, String>")]
pub struct GetOrchestratorState;

/// Get orchestrator startup phase
#[derive(Message)]
#[rtype(result = "Result<String, String>")]
pub struct GetStartupPhase;

/// Get current project information
#[derive(Message)]
#[rtype(result = "Result<Option<crate::types::ProjectInfo>, String>")]
pub struct GetCurrentProject;

/// Start orchestrator
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct StartOrchestrator;

/// Continue orchestrator startup
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ContinueOrchestratorStartup;

/// Frontend ready signal
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct FrontendReady;

/// Shutdown orchestrator
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ShutdownOrchestrator;

/// Restart Julia orchestrator
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct RestartJuliaOrchestrator;

/// Change project directory
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ChangeProjectDirectory {
    pub project_path: String,
}

/// Julia pipes ready signal - indicates Julia's pipes are available for connection
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct JuliaPipesReady;

/// Julia message loop ready signal - indicates Julia's message handling loop is ready
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct JuliaMessageLoopReady;

/// Project activation complete signal - indicates Julia project activation and package instantiation is complete
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ProjectActivationComplete;

/// Update startup phase (internal message for state synchronization)
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct UpdateStartupPhase {
    pub phase: crate::actors::orchestrator_actor::StartupPhase,
}

/// Update current project (internal message for state synchronization)
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct UpdateCurrentProject {
    pub project_path: Option<String>,
}

/// Phase timeout watchdog message - triggered when a phase exceeds its timeout duration
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct PhaseTimeout {
    pub phase: crate::actors::orchestrator_actor::StartupPhase,
    pub duration_secs: u64,
}

/// Startup event for state machine handler
#[derive(Debug, Clone, PartialEq)]
pub enum StartupEvent {
    // External events
    FrontendReady,
    UpdateCheckComplete,
    AuthenticationComplete,
    JuliaPipesReady,
    JuliaMessageLoopReady,
    ProjectActivationComplete,
    LspReady,
    // Work completion events
    JuliaCheckComplete,
    JuliaProcessStarted,
    PlotServerStarted,
    FileServerStarted,
    FileServerFailed(String), // Non-fatal: file server failed but startup continues
    ProjectCheckComplete {
        has_project: bool,
    },
    LspStarted,
    // Error event
    StartupFailed(String),
}

/// Startup event message - sent to state machine handler
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct StartupEventMessage {
    pub event: StartupEvent,
}

/// State entered message - emitted when entering a new startup phase
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct StartupStateEntered {
    pub phase: crate::actors::orchestrator_actor::StartupPhase,
}


// Julia activity update removed