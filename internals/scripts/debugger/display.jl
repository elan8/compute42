# Compute42 Debugger Display System
# Display system for plot capture

# Display system for plot capture
struct Compute42Display <: AbstractDisplay
    is_repl::Bool
end

Compute42Display() = Compute42Display(false)

# Send display message to backend
function send_display_msg(kind, data)
    try
        if JJ_PLOT_SOCKET !== nothing && isopen(JJ_PLOT_SOCKET)
            plot_id = string(UUIDs.uuid4())
            timestamp = round(Int, time() * 1000)
            
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
                    "session_id" => "debug_session"
                )
            )
            message_json = JSON.json(msg)
            println(JJ_PLOT_SOCKET, message_json)
            flush(JJ_PLOT_SOCKET)
        end
    catch e
        # Error sending plot - silently fail
    end
end

# Display method for Compute42Display
function Base.display(d::Compute42Display, m::MIME, @nospecialize(x))
    if JJ_PLOT_PANE_ENABLED[]
        mime = string(m)
        if mime in JJ_DISPLAYABLE_MIMES
            payload = startswith(mime, "image") ? stringmime(m, x) : String(repr(m, x))
            send_display_msg(mime, payload)
        end
    end
    return nothing
end

# Display method for general objects
function Base.display(d::Compute42Display, @nospecialize(x))
    if JJ_PLOT_PANE_ENABLED[]
        for mime in JJ_DISPLAYABLE_MIMES
            if showable(mime, x)
                return display(d, mime, x)
            end
        end
    end
    return nothing
end

# Setup display system
function setup_display_system()
    for d in reverse(Base.Multimedia.displays)
        if d isa Compute42Display
            popdisplay(d)
        end
    end
    pushdisplay(Compute42Display())
end




















