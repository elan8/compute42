// Message handlers for InstallationActor

use actix::prelude::*;
use log::{debug, error};

use crate::messages::installation::*;
use crate::messages::installation::GetJuliaPathFromInstallation;
use crate::messages::coordination::JuliaInstallationCompleted;
use crate::messages::orchestrator::{StartupEventMessage, StartupEvent};
use crate::types::JuliaInstallation;

use super::InstallationActor;
use super::discovery::{find_julia_executable, verify_julia_executable, get_julia_version_from_executable};
use super::installation::install_julia;

impl Handler<CheckJuliaInstallation> for InstallationActor {
    type Result = ResponseActFuture<Self, Result<bool, String>>;
    
    fn handle(&mut self, _msg: CheckJuliaInstallation, _ctx: &mut Context<Self>) -> Self::Result {
        Box::pin(
            async move {
                debug!("InstallationActor: Handling CheckJuliaInstallation message");
                
                // Use our own find_julia_executable function to check if Julia is available
                match find_julia_executable(None).await {
                    Ok(julia_path) => {
                        debug!("InstallationActor: Found Julia executable at: {}", julia_path);
                        
                        // Verify the executable actually works by running julia --version
                        if verify_julia_executable(&julia_path).await {
                            debug!("InstallationActor: Julia installation verified successfully");
                            Ok((true, Some(julia_path)))
                        } else {
                            debug!("InstallationActor: Julia executable found but verification failed");
                            Ok((false, None))
                        }
                    }
                    Err(e) => {
                        debug!("InstallationActor: Julia installation not found: {}", e);
                        Ok((false, None))
                    }
                }
            }
            .into_actor(self)
            .map(|res, actor, _| {
                match res {
                    Ok((is_installed, julia_path)) => {
                        // Store the Julia path locally in the actor state
                        if let Some(path) = julia_path {
                            actor.julia_path = Some(path.clone());
                            debug!("InstallationActor: Julia path stored locally");
                        }
                        debug!("InstallationActor: Successfully returning Julia installation status: {}", is_installed);
                        Ok(is_installed)
                    }
                    Err(e) => {
                        error!("InstallationActor: Error checking Julia installation: {}", e);
                        Err(e)
                    }
                }
            })
        )
    }
}

impl Handler<InstallJulia> for InstallationActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: InstallJulia, ctx: &mut Context<Self>) -> Self::Result {
        debug!("InstallationActor: Received InstallJulia message with version: {:?}", msg.julia_version);
        
        let installation_in_progress = self.installation_in_progress.clone();
        let julia_version = msg.julia_version;
        let orchestrator_actor = self.orchestrator_actor.clone();
        let event_manager = self.event_manager.clone();
        
        let future = async move {
            match install_julia(julia_version.as_deref(), installation_in_progress.clone(), Some(&event_manager)).await {
                Ok(()) => {
                    debug!("InstallationActor: Julia installation completed successfully");
                    
                    // Get the Julia installation info and send completion message to orchestrator
                    if let Ok(julia_path) = find_julia_executable(None).await {
                        let version = julia_version.unwrap_or_else(|| crate::version::get_julia_version().to_string());
                        
                        if let Some(orchestrator) = orchestrator_actor {
                            let _ = orchestrator.send(JuliaInstallationCompleted {
                                installation_path: julia_path.clone(),
                                version,
                            }).await;
                        }
                    } else {
                        error!("InstallationActor: Failed to get Julia path after installation completion");
                    }
                }
                Err(e) => {
                    error!("InstallationActor: Julia installation failed: {}", e);
                    // Note: We can't return this error directly since we're in a spawned task
                    // The orchestrator will detect the failure by checking installation status
                }
            }
        };
        
        ctx.spawn(future.into_actor(self).map(|_, _actor, _| {
            // Installation progress flag is managed inside install_julia
        }));
        Ok(())
    }
}

impl Handler<SetOrchestratorActor> for InstallationActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: SetOrchestratorActor, _ctx: &mut Context<Self>) -> Self::Result {
        self.orchestrator_actor = Some(msg.orchestrator_actor);
        Ok(())
    }
}

impl Handler<GetJuliaVersion> for InstallationActor {
    type Result = ResponseActFuture<Self, Result<Option<String>, String>>;
    
    fn handle(&mut self, _msg: GetJuliaVersion, _ctx: &mut Context<Self>) -> Self::Result {
        let julia_path = self.julia_path.clone();
        Box::pin(
            async move {
                if let Some(path) = julia_path {
                    match get_julia_version_from_executable(&path).await {
                        Ok(version) => Ok(Some(version)),
                        Err(_) => Ok(None),
                    }
                } else {
                    // Try to find Julia and get version
                    match find_julia_executable(None).await {
                        Ok(path) => {
                            match get_julia_version_from_executable(&path).await {
                                Ok(version) => Ok(Some(version)),
                                Err(_) => Ok(None),
                            }
                        }
                        Err(_) => Ok(None),
                    }
                }
            }
            .into_actor(self)
            .map(|res, actor, _| {
                if let Ok(Some(version)) = &res {
                    // Update current installation if we found a version
                    if let Some(path) = &actor.julia_path {
                        actor.current_installation = Some(JuliaInstallation {
                            path: path.clone(),
                            version: version.clone(),
                            is_valid: true,
                        });
                    }
                }
                res
            })
        )
    }
}

impl Handler<IsInstallationInProgress> for InstallationActor {
    type Result = ResponseActFuture<Self, Result<bool, String>>;
    
    fn handle(&mut self, _msg: IsInstallationInProgress, _ctx: &mut Context<Self>) -> Self::Result {
        let installation_in_progress = self.installation_in_progress.clone();
        Box::pin(
            async move {
                Ok(*installation_in_progress.lock().await)
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<GetJuliaExecutablePath> for InstallationActor {
    type Result = Result<Option<String>, String>;
    
    fn handle(&mut self, _msg: GetJuliaExecutablePath, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(self.current_installation.as_ref().map(|i| i.path.clone()).or_else(|| self.julia_path.clone()))
    }
}

impl Handler<GetJuliaPathFromInstallation> for InstallationActor {
    type Result = ResponseActFuture<Self, Result<Option<String>, String>>;
    
    fn handle(&mut self, _msg: GetJuliaPathFromInstallation, _ctx: &mut Context<Self>) -> Self::Result {
        Box::pin(
            async move {
                debug!("InstallationActor: Getting Julia path from installation");
                match find_julia_executable(None).await {
                    Ok(path) => {
                        debug!("InstallationActor: Julia path found: {}", path);
                        Ok(Some(path))
                    }
                    Err(e) => {
                        debug!("InstallationActor: Julia path not found: {}", e);
                        Ok(None)
                    }
                }
            }
            .into_actor(self)
            .map(|res, actor, _| {
                if let Ok(Some(path)) = &res {
                    actor.julia_path = Some(path.clone());
                }
                res
            })
        )
    }
}

impl Handler<EnsureJuliaInstalled> for InstallationActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: EnsureJuliaInstalled, ctx: &mut Context<Self>) -> Self::Result {
        debug!("InstallationActor: Received EnsureJuliaInstalled message");
        
        let orchestrator_addr = msg.orchestrator_addr;
        let installation_in_progress = self.installation_in_progress.clone();
        let event_manager = self.event_manager.clone();
        
        ctx.spawn(
            async move {
                // Check if Julia is installed
                match find_julia_executable(None).await {
                    Ok(julia_path) => {
                        // Verify the executable actually works
                        if verify_julia_executable(&julia_path).await {
                            debug!("InstallationActor: Julia is already installed and verified");
                            // Emit completion event
                            orchestrator_addr.do_send(StartupEventMessage {
                                event: StartupEvent::JuliaCheckComplete,
                            });
                            return;
                        } else {
                            debug!("InstallationActor: Julia executable found but verification failed, will reinstall");
                        }
                    }
                    Err(e) => {
                        debug!("InstallationActor: Julia installation not found: {}, will install", e);
                    }
                }
                
                // Julia not installed or invalid, install it
                debug!("InstallationActor: Starting Julia installation");
                
                match install_julia(None, installation_in_progress.clone(), Some(&event_manager)).await {
                    Ok(()) => {
                        debug!("InstallationActor: Julia installation completed successfully");
                        // Emit completion event
                        orchestrator_addr.do_send(StartupEventMessage {
                            event: StartupEvent::JuliaCheckComplete,
                        });
                    }
                    Err(e) => {
                        error!("InstallationActor: Failed to install Julia: {}", e);
                        // Emit failure event
                        orchestrator_addr.do_send(StartupEventMessage {
                            event: StartupEvent::StartupFailed(format!("Failed to install Julia: {}", e)),
                        });
                    }
                }
            }
            .into_actor(self)
            .map(|_, _actor, _| {})
        );
        
        Ok(())
    }
}



















