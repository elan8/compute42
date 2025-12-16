# Compute42 Display System
# Plot pane configuration constants (defined directly in Main)
# Only define if not already defined (avoids redefinition warnings)
if !@isdefined(JJ_PLOT_PANE_ENABLED)
    const JJ_PLOT_PANE_ENABLED = Ref(true)
end

if !@isdefined(JJ_DISPLAYABLE_MIMES)
    const JJ_DISPLAYABLE_MIMES = [
        "application/vnd.vegalite.v5+json",
        "application/vnd.vegalite.v4+json",
        "application/vnd.vegalite.v3+json",
        "application/vnd.vegalite.v2+json",
        "application/vnd.vega.v5+json",
        "application/vnd.vega.v4+json",
        "application/vnd.vega.v3+json",
        "application/vnd.plotly.v1+json",
        "juliavscode/html",
        "image/svg+xml",
        "image/png",
        "image/gif",
    ]
end

# Plot capture system (similar to VS Code)
struct Compute42Display <: AbstractDisplay
    is_repl::Bool
end

Compute42Display() = Compute42Display(false)

# Check if data should be filtered out
function should_filter_plot_data(data)
    # Filter out basic types that shouldn't be plots
    if data === false || data === true || data === nothing
        return true
    end

    # Filter out empty or meaningless strings
    if isa(data, String)
        stripped = strip(data)
        if isempty(stripped)
            return true
        end

        # Filter out common meaningless values
        meaningless_values = [
            "false", "true", "none", "nothing",
            "Any[]", "Dict{String, Vector{Int64}}()",
            "send_debug_frame_info", "handle_debug_message"
        ]

        if data in meaningless_values
            return true
        end

        # Filter out empty collections
        if startswith(data, "Dict{") && endswith(data, "}()")
            return true
        end

        if startswith(data, "Vector{") && endswith(data, "}()")
            return true
        end

        if startswith(data, "Array{") && endswith(data, "}()")
            return true
        end

        # Filter out function names (common during initialization)
        if occursin(r"^[a-zA-Z_][a-zA-Z0-9_]*\$", data) && length(data) > 3
            return true
        end
        
        # Filter out plot object string representations
        if startswith(data, "Plots.Plot{") || contains(data, "string conversion failed")
            return true
        end
    end

    return false
end

# Convert MIME types to base64-encoded strings for image types
function stringmime(m::MIME, x)
    buf = IOBuffer()
    show(buf, m, x)
    return base64encode(take!(buf))
end

# Send display message to backend
        # Get plot socket from Main namespace
function send_display_msg(kind, data)
    try
        # Skip sending empty or meaningless data
        if should_filter_plot_data(data)
            return nothing
        end

        # Get plot socket (now in Main namespace)
        plot_socket = Main.get_plot_socket()
        
        if plot_socket !== nothing && isopen(plot_socket)
            # Generate a unique ID for the plot
            plot_id = string(UUIDs.uuid4())
            timestamp = round(Int, time() * 1000)  # Unix timestamp in milliseconds

            # Create PlotData message in the format expected by Rust
            msg = Dict{String,Any}(
                "PlotData" => Dict{String,Any}(
                    "id" => plot_id,
                    "mime_type" => kind,
                    "data" => data,
                    "timestamp" => timestamp,
                    "title" => nothing,
                    "description" => nothing,
                    "source_file" => nothing,
                    "line_number" => nothing,
                    "code_context" => nothing,
                    "session_id" => nothing
                )
            )
            message_json = JSON.json(msg)
            println(plot_socket, message_json)
            flush(plot_socket)
        else
            println(stderr, "Compute42: Plot socket not available or not open")
        end
    catch e
        println(stderr, "Compute42: Failed to send display message: ", sprint(showerror, e))
    end
end

# Display method for Compute42Display
function Base.display(d::Compute42Display, m::MIME, @nospecialize(x))
    if !JJ_PLOT_PANE_ENABLED[]
        # Fall back to default display
        return nothing
    else
        mime = string(m)
        if mime in JJ_DISPLAYABLE_MIMES
            # Handle binary vs text MIME types
            # SVG is text, so don't base64-encode it. Only binary images need base64 encoding.
            if mime == "image/svg+xml"
                # SVG is already text, just convert to string directly
                payload = String(repr(m, x))
            elseif startswith(mime, "image/")
                # Binary images need base64 encoding
                payload = stringmime(m, x)
            else
                # Other MIME types (JSON, HTML, etc.) are already text
                payload = String(repr(m, x))
            end
            send_display_msg(mime, payload)
        else
            throw(MethodError(display, (d, m, x)))
        end
    end
    return nothing
end

# Display method for general objects (simplified like VS Code)
function Base.display(d::Compute42Display, @nospecialize(x))
    if JJ_PLOT_PANE_ENABLED[]
        for mime in JJ_DISPLAYABLE_MIMES
            if showable(mime, x)
                display(d, mime, x)
                return nothing
            end
        end
    end

    # Skip displaying display stack objects to prevent empty PLAIN plots
    if x isa AbstractDisplay || typeof(x) <: AbstractDisplay
        return nothing
    end

    # If no displayable MIME type was found, throw MethodError
    # This allows the display system to try other displays
    throw(MethodError(display, (d, x)))
end

# Check if displayable
Base.Multimedia.displayable(d::Compute42Display, mime::MIME) = JJ_PLOT_PANE_ENABLED[] && string(mime) in JJ_DISPLAYABLE_MIMES

# Check if a result can be displayed (like VS Code does)
function can_display_result(x)
    # Check if it's displayable with any of our supported MIME types
    for mime in JJ_DISPLAYABLE_MIMES
        if showable(mime, x)
            return true
        end
    end
    
    # Also check for table-like objects (like VS Code does)
    if is_table_like(x)
        return true
    end
    
    return false
end

# Check if an object is table-like (copied from VS Code)
function is_table_like(x)
    if showable("application/vnd.dataresource+json", x)
        return true
    end

    # Check if it's iterable and table-like
    try
        # This is a simplified check - VS Code has more sophisticated logic
        if x isa AbstractVector || x isa AbstractMatrix
            return true
        end
    catch
        # If we can't check, assume it's not table-like
    end

    return false
end

# Setup display system (similar to VS Code's fix_displays)
function setup_display_system(; is_repl = false)
    # Remove any existing Compute42Display
    for d in reverse(Base.Multimedia.displays)
        if d isa Compute42Display
            popdisplay(d)
        end
    end
    # Add our custom display
    pushdisplay(Compute42Display(is_repl))
end





















