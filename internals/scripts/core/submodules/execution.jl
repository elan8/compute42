# Compute42 Code Execution Handlers

# Handle new CodeExecution requests
function handle_code_execution(data)
    # Store execution_type early for use in error handler
    local execution_type = nothing
    local id = nothing
    local code = nothing
    
    try
        # Extract required fields using new JSON utilities
        code = extract_required_string(data, "code")
        execution_type = extract_required_string(data, "execution_type")
        id = extract_required_string(data, "id")

        # Log execution start
        start_time = time()
        code_length = length(code)
        # println(stderr, "Compute42: [EXEC] Starting $(execution_type) - ID: $(id), Code length: $(code_length) bytes")
        
        # For FileExecution, also log the first few lines for context
        if execution_type == "FileExecution"
            code_lines = split(code, '\n')
            num_lines = length(code_lines)
            preview_lines = min(3, num_lines)
            if preview_lines > 0
                preview = join(code_lines[1:preview_lines], '\n')
                # println(stderr, "Compute42: [EXEC] FileExecution preview (first $(preview_lines)/$(num_lines) lines):")
                # println(stderr, preview)
            end
        end

        # Setup display system before executing code
        Main.setup_display_system()

        # Check if this is notebook cell execution
        is_notebook_cell = startswith(execution_type, "notebook_cell:")
        cell_id = nothing
        if is_notebook_cell
            cell_id = split(execution_type, ":", limit=2)[2]
        end

        # Execute the code and capture the result
        # For multi-line code (FileExecution or ReplExecution with newlines), use include_string
        # For single-line REPL input, use parse_input_line + eval
        use_include_string = execution_type == "FileExecution" || contains(code, '\n')
        
        if use_include_string
            # Use include_string for multi-line code blocks (handles all syntax properly)
            # println(stderr, "Compute42: [EXEC] Executing multi-line code with Base.include_string...")
            flush(stdout)
            
            result = Base.include_string(Main, code, "code_execution")
            
            flush(stdout)
            # println(stderr, "Compute42: [EXEC] Multi-line execution completed successfully")
        else
            # For single-line REPL input, use parse + eval
            ast = Base.parse_input_line(code, depwarn=false)

            # Handle empty input
            if ast === nothing
                result = nothing
            elseif isa(ast, Expr) && ast.head === :incomplete
                error("syntax: incomplete: premature end of input")
            elseif isa(ast, Expr) && ast.head === :error
                error("syntax: $(ast.args[1])")
            else
                # Note: Debug execution now goes through Julia Debug Adapter, not main process
                if execution_type == "DebugExecution"
                    error("Debug execution should use Julia Debug Adapter, not main Julia process")
                else
                    # For ReplExecution, use Core.eval for single expressions
                    # println(stderr, "Compute42: [EXEC] Executing REPL expression with Core.eval...")
                    flush(stdout)
                    result = Core.eval(Main, ast)
                    flush(stdout)
                    # println(stderr, "Compute42: [EXEC] REPL execution completed successfully")
                end
            end
        end

        # For ReplExecution, print the result to stdout so it appears in the REPL
        if execution_type == "ReplExecution"
            try
                # Use invokelatest to handle world age issues (e.g., with PooledArrays in DataFrames)
                Base.invokelatest(println, stdout, result)
            catch e
                # If printing fails due to world age or other issues, try a safer approach
                try
                    # Try using show instead, which is more robust
                    Base.invokelatest(show, stdout, result)
                    println(stdout)  # Add newline after show
                catch e2
                    # Last resort: print type and error message
                    println(stderr, "Compute42: Failed to print result: ", sprint(showerror, e2))
                    println(stdout, typeof(result), " (printing failed)")
                end
            end
        end

        # For FileExecution, add an empty println to ensure any buffered print() output is flushed
        if execution_type == "FileExecution"
            println()  # This ensures any buffered print() output is flushed
            flush(stdout)  # Ensure the newline is flushed immediately
        end

        # CRITICAL: Automatically display the result to capture plots (like VS Code does)
        # This is what makes plot(x, y) work without needing display(plot(x, y))
        # BUT: Don't display data structures returned from API calls (like workspace variables)
        if result !== nothing && !(result isa Exception)
            # Don't display data structures that are meant to be returned as data
            should_display = true
            
            # Don't display dictionaries (like workspace variables)
            if isa(result, Dict)
                should_display = false
            end
            
            # Don't display arrays of data (like variable lists)
            if isa(result, AbstractArray) && !Main.can_display_result(result)
                should_display = false
            end
            
            if should_display
                try
                    # Try to display the result directly (like VS Code does)
                    # Use invokelatest to ensure we get the most recent display method
                    Base.invokelatest(display, result)
                catch e
                    # If display fails, don't crash the execution
                    println(stderr, "Compute42: Display failed: ", sprint(showerror, e))
                end
            end
        end

        # CRITICAL: Force flush of all output buffers before completion
        flush(stdout)
        flush(stderr)

        # Calculate execution duration
        end_time = time()
        duration_seconds = end_time - start_time
        duration_ms = round(Int, duration_seconds * 1000)

        # Send the result back to Rust using the new message format
        # Use safe string conversion to avoid method age issues with plot objects
        result_string = try
            if result === nothing
                "nothing"
            elseif result isa Exception
                string(result)
            else
                # For displayable objects like plots, just indicate the type
                if Main.can_display_result(result)
                    "$(typeof(result)) (displayed)"
                else
                    string(result)
                end
            end
        catch e
            # If string conversion fails, just show the type
            "$(typeof(result)) (string conversion failed)"
        end

        # Log execution completion with timing
        # println(stderr, "Compute42: [EXEC] Completed $(execution_type) - ID: $(id), Duration: $(duration_ms)ms, Success: true")

        # Ensure execution_type is preserved correctly (especially for notebook cells)
        # execution_type should already be in the correct format from extraction
        response = Dict(
            "ExecutionComplete" => Dict(
                "id" => id,
                "execution_type" => execution_type,  # This should be "notebook_cell:{cell_id}" for notebook cells
                "result" => result_string,
                "error" => nothing,
                "success" => true,
                "duration_ms" => duration_ms,
                "timestamp" => round(Int, end_time),
                "metadata" => nothing
            )
        )
        send_message_to_backend(response)

    catch e
        # Calculate execution duration even on error (use try-catch to handle cases where start_time might not be set)
        end_time = time()
        duration_ms = try
            duration_seconds = end_time - start_time
            round(Int, duration_seconds * 1000)
        catch
            # If start_time wasn't set, we can't calculate duration
            0
        end

        # Capture full error information including stacktrace
        error_info = sprint(showerror, e, catch_backtrace())

        # Get execution type and ID safely (with fallbacks for error cases)
        # Use the stored values if available, otherwise try to extract from data
        if execution_type === nothing
            try
                data_dict = isa(data, JSON.Object) ? Dict{String, Any}(data) : data
                if haskey(data_dict, "execution_type")
                    execution_type = extract_string(data_dict["execution_type"])
                end
            catch
                # If extraction fails, use default
            end
            if execution_type === nothing
                execution_type = "Unknown"
            end
        end
        
        if id === nothing
            try
                data_dict = isa(data, JSON.Object) ? Dict{String, Any}(data) : data
                if haskey(data_dict, "id")
                    id = extract_string(data_dict["id"])
                end
            catch
                # If extraction fails, use default
            end
            if id === nothing
                id = "unknown"
            end
        end
        
        execution_id = id

        # Print clean error output (without [EXEC] prefix)
        println(stderr, error_info)

        # CRITICAL: Force flush of error output before completion
        flush(stderr)

        # Send error response with clean error information
        response = Dict(
            "ExecutionComplete" => Dict(
                "id" => execution_id,
                "execution_type" => execution_type,
                "result" => nothing,
                "error" => error_info,
                "success" => false,
                "duration_ms" => duration_ms,
                "timestamp" => round(Int, end_time),
                "metadata" => nothing
            )
        )
        send_message_to_backend(response)
    end
end

# Handle new ApiRequest requests
function handle_api_request(data)
    try
        # Extract required fields using new JSON utilities
        code = extract_required_string(data, "code")
        id = extract_required_string(data, "id")

        # Setup display system before executing code
        Main.setup_display_system()

        # Execute the code and capture the result
        ast = Base.parse_input_line(code, depwarn=false)

        # Handle empty input
        if ast === nothing
            result = nothing
        elseif isa(ast, Expr) && ast.head === :incomplete
            error("syntax: incomplete: premature end of input")
        elseif isa(ast, Expr) && ast.head === :error
            error("syntax: $(ast.args[1])")
        else
            result = Core.eval(Main, ast)
        end

        # CRITICAL: Automatically display the result to capture plots (like VS Code does)
        # This is what makes plot(x, y) work without needing display(plot(x, y))
        # BUT: Don't display data structures returned from API calls (like workspace variables)
        if result !== nothing && !(result isa Exception)
            # Don't display data structures that are meant to be returned as data
            should_display = true
            
            # Don't display dictionaries (like workspace variables)
            if isa(result, Dict)
                should_display = false
            end
            
            # Don't display arrays of data (like variable lists)
            if isa(result, AbstractArray) && !can_display_result(result)
                should_display = false
            end
            
            if should_display
                try
                    # Try to display the result directly (like VS Code does)
                    # Use invokelatest to ensure we get the most recent display method
                    Base.invokelatest(display, result)
                catch e
                    # If display fails, don't crash the execution
                    println(stderr, "Compute42: Display failed: ", sprint(showerror, e))
                end
            end
        end

        # Check if this is a workspace variables request
        if code == "get_workspace_variables()"
            # Send workspace variables message instead of execution complete
            variables = get_workspace_variables()
            response = Dict(
                "WorkspaceVariables" => Dict(
                    "id" => id,
                    "variables" => variables
                )
            )
            send_message_to_backend(response)
        else
            # Send the result back to Rust
            # Use safe string conversion to avoid method age issues with plot objects
            result_string = try
                if result === nothing
                    "nothing"
                elseif result isa Exception
                    string(result)
                else
                    # For displayable objects like plots, just indicate the type
                    if Main.can_display_result(result)
                        "$(typeof(result)) (displayed)"
                    else
                        string(result)
                    end
                end
            catch e
                # If string conversion fails, just show the type
                "$(typeof(result)) (string conversion failed)"
            end

            response = Dict(
                "ExecutionComplete" => Dict(
                    "id" => id,
                    "execution_type" => "api_call",
                    "result" => result_string,
                    "error" => nothing,
                    "success" => true,
                    "duration_ms" => nothing,
                    "timestamp" => round(Int, time()),
                    "metadata" => nothing
                )
            )
            send_message_to_backend(response)
        end

    catch e
        error_info = sprint(showerror, e, catch_backtrace())
        println(stderr, error_info)
        flush(stderr)

        # Extract id safely for error response
        id = extract_optional_string(data, "id")
        if id === nothing
            id = "unknown"
        end
        
        response = Dict(
            "ExecutionComplete" => Dict(
                "id" => id,
                "execution_type" => "api_call",
                "result" => nothing,
                "error" => error_info,
                "success" => false,
                "duration_ms" => nothing,
                "timestamp" => round(Int, time()),
                "metadata" => nothing
            )
        )
        send_message_to_backend(response)
    end
end

# Handle ConnectionTest requests
function handle_connection_test(data)
    try
        # Extract required fields using new JSON utilities
        id = extract_required_string(data, "id")
        message = extract_required_string(data, "message")
        timestamp = data["timestamp"]

        # Send connection test response back to Rust
        response = Dict(
            "ConnectionTestResponse" => Dict(
                "id" => id,
                "response" => "Hello Rust! Connection test successful from Julia.",
                "timestamp" => timestamp
            )
        )
        send_message_to_backend(response)

    catch e
        println(stderr, "Compute42: Error handling connection test: ", sprint(showerror, e))

        # Send error response (with safe field extraction)
        id = extract_optional_string(data, "id")
        if id === nothing
            id = "unknown"
        end
        timestamp = get(data, "timestamp", round(Int, time()))
        
        response = Dict(
            "ConnectionTestResponse" => Dict(
                "id" => id,
                "response" => "Error: " * string(e),
                "timestamp" => timestamp
            )
        )
        send_message_to_backend(response)
    end
end

