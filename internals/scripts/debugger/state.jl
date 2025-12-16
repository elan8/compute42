# Compute42 Debugger State
# Global debug state variables and constants

# Global debug state
global JJ_DEBUG_MODE = false
global JJ_DEBUG_SESSION_ACTIVE = false
global JJ_CURRENT_FRAME = nothing
global JJ_BREAKPOINTS = Dict{String, Set{Int}}()  # file_path => Set of line numbers
global JJ_LAST_STOP_REASON = nothing
global JJ_DEBUG_PAUSED = false  # Whether execution is paused at a breakpoint
global JJ_DEBUG_CONTINUE_REQUESTED = false  # Whether user clicked Continue
global JJ_DEBUG_EXECUTION_STATE = nothing  # Stores execution state when paused

# Constants for plot communication (same as setup.jl)
# Only define if not already defined (avoids redefinition warnings when debugger.jl is reloaded)
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




















