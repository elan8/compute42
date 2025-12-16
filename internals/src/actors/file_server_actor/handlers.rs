use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
};
use log::{debug, error};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::server::FileServerState;
use super::csv;

pub async fn index_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/html")],
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Compute42 File Server</title>
            <style>
                body { font-family: Arial, sans-serif; margin: 40px; }
                .container { max-width: 600px; margin: 0 auto; }
                h1 { color: #333; }
                .info { background: #f0f0f0; padding: 20px; border-radius: 5px; }
            </style>
        </head>
        <body>
            <div class="container">
                <h1>Compute42 File Server</h1>
                <div class="info">
                    <p>This server provides access to project files for the Compute42 IDE.</p>
                    <p>Use the following URL pattern to access files:</p>
                    <code>http://127.0.0.1:[port]/files/[relative-path]</code>
                </div>
            </div>
        </body>
        </html>
        "#,
    )
}

pub async fn health_check_handler() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

pub async fn serve_file_handler(
    State(state): State<Arc<Mutex<FileServerState>>>,
    AxumPath(requested_path): AxumPath<String>,
) -> impl IntoResponse {
    let state_guard = state.lock().await;

    let base_path = match &state_guard.base_path {
        Some(path) => path,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "File server not properly configured",
            )
                .into_response();
        }
    };

    let full_path = Path::new(base_path).join(&requested_path);

    // Security check: ensure the requested path is within the base directory
    let base_path_buf = PathBuf::from(base_path);
    if !full_path.starts_with(&base_path_buf) {
        debug!("Attempted directory traversal attack: {}", requested_path);
        return (StatusCode::FORBIDDEN, "Access denied").into_response();
    }

    // Check if file exists
    if !full_path.exists() {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }

    if !full_path.is_file() {
        return (StatusCode::BAD_REQUEST, "Not a file").into_response();
    }

    // Serve the file using tower-http
    match tokio::fs::read(&full_path).await {
        Ok(content) => {
            debug!("Serving file: {}", requested_path);

            // Determine content type based on file extension
            let content_type = mime_guess::from_path(&full_path)
                .first_or_octet_stream()
                .to_string();

            (StatusCode::OK, [("content-type", content_type)], content).into_response()
        }
        Err(e) => {
            error!("Failed to serve file {}: {}", requested_path, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to serve file").into_response()
        }
    }
}

pub async fn parse_csv_handler(
    State(state): State<Arc<Mutex<FileServerState>>>,
    AxumPath(requested_path): AxumPath<String>,
) -> impl IntoResponse {
    debug!("File server: Received CSV parsing request for path: {}", requested_path);
    
    let state_guard = state.lock().await;

    let base_path = match &state_guard.base_path {
        Some(path) => {
            debug!("File server: Base path: {}", path);
            path
        },
        None => {
            error!("File server: No base path configured");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "File server not properly configured",
            )
                .into_response();
        }
    };

    let full_path = Path::new(base_path).join(&requested_path);
    debug!("File server: Full path: {}", full_path.display());

    // Security check: ensure the requested path is within the base directory
    let base_path_buf = PathBuf::from(base_path);
    if !full_path.starts_with(&base_path_buf) {
        debug!("Attempted directory traversal attack: {}", requested_path);
        return (StatusCode::FORBIDDEN, "Access denied").into_response();
    }

    // Check if file exists
    if !full_path.exists() {
        error!("File server: File not found: {}", full_path.display());
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }

    if !full_path.is_file() {
        error!("File server: Not a file: {}", full_path.display());
        return (StatusCode::BAD_REQUEST, "Not a file").into_response();
    }

    // Check if it's a CSV file
    if let Some(extension) = full_path.extension() {
        if extension.to_string_lossy().to_lowercase() != "csv" {
            error!("File server: Not a CSV file: {}", full_path.display());
            return (StatusCode::BAD_REQUEST, "Not a CSV file").into_response();
        }
    } else {
        error!("File server: No file extension: {}", full_path.display());
        return (StatusCode::BAD_REQUEST, "Not a CSV file").into_response();
    }

    // Read and parse the CSV file
    match tokio::fs::read(&full_path).await {
        Ok(content) => {
            debug!("Parsing CSV file: {}", requested_path);
            debug!("File server: File size: {} bytes", content.len());

            // Convert bytes to string, handling encoding
            let (text, _, _) = encoding_rs::UTF_8.decode(&content);
            let csv_text = text.into_owned();

            // Parse CSV
            match csv::parse_csv_content(&csv_text) {
                Ok(parsed_data) => {
                    debug!("File server: Successfully parsed CSV with {} rows", parsed_data["total_rows"]);
                    let response = serde_json::json!({
                        "success": true,
                        "data": parsed_data,
                        "file_path": requested_path
                    });
                    
                    (
                        StatusCode::OK,
                        [("content-type", "application/json")],
                        serde_json::to_string(&response).unwrap()
                    )
                        .into_response()
                }
                Err(e) => {
                    error!("Failed to parse CSV file {}: {}", requested_path, e);
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": e.to_string(),
                        "file_path": requested_path
                    });
                    
                    (
                        StatusCode::BAD_REQUEST,
                        [("content-type", "application/json")],
                        serde_json::to_string(&error_response).unwrap()
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to read CSV file {}: {}", requested_path, e);
            let error_response = serde_json::json!({
                "success": false,
                "error": format!("Failed to read file: {}", e),
                "file_path": requested_path
            });
            
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "application/json")],
                serde_json::to_string(&error_response).unwrap()
            )
                .into_response()
        }
    }
}

