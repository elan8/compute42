// Code execution management for CommunicationActor
// Handles code execution requests and responses

use crate::services::base::file_utils::convert_path_for_julia;
use log::{debug, error};
use uuid::Uuid;

use super::state::State;

/// Get the current busy status
pub async fn is_busy(state: &State) -> bool {
    // We'll track this in state if needed, for now check if there's a current request
    let current_request_guard = state.current_request.lock().await;
    current_request_guard.is_some()
}

/// Execute code with Julia
pub async fn execute_code(
    state: &State,
    code: String,
    execution_type: crate::messages::ExecutionType,
    file_path: Option<String>,
    suppress_busy_events: bool,
) -> Result<crate::messages::JuliaMessage, String> {
    let request_id = Uuid::new_v4().to_string();
    execute_single_request(state, code, execution_type, file_path, request_id, suppress_busy_events).await
}

/// Execute a single request (internal method)
async fn execute_single_request(
    state: &State,
    code: String,
    execution_type: crate::messages::ExecutionType,
    file_path: Option<String>,
    request_id: String,
    suppress_busy_events: bool,
) -> Result<crate::messages::JuliaMessage, String> {
    // Emit backend-busy event (unless suppressed for batch execution)
    if !suppress_busy_events {
        if let Err(e) = state.event_manager.emit_backend_busy(&request_id).await {
            error!("[CommunicationActor::Execution] Failed to emit backend-busy event: {}", e);
        }
    }

    // Prepare the code with working directory handling for file execution or notebook cells
    let final_code = if let Some(path) = file_path {
        // Extract the directory from the file path
        let file_path_std = std::path::Path::new(&path);
        let file_dir = if file_path_std.is_absolute() {
            // For absolute paths, use the directory containing the file
            file_path_std
                .parent()
                .and_then(|p| p.to_str())
                .map(convert_path_for_julia)
                .unwrap_or_else(|| ".".to_string())
        } else {
            // For relative paths, use current directory
            file_path_std
                .parent()
                .and_then(|p| p.to_str())
                .map(convert_path_for_julia)
                .unwrap_or_else(|| ".".to_string())
        };

        debug!(
            "[CommunicationActor::Execution] Setting working directory to: {}",
            file_dir
        );

        // Check if this is a notebook cell execution (just change directory, don't include file)
        let is_notebook_cell = matches!(execution_type, crate::messages::ExecutionType::NotebookCell { .. });
        
        if is_notebook_cell {
            // For notebook cells, change to the notebook's directory before executing
            // Prepend cd() call to the code so it executes first, but don't wrap in begin/end
            // This preserves output isolation per cell
            format!(
                r#"cd("{}"); {}"#,
                file_dir, code
            )
        } else {
            // For file execution, use include() to execute the actual file
            // This ensures @__DIR__ resolves correctly
            let julia_file_path = convert_path_for_julia(&path);
            
            // Try to extract module name from filename (filename without extension)
            // This helps detect if we need to reload a module
            let file_stem = file_path_std
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            
            format!(
            r#"
            begin
                # Change to the directory containing the file
                cd("{}")
                
                # Try to reload modules that might be imported by this file
                # This ensures changes to module files are picked up
                try
                    # First, check if the file itself defines a module (filename-based)
                    file_module_name = "{}"
                    # println("Revise: Checking if file module '", file_module_name, "' needs reloading...")
                    if isdefined(Main, Symbol(file_module_name))
                        module_symbol = Symbol(file_module_name)
                        module_obj = getfield(Main, module_symbol)
                        if isa(module_obj, Module)
                            # println("Revise: File module '", file_module_name, "' found in Main, attempting reload...")
                            if isdefined(Main, :reload_package)
                                try
                                    reload_package(module_symbol)
                                    # println("Revise: ✓ Reloaded file module '", file_module_name, "'")
                                catch e
                                    # println("Revise: Failed to reload file module: ", e)
                                end
                            end
                        end
                    end
                    
                    # Parse the file to find modules imported with 'using' or 'import'
                    file_path_to_parse = "{}"
                    try
                        if isfile(file_path_to_parse)
                            file_content = open(file_path_to_parse) do f
                                read(f, String)
                            end
                            # println("Revise: Parsing file for 'using' statements...")
                            
                            # Extract module names from 'using' statements
                            imported_modules = String[]
                            for line in split(file_content, '\n')
                                stripped = strip(line)
                                # Skip comments
                                if !startswith(stripped, '#')
                                    # Check for 'using ModuleName' or 'using Package.Module'
                                    if startswith(stripped, "using ")
                                        # Extract the module name
                                        using_part = strip(stripped[7:end])  # Remove "using "
                                        # Handle cases like "using Package.Module" or "using Package: Module"
                                        if contains(using_part, ':')
                                            # Format: using Package: Module1, Module2
                                            package_part = split(using_part, ':')[1]
                                            for module_name in split(strip(package_part), ',')
                                                module_name_clean = strip(module_name)
                                                if !isempty(module_name_clean)
                                                    push!(imported_modules, module_name_clean)
                                                end
                                            end
                                        elseif contains(using_part, '.')
                                            # Format: using Package.Module - extract just the Module part
                                            parts = split(using_part, '.')
                                            if length(parts) > 1
                                                # For now, try the full path and also the last part
                                                push!(imported_modules, using_part)
                                                push!(imported_modules, parts[end])
                                            else
                                                push!(imported_modules, using_part)
                                            end
                                        else
                                            # Simple: using ModuleName
                                            push!(imported_modules, using_part)
                                        end
                                    elseif startswith(stripped, "import ")
                                        # Similar handling for 'import'
                                        import_part = strip(stripped[8:end])  # Remove "import "
                                        if contains(import_part, '.')
                                            parts = split(import_part, '.')
                                            if length(parts) > 1
                                                push!(imported_modules, import_part)
                                                push!(imported_modules, parts[end])
                                            else
                                                push!(imported_modules, import_part)
                                            end
                                        else
                                            push!(imported_modules, import_part)
                                        end
                                    end
                                end
                            end
                            
                            # Reload any imported modules that are already loaded
                            # println("Revise: Found ", length(imported_modules), " potential module imports")
                            for mod_name in imported_modules
                                mod_name_clean = strip(mod_name)
                                if !isempty(mod_name_clean)
                                    # println("Revise: Checking if imported module '", mod_name_clean, "' needs reloading...")
                                    if isdefined(Main, Symbol(mod_name_clean))
                                        mod_symbol = Symbol(mod_name_clean)
                                        mod_obj = getfield(Main, mod_symbol)
                                        if isa(mod_obj, Module)
                                            # println("Revise: Imported module '", mod_name_clean, "' found in Main, attempting reload...")
                                            if isdefined(Main, :reload_package)
                                                try
                                                    reload_package(mod_symbol)
                                                    # println("Revise: ✓ Reloaded imported module '", mod_name_clean, "'")
                                                catch e
                                                    # println("Revise: reload_package() failed: ", e)
                                                    # Fallback: try tracking and reimporting
                                                    try
                                                        # println("Revise: Attempting to track and reimport '", mod_name_clean, "'...")
                                                        using Revise
                                                        # Try to track the module
                                                        try
                                                            Revise.track(String(mod_name_clean))
                                                            # println("Revise: Tracked '", mod_name_clean, "'")
                                                        catch
                                                            # If tracking by name fails, try to find the module path
                                                            try
                                                                mod_path = Base.find_package(mod_name_clean)
                                                                if mod_path !== nothing
                                                                    Revise.track(mod_path)
                                                                    # println("Revise: Tracked '", mod_name_clean, "' by path: ", mod_path)
                                                                end
                                                            catch
                                                            end
                                                        end
                                                        # Reimport to trigger reload
                                                        # println("Revise: Reimporting '", mod_name_clean, "' to trigger reload...")
                                                        eval(Main, :(using $(mod_symbol)))
                                                        # Call revise() to apply any changes
                                                        revise(mod_obj; force=true)
                                                        # println("Revise: ✓ Reloaded imported module '", mod_name_clean, "' via track and reimport")
                                                    catch e2
                                                        # println("Revise: ✗ Failed to reload imported module '", mod_name_clean, "' via track/reimport: ", e2)
                                                    end
                                                end
                                            else
                                                # reload_package not available, try tracking and reimporting directly
                                                try
                                                    # println("Revise: reload_package not available, using track and revise for '", mod_name_clean, "'...")
                                                    using Revise
                                                    # Try to track the module
                                                    try
                                                        Revise.track(String(mod_name_clean))
                                                        # println("Revise: Tracked '", mod_name_clean, "'")
                                                    catch
                                                        # If tracking by name fails, try to find the module path
                                                        try
                                                            mod_path = Base.find_package(mod_name_clean)
                                                            if mod_path !== nothing
                                                                Revise.track(mod_path)
                                                                # println("Revise: Tracked '", mod_name_clean, "' by path: ", mod_path)
                                                            end
                                                        catch
                                                        end
                                                    end
                                                    # Use revise() to reload the module
                                                    # println("Revise: Calling revise() on '", mod_name_clean, "'...")
                                                    revise(mod_obj; force=true)
                                                    # println("Revise: ✓ Reloaded imported module '", mod_name_clean, "' via revise()")
                                                catch e
                                                    # println("Revise: revise() failed, trying reimport...")
                                                    # If revise() fails, try reimporting
                                                    try
                                                        eval(Main, :(using $(mod_symbol)))
                                                        revise(mod_obj; force=true)
                                                        # println("Revise: ✓ Reloaded imported module '", mod_name_clean, "' via reimport and revise()")
                                                    catch e2
                                                        # println("Revise: ✗ Failed to reload imported module '", mod_name_clean, "': ", e2)
                                                    end
                                                end
                                            end
                                        end
                                    else
                                        # println("Revise: Imported module '", mod_name_clean, "' not yet loaded, will load fresh")
                                    end
                                end
                            end
                        end
                    catch e
                        # println("Revise: Could not parse file for imports: ", e)
                    end
                catch e
                    # If reload fails, continue anyway - the include will work
                    # println("Revise: Error during reload check: ", e)
                    # println("Revise: Will attempt to include file anyway...")
                end
                
                # Include the actual file (not its contents)
                include("{}")
            end
            "#,
                file_dir, file_stem, julia_file_path, julia_file_path
            )
        }
    } else {
        // No file path provided - use the code as-is
        code
    };

    // Send code execution request
    debug!(
        "[CommunicationActor::Execution] Sending code execution request with ID: {}",
        request_id
    );
    debug!(
        "[CommunicationActor::Execution] Execution type: {:?}",
        execution_type
    );

    // No breakpoints for regular execution (debug execution uses different path)
    let breakpoints = None;

    let message = crate::messages::JuliaMessage::CodeExecution {
        id: request_id.clone(),
        code: final_code,
        execution_type: execution_type.clone(),
        timeout_ms: None, // No timeout - use heartbeat instead
        breakpoints,
    };

    // Create a channel for the response
    let (tx, rx) = tokio::sync::oneshot::channel::<crate::messages::JuliaMessage>();

    // Store the current request
    debug!(
        "[CommunicationActor::Execution] Setting current request for ID: {}",
        request_id
    );
    {
        let mut current_request_guard = state.current_request.lock().await;
        *current_request_guard = Some((request_id.clone(), tx));
    } // Release the lock here
    debug!("[CommunicationActor::Execution] Current request set, lock released");

    // Send the message
    let message_sender_guard = state.message_sender.lock().await;
    if let Some(sender) = message_sender_guard.as_ref() {
        sender
            .send(message)
            .await
            .map_err(|e| format!("Failed to send message: {}", e))?;
    } else {
        return Err("Message sender not initialized".to_string());
    }
    debug!("[CommunicationActor::Execution] Message sent, waiting for response...");

    // Wait for ExecutionComplete response (no timeout - use heartbeat for health checks)
    let result = match rx.await {
        Ok(response) => {
            if let crate::messages::JuliaMessage::ExecutionComplete { success, error, execution_type: exec_type, .. } = &response {
                if *success {
                    // For FileExecution and NotebookCell, trigger workspace variables retrieval in background
                    let should_get_variables = matches!(
                        exec_type,
                        crate::messages::ExecutionType::FileExecution
                        | crate::messages::ExecutionType::NotebookCell { .. }
                    );
                    
                    if should_get_variables {
                        debug!("[CommunicationActor::Execution] Execution completed successfully (type: {:?}), scheduling workspace variables retrieval", exec_type);
                        
                        // Clone the necessary data for the background task
                        let message_sender = state.message_sender.clone();
                        
                        // Spawn a background task to retrieve workspace variables
                        // This won't block the backend-done event
                        tokio::spawn(async move {
                            // Small delay to ensure the main execution has fully completed
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            
                            // Request workspace variables from Julia
                            let workspace_request_id = Uuid::new_v4().to_string();
                            let message = crate::messages::JuliaMessage::GetWorkspaceVariables {
                                id: workspace_request_id.clone(),
                            };
                            
                            // Send the request
                            let message_sender_guard = message_sender.lock().await;
                            if let Some(sender) = message_sender_guard.as_ref() {
                                if sender.send(message).await.is_ok() {
                                    debug!("[CommunicationActor::Execution] Workspace variables request sent (background)");
                                } else {
                                    debug!("[CommunicationActor::Execution] Failed to send workspace variables request");
                                }
                            }
                        });
                    }
                    
                    // Emit backend-done event after execution completes (unless suppressed)
                    if !suppress_busy_events {
                        if let Err(e) = state.event_manager.emit_backend_done(&request_id).await {
                            error!("[CommunicationActor::Execution] Failed to emit backend-done event: {}", e);
                        }
                    }
                    
                    Ok(response)
                } else {
                    // Emit backend-done event even on failure (unless suppressed)
                    if !suppress_busy_events {
                        if let Err(e) = state.event_manager.emit_backend_done(&request_id).await {
                            error!("[CommunicationActor::Execution] Failed to emit backend-done event: {}", e);
                        }
                    }
                    Err(error.clone().unwrap_or_else(|| "Unknown error".to_string()))
                }
            } else {
                Err("Received unexpected response type".to_string())
            }
        }
        Err(_) => Err("Failed to receive response".to_string()),
    };

    // Add a longer delay to allow stdout to be fully processed and displayed
    // This ensures the Julia prompt doesn't appear before the output is complete
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    result
}

