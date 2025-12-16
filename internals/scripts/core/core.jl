# Compute42 Core Functionality
# All functions are defined directly in Main namespace (no module wrapper)

# Include submodules in dependency order
# 1. Communication (no dependencies on other submodules)
include(joinpath(@__DIR__, "submodules", "communication.jl"))

# 1.5. JSON Utilities (no dependencies, used by other modules)
include(joinpath(@__DIR__, "submodules", "json_utils.jl"))

# 2. Execution (depends on communication and json_utils)
include(joinpath(@__DIR__, "submodules", "execution.jl"))

# 2.5. Notebook support (depends on communication)
include(joinpath(@__DIR__, "submodules", "notebook.jl"))

# 3. Workspace (depends on communication)
include(joinpath(@__DIR__, "submodules", "workspace.jl"))

# 4. Handlers (depends on execution, workspace, communication, debugger)
include(joinpath(@__DIR__, "submodules", "handlers.jl"))


















