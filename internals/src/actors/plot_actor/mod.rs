// PlotActor - manages plot server lifecycle and plot data
// This actor handles all plot-related functionality including HTTP server, storage, and filtering

mod filters;
mod handlers;
mod server;
mod storage;

use actix::prelude::*;
use async_trait::async_trait;
use std::sync::Arc;
use log::{debug, error, warn};

use crate::messages::plot::*;
use crate::services::events::EventService;
use crate::service_traits::{PlotService as PlotServiceTrait, EventEmitter};
use crate::types::PlotServerInfo;

use self::filters::should_filter_plot_data;
use self::server::PlotServer;
use self::storage::PlotStorage;

/// PlotActor - manages plot server lifecycle and plot data
/// This replaces the mutex-based PlotServer with a clean actor model
#[derive(Clone)]
pub struct PlotActor {
    // Actor state
    is_running: bool,
    server_info: Option<PlotServerInfo>,
    plots: Vec<PlotData>,
    current_port: Option<u16>,
    
    // Internal services
    plot_storage: PlotStorage,
    plot_server: PlotServer,
    event_emitter: Arc<dyn EventEmitter>,
    event_manager: EventService,
}

impl PlotActor {
    /// Create a new PlotActor instance
    pub fn new(
        event_emitter: Arc<dyn EventEmitter>,
        event_manager: EventService,
    ) -> Self {
        let plot_server = PlotServer::new(event_emitter.clone());
        
        Self {
            is_running: false,
            server_info: None,
            plots: Vec::new(),
            current_port: None,
            plot_storage: PlotStorage::new(),
            plot_server,
            event_emitter,
            event_manager,
        }
    }

    /// Handle plot data received from CommunicationManager
    async fn handle_plot_data_received_internal(&self, plot_data_json: serde_json::Value) -> Result<(), String> {
        debug!("[PlotActor] Handling plot data received from CommunicationManager");

        // Extract fields from the JSON
        let id = plot_data_json["id"]
            .as_str()
            .ok_or("Missing plot id")?
            .to_string();
        let mime_type = plot_data_json["mime_type"]
            .as_str()
            .unwrap_or("image/svg+xml")
            .to_string();
        let data = plot_data_json["data"]
            .as_str()
            .ok_or("Missing plot data")?
            .to_string();
        let timestamp = plot_data_json["timestamp"].as_u64().unwrap_or(0);

        // Filter out empty or meaningless plot data
        if should_filter_plot_data(&data, &mime_type) {
            debug!("[PlotActor] Filtering out empty/meaningless plot data: id={}, mime_type={}, data_length={}", 
                   id, mime_type, data.len());
            return Ok(());
        }

        // Create PlotData struct
        let plot_data_struct = PlotData {
            id: id.clone(),
            mime_type: mime_type.clone(),
            data: data.clone(),
            timestamp: timestamp as i64,
            title: plot_data_json["title"].as_str().map(|s| s.to_string()),
            description: plot_data_json["description"].as_str().map(|s| s.to_string()),
            source_file: plot_data_json["source_file"].as_str().map(|s| s.to_string()),
            line_number: plot_data_json["line_number"].as_u64().map(|n| n as u32),
            code_context: plot_data_json["code_context"].as_str().map(|s| s.to_string()),
            session_id: plot_data_json["session_id"].as_str().map(|s| s.to_string()),
        };

        debug!("[PlotActor] Processing plot: {}", id);

        // Add plot to storage
        self.plot_storage.insert(plot_data_struct.clone()).await;

        // Get the plot port for URL generation - only create URL if server is running
        let port_opt = self.plot_server.get_port().await;

        // Create frontend plot data
        let frontend_plot_data = if let Some(port) = port_opt {
            // Server is running - create HTTP URL
            let image_url = format!("http://127.0.0.1:{}/plots/{}/image", port, id);
            serde_json::json!({
                "id": id,
                "mime_type": mime_type,
                "image_url": image_url,
                "timestamp": timestamp,
                "title": plot_data_json["title"],
                "description": plot_data_json["description"],
                "source_file": plot_data_json["source_file"],
                "line_number": plot_data_json["line_number"],
                "code_context": plot_data_json["code_context"],
                "session_id": plot_data_json["session_id"]
            })
        } else {
            // Server not running - use data URL fallback (frontend will handle this)
            debug!("[PlotActor] Cannot create plot URL: server not running, plot data stored with base64 fallback");
            serde_json::json!({
                "id": id,
                "mime_type": mime_type,
                "data": data,
                "timestamp": timestamp,
                "title": plot_data_json["title"],
                "description": plot_data_json["description"],
                "source_file": plot_data_json["source_file"],
                "line_number": plot_data_json["line_number"],
                "code_context": plot_data_json["code_context"],
                "session_id": plot_data_json["session_id"]
            })
        };

        // Emit unified plot event to frontend
        let plot_event_payload = serde_json::json!({
            "plot_id": id,
            "plot_data": frontend_plot_data
        });

        // Emit unified event
        match self.event_emitter.emit("plot:plot-added", plot_event_payload.clone()).await {
            Ok(_) => {}
            Err(_e) => {}
        }

        // Also emit legacy event for backward compatibility
        let legacy_plot_event = serde_json::json!({
            "event_type": "PlotCreated",
            "plot_data": frontend_plot_data
        });

        match self.event_emitter.emit("julia-plot", legacy_plot_event).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to emit legacy plot event: {}", e))
        }
    }

    /// Update all plot URLs when server restarts on a new port
    async fn update_plot_urls_for_new_port_internal(&self, new_port: u16) -> Result<(), String> {
        let plots = self.plot_storage.get_all().await;

        if plots.is_empty() {
            debug!("[PlotActor] Server restarted on port {}, no plots to update", new_port);
            return Ok(());
        }

        debug!("[PlotActor] Server restarted on port {}, updating {} plots and re-emitting events", new_port, plots.len());

        // Update each plot's URL and re-emit update events
        for plot in plots.iter() {
            // Create the new URL with the new port
            let image_url = format!("http://127.0.0.1:{}/plots/{}/image", new_port, plot.id);
            
            // Create frontend plot data with new URL
            let frontend_plot_data = serde_json::json!({
                "id": plot.id,
                "mime_type": plot.mime_type,
                "image_url": image_url,
                "timestamp": plot.timestamp,
                "title": plot.title,
                "description": plot.description,
                "source_file": plot.source_file,
                "line_number": plot.line_number,
                "code_context": plot.code_context,
                "session_id": plot.session_id
            });

            // Emit plot-updated event with new URL
            let plot_updated_payload = serde_json::json!({
                "plot_id": plot.id,
                "plot_data": frontend_plot_data
            });

            if let Err(e) = self.event_emitter.emit("plot:plot-updated", plot_updated_payload.clone()).await {
                error!("[PlotActor] Failed to emit plot-updated event for plot {}: {}", plot.id, e);
            } else {
                debug!("[PlotActor] Emitted plot-updated event for plot {} with new URL", plot.id);
            }

            // Also emit legacy event for backward compatibility
            let legacy_plot_event = serde_json::json!({
                "event_type": "PlotUpdated",
                "plot_data": frontend_plot_data
            });

            if let Err(e) = self.event_emitter.emit("julia-plot", legacy_plot_event).await {
                error!("[PlotActor] Failed to emit legacy plot-updated event for plot {}: {}", plot.id, e);
            }
        }

        Ok(())
    }
}

// Implement PlotService trait for PlotActor
#[async_trait]
impl PlotServiceTrait for PlotActor {
    async fn start_plot_server(&self) -> Result<u16, String> {
        // Find an available port by actually binding to it (atomic operation)
        let (port, listener) = self.plot_server.find_available_port().await?;

        // Get plots storage for the server
        let plots = self.plot_storage.clone_plots();

        // Start the HTTP server with the pre-bound listener
        self.plot_server.start_with_listener(port, listener, plots).await?;

        // Store the port
        self.plot_server.set_port(port).await;
        Ok(port)
    }

    async fn stop_plot_server(&self) -> Result<(), String> {
        self.plot_server.stop().await?;
        self.plot_server.clear_port().await;
        Ok(())
    }

    async fn get_plot_port(&self) -> Option<u16> {
        self.plot_server.get_port().await
    }

    async fn is_plot_server_running(&self) -> bool {
        self.plot_server.get_port().await.is_some()
    }

    async fn add_plot(&self, plot_data: PlotData) -> Result<(), String> {
        self.plot_storage.insert(plot_data).await;
        Ok(())
    }

    async fn get_plots(&self) -> Result<Vec<PlotData>, String> {
        let plots = self.plot_storage.get_all().await;
        let port_opt = self.plot_server.get_port().await;
        
        // Log warning if plots exist but server is not running (plots won't have image URLs)
        if !plots.is_empty() && port_opt.is_none() {
            warn!(
                "[PlotActor] Retrieved {} plots but plot server is not running - plots will not have image URLs",
                plots.len()
            );
        } else if !plots.is_empty() {
            debug!(
                "[PlotActor] Retrieved {} plots, plot server running on port {:?}",
                plots.len(),
                port_opt
            );
        }
        
        Ok(plots)
    }

    async fn delete_plot(&self, plot_id: String) -> Result<(), String> {
        self.plot_storage.remove(&plot_id).await;
        Ok(())
    }

    async fn handle_plot_data_received(&self, plot_data_json: serde_json::Value) -> Result<(), String> {
        self.handle_plot_data_received_internal(plot_data_json).await
    }

    async fn clear_plots(&self) -> Result<(), String> {
        self.plot_storage.clear().await;
        Ok(())
    }

    async fn save_plot_to_file(&self, plot: &PlotData, file_path: &str) -> Result<(), String> {
        let json = serde_json::to_string_pretty(plot)
            .map_err(|e| format!("Failed to serialize plot: {}", e))?;
        tokio::fs::write(file_path, json)
            .await
            .map_err(|e| format!("Failed to write plot file: {}", e))
    }

    async fn update_plot(&self, plot: &PlotData) -> Result<(), String> {
        self.plot_storage.insert(plot.clone()).await;
        Ok(())
    }

    async fn update_plot_urls_for_new_port(&self, new_port: u16) -> Result<(), String> {
        self.update_plot_urls_for_new_port_internal(new_port).await
    }
}

impl Actor for PlotActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        ctx.set_mailbox_capacity(64);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("PlotActor: Actor stopped");
    }
}

// Message handlers
impl Handler<StartPlotServer> for PlotActor {
    type Result = ResponseActFuture<Self, Result<u16, String>>;
    
    fn handle(&mut self, msg: StartPlotServer, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("PlotActor: Received StartPlotServer message");
        let plot_actor = self.clone();
        let event_manager = self.event_manager.clone();
        let orchestrator_addr = msg.orchestrator_addr;
        let is_restart = self.current_port.is_some();
        let previous_port = self.current_port;
        
        Box::pin(
            async move {
                match plot_actor.start_plot_server().await {
                    Ok(port) => {
                        // Update URLs if this is a restart
                        if is_restart && previous_port != Some(port) {
                            debug!("PlotActor: Server restarting from port {:?} to {}", previous_port, port);
                            plot_actor.update_plot_urls_for_new_port(port).await.ok();
                            event_manager.emit_plot_server_restarted(port).await.ok();
                        } else {
                            event_manager.emit_plot_server_started(port).await.ok();
                        }
                        
                        // Emit StartupEvent to orchestrator
                        if let Some(ref orchestrator) = orchestrator_addr {
                            orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                event: crate::messages::orchestrator::StartupEvent::PlotServerStarted,
                            });
                        }
                        
                        Ok((port, is_restart))
                    }
                    Err(e) => {
                        error!("PlotActor: Failed to start plot server: {}", e);
                        if let Some(ref orchestrator) = orchestrator_addr {
                            orchestrator.do_send(crate::messages::orchestrator::StartupEventMessage {
                                event: crate::messages::orchestrator::StartupEvent::StartupFailed(format!("Failed to start plot server: {}", e)),
                            });
                        }
                        Err(e)
                    }
                }
            }
            .into_actor(self)
            .map(|res, actor, _| {
                match res {
                    Ok((port, was_restart)) => {
                        actor.is_running = true;
                        actor.current_port = Some(port);
                        actor.server_info = Some(PlotServerInfo { port, is_running: true, plots: Vec::new() });
                        if was_restart {
                            debug!("PlotActor: Plot server restarted successfully on port {}", port);
                        } else {
                            debug!("PlotActor: Plot server started successfully on port {}", port);
                        }
                        Ok(port)
                    }
                    Err(e) => Err(e),
                }
            })
        )
    }
}

impl Handler<StopPlotServer> for PlotActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: StopPlotServer, ctx: &mut Context<Self>) -> Self::Result {
        debug!("PlotActor: Received StopPlotServer message");
        
        // Spawn async operation
        let plot_actor = self.clone();
        
        ctx.spawn(
            async move {
                debug!("PlotActor: Stopping plot server in async task");
                match plot_actor.stop_plot_server().await {
                    Ok(_) => {
                        debug!("PlotActor: Plot server stopped successfully");
                    }
                    Err(e) => {
                        error!("PlotActor: Failed to stop plot server: {}", e);
                    }
                }
            }
            .into_actor(self)
        );
        
        Ok(())
    }
}

impl Handler<GetPlotPort> for PlotActor {
    type Result = Result<Option<u16>, String>;
    
    fn handle(&mut self, _msg: GetPlotPort, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(self.current_port)
    }
}

impl Handler<AddPlot> for PlotActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: AddPlot, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("PlotActor: Received AddPlot message for plot {}", msg.plot_data.id);
        let plot_actor = self.clone();
        let event_manager = self.event_manager.clone();
        let plot_data = msg.plot_data.clone();
        
        Box::pin(
            async move {
                plot_actor.add_plot(plot_data.clone()).await?;
                event_manager.emit_plot_added(&plot_data.id).await.ok();
                Ok(())
            }
            .into_actor(self)
            .map(|res, actor, _| {
                match res {
                    Ok(_) => {
                        actor.plots.push(msg.plot_data);
                        debug!("PlotActor: Plot added successfully");
                        Ok(())
                    }
                    Err(e) => {
                        error!("PlotActor: Failed to add plot: {}", e);
                        Err(e)
                    }
                }
            })
        )
    }
}

impl Handler<GetPlots> for PlotActor {
    type Result = ResponseActFuture<Self, Result<Vec<PlotData>, String>>;
    
    fn handle(&mut self, _msg: GetPlots, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("PlotActor: Received GetPlots message");
        let plot_actor = self.clone();
        
        Box::pin(
            async move {
                plot_actor.get_plots().await
            }
            .into_actor(self)
            .map(|res, actor, _| {
                match res {
                    Ok(plots) => {
                        debug!("PlotActor: Retrieved {} plots", plots.len());
                        // Update local state for consistency
                        actor.plots = plots.clone();
                        Ok(plots)
                    }
                    Err(e) => {
                        error!("PlotActor: Failed to get plots: {}", e);
                        Err(e)
                    }
                }
            })
        )
    }
}

impl Handler<DeletePlot> for PlotActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: DeletePlot, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("PlotActor: Received DeletePlot message for plot {}", msg.plot_id);
        let plot_actor = self.clone();
        let event_manager = self.event_manager.clone();
        let plot_id = msg.plot_id.clone();
        
        Box::pin(
            async move {
                plot_actor.delete_plot(plot_id.clone()).await?;
                event_manager.emit_plot_deleted(&plot_id).await.ok();
                Ok(())
            }
            .into_actor(self)
            .map(move |res, actor, _| {
                match res {
                    Ok(_) => {
                        // Remove plot from actor's local state
                        actor.plots.retain(|p| p.id != msg.plot_id);
                        debug!("PlotActor: Plot {} deleted successfully", msg.plot_id);
                        Ok(())
                    }
                    Err(e) => {
                        error!("PlotActor: Failed to delete plot {}: {}", msg.plot_id, e);
                        Err(e)
                    }
                }
            })
        )
    }
}

impl Handler<HandlePlotDataReceived> for PlotActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: HandlePlotDataReceived, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("PlotActor: Received HandlePlotDataReceived message");
        let plot_actor = self.clone();
        
        Box::pin(
            async move {
                plot_actor.handle_plot_data_received(msg.plot_data_json).await?;
                Ok(())
            }
            .into_actor(self)
            .map(|res, _actor, _| {
                match res {
                    Ok(_) => {
                        debug!("PlotActor: Plot data received and processed successfully");
                        Ok(())
                    }
                    Err(e) => {
                        error!("PlotActor: Failed to handle plot data: {}", e);
                        Err(e)
                    }
                }
            })
        )
    }
}
