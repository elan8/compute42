// File persistence operations for ConfigurationActor
// Handles loading and saving configuration to/from files

use std::sync::Arc;
use log::debug;
use crate::types::UserPreferences;
use crate::services::persistence::persistence_service::FilePersistenceServiceImpl;
use crate::service_traits::FilePersistenceService;

/// File persistence helper for configuration
pub struct PersistenceHelper {
    persistence_service: Arc<FilePersistenceServiceImpl>,
}

impl PersistenceHelper {
    /// Create a new persistence helper
    pub fn new() -> Result<Self, String> {
        let persistence_service = Arc::new(
            FilePersistenceServiceImpl::new()
                .map_err(|e| format!("Failed to create persistence service: {}", e))?
        );
        
        Ok(Self {
            persistence_service,
        })
    }
    
    /// Load configuration from file
    pub async fn load_config_from_file(&self) -> Result<UserPreferences, String> {
        match (&*self.persistence_service as &dyn FilePersistenceService).load_json_value("app_config").await {
            Ok(json_value) => {
                UserPreferences::from_json_value(json_value)
                    .map_err(|e| format!("Failed to deserialize configuration: {}", e))
            }
            Err(e) => {
                // If file doesn't exist or can't be loaded, return default configuration
                debug!("PersistenceHelper: Configuration file not found or error loading: {}, using defaults", e);
                Ok(UserPreferences::default())
            }
        }
    }
    
    /// Save configuration to file
    pub async fn save_config_to_file(&self, config: &UserPreferences) -> Result<(), String> {
        let json_value = serde_json::to_value(config)
            .map_err(|e| format!("Failed to serialize configuration: {}", e))?;
        
        (&*self.persistence_service as &dyn FilePersistenceService).save_json_value("app_config", &json_value).await
    }
    
    /// Check if configuration file exists
    pub async fn config_file_exists(&self) -> bool {
        (&*self.persistence_service as &dyn FilePersistenceService).exists("app_config").await
    }
}

