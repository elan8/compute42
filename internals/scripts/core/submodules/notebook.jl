# Compute42 Notebook Support

# Send notebook cell output event
function send_notebook_cell_output(cell_id, stream_type, content)
    try
        # Get socket from Main namespace
        socket = Main.get_socket_from_julia()
        
        if socket !== nothing && isopen(socket)
            # Create NotebookCellOutput message
            msg = Dict{String,Any}(
                "NotebookCellOutput" => Dict{String,Any}(
                    "cell_id" => cell_id,
                    "stream" => stream_type,  # "stdout" or "stderr"
                    "content" => content
                )
            )
            message_json = JSON.json(msg)
            println(socket, message_json)
            flush(socket)
        end
    catch e
        println(stderr, "Compute42: Failed to send notebook cell output: ", sprint(showerror, e))
    end
end

# Send notebook cell plot event
function send_notebook_cell_plot(cell_id, mime_type, data)
    try
        # Get socket from Main namespace
        socket = Main.get_socket_from_julia()
        
        if socket !== nothing && isopen(socket)
            plot_id = string(UUIDs.uuid4())
            timestamp = round(Int, time() * 1000)
            
            # Create NotebookCellPlot message
            msg = Dict{String,Any}(
                "NotebookCellPlot" => Dict{String,Any}(
                    "cell_id" => cell_id,
                    "plot_id" => plot_id,
                    "mime_type" => mime_type,
                    "data" => data,
                    "timestamp" => timestamp
                )
            )
            message_json = JSON.json(msg)
            println(socket, message_json)
            flush(socket)
        end
    catch e
        println(stderr, "Compute42: Failed to send notebook cell plot: ", sprint(showerror, e))
    end
end

# Send notebook cell result event
function send_notebook_cell_result(cell_id, result_string)
    try
        # Get socket from Main namespace  
        socket = Main.get_socket_from_julia()
        
        if socket !== nothing && isopen(socket)
            # Create NotebookCellResult message
            msg = Dict{String,Any}(
                "NotebookCellResult" => Dict{String,Any}(
                    "cell_id" => cell_id,
                    "result" => result_string
                )
            )
            message_json = JSON.json(msg)
            println(socket, message_json)
            flush(socket)
        end
    catch e
        println(stderr, "Compute42: Failed to send notebook cell result: ", sprint(showerror, e))
    end
end











