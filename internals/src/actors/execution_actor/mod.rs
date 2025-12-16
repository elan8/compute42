use actix::prelude::*;
use log::{debug, error};
use uuid;

use crate::messages::execution::*;
use crate::messages::communication::{ExecuteCode, IsConnected};
use crate::services::events::EventService;
use crate::messages::{ExecutionType, JuliaMessage};
use crate::actors::communication_actor::CommunicationActor;

/// ExecutionActor - manages Julia code execution
/// This replaces the mutex-based ExecutionManager with a clean actor model
pub struct ExecutionActor {
    // Actor state (no mutexes needed)
    current_project: Option<String>,
    execution_queue: Vec<String>,
    is_executing: bool,
    last_execution_result: Option<String>,
    
    // Actor addresses for inter-actor communication
    communication_actor: Addr<CommunicationActor>,
    event_manager: EventService,
}

impl ExecutionActor {
    /// Create a new ExecutionActor instance
    pub fn new(
        communication_actor: Addr<CommunicationActor>,
        event_manager: EventService,
    ) -> Self {
        
        Self {
            current_project: None,
            execution_queue: Vec::new(),
            is_executing: false,
            last_execution_result: None,
            communication_actor,
            event_manager,
        }
    }

    /// Execute code with the specified execution type
    /// This helper eliminates duplication between API and REPL execution handlers
    async fn execute_code_with_type(
        code: String,
        execution_type: crate::messages::execution::ExecutionType,
        communication_actor: Addr<CommunicationActor>,
    ) -> Result<String, String> {
        Self::execute_code_with_type_and_path(code, execution_type, None, communication_actor, false).await
    }

    /// Execute code with the specified execution type and file path
    /// This helper eliminates duplication and allows setting working directory
    async fn execute_code_with_type_and_path(
        code: String,
        execution_type: crate::messages::execution::ExecutionType,
        file_path: Option<String>,
        communication_actor: Addr<CommunicationActor>,
        suppress_busy_events: bool,
    ) -> Result<String, String> {
        // Check if connected via message to CommunicationActor
        let is_connected = communication_actor.send(IsConnected).await
            .map_err(|e| format!("Failed to check connection status: {}", e))?
            .map_err(|e| format!("Connection check failed: {}", e))?;
        
        if !is_connected {
            return Err("Not connected to Julia process".to_string());
        }
        
        // Execute the code via message to CommunicationActor
        let message = communication_actor.send(ExecuteCode {
            code,
            execution_type,
            file_path,
            suppress_busy_events,
        }).await
            .map_err(|e| format!("Failed to send execute code message: {}", e))?
            .map_err(|e| format!("Code execution failed: {}", e))?;
        
        // Process the result
        match message {
            crate::messages::JuliaMessage::ExecutionComplete { result, error, success, .. } => {
                if success {
                    Ok(result.unwrap_or_default())
                } else {
                    Err(error.unwrap_or_else(|| "Execution failed".to_string()))
                }
            }
            crate::messages::JuliaMessage::PlotData { .. } => Ok("Plot generated".to_string()),
            crate::messages::JuliaMessage::Error { message, .. } => Err(message),
            _ => Ok("Unknown message type".to_string()),
        }
    }
}

impl Actor for ExecutionActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // debug!("ExecutionActor: Actor started");
        ctx.set_mailbox_capacity(256);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("ExecutionActor: Actor stopped");
    }
}

// Message handlers
impl Handler<ExecuteApiRequest> for ExecutionActor {
    type Result = ResponseActFuture<Self, Result<String, String>>;
    
    fn handle(&mut self, msg: ExecuteApiRequest, _ctx: &mut Context<Self>) -> Self::Result {
        let code = msg.code;
        let communication_actor = self.communication_actor.clone();
        
        Box::pin(
            async move {
                Self::execute_code_with_type(code, ExecutionType::ApiCall, communication_actor).await
            }
            .into_actor(self)
        )
    }
}

impl Handler<ExecuteReplRequest> for ExecutionActor {
    type Result = ResponseActFuture<Self, Result<String, String>>;
    
    fn handle(&mut self, msg: ExecuteReplRequest, _ctx: &mut Context<Self>) -> Self::Result {
        let code = msg.code;
        let communication_actor = self.communication_actor.clone();
        
        Box::pin(
            async move {
                Self::execute_code_with_type(code, ExecutionType::ReplExecution, communication_actor).await
            }
            .into_actor(self)
        )
    }
}

impl Handler<ExecuteNotebookCell> for ExecutionActor {
    type Result = ResponseActFuture<Self, Result<String, String>>;
    
    fn handle(&mut self, msg: ExecuteNotebookCell, _ctx: &mut Context<Self>) -> Self::Result {
        let code = msg.code;
        let cell_id = msg.cell_id;
        let notebook_path = msg.notebook_path;
        let communication_actor = self.communication_actor.clone();
        
        Box::pin(
            async move {
                Self::execute_code_with_type_and_path(
                    code,
                    ExecutionType::NotebookCell { cell_id },
                    notebook_path,
                    communication_actor,
                    true, // Suppress busy events for notebook cells - busy state is managed at notebook level
                ).await
            }
            .into_actor(self)
        )
    }
}

impl Handler<ExecuteNotebookCellsBatch> for ExecutionActor {
    type Result = ResponseActFuture<Self, Result<Vec<(String, Result<String, String>)>, String>>;
    
    fn handle(&mut self, msg: ExecuteNotebookCellsBatch, _ctx: &mut Context<Self>) -> Self::Result {
        let cells = msg.cells;
        let communication_actor = self.communication_actor.clone();
        let event_manager = self.event_manager.clone();
        
        Box::pin(
            async move {
                // Check if connected
                let is_connected = communication_actor.send(IsConnected).await
                    .map_err(|e| format!("Failed to check connection status: {}", e))?
                    .map_err(|e| format!("Connection check failed: {}", e))?;
                
                if !is_connected {
                    return Err("Not connected to Julia process".to_string());
                }
                
                // Emit backend-busy event at the start of batch
                let batch_request_id = uuid::Uuid::new_v4().to_string();
                if let Err(e) = event_manager.emit_backend_busy(&batch_request_id).await {
                    error!("[ExecutionActor] Failed to emit backend-busy event for batch: {}", e);
                }
                
                // Execute all cells sequentially with suppressed busy events
                let mut results = Vec::new();
                for cell in cells {
                    let cell_id = cell.cell_id.clone();
                    let notebook_path = cell.notebook_path.clone();
                    let result = communication_actor.send(ExecuteCode {
                        code: cell.code,
                        execution_type: ExecutionType::NotebookCell { cell_id: cell.cell_id },
                        file_path: notebook_path,
                        suppress_busy_events: true, // Suppress individual busy events
                    }).await
                        .map_err(|e| format!("Failed to send execute code message: {}", e))?
                        .map_err(|e| format!("Code execution failed: {}", e));
                    
                    // Process the result
                    let cell_result = match &result {
                        Ok(message) => {
                            match message {
                                crate::messages::JuliaMessage::ExecutionComplete { result, error, success, .. } => {
                                    if *success {
                                        Ok(result.clone().unwrap_or_default())
                                    } else {
                                        Err(error.clone().unwrap_or_else(|| "Execution failed".to_string()))
                                    }
                                }
                                crate::messages::JuliaMessage::PlotData { .. } => Ok("Plot generated".to_string()),
                                crate::messages::JuliaMessage::Error { message, .. } => Err(message.clone()),
                                _ => Ok("Unknown message type".to_string()),
                            }
                        }
                        Err(e) => Err(e.clone()),
                    };
                    
                    results.push((cell_id, cell_result));
                }
                
                // Emit backend-done event at the end of batch
                if let Err(e) = event_manager.emit_backend_done(&batch_request_id).await {
                    error!("[ExecutionActor] Failed to emit backend-done event for batch: {}", e);
                }
                
                Ok(results)
            }
            .into_actor(self)
        )
    }
}


impl Handler<ExecuteFile> for ExecutionActor {
    type Result = ResponseActFuture<Self, Result<String, String>>;
    
    fn handle(&mut self, msg: ExecuteFile, _ctx: &mut Context<Self>) -> Self::Result {
        let file_path = msg.file_path;
        let communication_actor = self.communication_actor.clone();
        
        Box::pin(
            async move {
                // Check if connected via message
                let is_connected = communication_actor.send(IsConnected).await
                    .map_err(|e| format!("Failed to check connection: {}", e))?
                    .map_err(|e| format!("Connection check failed: {}", e))?;
                
                if !is_connected {
                    return Err("Not connected to Julia process".to_string());
                }
                
                // Read file content
                let content = std::fs::read_to_string(&file_path)
                    .map_err(|e| format!("Failed to read file {}: {}", file_path, e))?;
                
                // Execute code via message
                let message = communication_actor.send(ExecuteCode {
                    code: content,
                    execution_type: ExecutionType::FileExecution,
                    file_path: Some(file_path),
                    suppress_busy_events: false,
                }).await
                    .map_err(|e| format!("Failed to send execute code message: {}", e))?
                    .map_err(|e| format!("Code execution failed: {}", e))?;
                
                match message {
                    JuliaMessage::ExecutionComplete { result, error, success, .. } => {
                        if success {
                            Ok(result.unwrap_or_default())
                        } else {
                            Err(error.unwrap_or_else(|| "Execution failed".to_string()))
                        }
                    }
                    JuliaMessage::PlotData { .. } => Ok("Plot generated".to_string()),
                    JuliaMessage::Error { message, .. } => Err(message),
                    _ => Ok("Unknown message type".to_string()),
                }
            }
            .into_actor(self)
        )
    }
}

impl Handler<ActivateProject> for ExecutionActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: ActivateProject, _ctx: &mut Context<Self>) -> Self::Result {
        let project_path = msg.project_path;
        let communication_actor = self.communication_actor.clone();
        
        Box::pin(
            async move {
                debug!("ExecutionActor: Activating project {}", project_path);
                
                // Check if project exists
                if !std::path::Path::new(&project_path).exists() {
                    return Err(format!("Project path does not exist: {}", project_path));
                }
                
                // Check if it's a Julia project (has Project.toml)
                let project_toml_path = format!("{}/Project.toml", project_path);
                if !std::path::Path::new(&project_toml_path).exists() {
                    return Err(format!("Not a valid Julia project (no Project.toml): {}", project_path));
                }
                
                // Check if connected via message
                let is_connected = communication_actor.send(IsConnected).await
                    .map_err(|e| format!("Failed to check connection: {}", e))?
                    .map_err(|e| format!("Connection check failed: {}", e))?;
                
                // Send activation command to Julia
                if is_connected {
                    // Convert Windows path to Julia-compatible format
                    let julia_path = project_path.replace('\\', "/");
                    
                    // Step 1: Activate the project
                    debug!("ExecutionActor: Activating Julia project...");
                    
                    let activation_code = format!(
                        r#"
                        try
                            using Pkg
                            Pkg.activate("{}")
                            println("Project activated: {}")
                            true
                        catch e
                            println("Failed to activate project: ", e)
                            false
                        end
                        "#,
                        julia_path, julia_path
                    );
                    
                    let activation_result = communication_actor.send(ExecuteCode {
                        code: activation_code,
                        execution_type: ExecutionType::ApiCall,
                        file_path: None,
                        suppress_busy_events: false,
                    }).await
                        .map_err(|e| format!("Failed to send activation code: {}", e))?
                        .map_err(|e| format!("Activation failed: {}", e))?;
                    
                    // Check if activation was successful
                    let activation_success = match activation_result {
                        JuliaMessage::ExecutionComplete { result: Some(result_str), success, .. } => {
                            success && result_str.trim() == "true"
                        }
                        JuliaMessage::ExecutionComplete { error: Some(error_msg), .. } => {
                            return Err(format!("Julia activation error: {}", error_msg));
                        }
                        _ => return Err("Unexpected response from Julia activation".to_string()),
                    };
                    
                    if !activation_success {
                        return Err("Failed to activate Julia project".to_string());
                    }
                    
                    debug!("ExecutionActor: Project activated, installing dependencies...");
                    
                    // Step 2: Always run instantiate during startup to ensure packages are installed
                    // This is idempotent - it will only install missing packages
                    let instantiate_code = format!(
                        r#"
                        try
                            using Pkg
                            
                            # Always run instantiate during startup to ensure all packages are installed
                            # This is idempotent and safe - it will only install what's missing
                            println("Ensuring all project dependencies are installed...")
                            flush(stdout)
                            flush(stderr)
                            
                            # Run instantiate with retry logic if dependencies are missing
                            max_instantiate_attempts = 2
                            instantiate_successful = false
                            
                            for attempt in 1:max_instantiate_attempts
                                try
                                    println("Starting Pkg.instantiate() (attempt $attempt)...")
                                    flush(stdout)
                                    flush(stderr)
                                    Pkg.instantiate()
                                    println("Pkg.instantiate() completed successfully")
                                    flush(stdout)
                                    flush(stderr)
                                    
                                    # Resolve dependencies to ensure all transitive dependencies are properly resolved
                                    println("Resolving dependencies...")
                                    flush(stdout)
                                    flush(stderr)
                                    Pkg.resolve()
                                    println("Dependencies resolved successfully")
                                    flush(stdout)
                                    flush(stderr)
                                    
                                    # Verify that project dependencies are actually available
                                    # Check by trying to get dependency information - if this fails, dependencies might be missing
                                    println("Verifying dependencies are installed...")
                                    deps = Pkg.dependencies()
                                    
                                    # Check if we can access the project's direct dependencies
                                    project_toml_path = joinpath("{}", "Project.toml")
                                    if isfile(project_toml_path)
                                        project_data = Pkg.TOML.parsefile(project_toml_path)
                                        project_deps = get(project_data, "deps", Dict{{String, Any}}())
                                        
                                        # Verify each direct dependency is available
                                        missing_deps = String[]
                                        for (dep_name, dep_uuid) in project_deps
                                            # Check if dependency is in the resolved dependencies
                                            dep_found = false
                                            for (uuid, dep_info) in deps
                                                if dep_info.name == dep_name
                                                    dep_found = true
                                                    break
                                                end
                                            end
                                            
                                            if !dep_found
                                                push!(missing_deps, dep_name)
                                            end
                                        end
                                        
                                        if !isempty(missing_deps)
                                            if attempt < max_instantiate_attempts
                                                println("Some dependencies appear to be missing: ", join(missing_deps, ", "))
                                                println("Retrying Pkg.instantiate()...")
                                                continue
                                            else
                                                println("Warning: Some dependencies may still be missing after instantiate attempts")
                                            end
                                        else
                                            instantiate_successful = true
                                            break
                                        end
                                    else
                                        # No Project.toml, just verify instantiate completed
                                        instantiate_successful = true
                                        break
                                    end
                                catch e
                                    error_msg = sprint(showerror, e)
                                    if attempt < max_instantiate_attempts
                                        println("Pkg.instantiate() encountered an error, retrying...")
                                        println("Error: ", error_msg)
                                        continue
                                    else
                                        println("Warning: Pkg.instantiate() failed after $max_instantiate_attempts attempts: ", error_msg)
                                        # Continue anyway - some packages might still work
                                        instantiate_successful = true
                                    end
                                end
                            end
                            
                            if instantiate_successful
                                println("Project dependencies ready")
                            else
                                println("Warning: Dependency installation may be incomplete")
                            end
                            
                            println("Project dependencies ready")
                            
                            # Put the local package in dev mode so Revise can track it
                            # This is critical for Revise to work with local packages
                            try
                                project_toml_check = joinpath("{}", "Project.toml")
                                if isfile(project_toml_check)
                                    project_toml_check_data = Pkg.TOML.parsefile(project_toml_check)
                                    if haskey(project_toml_check_data, "name")
                                        pkg_name_dev = project_toml_check_data["name"]
                                        
                                        # Check if this package is the active project
                                        # You can't add the active project to dev mode - it's already effectively in dev mode
                                        active_project = Pkg.project()
                                        is_active_project = active_project.name == pkg_name_dev
                                        
                                        if is_active_project
                                            # Package is the active project - no need to add to dev mode
                                            # It's already effectively in dev mode since it's the active project
                                            # println("Package '$pkg_name_dev' is the active project, skipping dev mode setup")
                                        else
                                            # Check if package is already in dev mode
                                            deps = Pkg.dependencies()
                                            pkg_in_dev = false
                                            for (uuid, dep) in deps
                                                if dep.name == pkg_name_dev && dep.is_tracking_path
                                                    pkg_in_dev = true
                                                    break
                                                end
                                            end
                                            
                                            # If not in dev mode, add it
                                            if !pkg_in_dev
                                                Pkg.develop(path="{}")
                                                println("Package '$pkg_name_dev' set to dev mode for Revise tracking")
                                            end
                                        end
                                    end
                                end
                            catch e
                                # If dev mode setup fails, that's OK - Revise might still work
                                println("Note: Could not set package to dev mode: ", e)
                            end
                            
                            # Configure Revise to track this project's source files
                            # This enables automatic reloading when files change
                            try
                                # println("Revise: Setting up automatic tracking for project...")
                                using Revise
                                # println("Revise: Revise module loaded")
                                
                                # Check if this project has a Project.toml (it's a package)
                                project_toml_path = joinpath("{}", "Project.toml")
                                if isfile(project_toml_path)
                                    # println("Revise: Found Project.toml, checking for package name...")
                                    try
                                        # Try to get the package name from Project.toml
                                        project_toml = Pkg.TOML.parsefile(project_toml_path)
                                        if haskey(project_toml, "name")
                                            pkg_name = project_toml["name"]
                                            # println("Revise: Package name found: '", pkg_name, "'")
                                            
                                            # If the package is already loaded, we need to ensure Revise tracks it
                                            # Revise tracks packages loaded AFTER it's initialized, so if a package
                                            # was loaded before Revise, we need to reload it
                                            if isdefined(Main, Symbol(pkg_name))
                                                # println("Revise: Package '", pkg_name, "' is already loaded, ensuring Revise tracking...")
                                                # Try to reload the package using Revise so it gets tracked
                                                try
                                                    # Use the reload_package helper function if available
                                                    if isdefined(Main, :reload_package)
                                                        # println("Revise: Using reload_package() to reload '", pkg_name, "'...")
                                                        reload_package(Symbol(pkg_name))
                                                        # println("Revise: ✓ Reloaded package '", pkg_name, "' for automatic tracking")
                                                        
                                                        # Verify tracking
                                                        if isdefined(Main, :check_revise_tracking)
                                                            check_revise_tracking(Symbol(pkg_name))
                                                        end
                                                    else
                                                        # println("Revise: reload_package helper not available, trying manual tracking...")
                                                        # Fallback: try to track and reload manually
                                                        pkg_module = getfield(Main, Symbol(pkg_name))
                                                        if isa(pkg_module, Module)
                                                            # Try tracking the package
                                                            try
                                                                # println("Revise: Attempting to track '", pkg_name, "' by name...")
                                                                Revise.track(pkg_name)
                                                                # println("Revise: ✓ Tracked '", pkg_name, "' by name")
                                                            catch e
                                                                # println("Revise: Tracking by name failed: ", e)
                                                                # If tracking by name fails, try tracking the package directory
                                                                pkg_uuid = get(project_toml, "uuid", nothing)
                                                                if pkg_uuid !== nothing
                                                                    try
                                                                        # println("Revise: Attempting to track '", pkg_name, "' by path...")
                                                                        # Try to find and track the package path
                                                                        pkg_path = Base.find_package(pkg_name)
                                                                        if pkg_path !== nothing
                                                                            Revise.track(pkg_path)
                                                                            # println("Revise: ✓ Tracked '", pkg_name, "' by path: ", pkg_path)
                                                                        else
                                                                            # println("Revise: Could not find package path for '", pkg_name, "'")
                                                                        end
                                                                    catch e2
                                                                        # println("Revise: Tracking by path failed: ", e2)
                                                                    end
                                                                end
                                                            end
                                                            # Force a reload by reimporting
                                                            # println("Revise: Reimporting '", pkg_name, "' to trigger tracking...")
                                                            eval(Main, :(using $pkg_name))
                                                            # println("Revise: ✓ Reloaded package '", pkg_name, "' for automatic tracking")
                                                        end
                                                    end
                                                catch e
                                                    # println("Revise: Reload failed: ", e)
                                                    # If reload fails, at least try to track it for future changes
                                                    try
                                                        # println("Revise: Attempting to track '", pkg_name, "' for future changes...")
                                                        Revise.track(pkg_name)
                                                        # println("Revise: ✓ Tracking package '", pkg_name, "' (reload may be needed for current session)")
                                                    catch e2
                                                        # println("Revise: ✗ Could not set up Revise tracking for '", pkg_name, "': ", e2)
                                                        # println("Revise: Try manually reloading with: reload_package(:", pkg_name, ") or restart Julia")
                                                    end
                                                end
                                            else
                                                # Package not loaded yet - when it's loaded with 'using', Revise will track it automatically
                                                # println("Revise: Package '", pkg_name, "' not loaded yet - will be tracked automatically when loaded")
                                            end
                                        else
                                            # println("Revise: Project.toml found but no 'name' field")
                                        end
                                    catch e
                                        # println("Revise: ✗ Could not parse Project.toml for Revise tracking: ", e)
                                    end
                                else
                                    # println("Revise: No Project.toml found, skipping package tracking")
                                end
                                
                                # Also track the project's src/ directory for any files loaded with include()
                                # This is useful for non-package projects or additional files
                                project_src_dir = joinpath("{}", "src")
                                if isdir(project_src_dir)
                                    # println("Revise: Found src/ directory, tracking for automatic reloading...")
                                    # Track the src directory - this helps with files loaded via include()
                                    try
                                        Revise.track(project_src_dir)
                                        # println("Revise: ✓ Tracking project src/ directory: ", project_src_dir)
                                    catch e
                                        # If tracking directory fails, that's OK - package tracking above should work
                                        # println("Revise: ✗ Could not track src/ directory: ", e)
                                    end
                                else
                                    # println("Revise: No src/ directory found")
                                end
                            catch e
                                # If Revise tracking fails, log but don't fail activation
                                # println("Revise: ✗ Revise tracking setup failed: ", e)
                            end
                            
                            # Wait for precompilation to complete to ensure all packages are ready
                            # This ensures packages are fully installed and precompiled before we signal completion
                            println("Waiting for package precompilation to complete...")
                            try
                                # Precompile packages - this ensures all packages are ready
                                Pkg.precompile()
                                println("Package precompilation complete")
                            catch e
                                # If precompilation fails, that's OK - packages are still installed
                                # They'll be precompiled on first use
                                println("Note: Some packages may still be precompiling (will happen on first use)")
                            end
                            
                            # Emit signal after activation, instantiate, and precompilation are complete
                            println(stderr, "Compute42: PROJECT_ACTIVATION_COMPLETE")
                            
                            true
                        catch e
                            println("Failed to instantiate project: ", e)
                            println(stderr, "Compute42: PROJECT_ACTIVATION_COMPLETE")
                            # Still emit complete signal even on error to allow startup to continue
                            # The error will be visible in the terminal
                            false
                        end
                        "#,
                        julia_path, // line 313: project_toml_check path
                        julia_path, // line 330: Pkg.develop path
                        julia_path, // line 344: project_toml_path (dependency verification)
                        julia_path, // line 346: project_toml_path (Revise tracking)
                        julia_path  // line 412: project_src_dir
                    );
                    
                    let instantiate_result = communication_actor.send(ExecuteCode {
                        code: instantiate_code,
                        execution_type: ExecutionType::ApiCall,
                        file_path: None,
                        suppress_busy_events: false,
                    }).await
                        .map_err(|e| format!("Failed to send instantiate code: {}", e))?
                        .map_err(|e| format!("Instantiate failed: {}", e))?;
                    
                    // Check if instantiate was successful
                    let instantiate_success = match instantiate_result {
                        JuliaMessage::ExecutionComplete { result: Some(result_str), success, .. } => {
                            success && result_str.trim() == "true"
                        }
                        JuliaMessage::ExecutionComplete { error: Some(error_msg), .. } => {
                            return Err(format!("Julia instantiate error: {}", error_msg));
                        }
                        _ => return Err("Unexpected response from Julia instantiate".to_string()),
                    };
                    
                    if !instantiate_success {
                        return Err("Failed to instantiate Julia project".to_string());
                    }
                    
                    debug!("ExecutionActor: Project activated and instantiated successfully: {}", project_path);
                    // Do NOT emit status update here - let the state machine handle status messages
                    // The state machine will transition from ActivatingProject to StartingLsp and emit the appropriate status
                    
                    Ok(())
                } else {
                    Err("Communication service not connected".to_string())
                }
            }
            .into_actor(self)
        )
    }
}

impl Handler<DeactivateProject> for ExecutionActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, _msg: DeactivateProject, _ctx: &mut Context<Self>) -> Self::Result {
        let communication_actor = self.communication_actor.clone();
        
        Box::pin(
            async move {
                debug!("ExecutionActor: Deactivating current project");
                
                // Check if connected via message
                let is_connected = communication_actor.send(IsConnected).await
                    .map_err(|e| format!("Failed to check connection: {}", e))?
                    .map_err(|e| format!("Connection check failed: {}", e))?;
                
                // Send deactivation command to Julia
                if is_connected {
                    let deactivation_code = r#"
                        try
                            using Pkg
                            Pkg.activate()
                            println("Project deactivated")
                            true
                        catch e
                            println("Failed to deactivate project: ", e)
                            false
                        end
                    "#;
                    
                    let result = communication_actor.send(ExecuteCode {
                        code: deactivation_code.to_string(),
                        execution_type: ExecutionType::ApiCall,
                        file_path: None,
                        suppress_busy_events: false,
                    }).await
                        .map_err(|e| format!("Failed to send deactivation code: {}", e))?
                        .map_err(|e| format!("Deactivation failed: {}", e))?;
                    
                    // Check if the result indicates success
                    match result {
                        JuliaMessage::ExecutionComplete { result: Some(result_str), success, .. } => {
                            if success && result_str.trim() == "true" {
                                debug!("ExecutionActor: Project deactivated successfully");
                                Ok(())
                            } else {
                                Err(format!("Julia deactivation failed: {}", result_str))
                            }
                        }
                        JuliaMessage::ExecutionComplete { error: Some(error_msg), .. } => {
                            Err(format!("Julia deactivation error: {}", error_msg))
                        }
                        _ => Err("Unexpected response from Julia deactivation".to_string()),
                    }
                } else {
                    debug!("ExecutionActor: Communication service not connected, skipping deactivation");
                    Ok(())
                }
            }
            .into_actor(self)
        )
    }
}


// Clone implementation for async operations
impl Clone for ExecutionActor {
    fn clone(&self) -> Self {
        Self {
            current_project: self.current_project.clone(),
            execution_queue: self.execution_queue.clone(),
            is_executing: self.is_executing,
            last_execution_result: self.last_execution_result.clone(),
            communication_actor: self.communication_actor.clone(),
            event_manager: self.event_manager.clone(),
        }
    }
}
