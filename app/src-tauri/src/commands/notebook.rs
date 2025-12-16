use log::{debug, error};
use serde_json;
use shared::frontend::Notebook;
use std::fs;
use tauri::{command, State};
use uuid::Uuid;

use crate::state::AppState;
use internals::services::base::file_utils::convert_path_for_julia;
use internals::services::events::event_service::NotebookCellEventPayload;

/// Read and parse a Jupyter notebook file
#[command]
pub fn read_notebook(path: String) -> Result<Notebook, String> {
    debug!("[Notebook] Reading notebook from: {}", path);

    // Read file content
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read notebook file: {}", e))?;

    // Parse JSON
    let notebook: Notebook = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse notebook JSON: {}", e))?;

    // Validate nbformat version (support 4.x)
    if notebook.nbformat != 4 {
        return Err(format!(
            "Unsupported nbformat version: {}. Only version 4.x is supported.",
            notebook.nbformat
        ));
    }

    // Cell sources are already normalized by the deserializer

    debug!("[Notebook] Successfully parsed notebook with {} cells", notebook.cells.len());
    Ok(notebook)
}

/// Write a Jupyter notebook to file
#[command]
pub fn write_notebook(path: String, notebook: Notebook) -> Result<(), String> {
    debug!("[Notebook] Writing notebook to: {}", path);

    // Validate notebook structure
    if notebook.nbformat != 4 {
        return Err(format!(
            "Invalid nbformat version: {}. Only version 4.x is supported.",
            notebook.nbformat
        ));
    }

    // Serialize to JSON with pretty printing
    let json = serde_json::to_string_pretty(&notebook)
        .map_err(|e| format!("Failed to serialize notebook: {}", e))?;

    // Write to file
    fs::write(&path, json)
        .map_err(|e| format!("Failed to write notebook file: {}", e))?;

    debug!("[Notebook] Successfully wrote notebook with {} cells", notebook.cells.len());
    Ok(())
}

/// Execute an entire notebook sequentially and emit per-cell output events
#[command]
pub async fn execute_notebook_file(
    path: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    debug!("[Notebook] Executing notebook file: {}", path);
    
    // Emit backend-busy event at the start of notebook execution
    // This keeps the button disabled throughout the entire execution
    let notebook_request_id = Uuid::new_v4().to_string();
    if let Err(e) = app_state
        .actor_system
        .event_manager
        .emit_backend_busy(&notebook_request_id)
        .await
    {
        error!("[Notebook] Failed to emit backend-busy event: {}", e);
    } else {
        debug!("[Notebook] Emitted backend-busy event with request_id: {}", notebook_request_id);
    }

    // Read notebook from disk
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read notebook file: {}", e))?;
    let notebook: Notebook = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse notebook JSON: {}", e))?;

    // Determine working directory (notebook folder) for includes
    let dir = std::path::Path::new(&path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    let julia_dir = convert_path_for_julia(&dir);

    // Execute code cells sequentially
    for (idx, cell) in notebook.cells.iter().enumerate() {
        if cell.cell_type != shared::frontend::CellType::Code {
            continue;
        }

        // Use index-based cell_id (frontend will match by cell_index to get actual cell IDs)
        let cell_id = format!("cell-{}", idx);
        let code = cell.source.trim();
        
        // Skip empty cells
        if code.is_empty() {
            continue;
        }

        // Separate 'using' and 'import' statements from other code
        // These must be executed at top level, not inside redirect blocks
        let mut using_lines = Vec::new();
        let mut other_lines = Vec::new();
        
        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("using ") || trimmed.starts_with("import ") {
                using_lines.push(line);
            } else {
                other_lines.push(line);
            }
        }
        
        let using_code = using_lines.join("\n");
        let other_code = other_lines.join("\n");
        
        // Build code: execute 'using' statements at top level, then execute rest
        // Just like file execution - simple and clean
        // Note: We execute at top level (Main) to ensure closures work correctly with Zygote
        let wrapped_code = if !other_code.trim().is_empty() {
            format!(
                r#"
{}
cd("{}")
{}
"#,
                using_code, julia_dir, other_code
            )
        } else {
            // Only 'using' statements, no other code
            format!(
                r#"
{}
cd("{}")
"#,
                using_code, julia_dir
            )
        };

        // Set current notebook cell to start buffering output
        use internals::messages::process::{SetNotebookCell, GetNotebookCellOutput};
        app_state
            .actor_system
            .process_actor
            .send(SetNotebookCell {
                cell_id: Some(cell_id.clone()),
            })
            .await
            .map_err(|e| format!("Failed to set notebook cell: {}", e))?
            .map_err(|e| format!("Failed to set notebook cell: {}", e))?;

        // Execute cell sequentially using NotebookCell execution type
        // This ensures prompts are suppressed and execution type is correct
        use internals::messages::execution::ExecuteNotebookCell;
        let _result = app_state
            .actor_system
            .execution_actor
            .send(ExecuteNotebookCell {
                cell_id: cell_id.clone(),
                code: wrapped_code,
                notebook_path: Some(path.clone()),
            })
            .await
            .map_err(|e| format!("Actor comm failed: {}", e))?
            .map_err(|e| format!("Code execution failed: {}", e))?;

        // Get buffered output and clear the buffer
        let buffered_output = app_state
            .actor_system
            .process_actor
            .send(GetNotebookCellOutput)
            .await
            .map_err(|e| format!("Failed to get notebook cell output: {}", e))?
            .map_err(|e| format!("Failed to get notebook cell output: {}", e))?;

        // Build Jupyter-like outputs from buffered output
        let mut outputs: Vec<serde_json::Value> = Vec::new();
        if let Some(buffer) = buffered_output {
            // Join lines with newlines to preserve formatting
            let stdout_text = buffer.stdout.join("\n");
            let stderr_text = buffer.stderr.join("\n");
            
            debug!("[Notebook] Cell {} output - stdout: {} bytes, stderr: {} bytes, plots: {}", 
                cell_id, stdout_text.len(), stderr_text.len(), buffer.plots.len());
            
            if !stdout_text.trim().is_empty() {
                outputs.push(serde_json::json!({
                    "output_type": "stream",
                    "name": "stdout",
                    "text": stdout_text + "\n", // Add final newline for proper formatting
                }));
            }
            if !stderr_text.trim().is_empty() {
                outputs.push(serde_json::json!({
                    "output_type": "stream",
                    "name": "stderr",
                    "text": stderr_text + "\n", // Add final newline for proper formatting
                }));
            }
            
            // Add plot outputs
            for (mime_type, data) in buffer.plots {
                // Clean the data - extract raw SVG/base64, removing any HTML markup
                let cleaned_data = if mime_type == "image/svg+xml" {
                    // For SVG, extract raw SVG XML from HTML if present
                    if data.trim().starts_with("<svg") {
                        // Already raw SVG, use as-is
                        data
                    } else if data.contains("<svg") {
                        // Extract SVG from HTML (e.g., from <img> tag or other HTML)
                        // Try to find the SVG content
                        if let Some(svg_start) = data.find("<svg") {
                            if let Some(svg_end) = data.rfind("</svg>") {
                                let svg_content = &data[svg_start..svg_end + 6];
                                svg_content.to_string()
                            } else {
                                data // Fallback to original
                            }
                        } else {
                            data // Fallback to original
                        }
                    } else {
                        // Base64-encoded SVG, use as-is
                        data
                    }
                } else {
                    // For binary images, ensure we have clean base64 (no data: prefix or HTML)
                    if data.starts_with("data:") {
                        // Extract base64 part after comma
                        if let Some(comma_pos) = data.find(',') {
                            data[comma_pos + 1..].to_string()
                        } else {
                            data
                        }
                    } else if data.contains("<img") || data.contains("base64,") {
                        // Extract base64 from HTML img tag
                        if let Some(base64_start) = data.find("base64,") {
                            let base64_part = &data[base64_start + 7..];
                            // Remove any trailing HTML/attributes
                            if let Some(end_pos) = base64_part.find('"') {
                                base64_part[..end_pos].to_string()
                            } else if let Some(end_pos) = base64_part.find(' ') {
                                base64_part[..end_pos].to_string()
                            } else if let Some(end_pos) = base64_part.find('>') {
                                base64_part[..end_pos].to_string()
                            } else {
                                base64_part.to_string()
                            }
                        } else {
                            data
                        }
                    } else {
                        data
                    }
                };
                
                let mut output_data = serde_json::Map::new();
                output_data.insert(mime_type.clone(), serde_json::Value::String(cleaned_data));
                
                outputs.push(serde_json::json!({
                    "output_type": "display_data",
                    "data": output_data,
                    "metadata": {},
                }));
            }
        } else {
            debug!("[Notebook] Cell {} - no buffered output found", cell_id);
        }

        // Emit per-cell output event with buffered output
        app_state
            .actor_system
            .event_manager
            .emit_notebook_event(
                "julia:notebook-cell-output",
                NotebookCellEventPayload {
                    cell_id: cell_id.clone(),
                    cell_index: idx,
                    outputs: serde_json::json!(outputs),
                },
            )
            .await
            .map_err(|e| format!("Failed to emit notebook cell output event: {}", e))?;
    }

    // Emit notebook-done event
    app_state
        .actor_system
        .event_manager
        .emit_notebook_event(
            "julia:notebook-complete",
            NotebookCellEventPayload {
                cell_id: "done".to_string(),
                cell_index: usize::MAX,
                outputs: serde_json::json!([]),
            },
        )
        .await
        .map_err(|e| format!("Failed to emit notebook complete event: {}", e))?;
    
    debug!("[Notebook] Emitted notebook-complete event");

    // Emit backend-done event at the end of notebook execution
    // This re-enables the button after all cells complete
    if let Err(e) = app_state
        .actor_system
        .event_manager
        .emit_backend_done(&notebook_request_id)
        .await
    {
        error!("[Notebook] Failed to emit backend-done event: {}", e);
    } else {
        debug!("[Notebook] Emitted backend-done event with request_id: {}", notebook_request_id);
    }

    Ok(())
}
