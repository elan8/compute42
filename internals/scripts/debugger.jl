# Compute42 Custom Debugger
# Uses JuliaInterpreter.jl directly for debugging
# Integrated into main Julia process (setup.jl)
#
# This file includes all debugger modules in the correct dependency order.
# It maintains backward compatibility with existing include() calls from setup.jl.

# Include modules in dependency order:
# 1. State (no dependencies)
include(joinpath(@__DIR__, "debugger", "state.jl"))

# 2. Display (depends on state)
include(joinpath(@__DIR__, "debugger", "display.jl"))

# 3. Communication (depends on setup.jl's send_message_to_backend)
include(joinpath(@__DIR__, "debugger", "communication.jl"))

# 4. Execution (depends on state, communication)
include(joinpath(@__DIR__, "debugger", "execution.jl"))

# 5. Inspection (depends on state)
include(joinpath(@__DIR__, "debugger", "inspection.jl"))

# 6. Handlers (depends on all above)
include(joinpath(@__DIR__, "debugger", "handlers.jl"))

# Message handling loop
# Message routing now handled by setup.jl main message loop

# Communication initialization removed - using existing setup.jl pipes

# Main entry point
# Debugger functions are now called directly from setup.jl message handlers
# No standalone execution needed
