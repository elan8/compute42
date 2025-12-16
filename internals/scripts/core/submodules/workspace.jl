# Compute42 Workspace Variable Inspection

# Get workspace variables from Main module
function get_workspace_variables()
    try
        variables = Dict{String, Any}()
        
        # Get all names in Main module
        # CRITICAL: In Julia 1.12, we must use Base.eval() to execute names() in Main scope
        # Direct call to names() doesn't see variables created by Base.include_string()
        # but Base.eval(Main, :(names(...))) does!
        all_names = try
            Base.eval(Main, :(names(Main, all=true, imported=false)))
        catch e
            # Fallback to direct call if eval fails
            println(stderr, "Warning: Base.eval() failed, using direct names() call: $e")
            names(Main, all=true, imported=false)
        end
        
        # In Julia 1.12, variables created by Base.include_string() may not appear in names()
        # but are accessible via Base.eval(). We need to also try to discover variables
        # by attempting to eval common variable names or by using other discovery methods.
        # For now, we'll process names() first, then add a fallback to try common names.
        
        # Track which names we've already processed
        processed_names = Set{Symbol}()
        
        # Built-in modules and standard names to filter out
        builtin_names = Set([
            :Base, :Core, :Main, :InteractiveUtils, :REPL, :Pkg,
            :ans, :err, :stdout, :stderr, :stdin,
            :Compute42Display, :JJ_PLOT_PANE_ENABLED, :JJ_DISPLAYABLE_MIMES,
            :DEBUGGER_AVAILABLE, :varinfo, :whos,
            # Filter out functions we've defined in Main
            :should_filter_plot_data, :setup_display_system, :can_display_result, :is_table_like
        ])
        
        for name in all_names
            name_str = string(name)
            
            # Filter out Compute42 internal variables and Julia internals
            # All Compute42 internal variables are prefixed with JJ_ for easy filtering
            if startswith(name_str, "#") ||              # Julia compiler internals
               startswith(name_str, "_") ||              # Julia internals
               startswith(name_str, "JJ_") ||            # Compute42 internal (all internal variables)
               startswith(name_str, "jj_") ||            # Compute42 internal (lowercase)
               name_str == "Compute42Display" ||     # Custom display system
               name in builtin_names                     # Built-in modules and functions
                continue
            end
            
            # Try to access the variable - use eval since isdefined() may not work in Julia 1.12
            # Variables created by Base.include_string may not be visible to isdefined() but are accessible via eval
            value = nothing
            var_accessible = false
            try
                # First try eval to see if variable is accessible (works in Julia 1.12)
                value = Base.eval(Main, name)
                var_accessible = true
            catch e
                # If eval fails, try isdefined as fallback
                try
                    if isdefined(Main, name)
                        value = getfield(Main, name)
                        var_accessible = true
                    end
                catch
                    # Variable not accessible via either method
                    continue
                end
            end
            
            if !var_accessible || value === nothing
                continue
            end
            
            try
                
                # Skip modules, functions, and types
                if isa(value, Module)
                    continue
                end
                
                # Skip all functions (they're not data variables)
                if isa(value, Function)
                    continue
                end
                
                # Skip type definitions (DataType, UnionAll, etc.)
                if isa(value, Type)
                    continue
                end
                
                # Get type information
                value_type = typeof(value)
                type_str = string(value_type)
                
                # Get value string with smart truncation based on type
                value_str = nothing  # Will be nothing for large variables
                summary_str = ""
                needs_fetch = false  # Flag if full value needs separate fetch
                is_dataframe = false
                column_names = nothing
                
                try
                    # Check if it's a DataFrame (requires DataFrames.jl to be loaded)
                    if type_str == "DataFrame" || occursin("DataFrame", type_str)
                        is_dataframe = true
                        # Get DataFrame dimensions and column names
                        try
                            nrows = size(value, 1)
                            ncols = size(value, 2)
                            column_names = string.(names(value))
                            summary_str = "$(nrows)×$(ncols) DataFrame"
                            
                            # For small DataFrames, include the full value
                            if nrows <= 20 && ncols <= 10
                                value_str = string(value)
                            else
                                value_str = nothing
                                needs_fetch = true
                            end
                        catch e
                            # If we can't get DataFrame info, still mark as DataFrame but use fallback summary
                            # Don't set is_dataframe = false - the type check is sufficient
                            println(stderr, "Warning: Could not get DataFrame dimensions for $name_str: $e")
                            summary_str = "$(type_str) (unable to get dimensions)"
                            value_str = nothing
                            needs_fetch = true
                        end
                    # For large arrays/matrices, show dimensions only in summary
                    elseif isa(value, AbstractArray)
                        # Try to get dimensions, with fallback for types that don't support size()
                        dims = try
                            size(value)
                        catch e
                            # Some array types (like PooledArrays.PooledVector) may not implement size()
                            # Fall back to using length for 1D arrays
                            try
                                len = length(value)
                                [len]  # Return as 1D dimension
                            catch e2
                                # If both fail, just use type name
                                nothing
                            end
                        end
                        
                        ndims_val = try
                            ndims(value)
                        catch e
                            # If ndims fails, assume 1D if we have a length
                            dims !== nothing && length(dims) == 1 ? 1 : 0
                        end
                        
                        # Only include full values for small arrays
                        if dims === nothing
                            # Couldn't get dimensions - use type name only
                            summary_str = "$(typeof(value)) (unable to get size)"
                            value_str = nothing
                            needs_fetch = true
                        elseif ndims_val == 1 && length(dims) > 0 && dims[1] <= 10
                            # Small 1D array - include full value
                            value_str = string(value)
                            summary_str = string(value)
                        elseif ndims_val == 2 && length(dims) >= 2 && all(dims .<= 2)
                            # Small 2D array (2x2 or smaller) - include full value
                            value_str = string(value)
                            summary_str = string(value)
                        else
                            # Large array - dimensions only, value must be fetched separately
                            dim_str = dims !== nothing ? join(dims, "×") : "unknown"
                            summary_str = "$dim_str $(typeof(value))"
                            value_str = nothing  # Don't send full value in initial response
                            needs_fetch = true
                        end
                    else
                        # Non-array types - check if value is small enough
                        temp_str = repr(value)
                        if length(temp_str) <= 500
                            # Small value - include it
                            value_str = temp_str
                            summary_str = temp_str
                        else
                            # Large value (e.g., big string, complex struct) - needs fetch
                            summary_str = temp_str[1:200] * "... [large value]"
                            value_str = nothing
                            needs_fetch = true
                        end
                    end
                catch e1
                    try
                        temp_str = string(value)
                        if length(temp_str) <= 500
                            value_str = temp_str
                            summary_str = temp_str
                        else
                            summary_str = temp_str[1:200] * "..."
                            value_str = nothing
                            needs_fetch = true
                        end
                    catch e2
                        # Last resort: just show the type
                        value_str = nothing
                        summary_str = "<$type_str instance>"
                        needs_fetch = true
                    end
                end
                
                # Truncate summary if needed
                if length(summary_str) > 200
                    summary_str = summary_str[1:200] * "..."
                end
                
                # Check if expandable (arrays, dicts, structs, dataframes) - with error handling
                is_expandable = false
                is_array = false
                is_dict = false
                is_struct = false
                element_count = nothing
                
                try
                    if is_dataframe
                        is_expandable = true
                        # For DataFrames, element_count is number of rows
                        try
                            element_count = size(value, 1)
                        catch
                            element_count = nothing
                        end
                    elseif isa(value, AbstractArray)
                        is_expandable = true
                        is_array = true
                        try
                            element_count = length(value)
                        catch
                            element_count = nothing
                        end
                    elseif isa(value, AbstractDict)
                        is_expandable = true
                        is_dict = true
                        try
                            element_count = length(value)
                        catch
                            element_count = nothing
                        end
                    elseif !isa(value, Number) && !isa(value, String) && !isa(value, Bool) && !isa(value, Nothing) && !isa(value, Function)
                        is_expandable = true
                        is_struct = true
                    end
                catch
                    # If we can't determine expandability, assume it's not expandable
                    is_expandable = false
                end
                
                # Create variable entry
                var_entry = Dict(
                    "name" => name_str,
                    "type" => type_str,
                    "is_expandable" => is_expandable,
                    "is_array" => is_array,
                    "is_dict" => is_dict,
                    "is_struct" => is_struct,
                    "is_dataframe" => is_dataframe,
                    "element_count" => element_count,
                    "summary" => summary_str,    # Short summary for list view
                    "needs_fetch" => needs_fetch  # Whether full value needs separate request
                )
                
                # Add column names for DataFrames
                if is_dataframe && column_names !== nothing
                    var_entry["column_names"] = column_names
                end
                
                # Only include value if it's available (small variables)
                if value_str !== nothing
                    var_entry["value"] = value_str
                end
                
                variables[name_str] = var_entry
            catch e
                # Skip variables that cause errors during inspection
                println(stderr, "Warning: Could not inspect variable $name_str: $e")
                continue
            end
        end
        
        return variables
    catch e
        println(stderr, "Error getting workspace variables: ", sprint(showerror, e))
        return Dict{String, Any}()
    end
end

# Handle GetWorkspaceVariables request
function handle_get_workspace_variables(data)
    try
        # Extract required fields using new JSON utilities
        id = extract_required_string(data, "id")
        
        # Get workspace variables
        variables = get_workspace_variables()
        
        # Send response back to Rust
        response = Dict(
            "WorkspaceVariables" => Dict(
                "id" => id,
                "variables" => variables
            )
        )
        send_message_to_backend(response)
        
    catch e
        println(stderr, "Error handling get workspace variables: ", sprint(showerror, e))
        
        # Send error response (with safe field extraction)
        id = extract_optional_string(data, "id")
        if id === nothing
            id = "unknown"
        end
        
        response = Dict(
            "WorkspaceVariables" => Dict(
                "id" => id,
                "variables" => Dict{String, Any}()
            )
        )
        send_message_to_backend(response)
    end
end

# Get full value of a specific variable
function get_variable_value(var_name::String)
    try
        if !isdefined(Main, Symbol(var_name))
            return nothing
        end
        
        value = getfield(Main, Symbol(var_name))
        
        # Get the full value representation
        value_str = try
            repr(value)
        catch e1
            try
                string(value)
            catch e2
                "<error getting value: $e2>"
            end
        end
        
        # Truncate extremely large values (safety limit)
        max_length = 100000  # 100KB limit
        if length(value_str) > max_length
            value_str = value_str[1:max_length] * "... [truncated - too large]"
        end
        
        return value_str
    catch e
        println(stderr, "Error getting variable value for $var_name: ", sprint(showerror, e))
        return nothing
    end
end

# Handle GetVariableValue request
function handle_get_variable_value(data)
    try
        # Extract required fields using new JSON utilities
        id = extract_required_string(data, "id")
        var_name = extract_required_string(data, "variable_name")
        
        # Get the variable's full value
        value = get_variable_value(var_name)
        
        # Send response back to Rust
        response = Dict(
            "VariableValue" => Dict(
                "id" => id,
                "variable_name" => var_name,
                "value" => value
            )
        )
        send_message_to_backend(response)
        
    catch e
        println(stderr, "Error handling get variable value: ", sprint(showerror, e))
        
        # Send error response (with safe field extraction)
        id = extract_optional_string(data, "id")
        if id === nothing
            id = "unknown"
        end
        var_name = extract_optional_string(data, "variable_name")
        if var_name === nothing
            var_name = "unknown"
        end
        
        response = Dict(
            "VariableValue" => Dict(
                "id" => id,
                "variable_name" => var_name,
                "value" => nothing
            )
        )
        send_message_to_backend(response)
    end
end




















