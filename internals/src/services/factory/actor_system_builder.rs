// Actor system builder implementation
// This builder handles the construction of the ActorSystem with all services

use std::sync::Arc;
use log::{debug, error};

use crate::actor_system::ActorSystem;
use crate::services::events::EventService;

/// Builder for creating and initializing the ActorSystem
pub struct ActorSystemBuilder {
    event_emitter: Arc<dyn crate::service_traits::EventEmitter>,
    environment_config: crate::config::EnvironmentConfig,
}

impl ActorSystemBuilder {
    /// Create a new actor system builder
    pub fn new(event_emitter: Arc<dyn crate::service_traits::EventEmitter>) -> Self {
        // Create EnvironmentConfig once (singleton pattern)
        let environment_config = crate::config::EnvironmentConfig::from_env();
        
        Self {
            event_emitter,
            environment_config,
        }
    }

    /// Create a new actor system builder with custom config
    pub fn with_config(
        event_emitter: Arc<dyn crate::service_traits::EventEmitter>,
        config: crate::config::EnvironmentConfig,
    ) -> Self {
        Self {
            event_emitter,
            environment_config: config,
        }
    }

    /// Build and initialize the ActorSystem
    pub async fn build(&self) -> Result<Arc<ActorSystem>, String> {
        debug!("[ActorSystemBuilder] Building ActorSystem");
        
        // Initialize app start time tracking
        crate::app_time::init_app_start_time();
        
        // Create event manager (only shared service needed)
        let event_manager = EventService::new(self.event_emitter.clone());
        
        // Create the ActorSystem - all services are now constructed internally by actors
        let actor_system = ActorSystem::new(
            event_manager,
            self.environment_config.clone(),
        ).await;

        // Initialize the ActorSystem
        debug!("[ActorSystemBuilder] Initializing ActorSystem");
        if let Err(e) = actor_system.initialize().await {
            error!("[ActorSystemBuilder] Failed to initialize ActorSystem: {}", e);
            return Err(format!("Failed to initialize ActorSystem: {}", e));
        }

        debug!("[ActorSystemBuilder] ActorSystem built and initialized successfully");
        Ok(Arc::new(actor_system))
    }

    /// Build the ActorSystem in a blocking context (for use in non-async contexts)
    pub fn build_blocking(&self) -> Result<Arc<ActorSystem>, String> {
        debug!("[ActorSystemBuilder] Building ActorSystem in blocking context");
        
        // Use tokio runtime to run the async build function
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;
        
        rt.block_on(self.build())
    }
}

/// Convenience function to create an ActorSystem with default configuration
pub async fn create_actor_system(
    event_emitter: Arc<dyn crate::service_traits::EventEmitter>,
) -> Result<Arc<ActorSystem>, String> {
    let builder = ActorSystemBuilder::new(event_emitter);
    builder.build().await
}

/// Convenience function to create an ActorSystem with custom configuration
pub async fn create_actor_system_with_config(
    event_emitter: Arc<dyn crate::service_traits::EventEmitter>,
    config: crate::config::EnvironmentConfig,
) -> Result<Arc<ActorSystem>, String> {
    let builder = ActorSystemBuilder::with_config(event_emitter, config);
    builder.build().await
}

/// Convenience function to create an ActorSystem in a blocking context
pub fn create_actor_system_blocking(
    event_emitter: Arc<dyn crate::service_traits::EventEmitter>,
) -> Result<Arc<ActorSystem>, String> {
    let builder = ActorSystemBuilder::new(event_emitter);
    builder.build_blocking()
}
