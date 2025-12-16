# Compute42 Debugger Handlers
# All message handler functions

# Message handlers
function handle_start_debug(data)
    try
        file_path = data["file_path"]
        code = get(data, "code", nothing)
        
        global JJ_DEBUG_SESSION_ACTIVE = true
        global JJ_CURRENT_FRAME = nothing
        
        # Send DebugStarted confirmation immediately
        send_debug_message(Dict(
            "DebugStarted" => Dict(
                "id" => get(data, "id", "unknown"),
                "file_path" => file_path,
                "success" => true
            )
        ))
        
        # If code is provided, use it; otherwise read from file
        if code !== nothing
            # Normalize line endings and parse
            normalized_code = replace(code, "\r\n" => "\n")
            
            # Try to parse as a block of statements
            try
                # Wrap in a block to handle multiple statements
                block_code = "begin\n$normalized_code\nend"
                ast = Meta.parse(block_code, raise=false)
                
                if ast isa Expr && ast.head == :error
                    error_msg = "Syntax error: $(ast.args[1])"
                    send_debug_message(Dict(
                        "DebugError" => Dict(
                            "id" => get(data, "id", "unknown"),
                            "error" => error_msg
                        )
                    ))
                    return
                end
                
                # Package loading will be handled in the execution block below
                
                # Set working directory for proper file resolution (before try block so it's available in catch)
                # Path should already be normalized by Rust before being sent to Julia
                project_dir = normpath(dirname(file_path))
                original_dir = pwd()
                
                # Use ExprSplitter to debug code expression by expression (like VSCode DebugAdapter)
                try
                    cd(project_dir)
                    
                    # Parse the code
                    ast = Meta.parse("begin\n$normalized_code\nend", raise=false)
                    
                    if ast isa Expr && ast.head == :error
                        error_msg = "Syntax error: $(ast.args[1])"
                        send_debug_message(Dict(
                            "DebugError" => Dict(
                                "id" => get(data, "id", "unknown"),
                                "error" => error_msg
                            )
                        ))
                        cd(original_dir)
                        return
                    end
                    
                    # Split code into lines to track line numbers
                    code_lines = split(normalized_code, '\n')
                    
                    # Track which line we're on in the source
                    current_source_line = 0
                    
                    # Use ExprSplitter to split code into top-level expressions
                    expr_splitter = JuliaInterpreter.ExprSplitter(Main, ast)
                    
                    # Execute each expression
                    println(stderr, "[DEBUGGER] Starting ExprSplitter loop for file: $file_path")
                    expr_count = 0
                    for (mod, ex) in expr_splitter
                        expr_count += 1
                        expr_str = string(ex)[1:min(100, length(string(ex)))]
                        println(stderr, "[DEBUGGER] Processing expression $expr_count: $expr_str")
                        
                        # Extract line number from the expression if available
                        # Julia expressions have line number nodes embedded in them
                        expr_line = nothing
                        if Meta.isexpr(ex, :block) && length(ex.args) > 0
                            # Find the first LineNumberNode in the block
                            for arg in ex.args
                                if arg isa LineNumberNode
                                    # Subtract 1 because we wrapped code in "begin\n...\nend" which adds a line at the start
                                    expr_line = arg.line - 1
                                    println(stderr, "[DEBUGGER] Expression $expr_count is at source line: $expr_line (raw line: $(arg.line))")
                                    break
                                end
                            end
                        end
                        
                        # Show current variables before execution
                        if expr_line !== nothing
                            current_vars = get_frame_variables(nothing)
                            var_names = [var_info["name"] for var_info in values(current_vars) if var_info isa Dict && haskey(var_info, "name")]
                            println(stderr, "[DEBUGGER] Before executing line $expr_line, available variables: $var_names")
                        end
                        
                        # Note: We check breakpoints AFTER execution so we can see variables created by the current expression
                        # This is important for expressions that create variables (like df = CSV.read(...))
                        
                        # Check if expression contains global declarations (top-level assignments)
                        # JuliaInterpreter can't handle :globaldecl expressions, so we need to detect and handle them
                        has_globaldecl = false
                        if Meta.isexpr(ex, :block)
                            # Check if any argument is a :globaldecl expression
                            for arg in ex.args
                                if Meta.isexpr(arg, :globaldecl)
                                    println(stderr, "[DEBUGGER] Found :globaldecl in block: $(string(arg)[1:min(100, length(string(arg)))])")
                                    has_globaldecl = true
                                    break
                                elseif Meta.isexpr(arg, :(=)) && length(arg.args) >= 1
                                    # Check if this is a top-level assignment (not inside a function)
                                    println(stderr, "[DEBUGGER] Found assignment in block: $(string(arg)[1:min(100, length(string(arg)))])")
                                    has_globaldecl = true
                                    break
                                end
                            end
                        elseif Meta.isexpr(ex, :globaldecl)
                            println(stderr, "[DEBUGGER] Expression is :globaldecl: $(string(ex)[1:min(100, length(string(ex)))])")
                            has_globaldecl = true
                        elseif Meta.isexpr(ex, :(=)) && length(ex.args) >= 1
                            println(stderr, "[DEBUGGER] Expression is assignment: $(string(ex)[1:min(100, length(string(ex)))])")
                            has_globaldecl = true
                        end
                        
                        # Handle special cases that should be evaluated directly
                        if has_globaldecl || Meta.isexpr(ex, :global, 1) || Meta.isexpr(ex, :using) || Meta.isexpr(ex, :import)
                            # Evaluate these directly at top level (JuliaInterpreter can't handle global declarations)
                            if has_globaldecl
                                println(stderr, "[DEBUGGER] Expression contains global declaration, evaluating directly (JuliaInterpreter limitation)")
                            end
                            Core.eval(mod, ex)
                            #println(stderr, "[DEBUGGER] Evaluated directly: $(string(ex)[1:min(50, length(string(ex)))])")
                            continue
                        end
                        
                        # Also handle include() calls directly
                        if Meta.isexpr(ex, :call) && length(ex.args) > 0 && ex.args[1] == :include
                            Core.eval(mod, ex)
                            #println(stderr, "[DEBUGGER] Included file: $(string(ex)[1:min(50, length(string(ex)))])")
                            continue
                        end
                        
                        # Create frame and execute through interpreter
                        try
                            println(stderr, "[DEBUGGER] Creating frame for expression $expr_count at line $expr_line...")
                            global JJ_CURRENT_FRAME = Base.invokelatest(JuliaInterpreter.Frame, mod, ex)
                            println(stderr, "[DEBUGGER] Frame created successfully")
                            
                            # Execute the frame
                            ret = Base.invokelatest(JuliaInterpreter.debug_command, JuliaInterpreter.RecursiveInterpreter(), JJ_CURRENT_FRAME, :c)
                            
                            if ret !== nothing
                                # Update current frame if execution paused
                                JJ_CURRENT_FRAME = ret[1]
                                println(stderr, "[DEBUGGER] Execution paused, frame updated")
                            else
                                println(stderr, "[DEBUGGER] Expression $expr_count at line $expr_line executed successfully")
                            end
                            
                            # Show variables after execution
                            if expr_line !== nothing
                                vars_after = get_frame_variables(nothing)
                                var_names_after = [var_info["name"] for var_info in values(vars_after) if var_info isa Dict && haskey(var_info, "name")]
                                println(stderr, "[DEBUGGER] After executing line $expr_line, available variables: $var_names_after")
                                
                                # Check for breakpoint at this line AFTER execution
                                if should_break_at_line(file_path, expr_line)
                                    println(stderr, "[DEBUGGER] ✓ Hit breakpoint at $file_path:$expr_line AFTER execution")
                                    
                                    # Get current variables from Main (filter out internal JJ_ and DEBUG_ variables)
                                    variables = get_frame_variables(nothing)
                                    var_names_at_breakpoint = [var_info["name"] for var_info in values(variables) if var_info isa Dict && haskey(var_info, "name")]
                                    println(stderr, "[DEBUGGER] Variables available at breakpoint: $var_names_at_breakpoint")
                                    stacktrace = []
                                    
                                    send_debug_message(Dict(
                                        "DebugStopped" => Dict(
                                            "id" => get(data, "id", "unknown"),
                                            "reason" => "breakpoint",
                                            "file_path" => file_path,
                                            "line" => expr_line,
                                            "variables" => variables,
                                            "stacktrace" => stacktrace
                                        )
                                    ))
                                    
                                    # Pause execution and wait for Continue command
                                    global JJ_DEBUG_PAUSED = true
                                    global JJ_DEBUG_CONTINUE_REQUESTED = false
                                    global JJ_LAST_STOP_REASON = "breakpoint"
                                    
                                    # Wait loop - check for continue signal
                                    marker_path = joinpath(tempdir(), "compute42_debug_continue.marker")
                                    wait_start_time = time()
                                    while JJ_DEBUG_PAUSED && !JJ_DEBUG_CONTINUE_REQUESTED
                                        if isfile(marker_path)
                                            try
                                                rm(marker_path)
                                                global JJ_DEBUG_CONTINUE_REQUESTED = true
                                            catch e
                                                # Ignore errors
                                            end
                                            break
                                        end
                                        sleep(0.1)
                                    end
                                    
                                    # Clear pause state
                                    global JJ_DEBUG_PAUSED = false
                                    global JJ_DEBUG_CONTINUE_REQUESTED = false
                                end
                            end
                        catch e
                            # Some expressions might fail to create frames, evaluate them directly
                            println(stderr, "[DEBUGGER] Frame creation failed, evaluating directly: $(sprint(showerror, e))")
                            try
                                result = Core.eval(mod, ex)
                                println(stderr, "[DEBUGGER] Direct eval completed, result type: $(typeof(result))")
                                
                                if expr_line !== nothing
                                    # Check if specific variables were created (like df at line 29)
                                    if expr_line == 29
                                        try
                                            # Direct check if df exists
                                            df_check = Base.eval(Main, :(isdefined(Main, :df)))
                                            println(stderr, "[DEBUGGER] Direct check: isdefined(Main, :df) = $df_check")
                                            if df_check
                                                df_type = Base.eval(Main, :(typeof(df)))
                                                println(stderr, "[DEBUGGER] Direct check: df type = $df_type")
                                                # Try to get df value to force it into namespace
                                                df_val = Base.eval(Main, :(df))
                                                println(stderr, "[DEBUGGER] Direct check: df value retrieved, size: $(size(df_val, 1))")
                                            end
                                        catch check_err
                                            println(stderr, "[DEBUGGER] Direct check failed: $(sprint(showerror, check_err))")
                                        end
                                    end
                                    
                                    # Force refresh by calling names() explicitly
                                    try
                                        all_names = Base.eval(Main, :(names(Main, all=true, imported=false)))
                                        println(stderr, "[DEBUGGER] Main.names() returned $(length(all_names)) names")
                                        if expr_line == 29
                                            df_in_names = :df in all_names
                                            println(stderr, "[DEBUGGER] :df in names(Main): $df_in_names")
                                        end
                                    catch names_err
                                        println(stderr, "[DEBUGGER] names() check failed: $(sprint(showerror, names_err))")
                                    end
                                    
                                    # Small delay to ensure variable is registered
                                    sleep(0.01)
                                    vars_after = get_frame_variables(nothing)
                                    var_names_after = [var_info["name"] for var_info in values(vars_after) if var_info isa Dict && haskey(var_info, "name")]
                                    println(stderr, "[DEBUGGER] After direct eval of line $expr_line, available variables: $var_names_after")
                                    
                                    # Check for breakpoint at this line AFTER execution
                                    if should_break_at_line(file_path, expr_line)
                                        println(stderr, "[DEBUGGER] ✓ Hit breakpoint at $file_path:$expr_line AFTER direct eval")
                                        
                                        # Get current variables from Main
                                        variables = get_frame_variables(nothing)
                                        var_names_at_breakpoint = [var_info["name"] for var_info in values(variables) if var_info isa Dict && haskey(var_info, "name")]
                                        println(stderr, "[DEBUGGER] Variables available at breakpoint: $var_names_at_breakpoint")
                                        stacktrace = []
                                        
                                        send_debug_message(Dict(
                                            "DebugStopped" => Dict(
                                                "id" => get(data, "id", "unknown"),
                                                "reason" => "breakpoint",
                                                "file_path" => file_path,
                                                "line" => expr_line,
                                                "variables" => variables,
                                                "stacktrace" => stacktrace
                                            )
                                        ))
                                        
                                        # Pause execution and wait for Continue command
                                        global JJ_DEBUG_PAUSED = true
                                        global JJ_DEBUG_CONTINUE_REQUESTED = false
                                        global JJ_LAST_STOP_REASON = "breakpoint"
                                        
                                        # Wait loop - check for continue signal
                                        marker_path = joinpath(tempdir(), "compute42_debug_continue.marker")
                                        wait_start_time = time()
                                        while JJ_DEBUG_PAUSED && !JJ_DEBUG_CONTINUE_REQUESTED
                                            if isfile(marker_path)
                                                try
                                                    rm(marker_path)
                                                    global JJ_DEBUG_CONTINUE_REQUESTED = true
                                                catch e
                                                    # Ignore errors
                                                end
                                                break
                                            end
                                            sleep(0.1)
                                        end
                                        
                                        # Clear pause state
                                        global JJ_DEBUG_PAUSED = false
                                        global JJ_DEBUG_CONTINUE_REQUESTED = false
                                    end
                                end
                            catch eval_err
                                println(stderr, "[DEBUGGER] Direct eval failed: $(sprint(showerror, eval_err))")
                                rethrow(eval_err)
                            end
                        end
                    end
                    
                    # Capture final variable state before ending session
                    final_variables = get_frame_variables(nothing)
                    
                    send_debug_message(Dict(
                        "DebugCompleted" => Dict(
                            "id" => get(data, "id", "unknown"),
                            "variables" => final_variables
                        )
                    ))
                    cd(original_dir)
                catch e
                    cd(original_dir)  # Restore directory on error
                    send_debug_message(Dict(
                        "DebugError" => Dict(
                            "id" => get(data, "id", "unknown"),
                            "error" => "Execution failed: $(sprint(showerror, e))"
                        )
                    ))
                    return
                end
            catch e
                # Restore directory if it was changed (original_dir is in scope here)
                try
                    cd(original_dir)
                catch
                    # Ignore errors restoring directory
                end
                error_msg = "Failed to create debug frame: $(sprint(showerror, e))"
                send_debug_message(Dict(
                    "DebugError" => Dict(
                        "id" => get(data, "id", "unknown"),
                        "error" => error_msg
                    )
                ))
                return
            end
        else
            # Read from file if no code provided
            try
                file_content = read(file_path, String)
                normalized_code = replace(file_content, "\r\n" => "\n")
                
                # Wrap in a block to handle multiple statements
                block_code = "begin\n$normalized_code\nend"
                ast = Meta.parse(block_code, raise=false)
                
                if ast isa Expr && ast.head == :error
                    error_msg = "Syntax error: $(ast.args[1])"
                    send_debug_message(Dict(
                        "DebugError" => Dict(
                            "id" => get(data, "id", "unknown"),
                            "error" => error_msg
                        )
                    ))
                    return
                end
                
                # Execute using and include statements normally first, then debug the rest
                try
                    # First, set working directory for proper file resolution
                    # Path should already be normalized by Rust before being sent to Julia
                    project_dir = normpath(dirname(file_path))
                    original_dir = pwd()
                    cd(project_dir)
                    
                    # Execute using and include statements normally (not through JuliaInterpreter)
                    using_lines = String[]
                    include_lines = String[]
                    other_lines = String[]
                    
                    for line in split(normalized_code, '\n')
                        stripped = strip(line)
                        if startswith(stripped, "using ")
                            push!(using_lines, line)
                        elseif startswith(stripped, "include(")
                            push!(include_lines, line)
                        elseif !isempty(stripped) && !startswith(stripped, "#")
                            push!(other_lines, line)
                        end
                    end
                    
                    # Execute using statements in Main module normally
                    for line in using_lines
                        try
                            eval(Meta.parse(line))
                        catch e
                            # Failed to load - continue
                        end
                    end
                    
                    # Execute include statements normally (they need top-level execution)
                    for line in include_lines
                        try
                            eval(Meta.parse(line))
                        catch e
                            # Failed to include - continue
                        end
                    end
                    
                    if !isempty(other_lines)
                        # Execute remaining code normally instead of through debugger
                        # This avoids issues with macros like @__DIR__ and other top-level constructs
                        other_code = join(other_lines, '\n')
                        
                        try
                            # Execute the rest of the code normally at top level
                            eval(Meta.parse(other_code))
                            
                            # Capture final variable state before ending session
                            final_variables = get_frame_variables(nothing)
                            
                            send_debug_message(Dict(
                                "DebugCompleted" => Dict(
                                    "id" => get(data, "id", "unknown"),
                                    "variables" => final_variables
                                )
                            ))
                        catch e
                            send_debug_message(Dict(
                                "DebugError" => Dict(
                                    "id" => get(data, "id", "unknown"),
                                    "error" => "File execution failed: $(sprint(showerror, e))"
                                )
                            ))
                        finally
                            # Restore original directory after execution
                            cd(original_dir)
                        end
                    else
                        # Only using/include statements, no actual code to debug
                        # Capture final variable state before ending session
                        final_variables = get_frame_variables(nothing)
                        
                        send_debug_message(Dict(
                            "DebugCompleted" => Dict(
                                "id" => get(data, "id", "unknown"),
                                "variables" => final_variables
                            )
                        ))
                    end
                catch e
                    send_debug_message(Dict(
                        "DebugError" => Dict(
                            "id" => get(data, "id", "unknown"),
                            "error" => "File execution failed: $(sprint(showerror, e))"
                        )
                    ))
                    return
                end
            catch e
                error_msg = sprint(showerror, e, catch_backtrace())
                send_debug_message(Dict(
                    "DebugError" => Dict(
                        "id" => get(data, "id", "unknown"),
                        "error" => error_msg
                    )
                ))
            end
        end
    catch e
        error_msg = sprint(showerror, e, catch_backtrace())
        send_debug_message(Dict(
            "DebugError" => Dict(
                "id" => get(data, "id", "unknown"),
                "error" => error_msg
            )
        ))
    end
end

function handle_set_breakpoint(data)
    try
        file_path = data["file_path"]
        line = data["line"]
        
        println(stderr, "[DEBUGGER] Setting breakpoint at $file_path:$line")
        
        # Store breakpoint
        if !haskey(JJ_BREAKPOINTS, file_path)
            JJ_BREAKPOINTS[file_path] = Set{Int}()
        end
        push!(JJ_BREAKPOINTS[file_path], line)
        
        println(stderr, "[DEBUGGER] Breakpoint stored. Current breakpoints for $file_path: $(collect(JJ_BREAKPOINTS[file_path]))")
        
        # Set breakpoint in JuliaInterpreter
        # Note: JuliaInterpreter uses method-based breakpoints, so we need to handle this differently
        # For now, we'll use line-based tracking and check during stepping
        
        send_debug_message(Dict(
            "BreakpointSet" => Dict(
                "id" => get(data, "id", "unknown"),
                "file_path" => file_path,
                "line" => line,
                "success" => true
            )
        ))
    catch e
        error_msg = sprint(showerror, e, catch_backtrace())
        send_debug_message(Dict(
            "DebugError" => Dict(
                "id" => get(data, "id", "unknown"),
                "error" => error_msg
            )
        ))
    end
end

function handle_continue(data)
    try
        # Check if we're actually paused
        if !JJ_DEBUG_PAUSED
            send_debug_message(Dict(
                "DebugError" => Dict(
                    "id" => get(data, "id", "unknown"),
                    "error" => "Debugger is not paused"
                )
            ))
            return
        end
        
        # Set the continue flag to resume execution
        global JJ_DEBUG_CONTINUE_REQUESTED = true
        
        # Send confirmation
        send_debug_message(Dict(
            "DebugMessageResponse" => Dict(
                "id" => get(data, "id", "unknown"),
                "response" => "continue_acknowledged",
                "success" => true
            )
        ))
    catch e
        error_msg = sprint(showerror, e, catch_backtrace())
        send_debug_message(Dict(
            "DebugError" => Dict(
                "id" => get(data, "id", "unknown"),
                "error" => error_msg
            )
        ))
    end
end

function handle_step_in(data)
    try
        if JJ_CURRENT_FRAME === nothing
            send_debug_message(Dict(
                "DebugError" => Dict(
                    "id" => get(data, "id", "unknown"),
                    "error" => "No active debug frame"
                )
            ))
            return
        end
        
        # Step into next expression
        JuliaInterpreter.step_expr!(JJ_CURRENT_FRAME)
        
        # Get current state
        method_info = JuliaInterpreter.framecode(JJ_CURRENT_FRAME)
        file_path = string(method_info.scope.file)
        line_number = JJ_CURRENT_FRAME.pc[1]
        variables = get_frame_variables(JJ_CURRENT_FRAME)
        stacktrace = get_stacktrace(JJ_CURRENT_FRAME)
        
        send_debug_message(Dict(
            "DebugStopped" => Dict(
                "id" => get(data, "id", "unknown"),
                "reason" => "step",
                "file_path" => file_path,
                "line" => line_number,
                "variables" => variables,
                "stacktrace" => stacktrace
            )
        ))
    catch e
        error_msg = sprint(showerror, e, catch_backtrace())
        send_debug_message(Dict(
            "DebugError" => Dict(
                "id" => get(data, "id", "unknown"),
                "error" => error_msg
            )
        ))
    end
end

function handle_step_over(data)
    try
        if JJ_CURRENT_FRAME === nothing
            send_debug_message(Dict(
                "DebugError" => Dict(
                    "id" => get(data, "id", "unknown"),
                    "error" => "No active debug frame"
                )
            ))
            return
        end
        
        # Step over (next) - step to next line at same level
        initial_depth = 0
        current = JJ_CURRENT_FRAME
        while current !== nothing
            initial_depth += 1
            current = current.caller
        end
        
        # Step until we're back at the same depth
        while true
            JuliaInterpreter.step_expr!(JJ_CURRENT_FRAME)
            
            current_depth = 0
            current = JJ_CURRENT_FRAME
            while current !== nothing
                current_depth += 1
                current = current.caller
            end
            
            if current_depth <= initial_depth
                break
            end
        end
        
        # Get current state
        method_info = JuliaInterpreter.framecode(JJ_CURRENT_FRAME)
        file_path = string(method_info.scope.file)
        line_number = JJ_CURRENT_FRAME.pc[1]
        variables = get_frame_variables(JJ_CURRENT_FRAME)
        stacktrace = get_stacktrace(JJ_CURRENT_FRAME)
        
        send_debug_message(Dict(
            "DebugStopped" => Dict(
                "id" => get(data, "id", "unknown"),
                "reason" => "step",
                "file_path" => file_path,
                "line" => line_number,
                "variables" => variables,
                "stacktrace" => stacktrace
            )
        ))
    catch e
        error_msg = sprint(showerror, e, catch_backtrace())
        send_debug_message(Dict(
            "DebugError" => Dict(
                "id" => get(data, "id", "unknown"),
                "error" => error_msg
            )
        ))
    end
end

function handle_step_out(data)
    try
        if JJ_CURRENT_FRAME === nothing
            send_debug_message(Dict(
                "DebugError" => Dict(
                    "id" => get(data, "id", "unknown"),
                    "error" => "No active debug frame"
                )
            ))
            return
        end
        
        # Step out - finish current frame and return to caller
        if JJ_CURRENT_FRAME.caller !== nothing
            JuliaInterpreter.finish!(JJ_CURRENT_FRAME)
            global JJ_CURRENT_FRAME = JJ_CURRENT_FRAME.caller
            
            # Get current state
            method_info = JuliaInterpreter.framecode(JJ_CURRENT_FRAME)
            file_path = string(method_info.scope.file)
            line_number = JJ_CURRENT_FRAME.pc[1]
            variables = get_frame_variables(JJ_CURRENT_FRAME)
            stacktrace = get_stacktrace(JJ_CURRENT_FRAME)
            
            send_debug_message(Dict(
                "DebugStopped" => Dict(
                    "id" => get(data, "id", "unknown"),
                    "reason" => "step",
                    "file_path" => file_path,
                    "line" => line_number,
                    "variables" => variables,
                    "stacktrace" => stacktrace
                )
            ))
        else
            # No caller, execution completed
            # Capture final variable state before ending session
            final_variables = get_frame_variables(nothing)
            
            send_debug_message(Dict(
                "DebugCompleted" => Dict(
                    "id" => get(data, "id", "unknown"),
                    "variables" => final_variables
                )
            ))
        end
    catch e
        error_msg = sprint(showerror, e, catch_backtrace())
        send_debug_message(Dict(
            "DebugError" => Dict(
                "id" => get(data, "id", "unknown"),
                "error" => error_msg
            )
        ))
    end
end

function handle_get_variables(data)
    try
        variables = get_frame_variables(JJ_CURRENT_FRAME)
        
        send_debug_message(Dict(
            "DebugVariables" => Dict(
                "id" => get(data, "id", "unknown"),
                "variables" => variables
            )
        ))
    catch e
        error_msg = sprint(showerror, e, catch_backtrace())
        send_debug_message(Dict(
            "DebugError" => Dict(
                "id" => get(data, "id", "unknown"),
                "error" => error_msg
            )
        ))
    end
end

function handle_get_stacktrace(data)
    try
        stacktrace_data = get_stacktrace(JJ_CURRENT_FRAME)
        
        send_debug_message(Dict(
            "DebugStacktrace" => Dict(
                "id" => get(data, "id", "unknown"),
                "stacktrace" => stacktrace_data
            )
        ))
    catch e
        error_msg = sprint(showerror, e, catch_backtrace())
        send_debug_message(Dict(
            "DebugError" => Dict(
                "id" => get(data, "id", "unknown"),
                "error" => error_msg
            )
        ))
    end
end

function handle_stop_debug(data)
    try
        # Clear all debug state
        global JJ_DEBUG_SESSION_ACTIVE = false
        global JJ_CURRENT_FRAME = nothing
        global JJ_BREAKPOINTS = Dict{String, Set{Int}}()
        global JJ_DEBUG_PAUSED = false
        global JJ_DEBUG_CONTINUE_REQUESTED = true  # Unblock any waiting loops
        global JJ_LAST_STOP_REASON = nothing
        
        send_debug_message(Dict(
            "DebugStopped" => Dict(
                "id" => get(data, "id", "unknown"),
                "reason" => "terminated"
            )
        ))
    catch e
        error_msg = sprint(showerror, e, catch_backtrace())
        send_debug_message(Dict(
            "DebugError" => Dict(
                "id" => get(data, "id", "unknown"),
                "error" => error_msg
            )
        ))
    end
end




















