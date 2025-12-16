use log::{debug, error};

/// Startup phase state machine for orchestrator
/// This replaces multiple boolean flags with a single source of truth
/// Note: Some phases are reused for both startup and project switching
#[derive(Debug, Clone, PartialEq)]
pub enum StartupPhase {
    /// Startup has not been initiated
    NotStarted,
    /// Checking for application updates
    CheckingForUpdates,
    /// Checking if Julia is installed
    CheckingJulia,
    /// Julia installation is in progress (can take a long time)
    InstallingJulia,
    /// Starting the Julia process
    StartingJuliaProcess,
    /// Starting the plot server
    StartingPlotServer,
    /// Starting the file server
    StartingFileServer,
    /// Activating the Julia project (Pkg.instantiate)
    /// Used both during startup and when switching projects
    ActivatingProject,
    /// Starting the LSP server
    /// Used both during startup and when switching projects
    StartingLsp,
    /// Waiting for LSP server to be ready
    /// Used both during startup and when switching projects
    WaitingForLspReady,
    /// Startup completed successfully (system is running)
    Completed,
    /// Startup failed with error message
    Failed(String),
}

impl StartupPhase {
    /// Check if a transition from current phase to next phase is valid
    pub fn can_transition_to(&self, next: &StartupPhase) -> bool {
        match (self, next) {
            // Valid transitions
            (StartupPhase::NotStarted, StartupPhase::CheckingForUpdates) => true,
            (StartupPhase::CheckingForUpdates, StartupPhase::CheckingJulia) => true,
            (StartupPhase::CheckingJulia, StartupPhase::InstallingJulia) => true,
            (StartupPhase::CheckingJulia, StartupPhase::StartingJuliaProcess) => true,
            (StartupPhase::InstallingJulia, StartupPhase::StartingJuliaProcess) => true,
            (StartupPhase::StartingJuliaProcess, StartupPhase::StartingPlotServer) => true,
            (StartupPhase::StartingPlotServer, StartupPhase::StartingFileServer) => true,
            (StartupPhase::StartingFileServer, StartupPhase::ActivatingProject) => true,
            (StartupPhase::StartingJuliaProcess, StartupPhase::ActivatingProject) => true,
            (StartupPhase::StartingJuliaProcess, StartupPhase::StartingLsp) => true,
            (StartupPhase::StartingJuliaProcess, StartupPhase::Completed) => true,
            (StartupPhase::ActivatingProject, StartupPhase::StartingLsp) => true,
            (StartupPhase::StartingLsp, StartupPhase::WaitingForLspReady) => true,
            (StartupPhase::WaitingForLspReady, StartupPhase::Completed) => true,
            // Allow transitions from Completed back to project activation phases (for project switching)
            (StartupPhase::Completed, StartupPhase::ActivatingProject) => true,
            (StartupPhase::Completed, StartupPhase::StartingLsp) => true,
            
            // Can always transition to Failed
            (_, StartupPhase::Failed(_)) => true,
            
            // Can always stay in current state (idempotent transitions)
            (a, b) if a == b => true,
            
            // Invalid transitions
            _ => false,
        }
    }
    
    /// Transition to next phase if valid, otherwise log error and return current state
    pub fn transition_to(self, next: StartupPhase) -> StartupPhase {
        if self.can_transition_to(&next) {
            debug!("StartupPhase: Transitioning from {:?} to {:?}", self, next);
            next
        } else {
            error!("StartupPhase: Invalid transition from {:?} to {:?}", self, next);
            self
        }
    }
    
    /// Check if startup is in progress (not completed or failed)
    pub fn is_in_progress(&self) -> bool {
        !matches!(self, StartupPhase::Completed | StartupPhase::Failed(_) | StartupPhase::NotStarted)
    }
    
    /// Check if startup is completed
    pub fn is_completed(&self) -> bool {
        matches!(self, StartupPhase::Completed)
    }
    
    /// Check if startup failed
    pub fn is_failed(&self) -> bool {
        matches!(self, StartupPhase::Failed(_))
    }
    
    /// Get progress percentage for UI updates
    pub fn progress_percentage(&self) -> u8 {
        match self {
            StartupPhase::NotStarted => 0,
            StartupPhase::CheckingForUpdates => 5,
            StartupPhase::CheckingJulia => 15,
            StartupPhase::InstallingJulia => 30,
            StartupPhase::StartingJuliaProcess => 40,
            StartupPhase::StartingPlotServer => 50,
            StartupPhase::StartingFileServer => 55,
            StartupPhase::ActivatingProject => 65,
            StartupPhase::StartingLsp => 75,
            StartupPhase::WaitingForLspReady => 85,
            StartupPhase::Completed => 100,
            StartupPhase::Failed(_) => 0,
        }
    }
    
    /// Get user-friendly status message
    pub fn status_message(&self) -> &'static str {
        match self {
            StartupPhase::NotStarted => "Initializing...",
            StartupPhase::CheckingForUpdates => "Checking for updates...",
            StartupPhase::CheckingJulia => "Checking Julia installation...",
            StartupPhase::InstallingJulia => "Installing Julia...",
            StartupPhase::StartingJuliaProcess => "Starting Julia process...",
            StartupPhase::StartingPlotServer => "Starting plot server...",
            StartupPhase::StartingFileServer => "Starting file server...",
            StartupPhase::ActivatingProject => "Activating project...",
            StartupPhase::StartingLsp => "Starting language server...",
            StartupPhase::WaitingForLspReady => "Initializing language server...",
            StartupPhase::Completed => "Ready",
            StartupPhase::Failed(_) => "Startup failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_transitions() {
        assert!(StartupPhase::NotStarted.can_transition_to(&StartupPhase::CheckingForUpdates));
        assert!(StartupPhase::CheckingForUpdates.can_transition_to(&StartupPhase::CheckingJulia));
        assert!(StartupPhase::CheckingJulia.can_transition_to(&StartupPhase::InstallingJulia));
        assert!(StartupPhase::CheckingJulia.can_transition_to(&StartupPhase::StartingJuliaProcess));
    }
    
    #[test]
    fn test_invalid_transitions() {
        assert!(!StartupPhase::NotStarted.can_transition_to(&StartupPhase::Completed));
    }
    
    #[test]
    fn test_can_always_fail() {
        assert!(StartupPhase::NotStarted.can_transition_to(&StartupPhase::Failed("test".to_string())));
    }
}

