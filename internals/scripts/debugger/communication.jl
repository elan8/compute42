# Compute42 Debugger Communication
# Message sending utilities

# Send message to Rust
# Use existing send_message_to_backend from setup.jl
function send_debug_message(message::Dict)
    try
        send_message_to_backend(message)
    catch e
        # Error sending debug message - silently fail
    end
end




















