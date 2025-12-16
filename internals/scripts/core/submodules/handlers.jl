# Compute42 Message Routing and Loop
# Import required modules
using JSON

# Handle DebugMessage requests
function handle_debug_message(data)
    try
        # Extract the actual debug data from the DebugMessage structure
        # data["data"] might be a Dict or JSON.Object
        data_field = data["data"]
        debug_data = isa(data_field, Dict) ? data_field : extract_dict(data_field)
        if debug_data === nothing
            error("Invalid debug message structure: missing or invalid 'data' field")
        end
        
        id = extract_required_string(debug_data, "id")
        
        # message might be a Dict or JSON.Object
        message_field = debug_data["message"]
        message = isa(message_field, Dict) ? message_field : extract_dict(message_field)
        if message === nothing
            error("Invalid debug message structure: missing or invalid 'message' field")
        end
        
        # Route the debug command to the appropriate handler
        if haskey(message, "StartDebug")
            handle_start_debug(message["StartDebug"])
        elseif haskey(message, "SetBreakpoint")
            handle_set_breakpoint(message["SetBreakpoint"])
        elseif haskey(message, "RemoveBreakpoint")
            handle_remove_breakpoint(message["RemoveBreakpoint"])
        elseif haskey(message, "StepOver")
            handle_step_over(message["StepOver"])
        elseif haskey(message, "StepIn")
            handle_step_in(message["StepIn"])
        elseif haskey(message, "StepOut")
            handle_step_out(message["StepOut"])
        elseif haskey(message, "Continue")
            handle_continue(message["Continue"])
        elseif haskey(message, "GetVariables")
            handle_get_variables(message["GetVariables"])
        elseif haskey(message, "GetStacktrace")
            handle_get_stacktrace(message["GetStacktrace"])
        elseif haskey(message, "StopDebug")
            handle_stop_debug(message["StopDebug"])
        else
            # Send error response for unknown command
            response = Dict(
                "DebugMessageResponse" => Dict(
                    "id" => id,
                    "response" => "Unknown debug command: " * string(keys(message)),
                    "success" => false
                )
            )
            send_message_to_backend(response)
        end
        
    catch e
        println(stderr, "Compute42: Error handling debug message: ", sprint(showerror, e))
        
        # Send error response (with safe field extraction)
        id = "unknown"
        try
            data_field = get(data, "data", nothing)
            if data_field !== nothing
                debug_data = isa(data_field, Dict) ? data_field : extract_dict(data_field)
                if debug_data !== nothing
                    id_val = extract_optional_string(debug_data, "id")
                    if id_val !== nothing
                        id = id_val
                    end
                end
            end
        catch
            # Use default "unknown" if extraction fails
        end
        
        response = Dict(
            "DebugMessageResponse" => Dict(
                "id" => id,
                "response" => "Error: " * string(e),
                "success" => false
            )
        )
        send_message_to_backend(response)
    end
end

# Handle messages from Rust using direct connection
function handle_messages_loop()
    try
        # Check socket state before starting
        if JJ_PERMANENT_SOCKET === nothing
            println(stderr, "Compute42: ERROR - JJ_PERMANENT_SOCKET is nothing in handle_messages_loop")
            return
        end
        if !isopen(JJ_PERMANENT_SOCKET)
            println(stderr, "Compute42: ERROR - JJ_PERMANENT_SOCKET is not open in handle_messages_loop")
            return
        end
        
        # Signal that the message loop is now actively running and ready to receive messages
        println(stderr, "Compute42: MESSAGE_LOOP_READY")
        
        while JJ_PERMANENT_SOCKET !== nothing && isopen(JJ_PERMANENT_SOCKET)
            try
                # Read a line from the socket
                line = readline(JJ_PERMANENT_SOCKET)
                if isempty(line)
                    break
                end

                # Parse the JSON message
                message = JSON.parse(line)

                # Handle the message based on its type
                if haskey(message, "CodeExecution")
                    handle_code_execution(message["CodeExecution"])
                elseif haskey(message, "ApiRequest")
                    handle_api_request(message["ApiRequest"])
                elseif haskey(message, "ConnectionTest")
                    handle_connection_test(message["ConnectionTest"])
                elseif haskey(message, "DebugMessage")
                    handle_debug_message(message["DebugMessage"])
                elseif haskey(message, "GetWorkspaceVariables")
                    handle_get_workspace_variables(message["GetWorkspaceVariables"])
                elseif haskey(message, "GetVariableValue")
                    handle_get_variable_value(message["GetVariableValue"])
                # Debug command handlers
                elseif haskey(message, "StartDebug")
                    handle_start_debug(message["StartDebug"])
                elseif haskey(message, "SetBreakpoint")
                    handle_set_breakpoint(message["SetBreakpoint"])
                elseif haskey(message, "RemoveBreakpoint")
                    handle_remove_breakpoint(message["RemoveBreakpoint"])
                elseif haskey(message, "StepOver")
                    handle_step_over(message["StepOver"])
                elseif haskey(message, "StepIn")
                    handle_step_in(message["StepIn"])
                elseif haskey(message, "StepOut")
                    handle_step_out(message["StepOut"])
                elseif haskey(message, "Continue")
                    handle_continue(message["Continue"])
                elseif haskey(message, "GetVariables")
                    handle_get_variables(message["GetVariables"])
                elseif haskey(message, "GetStacktrace")
                    handle_get_stacktrace(message["GetStacktrace"])
                elseif haskey(message, "StopDebug")
                    handle_stop_debug(message["StopDebug"])
                end

            catch e
                println(stderr, "Compute42: Error handling message: ", sprint(showerror, e))
                # Don't break on individual message errors, continue listening
            end
        end

    catch e
        println(stderr, "Compute42: Fatal error in message handling loop: ", sprint(showerror, e))
    end
end

# Setup message handlers
function setup_message_handlers()
    # Additional message handlers can be added here
end

