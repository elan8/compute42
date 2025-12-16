use actix::prelude::*;

// ============================
// File operations messages
// ============================

#[derive(Message)]
#[rtype(result = "Result<String, String>")]
pub struct ReadFileContent {
    pub path: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct WriteFileContent {
    pub path: String,
    pub content: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct CreateFile {
    pub path: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct CreateDirectory {
    pub path: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct DeleteEntry {
    pub path: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct RenameEntry {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Message)]
#[rtype(result = "Result<bool, String>")]
pub struct PathExists {
    pub path: String,
}

#[derive(Message)]
#[rtype(result = "Result<serde_json::Value, String>")]
pub struct BuildFileTree {
    pub root_path: String,
}

#[derive(Message)]
#[rtype(result = "Result<serde_json::Value, String>")]
pub struct LoadDirectoryContents {
    pub path: String,
}

// ============================
// File watching messages
// ============================

#[derive(Message)]
#[rtype(result = "Result<String, String>")]
pub struct StartFileWatcher {
    pub path: String,
    pub recursive: bool,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct StopFileWatcher {
    pub watcher_id: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct StopAllFileWatchers;

// ============================================================================
// ProjectActor Messages
// ============================================================================

#[derive(Message)]
#[rtype(result = "Result<serde_json::Value, String>")]
pub struct ReadProjectToml {
    pub project_path: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct WriteProjectToml {
    pub config: serde_json::Value,
}
