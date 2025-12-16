// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
    // Fix PATH environment variable for GUI apps (especially important for AppImage)
    // This ensures Julia and other tools can be found even when PATH isn't inherited from shell
    let _ = fix_path_env::fix();
    
    // Start the main application
    app::run();
}
