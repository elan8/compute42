// I/O operations for CommunicationActor
// Handles sending and receiving messages via named pipes

use crate::services::events::EventService;
use actix::prelude::*;
use log::{debug, error};
use serde_json;
use std::io::{BufRead, Write};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use super::state::{State, LocalSocketStream};
use super::message_handler;

/// Start the message sender task (should be called before connection)
pub async fn start_message_sender_task(
    state: &State,
    mut rx: mpsc::Receiver<crate::messages::JuliaMessage>,
) {
    let code_stream = state.code_stream.clone();
    let event_manager = state.event_manager.clone();
    
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            // Send the message
            if let Err(e) = send_message_to_julia(&code_stream, message).await {
                error!(
                    "[CommunicationActor::IoOperations] Failed to send message to Julia: {}",
                    e
                );
                
                // Check if this is a broken pipe error
                if e.contains("Pipe connection broken") {
                    let elapsed = crate::app_time::get_app_start_time().elapsed();
                    error!(
                        "[CommunicationActor::IoOperations] Pipe connection broken after {:.2}s since app start while sending message to Julia",
                        elapsed.as_secs_f64()
                    );
                    let error_msg = "The connection to Julia has been lost. Please restart Compute42 to reconnect.";
                    if let Err(emit_err) = event_manager.emit_system_error(error_msg).await {
                        error!("[CommunicationActor::IoOperations] Failed to emit system error: {}", emit_err);
                    }
                    // Break out of the loop - connection is broken
                    break;
                }
                
                continue;
            }
        }
        debug!("[CommunicationActor::IoOperations] Message sender task ended");
    });
}

/// Send a message to Julia via the code pipe
async fn send_message_to_julia(
    code_stream: &Arc<Mutex<Option<LocalSocketStream>>>,
    message: crate::messages::JuliaMessage,
) -> Result<(), String> {
    let message_json = serde_json::to_string(&message)
        .map_err(|e| format!("Failed to serialize message: {}", e))?;

    // Use blocking I/O for writing to avoid concurrent access issues
    let write_result = tokio::task::spawn_blocking({
        let code_stream = code_stream.clone();
        let message_with_newline = format!("{}\n", message_json);
        move || {
            // Get the stream in the blocking context
            let mut code_stream_guard = code_stream.blocking_lock();
            if let Some(stream) = code_stream_guard.as_mut() {
                let write_result = stream.write_all(message_with_newline.as_bytes());

                if let Err(e) = write_result {
                    // Check for broken pipe errors
                    let is_broken_pipe = matches!(
                        e.kind(),
                        std::io::ErrorKind::BrokenPipe
                            | std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::ConnectionAborted
                    );
                    
                    if is_broken_pipe {
                        return Err(format!("Pipe connection broken: {}", e));
                    } else {
                        return Err(format!("Failed to write to Julia pipe: {}", e));
                    }
                }

                let flush_result = stream.flush();
                flush_result.map_err(|e| format!("Failed to flush Julia pipe: {}", e))
            } else {
                Err("No code stream to Julia available".to_string())
            }
        }
    })
    .await;

    match write_result {
        Ok(Ok(())) => {
            Ok(())
        }
        Ok(Err(e)) => {
            error!(
                "[CommunicationActor::IoOperations] Failed to send message to Julia: {}",
                e
            );
            Err(e)
        }
        Err(e) => {
            error!("[CommunicationActor::IoOperations] Blocking write task failed: {}", e);
            Err(format!("Blocking write task failed: {}", e))
        }
    }
}

/// Read a single response from Julia via the code pipe
#[allow(dead_code)]
#[allow(clippy::type_complexity)]
pub async fn read_julia_response(
    code_stream: &Arc<Mutex<Option<LocalSocketStream>>>,
    event_manager: &EventService,
    current_request: &Arc<Mutex<Option<(String, tokio::sync::oneshot::Sender<crate::messages::JuliaMessage>)>>>,
    plot_actor: Option<Addr<crate::actors::PlotActor>>,
    state: &super::state::State,
) -> Result<(), String> {
    // Check if we have a code stream available
    let has_stream = {
        let code_stream_guard = code_stream.lock().await;
        code_stream_guard.is_some()
    };

    if !has_stream {
        error!("[CommunicationActor::IoOperations] No code stream available for reading");
        return Err("No code stream available".to_string());
    }

    // Use blocking I/O for reading
    let read_result = tokio::task::spawn_blocking({
        let code_stream = code_stream.clone();
        move || {
            // Get the stream in the blocking context
            let mut code_stream_guard = code_stream.blocking_lock();
            if let Some(stream) = code_stream_guard.as_mut() {
                let mut buffer = String::new();
                let mut reader = std::io::BufReader::new(stream);

                let read_result = reader.read_line(&mut buffer);
                read_result.map(|bytes_read| (bytes_read, buffer))
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "No code stream available",
                ))
            }
        }
    })
    .await;

    match read_result {
        Ok(Ok((bytes_read, buffer))) => {
            if bytes_read == 0 {
                return Err("No data received from Julia".to_string());
            }

            if !buffer.trim().is_empty() {
                // Parse and handle the message
                // Julia sends messages in the same enum format that Rust expects:
                // {"ExecutionComplete": {"id": "...", "execution_type": "...", ...}}
                // With custom ExecutionType serialization, this should parse directly
                match serde_json::from_str::<crate::messages::JuliaMessage>(buffer.trim()) {
                    Ok(message) => {
                        // Direct parse succeeded - message format matches Rust's enum structure
                        let process_actor = {
                            let process_actor_guard = state.process_actor.lock().await;
                            process_actor_guard.clone()
                        };
                        let handler = message_handler::MessageHandler::new(
                            event_manager.clone(),
                            plot_actor.clone(),
                            process_actor,
                        );
                        
                        if let Err(e) = handler.handle_julia_message(&message, current_request).await {
                            error!("[CommunicationActor::IoOperations] Error handling message: {}", e);
                        }
                    }
                    Err(parse_err) => {
                        // Fallback: Try to parse as nested structure (should rarely be needed now)
                        // Log the parse error for debugging
                        debug!(
                            "[CommunicationActor::IoOperations] Direct parse failed, trying fallback: {}",
                            parse_err
                        );
                        
                        let process_actor = {
                            let process_actor_guard = state.process_actor.lock().await;
                            process_actor_guard.clone()
                        };
                        let handler = message_handler::MessageHandler::new(
                            event_manager.clone(),
                            plot_actor.clone(),
                            process_actor,
                        );
                        
                        match handler.parse_nested_message(buffer.trim()) {
                            Ok(Some(message)) => {
                                debug!("[CommunicationActor::IoOperations] Fallback parse succeeded");
                                if let Err(e) = handler.handle_julia_message(&message, current_request).await {
                                    error!("[CommunicationActor::IoOperations] Error handling nested message: {}", e);
                                }
                            }
                            Ok(None) => {
                                error!(
                                    "[CommunicationActor::IoOperations] Failed to parse message from Julia (both direct and fallback failed): {}",
                                    buffer.trim()
                                );
                            }
                            Err(e) => {
                                error!("[CommunicationActor::IoOperations] Failed to parse nested message: {}", e);
                            }
                        }
                    }
                }
            } else {
                debug!("[CommunicationActor::IoOperations] Received empty buffer from Julia");
            }
            Ok(())
        }
        Ok(Err(e)) => {
            // Check for broken pipe errors
            let is_broken_pipe = matches!(
                e.kind(),
                std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
            );
            
            if is_broken_pipe {
                // Pipe is broken - error will be handled by caller
                let elapsed = crate::app_time::get_app_start_time().elapsed();
                error!(
                    "[CommunicationActor::IoOperations] Pipe connection broken after {:.2}s since app start while reading from Julia: {}",
                    elapsed.as_secs_f64(),
                    e
                );
                // Note: event_manager is not available here, but the error will be caught by the caller
                Err(format!("Pipe connection broken: {}", e))
            } else if e.kind() == std::io::ErrorKind::UnexpectedEof {
                // EOF - connection closed
                debug!("[CommunicationActor::IoOperations] Connection closed in read task");
                debug!("[CommunicationActor::IoOperations] EOF detected - connection closed by Julia");
                Err("Connection closed by Julia".to_string())
            } else {
                error!(
                    "[CommunicationActor::IoOperations] Error reading from Julia connection: {}",
                    e
                );
                Err(format!("Error reading from Julia connection: {}", e))
            }
        }
        Err(e) => {
            error!("[CommunicationActor::IoOperations] Blocking read task failed: {}", e);
            Err(format!("Blocking read task failed: {}", e))
        }
    }
}

