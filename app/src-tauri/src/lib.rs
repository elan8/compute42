// pub mod state_management;
// Use internals library instead of local modules
use log::{debug, error};
use std::sync::Arc;
use tauri::{Manager, path::BaseDirectory};

use crate::updater_service::UpdaterService;

// Keep only app-specific modules (commands wrappers, logger, updater, version helper)
pub mod logger;
pub mod updater_service;
pub mod commands;
pub mod error;
pub mod state;
mod tauri_emitter; // new: Tauri event emitter adapter
mod orchestrator_tauri;
mod windows_titlebar; // Windows title bar customization

use crate::orchestrator_tauri::restart_julia_orchestrator;

/// Copy demo folder from bundled resources to AppData directory
/// Only copies if the demo folder doesn't exist yet
#[allow(dead_code)]
fn copy_demo_folder_to_appdata(app: &tauri::App) -> Result<(), String> {
    // Get AppData directory
    let app_data_dir = dirs::data_local_dir()
        .ok_or_else(|| "Failed to get AppData directory".to_string())?;
    let demo_dest = app_data_dir.join("com.compute42.dev").join("demo");
    
    // Only copy if demo folder doesn't exist
    if demo_dest.exists() {
        debug!("[Lib] Demo folder already exists at {:?}, skipping copy", demo_dest);
        return Ok(());
    }
    
    // Try to resolve the demo folder from bundled resources
    let demo_source = match app.path().resolve("demo", BaseDirectory::Resource) {
        Ok(path) => {
            debug!("[Lib] Found demo folder in bundled resources: {:?}", path);
            path
        }
        Err(e) => {
            // Fallback: try workspace root (for development)
            debug!("[Lib] Demo folder not found in bundled resources: {}, trying workspace root", e);
            if let Ok(cwd) = std::env::current_dir() {
                let workspace_demo = cwd.join("demo");
                if workspace_demo.exists() && workspace_demo.is_dir() {
                    debug!("[Lib] Found demo folder at workspace root: {:?}", workspace_demo);
                    workspace_demo
                } else {
                    return Err(format!("Demo folder not found in resources or workspace root: {}", e));
                }
            } else {
                return Err(format!("Failed to get current directory and demo not in resources: {}", e));
            }
        }
    };
    
    if !demo_source.exists() {
        return Err(format!("Demo source folder does not exist: {:?}", demo_source));
    }
    
    if !demo_source.is_dir() {
        return Err(format!("Demo source is not a directory: {:?}", demo_source));
    }
    
    debug!("[Lib] Copying demo folder from {:?} to {:?}", demo_source, demo_dest);
    
    // Copy recursively
    copy_dir_recursive(&demo_source, &demo_dest)?;
    
    debug!("[Lib] Successfully copied demo folder to {:?}", demo_dest);
    Ok(())
}

/// Copy directory recursively
#[allow(dead_code)]
fn copy_dir_recursive(src: &std::path::PathBuf, dst: &std::path::PathBuf) -> Result<(), String> {
    use std::fs;
    
    if !src.exists() {
        return Err(format!("Source does not exist: {:?}", src));
    }
    
    if src.is_file() {
        // Create parent directory if needed
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }
        fs::copy(src, dst)
            .map_err(|e| format!("Failed to copy file {:?}: {}", src, e))?;
        return Ok(());
    }
    
    // Create destination directory
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create destination directory: {}", e))?;
    
    // Copy all entries
    for entry in fs::read_dir(src)
        .map_err(|e| format!("Failed to read source directory {:?}: {}", src, e))? 
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy file {:?}: {}", src_path, e))?;
        }
    }
    
    Ok(())
}
use crate::commands::{
    startup::{start_orchestrator, continue_orchestrator_startup},
    lsp::{
        lsp_get_completions, lsp_get_definition, lsp_get_diagnostics,
        lsp_get_document_symbols, lsp_get_references,
        lsp_get_signature_help, lsp_hover, lsp_initialize, lsp_is_running, lsp_notify_did_change,
        lsp_notify_did_close, lsp_notify_did_open, lsp_notify_did_save,
        lsp_shutdown, lsp_restart,
    },
    syntax::{
        parse_julia_syntax, get_syntax_diagnostics, clear_syntax_cache, is_syntax_service_available,
    },
    updater::{check_for_updates, download_and_install_update, get_app_version, get_update_info},
    generic::{
        activate_julia_project_process,
        clear_compute42_depot,
        close_terminal_session,
        create_new_julia_project,
        // Julia operations
        execute_julia_code,
        execute_notebook_cell,
        execute_notebook_cells_batch,
        execute_julia_file,
        refresh_workspace_variables,
        get_variable_value,
        get_default_julia_environment_path,
        get_depot_size_info,
        get_julia_version,
        // Project management
        get_julia_project_data,
        instantiate_julia_project,
        // Storage management
        get_julia_storage_paths,
        // LSP management
        // Package management moved
        // File operations moved
        start_file_watcher,
        stop_file_watcher,
    },
    files::{get_file_tree, read_file_content, write_file_content, create_file_item, create_folder_item, delete_item, rename_item, check_path_exists, load_directory_contents},
    notebook::{read_notebook, write_notebook, execute_notebook_file},
    projects::{read_project_toml, write_project_toml, generate_uuid},
    process::{get_session_status, init_terminal_session, is_backend_ready, restart_julia, get_backend_busy_status},
    plot::{
        clear_all_plots, delete_plot, emit_plot_navigator_update, get_all_plots, get_plot,
        serve_plot_image, test_plot_system,
    },
    utils::{get_system_info, open_url, set_last_opened_folder, is_subscription_enabled, is_ai_enabled, get_app_settings, set_app_settings, get_available_fonts},
    file_server::{start_file_server, stop_file_server, get_file_server_url, is_file_server_running},
};
// Terminal manager removed - using single persistent Julia session instead
// PlotManager removed - now using orchestrator's plot server

// --- Main Application Entry Point ---
pub fn run() {
    // Initialize configuration early to ensure .env file is loaded
    internals::config::init_config();
    
    // Set up panic hook to capture crashes
    std::panic::set_hook(Box::new(|panic_info| {
        log::error!("=== PANIC OCCURRED ===");
        log::error!("Panic info: {:?}", panic_info);
        if let Some(location) = panic_info.location() {
            log::error!(
                "Panic location: {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            log::error!("Panic message: {}", s);
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            log::error!("Panic message: {}", s);
        }
        log::error!("=== END PANIC INFO ===");
    }));

    // Configure logging plugin with daily rotation
    let log_plugin_builder = logger::configure_logging_plugin();

    debug!("[Startup] Building Tauri app...");

    tauri::Builder::default()
        // NOTE: The redundant .setup() call has been removed here.
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(log_plugin_builder.build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // All state management removed - now using orchestrator
        .invoke_handler(tauri::generate_handler![
            start_orchestrator,
            continue_orchestrator_startup,
            // Orchestrator commands (actor-based)
            // Orchestrator commands
            crate::commands::generic::frontend_ready_handshake,
            crate::commands::generic::get_orchestrator_startup_phase,
            crate::commands::generic::project_changed,
            // Utility commands (keeping)
            open_url,
            // query_setup_status,
            set_last_opened_folder,
            get_system_info,
            is_subscription_enabled,
            is_ai_enabled,
            get_app_settings,
            set_app_settings,
            get_available_fonts,
            // File operations
            get_file_tree,
            read_file_content,
            write_file_content,
            create_file_item,
            create_folder_item,
            delete_item,
            rename_item,
            check_path_exists,
            load_directory_contents,
            start_file_watcher,
            stop_file_watcher,
            read_project_toml,
            write_project_toml,
            // Notebook operations
            read_notebook,
            write_notebook,
            execute_notebook_file,
            generate_uuid,
            // Julia operations
            execute_julia_code,
            execute_notebook_cell,
        execute_notebook_cells_batch,
            execute_julia_file,
            refresh_workspace_variables,
            get_variable_value,
            get_session_status,
            is_backend_ready,
            get_backend_busy_status,
            activate_julia_project_process,
            close_terminal_session,
            init_terminal_session,
            restart_julia,
            restart_julia_orchestrator,
            // Project management
            get_julia_project_data,
            create_new_julia_project,
            instantiate_julia_project,
            // Package management
            crate::commands::packages::run_julia_pkg_command,
            crate::commands::packages::get_julia_package_status,
            crate::commands::packages::clean_transitive_dependencies,
            get_default_julia_environment_path,
            get_julia_version,
            // Storage management
            get_julia_storage_paths,
            get_depot_size_info,
            clear_compute42_depot,
            // Plot commands
            get_all_plots,
            get_plot,
            delete_plot,
            clear_all_plots,
            test_plot_system,
            emit_plot_navigator_update,
            serve_plot_image,
            // LSP commands
            lsp_hover,
            lsp_notify_did_open,
            lsp_notify_did_close,
            lsp_notify_did_change,
            lsp_notify_did_save,
            lsp_get_completions,
            lsp_get_signature_help,
            lsp_get_definition,
            lsp_get_references,
            lsp_get_document_symbols,
            lsp_get_diagnostics,
            lsp_is_running,
            lsp_initialize,
            lsp_shutdown,
            lsp_restart,
            // Syntax commands
            parse_julia_syntax,
            get_syntax_diagnostics,
            clear_syntax_cache,
            is_syntax_service_available,
            // Tab management commands
            crate::commands::tab::add_tab,
            crate::commands::tab::remove_tab,
            crate::commands::tab::get_tabs,
            crate::commands::tab::update_tab,
            crate::commands::tab::clear_tabs,
            crate::commands::tab::update_tab_content,
            crate::commands::tab::save_tab_to_file,
            crate::commands::tab::get_dirty_tabs,
            // FileServer commands
            start_file_server,
            stop_file_server,
            get_file_server_url,
            is_file_server_running,
            // Updater commands
            check_for_updates,
            download_and_install_update,
            get_app_version,
            get_update_info,
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // Configure Windows title bar to be dark (black) regardless of system theme
            #[cfg(target_os = "windows")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    if let Ok(hwnd) = window.hwnd() {
                        if let Err(e) = crate::windows_titlebar::set_dark_title_bar(hwnd) {
                            debug!("[Windows] Failed to set dark title bar: {}", e);
                        } else {
                            debug!("[Windows] Successfully configured dark title bar");
                        }
                    }
                }
            }

            // Demo folder is now accessed directly from bundled resources
            // No need to copy to AppData - the configuration actor will use the resource path
            debug!("[Lib] Demo folder is available as a bundled resource");

            // Store emitter
            let emitter = Arc::new(crate::tauri_emitter::TauriEventEmitter::new(app_handle.clone()));
            app.manage(emitter.clone());

            // No explicit window listener needed; the LSP bridge emits orchestrator:startup-ready directly
            

            // Initialize the ActorSystem within a dedicated Actix System thread
            let app_handle_clone = app_handle.clone();
            let emitter_clone = emitter.clone();
            
            // Create a channel to signal when AppState is ready
            let (tx, rx) = std::sync::mpsc::channel();
            
            std::thread::spawn(move || {
                let system_runner = actix::System::new();

                // Build ActorSystem using the factory
                let actor_system = system_runner.block_on(async move {
                    internals::services::factory::create_actor_system(emitter_clone.clone()).await
                        .expect("Failed to create ActorSystem")
                });

                // Initialize and register in Tauri state
                if let Err(e) = system_runner.block_on(actor_system.initialize()) {
                    error!("[Lib] Failed to initialize ActorSystem: {}", e);
                }
                // Manage raw ActorSystem for backward-compat commands
                app_handle_clone.manage(actor_system.clone());
                // Also manage unified AppState for new commands
                let app_state = crate::state::AppState::new(actor_system.clone());
                app_handle_clone.manage(app_state);
                debug!("[Lib] ActorSystem initialized and added to state");
                
                // Signal that AppState is ready
                let _ = tx.send(());

                // Keep the Actix system running on this dedicated thread
                if let Err(e) = system_runner.run() {
                    error!("[Lib] Actix system terminated with error: {}", e);
                }
            });
            
            // Wait for AppState to be ready before continuing
            if let Err(e) = rx.recv_timeout(std::time::Duration::from_secs(10)) {
                error!("[Lib] Timeout waiting for ActorSystem initialization: {}", e);
            }

            // Set up update check coordination
            // The update check thread will be spawned in frontend_ready_handshake
            // after the UI is confirmed to be ready
            let updater_service = UpdaterService::new(app_handle.clone());
            app.manage(updater_service.clone());
            
            // Create a channel for update check coordination
            // The receiver will be stored in app state, sender will be passed to frontend_ready_handshake
            let (update_tx, update_rx) = std::sync::mpsc::channel::<()>();
            app.manage(std::sync::Mutex::new(update_rx)); // Store receiver in app state
            app.manage(update_tx); // Store sender in app state for frontend_ready_handshake
            debug!("[Lib] Update check coordination channel created");

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                debug!("Window close requested. Closing Julia session.");
                
                // Try to get actor system from window state, but handle case where it might not be initialized yet
                if let Some(actor_system_state) = window.try_state::<Arc<internals::actor_system::ActorSystem>>() {
                    let _actor_system_clone = actor_system_state.inner().clone();

                    // Run the async shutdown in a blocking context
                    tokio::spawn(async move {
                        // TODO: Implement proper shutdown for ActorSystem
                        debug!("ActorSystem shutdown requested");
                    });
                } else {
                    debug!("ActorSystem not yet initialized, skipping shutdown");
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
