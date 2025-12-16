// HTTP server management for plot serving
// This module handles starting, stopping, and managing the plot HTTP server

use crate::actors::plot_actor::handlers;
use crate::messages::plot::PlotData;
use crate::service_traits::EventEmitter;
use axum::{
    routing::{delete, get},
    Router,
};
use log::{debug, error};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower_http::cors::{CorsLayer, Any};

/// Plot server state
#[derive(Clone)]
pub struct PlotServer {
    plot_port: Arc<Mutex<Option<u16>>>,
    server_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    event_emitter: Arc<dyn EventEmitter>,
}

impl PlotServer {
    /// Create a new PlotServer instance
    pub fn new(event_emitter: Arc<dyn EventEmitter>) -> Self {
        Self {
            plot_port: Arc::new(Mutex::new(None)),
            server_handle: Arc::new(Mutex::new(None)),
            event_emitter,
        }
    }

    /// Start the HTTP server with a pre-bound listener (avoids race conditions)
    pub async fn start_with_listener(
        &self,
        _port: u16,
        listener: TcpListener,
        plots: Arc<Mutex<HashMap<String, PlotData>>>,
    ) -> Result<(), String> {
        let event_emitter = self.event_emitter.clone();
        let plot_port = self.plot_port.clone();
        let server_handle_mutex = self.server_handle.clone();

        // Configure CORS
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        debug!("[PlotActor] CORS configured to allow all origins");

        // Create the router
        let app = Router::new()
            .route("/plots", get(handlers::get_all_plots))
            .route("/plots/:plot_id", get(handlers::get_plot))
            .route("/plots/:plot_id", delete(handlers::delete_plot))
            .route("/plots/:plot_id/image", get(handlers::get_plot_image))
            .route("/health", get(handlers::health_check))
            .layer(cors)
            .with_state(plots);

        // Start the server with proper error handling and crash detection
        let server_handle = tokio::spawn(async move {
            let result = axum::serve(listener, app).await;
            if let Err(e) = result {
                error!("[PlotActor] Server error: {}", e);
            }
            
            // Server has stopped (either error or graceful shutdown)
            // Clear the port and attempt automatic restart if it was unexpected
            let mut port_guard = plot_port.lock().await;
            if port_guard.is_some() {
                debug!("[PlotActor] Server stopped unexpectedly, clearing port");
                let crashed_port = *port_guard;
                *port_guard = None;
                drop(port_guard);
                
                // Emit crash event (actor can handle restart)
                let crash_event = serde_json::json!({
                    "port": crashed_port,
                    "error": "Server stopped unexpectedly"
                });
                if let Err(e) = event_emitter.emit("plot:server-crashed", crash_event).await {
                    error!("[PlotActor] Failed to emit server crash event: {}", e);
                }
                
                // Note: Automatic restart should be handled by the actor or orchestrator
                // that listens for the server-crashed event and sends StartPlotServer message
            }
            
            // Remove the server handle from the mutex since it's done
            let mut handle_guard = server_handle_mutex.lock().await;
            *handle_guard = None;
        });

        // Store the server handle
        let mut handle_guard = self.server_handle.lock().await;
        *handle_guard = Some(server_handle);

        Ok(())
    }

    /// Stop the HTTP server
    pub async fn stop(&self) -> Result<(), String> {
        // Clear the port first to indicate graceful shutdown
        let mut port_guard = self.plot_port.lock().await;
        *port_guard = None;
        drop(port_guard);
        
        // Now abort the server handle
        let mut handle_guard = self.server_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
        Ok(())
    }

    /// Find an available port by actually binding to it (atomic operation)
    /// Returns the listener to prevent race conditions
    pub async fn find_available_port(&self) -> Result<(u16, TcpListener), String> {
        for port in 8080..9000 {
            if let Ok(listener) = TcpListener::bind(format!("127.0.0.1:{}", port)).await {
                return Ok((port, listener));
            }
        }
        Err("No available ports found in range 8080-9000".to_string())
    }

    /// Get the current plot port
    pub async fn get_port(&self) -> Option<u16> {
        let port_guard = self.plot_port.lock().await;
        *port_guard
    }

    /// Set the plot port
    pub async fn set_port(&self, port: u16) {
        let mut port_guard = self.plot_port.lock().await;
        *port_guard = Some(port);
    }

    /// Clear the plot port
    pub async fn clear_port(&self) {
        let mut port_guard = self.plot_port.lock().await;
        *port_guard = None;
    }
}

