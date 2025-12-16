use log::debug;
use std::path::PathBuf;
use tauri_plugin_log::{Builder as LogBuilder, Target, TargetKind, RotationStrategy};

/// Configure and build the logging plugin for the application
pub fn configure_logging_plugin() -> LogBuilder {
    debug!("[Startup] Configuring logging plugin...");
    
    // Log a startup separator to make it easier to identify app restarts
    log::info!("==========================================");
    log::info!("JuliaJunction App Starting Up");
    log::info!("Timestamp: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    log::info!("==========================================");

    // Create logs directory relative to executable (matching backend pattern)
    // Only in debug mode - in release mode, use Tauri's LogDir which uses app data directory
    let logs_dir = if cfg!(debug_assertions) {
        let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("juliajunction"));
        let logs_dir = if let Some(exe_dir) = exe_path.parent() {
            exe_dir.join("logs/")
        } else {
            PathBuf::from("target/debug/logs")
        };
        println!("Logs directory: {:?}", logs_dir);

        // Ensure logs directory exists (only in debug mode)
        if let Err(e) = std::fs::create_dir_all(&logs_dir) {
            eprintln!(
                "Warning: Could not create logs directory {:?}: {}",
                logs_dir, e
            );
        }
        Some(logs_dir)
    } else {
        // In release mode, LogDir will use the app data directory automatically
        None
    };

    let mut log_plugin_builder = LogBuilder::new()
        .clear_targets()
        .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
        // Suppress verbose third-party library logs and reduce frontend noise
        .filter(|metadata| {
            let target = metadata.target();
            // Suppress DEBUG logs from HTTP libraries
            if target.starts_with("hyper") || 
               target.starts_with("reqwest") || 
               target.starts_with("hyper_util") ||
               target.starts_with("h2") ||
               target.starts_with("tower") {
                metadata.level() <= log::LevelFilter::Warn
            }
            // Suppress LSP logs to reduce noise (can be re-enabled for debugging)
            else if target.starts_with("languageserver::") || 
                    target.starts_with("internals::actors::lsp_actor") ||
                    target.contains("lsp") {
                metadata.level() <= log::LevelFilter::Warn
            } else {
                true
            }
        });

    debug!("[Startup] Logging plugin builder created");

    if cfg!(debug_assertions) {
        // Debug build: print logs to both terminal (Stdout) and daily rotating file
        log_plugin_builder = log_plugin_builder
            .level(log::LevelFilter::Debug)
            .target(Target::new(TargetKind::Stdout))
            .target(Target::new(TargetKind::Dispatch(
                tauri_plugin_log::fern::Dispatch::new()
                    .chain(tauri_plugin_log::fern::DateBased::new(
                        logs_dir.as_ref().expect("logs_dir should be set in debug mode"),
                        "%Y-%m-%d-app.log"
                    ))
            )));
    } else {
        // Release build: write logs to the default app log directory with rotation
        // and a max file size to prevent the active log from growing too large.
        // Temporarily set to DEBUG level to help debug LSP startup issues
        log_plugin_builder = log_plugin_builder
            .level(log::LevelFilter::Debug)
            .max_file_size(1024 * 1024) // 1 MB
            .rotation_strategy(RotationStrategy::KeepSome(5))
            .target(Target::new(TargetKind::LogDir { file_name: None }));
    }

    log_plugin_builder
}
