// Types used by InstallationActor

/// Julia version information
#[derive(Debug, Clone)]
pub struct JuliaVersion {
    #[allow(dead_code)]
    pub version: String,
    pub download_url: String,
    pub filename: String,
}

/// Installation status payload for frontend events
#[derive(Clone, serde::Serialize)]
#[allow(dead_code)]
pub struct JuliaInstallationStatusPayload {
    pub message: String,
    pub is_complete: bool,
    pub is_error: bool,
    pub error_details: Option<String>,
    pub progress_percentage: Option<u32>,
    pub progress_text: Option<String>,
}




