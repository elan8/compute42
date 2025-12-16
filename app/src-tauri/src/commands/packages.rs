use log::{debug, error};
use tauri::State;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn get_julia_package_status(app_state: State<'_, AppState>) -> Result<serde_json::Value, AppError> {
    debug!("[Packages] Getting Julia package status");
    let code = r#"
        try
            # Use local scope to avoid polluting global namespace
            let
                # Import packages locally without affecting global scope
                local Pkg = Base.require(Base.PkgId(Base.UUID("44cfe95a-1eb2-52ea-b672-e2afdf69b78f"), "Pkg"))
                local JSON = Base.require(Base.PkgId(Base.UUID("682c06a0-de6a-54ab-a142-c8b1cf79cde6"), "JSON"))
                
                # Get current project
                current_project = Pkg.project()
                deps = Pkg.dependencies()
                direct_deps = Set{String}()
                
                # Read Project.toml to identify direct dependencies
                try
                    project_toml = Pkg.TOML.parsefile(joinpath(dirname(current_project.path), "Project.toml"))
                    if haskey(project_toml, "deps")
                        for (name, uuid) in project_toml["deps"]
                            push!(direct_deps, name)
                        end
                    end
                catch e
                    # If we can't read Project.toml, continue without direct dependency info
                end
                
                # Build package list
                packages = []
                for (uuid, dep) in deps
                    is_direct = dep.name in direct_deps
                    pkg_info = Dict(
                        "name" => dep.name,
                        "version" => string(dep.version),
                        "uuid" => string(uuid),
                        "description" => nothing,
                        "is_direct" => is_direct
                    )
                    push!(packages, pkg_info)
                end
                
                # Return the final result as JSON
                active_project = Pkg.project().path
                JSON.json(Dict("packages" => packages, "active_project" => active_project))
            end
        catch e
            # Fallback: try with global imports if local scope fails
            try
                if !isdefined(Main, :Pkg)
                    using Pkg
                end
                if !isdefined(Main, :JSON)
                    using JSON
                end
                
                current_project = Pkg.project()
                deps = Pkg.dependencies()
                direct_deps = Set{String}()
                
                try
                    project_toml = Pkg.TOML.parsefile(joinpath(dirname(current_project.path), "Project.toml"))
                    if haskey(project_toml, "deps")
                        for (name, uuid) in project_toml["deps"]
                            push!(direct_deps, name)
                        end
                    end
                catch e
                end
                
                packages = []
                for (uuid, dep) in deps
                    is_direct = dep.name in direct_deps
                    pkg_info = Dict(
                        "name" => dep.name,
                        "version" => string(dep.version),
                        "uuid" => string(uuid),
                        "description" => nothing,
                        "is_direct" => is_direct
                    )
                    push!(packages, pkg_info)
                end
                
                active_project = Pkg.project().path
                JSON.json(Dict("packages" => packages, "active_project" => active_project))
            catch e2
                JSON.json(Dict("error" => "Failed to get package status: " * string(e2), "packages" => [], "active_project" => nothing))
            end
        end
    "#;
    use internals::messages::execution::ExecuteApiRequest;
    match app_state.actor_system.execution_actor.send(ExecuteApiRequest { code: code.to_string() }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(result) => {
            let result = result.to_string();
            // Only log a summary to avoid cluttering logs with large package lists
            let preview = if result.len() > 200 {
                format!("{}... ({} chars)", &result[..200], result.len())
            } else {
                result.clone()
            };
            debug!("[Packages] Result from Julia (preview): {}", preview);
            match serde_json::from_str::<serde_json::Value>(&result) {
                Ok(json) => {
                    // Log package count instead of full list
                    if let Some(packages) = json.get("packages").and_then(|p| p.as_array()) {
                        debug!("[Packages] Successfully parsed JSON from Julia: {} packages", packages.len());
                    } else {
                        debug!("[Packages] Successfully parsed JSON from Julia");
                    }
                    Ok(json)
                },
                Err(e) => {
                    error!("[Packages] Failed to parse JSON from Julia: {}", e);
                    // Only log preview of raw result on error
                    let error_preview = if result.len() > 500 {
                        format!("{}... ({} chars)", &result[..500], result.len())
                    } else {
                        result
                    };
                    error!("[Packages] Raw result preview: {}", error_preview);
                    Ok(serde_json::json!({"packages": [], "active_project": null}))
                },
            }
        },
        Err(e) => {
            error!("[Packages] Failed to get package status: {}", e);
            Ok(serde_json::json!({"packages": [], "active_project": null}))
        }
    }
}

#[tauri::command]
pub async fn clean_transitive_dependencies(app_state: State<'_, AppState>) -> Result<(), AppError> {
    debug!("[Packages] Clean transitive dependencies");
    let code = r#"
        try
            # Use local scope to avoid polluting global namespace
            let
                local Pkg = Base.require(Base.PkgId(Base.UUID("44cfe95a-1eb2-52ea-b672-e2afdf69b78f"), "Pkg"))
                
                deps = Pkg.dependencies()
                transitive_packages = []
                direct_deps = Set{String}()
                try
                    current_project = Pkg.project()
                    project_toml = Pkg.TOML.parsefile(joinpath(dirname(current_project.path), "Project.toml"))
                    if haskey(project_toml, "deps")
                        for (name, uuid) in project_toml["deps"]
                            push!(direct_deps, name)
                        end
                    end
                catch e
                end
                for (uuid, dep) in deps
                    if !(dep.name in direct_deps)
                        push!(transitive_packages, dep.name)
                    end
                end
                for pkg in transitive_packages
                    try
                        Pkg.remove(pkg)
                    catch e
                    end
                end
                Pkg.instantiate()
                "Transitive dependencies cleaned successfully"
            end
        catch e
            "Failed to clean transitive dependencies: " * string(e)
        end
    "#;
    use internals::messages::execution::ExecuteApiRequest;
    match app_state.actor_system.execution_actor.send(ExecuteApiRequest { code: code.to_string() }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(result) => if result.contains("cleaned successfully") { Ok(()) } else { Err(AppError::InternalError(result)) },
        Err(e) => Err(AppError::InternalError(format!("Failed to clean transitive dependencies: {}", e))),
    }
}

#[tauri::command]
pub async fn search_julia_packages(query: String, app_state: State<'_, AppState>) -> Result<serde_json::Value, AppError> {
    debug!("[Packages] Search Julia packages: {}", query);
    let code = format!(r#"
        try
            # Use local scope to avoid polluting global namespace
            let
                local Pkg = Base.require(Base.PkgId(Base.UUID("44cfe95a-1eb2-52ea-b672-e2afdf69b78f"), "Pkg"))
                local JSON = Base.require(Base.PkgId(Base.UUID("682c06a0-de6a-54ab-a142-c8b1cf79cde6"), "JSON"))
                
                # Use Pkg.available() which is more reliable across Julia versions
                available_packages = Pkg.available()
                packages = []
                
                for pkg_name in available_packages
                    pkg_str = string(pkg_name)
                    if occursin("{}", lowercase(pkg_str))
                        push!(packages, Dict(
                            "name" => pkg_str,
                            "description" => "Package available in registry",
                            "version" => "unknown",
                            "uuid" => "unknown"
                        ))
                    end
                end
                
                # Sort by name and limit results
                sort!(packages, by = x -> x["name"])
                packages = packages[1:min(20, length(packages))]
                
                JSON.json(Dict("packages" => packages))
            end
        catch e
            "Failed to search packages: " * string(e)
        end
    "#, query);
    use internals::messages::execution::ExecuteApiRequest;
    match app_state.actor_system.execution_actor.send(ExecuteApiRequest { code }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(result) => match serde_json::from_str::<serde_json::Value>(&result) {
            Ok(v) => Ok(v),
            Err(_) => Ok(serde_json::json!({"packages":[]})),
        },
        Err(_) => Ok(serde_json::json!({"packages":[]})),
    }
}

#[tauri::command]
pub async fn run_julia_pkg_command(command: String, app_state: State<'_, AppState>) -> Result<String, AppError> {
    debug!("[Packages] Run pkg command: {}", command);
    let code = format!(r#"
        try
            # Use local scope to avoid polluting global namespace
            let
                local Pkg = Base.require(Base.PkgId(Base.UUID("44cfe95a-1eb2-52ea-b672-e2afdf69b78f"), "Pkg"))
                local JSON = Base.require(Base.PkgId(Base.UUID("682c06a0-de6a-54ab-a142-c8b1cf79cde6"), "JSON"))
                
                # Execute package operations directly
                
                cmd_parts = split("{}", " ")
                operation = cmd_parts[1]
                package_name = length(cmd_parts) > 1 ? cmd_parts[2] : ""
                
                # Execute package operations
                if operation == "add" && !isempty(package_name)
                    Pkg.add(package_name)
                    result = Dict("success" => true, "message" => "Successfully added package: " * package_name, "stdout" => "Package added successfully", "stderr" => "")
                elseif (operation == "remove" || operation == "rm") && !isempty(package_name)
                    Pkg.rm(package_name)
                    result = Dict("success" => true, "message" => "Successfully removed package: " * package_name, "stdout" => "Package removed successfully", "stderr" => "")
                elseif operation == "update"
                    Pkg.update()
                    result = Dict("success" => true, "message" => "Successfully updated packages", "stdout" => "Packages updated successfully", "stderr" => "")
                elseif operation == "instantiate"
                    Pkg.instantiate()
                    result = Dict("success" => true, "message" => "Successfully instantiated project", "stdout" => "Project instantiated successfully", "stderr" => "")
                else
                    result = Dict("success" => false, "message" => "Unknown or invalid command: " * "{}", "stdout" => "", "stderr" => "Invalid command")
                end
                
                
                JSON.json(result)
            end
        catch e
            error_msg = "Failed to execute Pkg command: " * string(e)
            JSON.json(Dict("success" => false, "message" => error_msg, "stdout" => "", "stderr" => error_msg))
        end
    "#, command, command);
    use internals::messages::execution::ExecuteApiRequest;
    match app_state.actor_system.execution_actor.send(ExecuteApiRequest { code }).await.map_err(|_| AppError::InternalError("Actor comm failed".to_string()))? {
        Ok(result) => Ok(result),
        Err(e) => Err(AppError::InternalError(format!("Failed to execute Pkg command: {}", e))),
    }
}

// ============================================================================
// SearchPackages Integration
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchPackagesResult {
    pub name: String,
    pub uuid: String,
    pub repository_url: Option<String>,
    pub description: Option<String>,
    pub stars: Option<u32>,
    pub topics: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchPackagesResponse {
    pub packages: Vec<SearchPackagesResult>,
    pub total: usize,
    pub query: String,
}
