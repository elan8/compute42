# Compute42 Debugger Execution
# Core debugger execution logic

# Run debug frame until breakpoint or completion
function run_until_breakpoint_or_completion(frame, id)
    try
        # Use JuliaInterpreter.debug_command for proper execution in the latest world age
        # This handles using statements better than finish! and resolves world age issues
        ret = Base.invokelatest(JuliaInterpreter.debug_command, JuliaInterpreter.RecursiveInterpreter(), frame, :c)
        
        if ret === nothing
            # Execution completed
            return "completed"
        else
            frame_result, pc = ret
            global JJ_CURRENT_FRAME = frame_result
            
            if pc isa JuliaInterpreter.BreakpointRef
                # Hit a breakpoint
                method_info = JuliaInterpreter.framecode(frame_result)
                file_path = string(method_info.scope.file)
                line_number = frame_result.pc[1]
                
                # Send breakpoint hit message
                send_debug_message(Dict(
                    "BreakpointHit" => Dict(
                        "id" => id,
                        "file_path" => file_path,
                        "line" => line_number
                    )
                ))
                return "breakpoint"
            else
                # Some other stop condition
                return "stopped"
            end
        end
    catch e
        rethrow(e)
    end
end




















