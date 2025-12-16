// HTTP route handlers for the plot server
// This module contains the HTTP endpoint handlers

use crate::messages::plot::PlotData;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use base64::Engine;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Get all plots
pub async fn get_all_plots(
    State(plots): State<Arc<Mutex<HashMap<String, PlotData>>>>,
) -> Result<Json<Vec<PlotData>>, StatusCode> {
    let plots_guard = plots.lock().await;
    let plots_vec: Vec<PlotData> = plots_guard.values().cloned().collect();
    Ok(Json(plots_vec))
}

/// Get a specific plot by ID
pub async fn get_plot(
    Path(plot_id): Path<String>,
    State(plots): State<Arc<Mutex<HashMap<String, PlotData>>>>,
) -> Result<Json<PlotData>, StatusCode> {
    let plots_guard = plots.lock().await;
    if let Some(plot) = plots_guard.get(&plot_id) {
        Ok(Json(plot.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Delete a plot by ID
pub async fn delete_plot(
    Path(plot_id): Path<String>,
    State(plots): State<Arc<Mutex<HashMap<String, PlotData>>>>,
) -> StatusCode {
    let mut plots_guard = plots.lock().await;
    if plots_guard.remove(&plot_id).is_some() {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Get plot image endpoint
pub async fn get_plot_image(
    Path(plot_id): Path<String>,
    State(plots): State<Arc<Mutex<HashMap<String, PlotData>>>>,
) -> Result<axum::response::Response<axum::body::Body>, StatusCode> {
    let plots_guard = plots.lock().await;
    if let Some(plot) = plots_guard.get(&plot_id) {
        if plot.mime_type.starts_with("image/") {
            // For SVG data, return the SVG content with proper content type
            if plot.mime_type == "image/svg+xml" {
                let response = axum::response::Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "image/svg+xml")
                    .body(axum::body::Body::from(plot.data.clone()))
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                Ok(response)
            } else {
                // For other image types, try to decode base64
                match base64::engine::general_purpose::STANDARD.decode(&plot.data) {
                    Ok(image_data) => {
                        let response = axum::response::Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", plot.mime_type.clone())
                            .body(axum::body::Body::from(image_data))
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                        Ok(response)
                    }
                    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                }
            }
        } else {
            Err(StatusCode::BAD_REQUEST)
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

