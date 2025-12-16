use std::env;
use log::info;

/// Environment configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    /// Backend server URL (removed in open-source version)
    #[allow(dead_code)]
    pub backend_url: String,
    /// Environment mode (development, production, etc.)
    pub environment: String,
    /// Log level for the application
    pub log_level: String,
}

impl EnvironmentConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        // Try to load .env file from multiple possible locations
        let env_result = dotenvy::dotenv();
        
        // If dotenvy::dotenv() didn't find it, try specific paths
        if env_result.is_err() {
            let possible_paths = [
                ".env",
                "../.env",
                "../../.env",
                "app/.env",
                "../app/.env",
            ];
            
            for path in &possible_paths {
                if std::path::Path::new(path).exists()
                    && dotenvy::from_filename(path).is_ok() {
                        break;
                    }
            }
        }
        
        match env_result {
            Ok(_) => {
                info!("[Config] Successfully loaded .env file");
            }
            Err(dotenvy::Error::Io(io_err)) if io_err.kind() == std::io::ErrorKind::NotFound => {
                info!("[Config] No .env file found, using default values");
            }
            Err(e) => {
                info!("[Config] Error loading .env file: {}, using default values", e);
            }
        }

        // Backend URL removed in open-source version
        let backend_url = env::var("BACKEND_URL")
            .unwrap_or_else(|_| "".to_string());
        let environment = env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "production".to_string());
        let log_level = env::var("RUST_LOG")
            .unwrap_or_else(|_| "info".to_string());

        info!("[Config] Loaded configuration: ENVIRONMENT={}, RUST_LOG={}", 
              environment, log_level);

        Self {
            backend_url,
            environment,
            log_level,
        }
    }

    /// Check if running in development mode
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    /// Check if running in production mode
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    /// Get the backend URL (removed in open-source version - returns empty string)
    #[allow(dead_code)]
    pub fn get_backend_url(&self) -> &str {
        &self.backend_url
    }
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

// Global configuration instance
lazy_static::lazy_static! {
    pub static ref CONFIG: EnvironmentConfig = EnvironmentConfig::from_env();
}

/// Get the global configuration instance
pub fn get_config() -> &'static EnvironmentConfig {
    &CONFIG
}

/// Force initialization of the configuration (useful for early logging)
pub fn init_config() {
    // Force the lazy static to initialize
    let _config = &*CONFIG;
}

/// Check if subscription feature is enabled via JJ_SUBSCRIPTION environment variable
pub fn is_subscription_enabled() -> bool {
    match env::var("JJ_SUBSCRIPTION") {
        Ok(val) => {
            let val_lower = val.to_lowercase();
            // Enabled if value is "1", "true", or any value other than disabled values
            val_lower != "0" && val_lower != "false" && val_lower != "disabled"
        }
        Err(_) => {
            // Not set - default to disabled
            false
        }
    }
}
