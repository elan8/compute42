use actix::prelude::*;

// ============================================================================
// ProcessActor Messages
// ============================================================================

/// Start Julia process
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct StartJuliaProcess {
    pub orchestrator_addr: Option<actix::Addr<crate::actors::orchestrator_actor::OrchestratorActor>>,
}

/// Stop Julia process
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct StopJuliaProcess;

/// Check if Julia is running
#[derive(Message)]
#[rtype(result = "Result<bool, String>")]
pub struct IsJuliaRunning;

/// Get pipe names
#[derive(Message)]
#[rtype(result = "Result<(String, String), String>")]
pub struct GetPipeNames;

/// Restart Julia
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct RestartJulia;

/// Set orchestrator actor address
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetOrchestratorActor {
    pub orchestrator_actor: actix::Addr<crate::actors::OrchestratorActor>,
}

/// Set output suppression state
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetOutputSuppression {
    pub suppressed: bool,
}

/// Set communication actor address
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetCommunicationActor {
    pub communication_actor: actix::Addr<crate::actors::CommunicationActor>,
}

/// Set current notebook cell for output buffering
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetNotebookCell {
    pub cell_id: Option<String>, // None to clear
}

/// Get and clear buffered output for current notebook cell
#[derive(Message)]
#[rtype(result = "Result<Option<crate::actors::process_actor::state::NotebookCellOutputBuffer>, String>")]
pub struct GetNotebookCellOutput;

/// Buffer plot data for current notebook cell
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct BufferNotebookCellPlot {
    pub mime_type: String,
    pub data: String,
}