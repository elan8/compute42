# Compute42 Package Management
# Install and load required packages first
# Only install packages if they're not already available

try
    using Pkg
    
    # Switch to default environment for Compute42 core packages
    Pkg.activate()
    
    # List of required packages for Compute42 core functionality
    required_packages = ["JuliaInterpreter", "JSON", "Revise"]
    
    # Check which packages are already installed in default environment
    installed_packages = keys(Pkg.project().dependencies)
    packages_to_install = String[]
    
    for pkg in required_packages
        if !(pkg in installed_packages)
            push!(packages_to_install, pkg)
        end
    end
    
    # Only install missing packages
    if !isempty(packages_to_install)
        println(stderr, "Compute42: Installing core packages: ", join(packages_to_install, ", "))
        Pkg.add(packages_to_install)
    end
    
    # Check if instantiate is needed for default environment
    manifest_file = joinpath(Pkg.project().path, "Manifest.toml")
    needs_instantiate = false
    
    if !isfile(manifest_file)
        println(stderr, "Compute42: Default environment Manifest.toml not found, running instantiate...")
        needs_instantiate = true
    else
        # Check if dependencies are properly installed
        try
            Pkg.status()
        catch e
            println(stderr, "Compute42: Default environment package status check failed, running instantiate...")
            needs_instantiate = true
        end
    end
    
    if needs_instantiate
        Pkg.instantiate()
        println(stderr, "Compute42: Core packages installed successfully")
    else
        println(stderr, "Compute42: Core packages already up to date")
    end
catch e
    # Only print errors, not success messages
    println(stderr, "Compute42: Failed to install packages: ", e)
end

# Load required modules
try
    using Sockets
    using JSON
    using Base64
    using UUIDs
    using JuliaInterpreter
    using Revise
    
    # Configure Revise for automatic tracking
    # Revise automatically tracks packages loaded with 'using' or 'import'
    # when 'using Revise' is called at startup (which we just did above)
    # This enables automatic reloading of modules when source files change
    # For local projects, Revise.track() will be called during project activation
    # to track the project's src/ directory
    
    # Helper function to manually reload a package with Revise
    # Usage: reload_package(Main.titanic) or reload_package(:titanic)
    if !@isdefined(reload_package)
        function reload_package(pkg::Union{Symbol, Module})
            try
                if isa(pkg, Symbol)
                    # println("Revise: Attempting to reload package: ", pkg)
                    if isdefined(Main, pkg)
                        pkg_module = getfield(Main, pkg)
                        if isa(pkg_module, Module)
                            # Check if Revise is tracking this module
                            try
                                if isdefined(Revise, :pkgdata)
                                    # Check if module is tracked by Revise
                                    tracked = Revise.pkgdata(pkg_module)
                                    if tracked !== nothing
                                        # println("Revise: Module '", pkg, "' is tracked by Revise")
                                        # Check what files are being tracked
                                        try
                                            if hasfield(typeof(tracked), :fileinfo)
                                                # println("Revise: Tracking info available for '", pkg, "'")
                                            end
                                        catch
                                        end
                                    else
                                        # println("Revise: Module '", pkg, "' is NOT tracked by Revise")
                                    end
                                end
                            catch e
                                # println("Revise: Could not check tracking status: ", e)
                            end
                            
                            # Try to reload using Revise
                            try
                                # println("Revise: Calling revise() on module '", pkg, "'...")
                                # Use revise() to reload all files that define this module
                                revise(pkg_module; force=true)
                                # println("Revise: ✓ Successfully reloaded package: ", pkg)
                            catch e
                                # println("Revise: revise() failed: ", e)
                                # If revise() doesn't work, try tracking and reimporting
                                try
                                    # println("Revise: Attempting to track and reimport '", pkg, "'...")
                                    # Try to track by package name
                                    try
                                        Revise.track(String(pkg))
                                        # println("Revise: Tracked '", pkg, "' by name")
                                    catch
                                        # If tracking by name fails, try to find the package path
                                        try
                                            pkg_path = Base.find_package(String(pkg))
                                            if pkg_path !== nothing
                                                Revise.track(pkg_path)
                                                # println("Revise: Tracked '", pkg, "' by path: ", pkg_path)
                                            end
                                        catch
                                        end
                                    end
                                    # println("Revise: Reimporting '", pkg, "' to trigger reload...")
                                    # Reimport to trigger reload
                                    eval(Main, :(using $pkg))
                                    # Then call revise() to apply any changes
                                    revise(pkg_module; force=true)
                                    # println("Revise: ✓ Reloaded package via track and reimport: ", pkg)
                                catch e2
                                    # println("Revise: ✗ Could not reload package '", pkg, "': ", e2)
                                end
                            end
                        else
                            # println("Revise: '", pkg, "' is not a Module")
                        end
                    else
                        # println("Revise: Package '", pkg, "' not found in Main")
                    end
                elseif isa(pkg, Module)
                    try
                        # println("Revise: Attempting to reload module: ", pkg)
                        # Check if Revise is tracking this module
                        try
                            if isdefined(Revise, :pkgdata)
                                tracked = Revise.pkgdata(pkg)
                                if tracked !== nothing
                                    # println("Revise: Module is tracked by Revise")
                                else
                                    # println("Revise: Module is NOT tracked by Revise")
                                end
                            end
                        catch
                        end
                        # Use revise() to reload all files that define this module
                        revise(pkg; force=true)
                        # println("Revise: ✓ Successfully reloaded module: ", pkg)
                    catch e
                        # println("Revise: ✗ Could not reload module: ", e)
                    end
                end
            catch e
                # println("Revise: ✗ Error reloading package: ", e)
            end
        end
    end
    
    # Helper function to check Revise tracking status
    if !@isdefined(check_revise_tracking)
        function check_revise_tracking(pkg::Union{Symbol, Module, String})
            try
                # println("Revise: Checking tracking status for: ", pkg)
                if isa(pkg, String)
                    pkg = Symbol(pkg)
                end
                if isa(pkg, Symbol)
                    if isdefined(Main, pkg)
                        pkg_module = getfield(Main, pkg)
                        if isa(pkg_module, Module)
                            try
                                if isdefined(Revise, :pkgdata)
                                    tracked = Revise.pkgdata(pkg_module)
                                    if tracked !== nothing
                                        # println("Revise: ✓ Module '", pkg, "' IS tracked by Revise")
                                        # Try to get more info
                                        try
                                            if hasfield(typeof(tracked), :fileinfo) || hasproperty(tracked, :fileinfo)
                                                # println("Revise:   File tracking info available")
                                            end
                                        catch
                                        end
                                        return true
                                    else
                                        # println("Revise: ✗ Module '", pkg, "' is NOT tracked by Revise")
                                        return false
                                    end
                                else
                                    # println("Revise: Cannot check tracking (Revise.pkgdata not available)")
                                    return nothing
                                end
                            catch e
                                # println("Revise: Error checking tracking: ", e)
                                return nothing
                            end
                        else
                            # println("Revise: '", pkg, "' is not a Module")
                            return false
                        end
                    else
                        # println("Revise: Module '", pkg, "' not found in Main")
                        return false
                    end
                elseif isa(pkg, Module)
                    try
                        if isdefined(Revise, :pkgdata)
                            tracked = Revise.pkgdata(pkg)
                            if tracked !== nothing
                                # println("Revise: ✓ Module IS tracked by Revise")
                                return true
                            else
                                # println("Revise: ✗ Module is NOT tracked by Revise")
                                return false
                            end
                        else
                            # println("Revise: Cannot check tracking (Revise.pkgdata not available)")
                            return nothing
                        end
                    catch e
                        # println("Revise: Error checking tracking: ", e)
                        return nothing
                    end
                end
            catch e
                # println("Revise: ✗ Error checking tracking: ", e)
                return nothing
            end
        end
    end
    
    # Make JuliaInterpreter functions available in Main immediately
    # Note: These are already available through the using statement, no need to reassign
    
    # Include debugger functionality
    include(joinpath(@__DIR__, "..", "debugger.jl"))
    println(stderr, "Compute42: Debugger functionality loaded")
catch e
    println(stderr, "Compute42: Failed to load required modules: ", e)
end




















