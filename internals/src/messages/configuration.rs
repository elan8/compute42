use actix::prelude::*;

// ============================================================================
// ConfigurationActor Messages
// ============================================================================

/// Update configuration state (internal message)
#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateConfigState {
    pub config_value: serde_json::Value,
}

/// Save configuration
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SaveConfig {
    pub config: serde_json::Value,
}

/// Get Julia path
#[derive(Message)]
#[rtype(result = "Result<Option<String>, String>")]
pub struct GetJuliaPath;

/// Set Julia path
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetJuliaPath {
    pub path: Option<String>,
}

/// Get root folder
#[derive(Message)]
#[rtype(result = "Result<Option<String>, String>")]
pub struct GetRootFolder;

/// Set root folder
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetRootFolder {
    pub folder: Option<String>,
}


/// Get user email
#[derive(Message)]
#[rtype(result = "Result<Option<String>, String>")]
pub struct GetUserEmail;

/// Set user email
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetUserEmail {
    pub email: Option<String>,
}

/// Get font and editor settings
#[derive(Message)]
#[rtype(result = "Result<serde_json::Value, String>")]
pub struct GetFontSettings;

/// Set font and editor settings
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SetFontSettings {
    pub editor_font_family: Option<String>,
    pub editor_font_size: Option<u16>,
    pub terminal_font_family: Option<String>,
    pub terminal_font_size: Option<u16>,
    pub editor_word_wrap: Option<bool>,
    pub editor_tab_size: Option<u16>,
    pub editor_line_numbers: Option<bool>,
    pub editor_minimap: Option<bool>,
    pub editor_color_scheme: Option<String>,
}