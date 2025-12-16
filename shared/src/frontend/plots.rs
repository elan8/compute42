use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct PlotData {
    pub id: String,
    pub mime_type: String,
    pub data: String,
    pub source_file: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    #[ts(type = "number")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../app/src/types/bindings/shared/")]
pub struct PlotServerInfo {
    pub port: u16,
    pub is_running: bool,
    pub plots: Vec<PlotData>,
}


