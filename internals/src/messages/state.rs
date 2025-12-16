use actix::prelude::*;
use crate::types::Tab;

// ============================================================================
// StateActor Messages
// ============================================================================

/// Add tab
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct AddTab {
    pub tab: Tab,
}

/// Remove tab
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct RemoveTab {
    pub tab_id: String,
}

/// Get tabs
#[derive(Message)]
#[rtype(result = "Result<Vec<Tab>, String>")]
pub struct GetTabs;

/// Update tab
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct UpdateTab {
    pub tab_id: String,
    pub updated_tab: Tab,
}

/// Clear tabs
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct ClearTabs;

/// Update tab content
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct UpdateTabContent {
    pub tab_id: String,
    pub content: String,
}

/// Save tab to file
#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct SaveTabToFile {
    pub tab_id: String,
}

/// Get dirty tabs
#[derive(Message)]
#[rtype(result = "Result<Vec<Tab>, String>")]
pub struct GetDirtyTabs;

/// Get current active tab
#[derive(Message)]
#[rtype(result = "Result<Option<Tab>, String>")]
pub struct GetCurrentTab;