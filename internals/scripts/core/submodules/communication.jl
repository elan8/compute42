# Compute42 Communication Infrastructure
# Import required modules
using Sockets
using JSON

# Global variables for communication
# Only define if not already defined (prevents issues during package precompilation)
# This check prevents redefinition during package loading/precompilation
if !@isdefined(C42_PERMANENT_SERVER)
    global C42_PERMANENT_SERVER = nothing
end
if !@isdefined(C42_PERMANENT_SOCKET)
    global C42_PERMANENT_SOCKET = nothing
end
if !@isdefined(C42_PLOT_SERVER)
    global C42_PLOT_SERVER = nothing
end
if !@isdefined(C42_PLOT_SOCKET)
    global C42_PLOT_SOCKET = nothing
end
if !@isdefined(C42_MESSAGE_HANDLERS)
    global C42_MESSAGE_HANDLERS = Dict{String, Function}()
end
if !@isdefined(C42_ACCEPT_TASK)
    global C42_ACCEPT_TASK = nothing
end

# Initialize permanent communication with Julia
function initialize_permanent_communication(pipe_name::String)
    try
        # Log the raw pipe name received
        println(stderr, "Compute42: [PIPE_SETUP] Received pipe name: '", pipe_name, "'")
        
        # Convert pipe name to platform-specific format
        # On Windows: use named pipe format (\\.\pipe\name)
        # On Unix/Linux: use Unix domain socket format (/tmp/name)
        if Sys.iswindows()
            platform_pipe_name = "\\\\.\\pipe\\" * pipe_name
        else
            # On Unix/Linux, use /tmp directory for Unix domain sockets
            platform_pipe_name = "/tmp/" * pipe_name
        end
        
        println(stderr, "Compute42: [PIPE_SETUP] Creating socket at: '", platform_pipe_name, "'")

        # Create the named pipe/server socket
        global JJ_PERMANENT_SERVER = Sockets.listen(platform_pipe_name)
        println(stderr, "Compute42: [PIPE_SETUP] Socket created successfully")

        # On Linux/Unix, verify socket file exists and give it a moment to be ready
        if !Sys.iswindows()
            # Check if socket file exists
            if isfile(platform_pipe_name) || ispath(platform_pipe_name)
                println(stderr, "Compute42: [PIPE_SETUP] Socket file exists at: '", platform_pipe_name, "'")
            else
                println(stderr, "Compute42: [PIPE_SETUP] WARNING: Socket file not found at: '", platform_pipe_name, "' (may be abstract namespace)")
            end
            sleep(0.1)  # 100ms delay to ensure socket is ready
            println(stderr, "Compute42: [PIPE_SETUP] Socket ready delay completed")
        end

        # Signal that the pipe is ready BEFORE accepting connections
        println(stderr, "Compute42: TO_JULIA_PIPE_READY")

        # Setup message handlers
        setup_message_handlers()

        # Accept a single connection from Rust
        println(stderr, "Compute42: Waiting for connection from Rust...")
        global JJ_PERMANENT_SOCKET = Sockets.accept(JJ_PERMANENT_SERVER)
        println(stderr, "Compute42: Connection accepted, starting message loop...")

        # Start message handling loop in background
        # Use @async to run in background, but also ensure MESSAGE_LOOP_READY is emitted
        @async begin
            try
                handle_messages_loop()
            catch e
                println(stderr, "Compute42: Error in message loop async task: ", sprint(showerror, e))
            end
        end

        return true
    catch e
        println(stderr, "Compute42: Failed to initialize permanent communication: ", sprint(showerror, e))
        return false
    end
end

# Initialize plot communication
function initialize_plot_communication(pipe_name::String)
    try
        # Log the raw pipe name received
        println(stderr, "Compute42: [PIPE_SETUP] Received from_julia pipe name: '", pipe_name, "'")
        
        # Convert pipe name to platform-specific format
        # On Windows: use named pipe format (\\.\pipe\name)
        # On Unix/Linux: use Unix domain socket format (/tmp/name)
        if Sys.iswindows()
            platform_pipe_name = "\\\\.\\pipe\\" * pipe_name
        else
            # On Unix/Linux, use /tmp directory for Unix domain sockets
            platform_pipe_name = "/tmp/" * pipe_name
        end
        
        println(stderr, "Compute42: [PIPE_SETUP] Creating from_julia socket at: '", platform_pipe_name, "'")

        # Create the named pipe/server socket for plot communication
        global JJ_PLOT_SERVER = Sockets.listen(platform_pipe_name)
        println(stderr, "Compute42: [PIPE_SETUP] From_julia socket created successfully")

        # On Linux/Unix, verify socket file exists and give it a moment to be ready
        if !Sys.iswindows()
            # Check if socket file exists
            if isfile(platform_pipe_name) || ispath(platform_pipe_name)
                println(stderr, "Compute42: [PIPE_SETUP] From_julia socket file exists at: '", platform_pipe_name, "'")
            else
                println(stderr, "Compute42: [PIPE_SETUP] WARNING: From_julia socket file not found at: '", platform_pipe_name, "' (may be abstract namespace)")
            end
            sleep(0.1)  # 100ms delay to ensure socket is ready
            println(stderr, "Compute42: [PIPE_SETUP] From_julia socket ready delay completed")
        end

        # Signal that the from_julia pipe is ready BEFORE accepting connections
        println(stderr, "Compute42: FROM_JULIA_PIPE_READY")

        # Accept a single connection from Rust
        global JJ_PLOT_SOCKET = Sockets.accept(JJ_PLOT_SERVER)

        # No need for plot data loop - plot socket is only for sending data to Rust
        return true
    catch e
        println(stderr, "Compute42: Failed to initialize plot communication: ", sprint(showerror, e))
        return false
    end
end

# Get plot socket (exported accessor for display system)
function get_plot_socket()
    return JJ_PLOT_SOCKET
end

# Send message to backend via FROM_JULIA pipe
# All responses from Julia to Rust go through the from_julia pipe for reliable one-way communication
function send_message_to_backend(message::Dict)
    try
        if JJ_PLOT_SOCKET !== nothing
            message_json = JSON.json(message)
            println(JJ_PLOT_SOCKET, message_json)
            flush(JJ_PLOT_SOCKET)
        else
            println(stderr, "Compute42: No active from_julia socket connection")
        end
    catch e
        println(stderr, "Compute42: Failed to send response via from_julia pipe: ", sprint(showerror, e))
    end
end

