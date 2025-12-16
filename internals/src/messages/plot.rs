use actix::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Plot Types
// ============================================================================

/// Plot data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotData {
    pub id: String,
    pub mime_type: String,
    pub data: String, // Base64 encoded
    pub timestamp: i64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub source_file: Option<String>,
    pub line_number: Option<u32>,
    pub code_context: Option<String>,
    pub session_id: Option<String>,
}

// ============================================================================
// PlotActor Messages
// ============================================================================

/// Start plot server
#[derive(Message)]
#[rtype(result = "Result<u16, String>")]
pub struct StartPlotServer {
    pub orchestrator_addr: Option<actix::Addr<crate::actors::orchestrator_actor::OrchestratorActor>>,
}

/// Stop plot server
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct StopPlotServer;

/// Get plot port
#[derive(Message)]
#[rtype(result = "Result<Option<u16>, String>")]
pub struct GetPlotPort;

/// Add plot
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct AddPlot {
    pub plot_data: PlotData,
}

/// Get plots
#[derive(Message)]
#[rtype(result = "Result<Vec<PlotData>, String>")]
pub struct GetPlots;

/// Delete plot
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct DeletePlot {
    pub plot_id: String,
}

/// Handle plot data received
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct HandlePlotDataReceived {
    pub plot_data_json: serde_json::Value,
}