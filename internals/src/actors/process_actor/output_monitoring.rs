use log::error;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;
use tokio::process::ChildStderr;
use tokio::sync::Mutex;
use crate::service_traits::EventEmitter;

use super::state::ProcessState;
use super::session::PersistentJuliaSession;

/// Check if a message should be filtered out from terminal display
/// This filters out internal synchronization messages that are needed for the system
/// but shouldn't be shown to users
pub fn should_filter_pipe_ready_message(line: &str) -> bool {
    // Filter out pipe ready messages that are used for internal synchronization
    line.contains("Compute42: TO_JULIA_PIPE_READY") ||
    line.contains("Compute42: FROM_JULIA_PIPE_READY") ||
    line.contains("Compute42: ALL_PIPES_READY") ||
    line.contains("Compute42: MESSAGE_LOOP_READY") ||
    line.contains("Compute42: PROJECT_ACTIVATION_COMPLETE")
}

/// Start monitoring Julia's stdout
pub fn start_stdout_monitoring(
    stdout: ChildStdout,
    event_emitter: Arc<dyn EventEmitter>,
    output_suppressed: Arc<tokio::sync::Mutex<bool>>,
    notebook_output_buffer: Arc<tokio::sync::Mutex<Option<super::state::NotebookCellOutputBuffer>>>,
    current_notebook_cell: Arc<tokio::sync::Mutex<Option<String>>>,
) {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let line_clone = line.clone();
            
            // Check if a notebook cell is currently executing
            let is_notebook_cell_executing = {
                let cell_guard = current_notebook_cell.lock().await;
                cell_guard.is_some()
            };
            
            // Buffer output if a notebook cell is executing
            if is_notebook_cell_executing {
                let mut buffer_guard = notebook_output_buffer.lock().await;
                if let Some(ref mut buffer) = *buffer_guard {
                    buffer.stdout.push(line.clone());
                }
                // Don't emit to terminal when notebook cell is executing - output will be sent via notebook events
                continue;
            }
            
            // Emit detailed output event for all messages (even if filtered for main terminal)
            let _ = event_emitter.emit(
                "julia:output-detailed",
                serde_json::to_value(vec![crate::messages::StreamOutput {
                    content: line.clone() + "\n",
                    stream_type: crate::messages::StreamType::Stdout,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                }]).unwrap_or_default(),
            ).await;
            
            // Only emit output if not suppressed (for main terminal)
            let should_emit = {
                let suppressed = output_suppressed.lock().await;
                !*suppressed
            };
            if should_emit {
                if let Err(e) = event_emitter.emit(
                    "julia:output",
                    serde_json::to_value(vec![crate::messages::StreamOutput {
                        content: line_clone + "\n",
                        stream_type: crate::messages::StreamType::Stdout,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                    }]).unwrap_or_default(),
                ).await {
                    error!("ProcessActor: Failed to emit julia-output event: {}", e);
                }
            }
        }
    });
}

/// Start monitoring Julia's stderr
pub fn start_stderr_monitoring(
    stderr: ChildStderr,
    event_emitter: Arc<dyn EventEmitter>,
    state: Arc<ProcessState>,
    julia_session: Arc<Mutex<Option<PersistentJuliaSession>>>,
) {
    let julia_message_loop_ready_sender = state.julia_message_loop_ready_sender.clone();
    let project_activation_complete_sender = state.project_activation_complete_sender.clone();
    let output_suppressed = state.output_suppressed.clone();
    let communication_actor_state = state.communication_actor.clone();
    let orchestrator_actor_state = state.orchestrator_actor.clone();
    let message_loop_ready_received = state.message_loop_ready_received.clone();
    
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            // Filter out pipe ready messages from terminal display
            // These messages are needed for internal synchronization but shouldn't be shown to users
            if should_filter_pipe_ready_message(&line) {
                // Connect to pipes individually as they become ready
                // Julia's initialize_permanent_communication blocks on accept(), so we must
                // connect to to_julia pipe first to unblock it, then from_julia pipe can initialize
                if line.contains("Compute42: TO_JULIA_PIPE_READY") {
                    // Get to_julia pipe name from session
                    let to_julia_pipe_name = {
                        let session_guard = julia_session.lock().await;
                        if let Some(session) = session_guard.as_ref() {
                            session.to_julia_pipe_name.clone()
                        } else {
                            None
                        }
                    };
                    
                    if let Some(to_julia_pipe) = to_julia_pipe_name {
                        let comm_actor_guard = communication_actor_state.lock().await;
                        if let Some(comm_actor) = comm_actor_guard.as_ref() {
                            let comm_actor_clone = comm_actor.clone();
                            drop(comm_actor_guard);
                            
                            match comm_actor_clone.send(crate::messages::communication::ConnectToJuliaPipe {
                                to_julia_pipe: to_julia_pipe.clone(),
                            }).await {
                                Ok(Ok(())) => {
                                    // Successfully connected
                                }
                                Ok(Err(e)) => {
                                    error!("ProcessActor: CommunicationActor failed to connect to Julia pipe (to_julia): {}", e);
                                    // Emit startup failed if we have orchestrator
                                    let orchestrator_guard = orchestrator_actor_state.lock().await;
                                    if let Some(orchestrator) = orchestrator_guard.as_ref() {
                                        orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                            event: crate::messages::orchestrator::StartupEvent::StartupFailed(
                                                format!("Failed to connect to Julia pipe (to_julia): {}", e)
                                            ),
                                        });
                                    }
                                }
                                Err(e) => {
                                    error!("ProcessActor: Failed to send ConnectToJuliaPipe to CommunicationActor: {:?}", e);
                                    // Emit startup failed if we have orchestrator
                                    let orchestrator_guard = orchestrator_actor_state.lock().await;
                                    if let Some(orchestrator) = orchestrator_guard.as_ref() {
                                        orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                            event: crate::messages::orchestrator::StartupEvent::StartupFailed(
                                                format!("Failed to send ConnectToJuliaPipe message: {:?}", e)
                                            ),
                                        });
                                    }
                                }
                            }
                        } else {
                            error!("ProcessActor: CommunicationActor not available - cannot connect to Julia pipe (to_julia)");
                            let orchestrator_guard = orchestrator_actor_state.lock().await;
                            if let Some(orchestrator) = orchestrator_guard.as_ref() {
                                orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                    event: crate::messages::orchestrator::StartupEvent::StartupFailed(
                                        "CommunicationActor not available".to_string()
                                    ),
                                });
                            }
                        }
                    }
                }
                
                if line.contains("Compute42: FROM_JULIA_PIPE_READY") {
                    // Get from_julia pipe name from session
                    let from_julia_pipe_name = {
                        let session_guard = julia_session.lock().await;
                        if let Some(session) = session_guard.as_ref() {
                            session.from_julia_pipe_name.clone()
                        } else {
                            None
                        }
                    };
                    
                    if let Some(from_julia_pipe) = from_julia_pipe_name {
                        let comm_actor_guard = communication_actor_state.lock().await;
                        if let Some(comm_actor) = comm_actor_guard.as_ref() {
                            let comm_actor_clone = comm_actor.clone();
                            drop(comm_actor_guard);
                            
                            match comm_actor_clone.send(crate::messages::communication::ConnectFromJuliaPipe {
                                from_julia_pipe: from_julia_pipe.clone(),
                            }).await {
                                Ok(Ok(())) => {
                                    // Successfully connected
                                }
                                Ok(Err(e)) => {
                                    error!("ProcessActor: CommunicationActor failed to connect from Julia pipe (from_julia): {}", e);
                                    // Emit startup failed if we have orchestrator
                                    let orchestrator_guard = orchestrator_actor_state.lock().await;
                                    if let Some(orchestrator) = orchestrator_guard.as_ref() {
                                        orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                            event: crate::messages::orchestrator::StartupEvent::StartupFailed(
                                                format!("Failed to connect from Julia pipe (from_julia): {}", e)
                                            ),
                                        });
                                    }
                                }
                                Err(e) => {
                                    error!("ProcessActor: Failed to send ConnectFromJuliaPipe to CommunicationActor: {:?}", e);
                                    // Emit startup failed if we have orchestrator
                                    let orchestrator_guard = orchestrator_actor_state.lock().await;
                                    if let Some(orchestrator) = orchestrator_guard.as_ref() {
                                        orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                            event: crate::messages::orchestrator::StartupEvent::StartupFailed(
                                                format!("Failed to send ConnectFromJuliaPipe message: {:?}", e)
                                            ),
                                        });
                                    }
                                }
                            }
                        } else {
                            error!("ProcessActor: CommunicationActor not available - cannot connect from Julia pipe (from_julia)");
                            let orchestrator_guard = orchestrator_actor_state.lock().await;
                            if let Some(orchestrator) = orchestrator_guard.as_ref() {
                                orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                    event: crate::messages::orchestrator::StartupEvent::StartupFailed(
                                        "CommunicationActor not available".to_string()
                                    ),
                                });
                            }
                        }
                    }
                }
                
                // When Julia signals message loop is ready, wait for both pipes to be connected
                // before signaling that communication is fully ready
                if line.contains("Compute42: MESSAGE_LOOP_READY") {
                    // Mark that MESSAGE_LOOP_READY was received
                    {
                        let mut flag_guard = message_loop_ready_received.lock().await;
                        *flag_guard = true;
                    }
                    
                    let _ = event_emitter.emit("julia:ready", serde_json::json!({})).await;
                    
                    // Check if both pipes are connected before sending JuliaMessageLoopReady
                    let comm_actor_guard = communication_actor_state.lock().await;
                    let comm_actor_clone = comm_actor_guard.clone();
                    let sender_clone = julia_message_loop_ready_sender.clone();
                    drop(comm_actor_guard);
                    
                    // Spawn task to check connection status and send JuliaMessageLoopReady when ready
                    tokio::spawn(async move {
                        // Check if both pipes are connected
                        if let Some(comm_actor) = comm_actor_clone.as_ref() {
                            // Poll until both pipes are connected
                            let mut attempts = 0;
                            const MAX_ATTEMPTS: u32 = 100; // 10 seconds max (100 * 100ms)
                            
                            loop {
                                match comm_actor.send(crate::messages::communication::IsConnected).await {
                                    Ok(Ok(true)) => {
                                        // Both pipes are connected - send the signal
                                        let sender_guard = sender_clone.lock().await;
                                        if let Some(sender) = sender_guard.as_ref() {
                                            let _ = sender.send(());
                                        }
                                        break;
                                    }
                                    Ok(Ok(false)) => {
                                        attempts += 1;
                                        if attempts >= MAX_ATTEMPTS {
                                            error!("ProcessActor: Timeout waiting for both pipes to connect after MESSAGE_LOOP_READY");
                                            // Still send the signal to avoid blocking startup forever
                                            let sender_guard = sender_clone.lock().await;
                                            if let Some(sender) = sender_guard.as_ref() {
                                                let _ = sender.send(());
                                            }
                                            break;
                                        }
                                        // Wait a bit before checking again
                                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                    }
                                    Ok(Err(e)) => {
                                        error!("ProcessActor: Error checking connection status: {}", e);
                                        // Still send the signal to avoid blocking startup
                                        let sender_guard = sender_clone.lock().await;
                                        if let Some(sender) = sender_guard.as_ref() {
                                            let _ = sender.send(());
                                        }
                                        break;
                                    }
                                    Err(e) => {
                                        error!("ProcessActor: Failed to send IsConnected message: {:?}", e);
                                        // Still send the signal to avoid blocking startup
                                        let sender_guard = sender_clone.lock().await;
                                        if let Some(sender) = sender_guard.as_ref() {
                                            let _ = sender.send(());
                                        }
                                        break;
                                    }
                                }
                            }
                        } else {
                            error!("ProcessActor: CommunicationActor not available when MESSAGE_LOOP_READY received");
                            // Still send the signal to avoid blocking startup
                            let sender_guard = sender_clone.lock().await;
                            if let Some(sender) = sender_guard.as_ref() {
                                let _ = sender.send(());
                            }
                        }
                    });
                    
                    // If no project is being activated, enable output after a delay as fallback
                    // Otherwise, PROJECT_ACTIVATION_COMPLETE will disable suppression
                    let suppressed_clone = output_suppressed.clone();
                    tokio::spawn(async move {
                        // Wait a bit longer as fallback if no project activation happens
                        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                        // Only disable if still suppressed (i.e., no PROJECT_ACTIVATION_COMPLETE was received)
                        let mut suppressed = suppressed_clone.lock().await;
                        if *suppressed {
                            *suppressed = false;
                        }
                    });
                }
                
                // When project activation completes, disable output suppression
                // This signal is emitted after Pkg.activate() and Pkg.instantiate() complete
                if line.contains("Compute42: PROJECT_ACTIVATION_COMPLETE") {
                    let mut suppressed = output_suppressed.lock().await;
                    *suppressed = false;
                    
                    // Emit project activation complete event to notify orchestrator (for frontend)
                    let _ = event_emitter.emit("orchestrator:project-activation-complete", serde_json::json!({})).await;
                    
                    // Send project activation complete signal via channel to ProcessActor
                    let sender_guard = project_activation_complete_sender.lock().await;
                    if let Some(sender) = sender_guard.as_ref() {
                        let _ = sender.send(());
                    }
                }
                
                // Emit detailed output event (includes filtered messages for StartupModal)
                // This bypasses filters and shows all messages in the detailed terminal
                let _ = event_emitter.emit(
                    "julia:output-detailed",
                    serde_json::to_value(vec![crate::messages::StreamOutput {
                        content: line.clone() + "\n",
                        stream_type: crate::messages::StreamType::Stderr,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                    }]).unwrap_or_default(),
                ).await;
                
                continue;
            }
            
            // Check if a notebook cell is currently executing
            let is_notebook_cell_executing = {
                let cell_guard = state.current_notebook_cell.lock().await;
                cell_guard.is_some()
            };
            
            // Buffer output if a notebook cell is executing
            if is_notebook_cell_executing {
                let mut buffer_guard = state.notebook_cell_output_buffer.lock().await;
                if let Some(ref mut buffer) = *buffer_guard {
                    buffer.stderr.push(line.clone());
                }
                // Don't emit to terminal when notebook cell is executing - output will be sent via notebook events
                continue;
            }
            
            // Emit detailed output event for all messages (even if filtered for main terminal)
            let line_clone = line.clone();
            if let Err(e) = event_emitter.emit(
                "julia:output-detailed",
                serde_json::to_value(vec![crate::messages::StreamOutput {
                    content: line.clone() + "\n",
                    stream_type: crate::messages::StreamType::Stderr,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                }]).unwrap_or_default(),
            ).await {
                error!("ProcessActor: Failed to emit julia:output-detailed event: {}", e);
            }
            
            // Only emit output if not suppressed (for main terminal)
            let should_emit = {
                let suppressed = output_suppressed.lock().await;
                !*suppressed
            };
            if should_emit {
                if let Err(e) = event_emitter.emit(
                    "julia:output",
                    serde_json::to_value(vec![crate::messages::StreamOutput {
                        content: line_clone + "\n",
                        stream_type: crate::messages::StreamType::Stderr,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                    }]).unwrap_or_default(),
                ).await {
                    error!("ProcessActor: Failed to emit julia-output event: {}", e);
                }
            }
        }
    });
}

