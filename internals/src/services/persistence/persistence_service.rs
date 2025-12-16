use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use log::debug;

use crate::service_traits::FilePersistenceService;


/// File-based persistence service implementation
pub struct FilePersistenceServiceImpl {
    base_dir: PathBuf,
    cache: Arc<Mutex<HashMap<String, String>>>,
}

impl FilePersistenceServiceImpl {
    /// Create a new file persistence service
    pub fn new() -> Result<Self, String> {
        // Check for test data directory environment variable (for integration tests)
        let base_dir = if let Ok(test_dir) = std::env::var("COMPUTE42_TEST_DATA_DIR") {
            let test_path = PathBuf::from(test_dir).join("config");
            test_path
        } else {
            let app_data_dir = dirs::data_local_dir()
                .ok_or("Could not determine app data directory")?
                .join("com.compute42.dev")
                .join("config");
            app_data_dir
        };
        
        // Ensure the directory exists
        fs::create_dir_all(&base_dir)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;
        
        Ok(Self {
            base_dir,
            cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create a new file persistence service that stores data in the provided base directory
    pub fn new_in_dir<P: AsRef<Path>>(base_dir: P) -> Result<Self, String> {
        let base_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
        Ok(Self {
            base_dir,
            cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    
    /// Get the full path for a key
    fn get_file_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", key))
    }
}

#[async_trait]
impl FilePersistenceService for FilePersistenceServiceImpl {
    async fn load_json_value(&self, key: &str) -> Result<serde_json::Value, String> {
        let file_path = self.get_file_path(key);
        debug!("FilePersistenceService: Loading configuration from file: {}", file_path.display());
        
        // Check cache first
        {
            let cache_guard = self.cache.lock().await;
            if let Some(json_str) = cache_guard.get(key) {
                debug!("FilePersistenceService: Using cached data for key: {}", key);
                return serde_json::from_str(json_str)
                    .map_err(|e| format!("Failed to deserialize cached data for {}: {}", key, e));
            }
        }
        
        // Load from file
        if !file_path.exists() {
            debug!("FilePersistenceService: Configuration file does not exist: {}", file_path.display());
            return Ok(serde_json::Value::Null);
        }
        
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read file {}: {}", file_path.display(), e))?;
        
        // Handle empty files gracefully by returning default
        if content.trim().is_empty() {            return Ok(serde_json::Value::Null);
        }        let data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to deserialize data for {}: {}", key, e))?;
        
        // Cache the result
        {
            let mut cache_guard = self.cache.lock().await;
            cache_guard.insert(key.to_string(), content);
        }
        
        Ok(data)
    }
    
    async fn save_json_value(&self, key: &str, data: &serde_json::Value) -> Result<(), String> {
        let file_path = self.get_file_path(key);
        debug!("FilePersistenceService: Saving configuration to file: {}", file_path.display());
        
        let json_str = serde_json::to_string_pretty(data)
            .map_err(|e| format!("Failed to serialize data for {}: {}", key, e))?;
        
        // Write to file
        fs::write(&file_path, &json_str)
            .map_err(|e| format!("Failed to write file {}: {}", file_path.display(), e))?;
        
        // Update cache
        {
            let mut cache_guard = self.cache.lock().await;
            cache_guard.insert(key.to_string(), json_str);
        }
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<(), String> {
        let file_path = self.get_file_path(key);
        
        if file_path.exists() {
            fs::remove_file(&file_path)
                .map_err(|e| format!("Failed to delete file {}: {}", file_path.display(), e))?;
        }
        
        // Remove from cache
        {
            let mut cache_guard = self.cache.lock().await;
            cache_guard.remove(key);
        }        Ok(())
    }
    
    async fn exists(&self, key: &str) -> bool {
        let file_path = self.get_file_path(key);
        file_path.exists()
    }
}

impl Default for FilePersistenceServiceImpl {
    fn default() -> Self {
        Self::new().expect("Failed to create default FilePersistenceService")
    }
}

