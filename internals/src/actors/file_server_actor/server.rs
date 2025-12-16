use log::{debug, error};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use axum::{
    routing::get,
    Router,
};
use tower_http::cors::{CorsLayer, Any};

use super::handlers;

#[derive(Clone, Debug, Default)]
pub struct FileServerState {
    pub base_path: Option<String>,
    pub server_port: Option<u16>,
    pub is_running: bool,
}

/// Find an available port by actually binding to it (atomic operation)
/// Returns the listener to prevent race conditions
pub async fn find_available_port() -> Result<(u16, TcpListener), String> {
    // Try ports in a reasonable range (8080-9000)
    for port in 8080..9000 {
        if let Ok(listener) = TcpListener::bind(format!("127.0.0.1:{}", port)).await {
            return Ok((port, listener));
        }
    }

    Err("No available ports found in range 8080-9000".to_string())
}

pub async fn start_server_internal(
    state: Arc<Mutex<FileServerState>>,
    base_path: String,
) -> Result<u16, String> {
    let mut state_guard = state.lock().await;

    if state_guard.is_running {
        debug!("File server is already running");
        return Ok(state_guard.server_port.unwrap_or(0));
    }

    // Find an available port by actually binding to it (atomic operation)
    let (port, listener) = find_available_port().await?;
    debug!(
        "Starting file server on port {} for path: {}",
        port, base_path
    );

    // Update state first
    state_guard.base_path = Some(base_path.clone());
    state_guard.server_port = Some(port);
    state_guard.is_running = true;

    // Drop the lock before spawning the task
    drop(state_guard);

    // Start server in a separate task with the pre-bound listener
    let state_clone = state.clone();

    tokio::spawn(async move {
        let server_state = state_clone.clone();

        // Configure CORS
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        debug!("File server: CORS configured to allow all origins");

        // Create the router
        let app = Router::new()
            .route("/", get(handlers::index_handler))
            .route("/files/*path", get(handlers::serve_file_handler))
            .route("/csv/*path", get(handlers::parse_csv_handler))
            .route("/health", get(handlers::health_check_handler))
            .layer(cors)
            .with_state(server_state.clone());

        debug!("File server started successfully on port {}", port);
        debug!("File server: Available endpoints:");
        debug!("File server:   - GET / (index)");
        debug!("File server:   - GET /files/*path (serve files)");
        debug!("File server:   - GET /csv/*path (parse CSV)");
        debug!("File server: Base path: {}", base_path);

        // Start serving with proper error handling
        if let Err(e) = axum::serve(listener, app).await {
            error!("File server error: {}", e);
            let mut state = server_state.lock().await;
            state.is_running = false;
            state.server_port = None;
        }
    });

    Ok(port)
}

pub async fn stop_server_internal(
    state: Arc<Mutex<FileServerState>>,
) -> Result<(), String> {
    let mut state_guard = state.lock().await;

    if !state_guard.is_running {
        debug!("File server is not running");
        return Ok(());
    }

    debug!("Stopping file server");
    state_guard.is_running = false;
    state_guard.base_path = None;
    state_guard.server_port = None;

    Ok(())
}

