# Compute42 Debugger Inspection
# Variable and stacktrace inspection utilities

# Get variables from current frame or from Main module
function get_frame_variables(frame)
    variables = Dict{String, Any}()
    
    # If we have a frame, get variables from it
    if frame !== nothing
        try
            # Get local variables from the frame
            frame_locals = JuliaInterpreter.locals(frame)
            
            # Check if frame_locals is a dict/iterable or a single Variable
            if frame_locals isa JuliaInterpreter.Variable
                # Single variable - get the slot and value
                var_name_str = string(frame_locals.name)
                if !startswith(var_name_str, "#")
                    try
                        var_value = frame_locals.value
                        value_type = typeof(var_value)
                        type_str = string(value_type)
                        
                        # Get string representation
                        value_str = try
                            repr(var_value)
                        catch
                            string(var_value)
                        end
                        
                        # Truncate long values
                        if length(value_str) > 1000
                            value_str = value_str[1:1000] * "..."
                        end
                        
                        variables[var_name_str] = Dict(
                            "type" => type_str,
                            "value" => value_str
                        )
                    catch e
                        # Error formatting single variable - skip it
                    end
                end
            elseif frame_locals isa AbstractDict
                # Dictionary of variables - iterate over key-value pairs
                try
                    for (var_name, var_value) in frame_locals
                        var_name_str = string(var_name)
                        
                        # Skip internal variables
                        if startswith(var_name_str, "#")
                            continue
                        end
                        
                        try
                            value_type = typeof(var_value)
                            type_str = string(value_type)
                            
                            # Get string representation
                            value_str = try
                                repr(var_value)
                            catch
                                string(var_value)
                            end
                            
                            # Truncate long values
                            if length(value_str) > 200
                                value_str = value_str[1:200] * "..."
                            end
                            
                            variables[var_name_str] = Dict(
                                "name" => var_name_str,
                                "type" => type_str,
                                "value" => value_str,
                                "is_expandable" => false
                            )
                        catch e
                            # Error formatting variable - skip it
                        end
                    end
                catch e
                    # Error iterating frame_locals dict - skip it
                end
            elseif frame_locals isa AbstractVector || frame_locals isa Tuple
                # Vector/Tuple of Variable objects - iterate over elements
                try
                    for var in frame_locals
                        if var isa JuliaInterpreter.Variable
                            var_name_str = string(var.name)
                            
                            # Skip internal variables
                            if startswith(var_name_str, "#")
                                continue
                            end
                            
                            try
                                var_value = var.value
                                value_type = typeof(var_value)
                                type_str = string(value_type)
                                
                                # Get string representation
                                value_str = try
                                    repr(var_value)
                                catch
                                    string(var_value)
                                end
                                
                                # Truncate long values
                                if length(value_str) > 200
                                    value_str = value_str[1:200] * "..."
                                end
                                
                                variables[var_name_str] = Dict(
                                    "name" => var_name_str,
                                    "type" => type_str,
                                    "value" => value_str,
                                    "is_expandable" => false
                                )
                            catch e
                                # Error formatting variable - skip it
                            end
                        end
                    end
                catch e
                    # Error iterating frame_locals vector - skip it
                end
            else
                # Unexpected type for frame_locals - skip it
            end
        catch e
            # Error getting frame variables - return empty dict
        end
    else
        # No frame available, get variables from Main module
        try
            # Get all names in Main module
            # CRITICAL: In Julia 1.12, we must use Base.eval() to execute names() in Main scope
            # Direct call to names() doesn't see variables created by Base.include_string()
            # but Base.eval(Main, :(names(...))) does!
            all_names = try
                Base.eval(Main, :(names(Main, all=true, imported=false)))
            catch e
                # Fallback to direct call if eval fails
                println(stderr, "Warning: Base.eval() failed in get_frame_variables, using direct names() call: $e")
                names(Main, all=true, imported=false)
            end
            
            for name in all_names
                # Skip internal and module names
                name_str = string(name)
                
                # Filter out Compute42 internal variables and Julia internals
                # All Compute42 internal variables are prefixed with JJ_ for easy filtering
                if startswith(name_str, "#") ||              # Julia compiler internals
                   startswith(name_str, "_") ||              # Julia internals
                   startswith(name_str, "JJ_") ||            # Compute42 internal (debug state, plot config, etc.)
                   startswith(name_str, "jj_") ||            # Compute42 internal (lowercase)
                   name_str == "JJ_CURRENT_FRAME" ||            # Debugger frame
                   name_str == "JJ_BREAKPOINTS" ||              # Debugger breakpoints
                   name_str == "JJ_LAST_STOP_REASON" ||         # Debugger state
                   name_str == "Compute42Display" ||     # Custom display system
                   name == :Main ||                          # Main module
                   name == :Core ||                          # Core module
                   name == :Base                             # Base module
                    continue
                end
                
                try
                    # Try to access the variable - use eval since isdefined() may not work in Julia 1.12
                    # Variables created by Base.include_string may not be visible to isdefined() but are accessible via eval
                    var_value = nothing
                    var_accessible = false
                    try
                        # First try eval to see if variable is accessible (works in Julia 1.12)
                        var_value = Base.eval(Main, name)
                        var_accessible = true
                    catch e
                        # If eval fails, try isdefined as fallback
                        try
                            if isdefined(Main, name)
                                var_value = getfield(Main, name)
                                var_accessible = true
                            end
                        catch
                            # Variable not accessible via either method
                            continue
                        end
                    end
                    
                    if !var_accessible || var_value === nothing
                        continue
                    end
                    
                    # Skip functions, modules, and types
                    if !(var_value isa Function || var_value isa Module || var_value isa Type)
                        value_type = typeof(var_value)
                        type_str = string(value_type)
                        
                        # Get string representation - use invokelatest for methods that might have world age issues
                        # This is especially important for DataFrames and other complex types
                        value_str = try
                            # For DataFrames and complex types, use invokelatest to handle world age issues
                            if type_str == "DataFrame" || occursin("DataFrame", type_str)
                                # DataFrame methods might be too new - use invokelatest
                                Base.invokelatest(repr, var_value)
                            else
                                # For simple types, try regular repr first
                                repr(var_value)
                            end
                        catch repr_err
                            # If repr fails, try string() as fallback
                            try
                                if type_str == "DataFrame" || occursin("DataFrame", type_str)
                                    Base.invokelatest(string, var_value)
                                else
                                    string(var_value)
                                end
                            catch string_err
                                # If both fail, use type name
                                type_str
                            end
                        end
                        
                        # Truncate long values
                        if length(value_str) > 200
                            value_str = value_str[1:200] * "..."
                        end
                        
                        variables[name_str] = Dict(
                            "name" => name_str,
                            "type" => type_str,
                            "value" => value_str,
                            "is_expandable" => false
                        )
                    end
                catch e
                    # Log which variable failed to be accessed (for debugging)
                    if name_str == "df"
                        println(stderr, "[DEBUGGER] get_frame_variables: Failed to access variable 'df': $(sprint(showerror, e))")
                    end
                    # Skip variables that can't be accessed
                    continue
                end
            end
        catch e
            # Error getting Main module variables - return empty dict
        end
    end
    
    return variables
end

# Get stacktrace from current frame
function get_stacktrace(frame)
    stacktrace_data = []
    
    if frame === nothing
        return stacktrace_data
    end
    
    try
        current = frame
        depth = 0
        
        while current !== nothing && depth < 100  # Limit depth
            try
                # Get frame information
                method_info = JuliaInterpreter.framecode(current)
                file_path = string(method_info.scope.file)
                line_number = current.pc[1]
                
                # Get function name
                func_name = string(method_info.scope.name)
                
                push!(stacktrace_data, Dict(
                    "depth" => depth,
                    "file" => file_path,
                    "line" => line_number,
                    "function" => func_name
                ))
                
                depth += 1
                current = current.caller
            catch e
                break
            end
        end
    catch e
        # Error getting stacktrace - return empty
    end
    
    return stacktrace_data
end

# Helper function to check if we should break at a specific line
function should_break_at_line(file_path::String, line::Int)
    if haskey(JJ_BREAKPOINTS, file_path)
        should_break = line in JJ_BREAKPOINTS[file_path]
        if should_break
            println(stderr, "[DEBUGGER] should_break_at_line: MATCH! line $line is in breakpoints $(collect(JJ_BREAKPOINTS[file_path]))")
        else
            println(stderr, "[DEBUGGER] should_break_at_line: NO MATCH. line $line not in breakpoints $(collect(JJ_BREAKPOINTS[file_path]))")
        end
        return should_break
    end
    println(stderr, "[DEBUGGER] should_break_at_line: No breakpoints registered for $file_path")
    return false
end




















