use actix::prelude::*;

// ============================================================================
// InstallationActor Messages
// ============================================================================

/// Check Julia installation
#[derive(Message)]
#[rtype(result = "Result<bool, String>")]
pub struct CheckJuliaInstallation;

/// Install Julia
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct InstallJulia {
    pub julia_version: Option<String>,
}

/// Get Julia version
#[derive(Message)]
#[rtype(result = "Result<Option<String>, String>")]
pub struct GetJuliaVersion;

/// Set orchestrator actor address for completion notifications
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetOrchestratorActor {
    pub orchestrator_actor: actix::Addr<crate::actors::orchestrator_actor::OrchestratorActor>,
}

/// Check if installation is in progress
#[derive(Message)]
#[rtype(result = "Result<bool, String>")]
pub struct IsInstallationInProgress;

/// Get Julia executable path
#[derive(Message)]
#[rtype(result = "Result<Option<String>, String>")]
pub struct GetJuliaExecutablePath;

/// Get Julia path from InstallationActor
#[derive(Message)]
#[rtype(result = "Result<Option<String>, String>")]
pub struct GetJuliaPathFromInstallation;

/// Ensure Julia is installed - checks if installed, installs if not, then emits StartupEvent
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct EnsureJuliaInstalled {
    pub orchestrator_addr: actix::Addr<crate::actors::orchestrator_actor::OrchestratorActor>,
}