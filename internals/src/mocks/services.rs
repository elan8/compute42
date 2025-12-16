use crate::service_traits::*;
use crate::types::*;
use async_trait::async_trait;
use serde_json;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

// ============================================================================
// Mock LoggingService
// ============================================================================

#[derive(Clone)]
pub struct MockLoggingService {
    #[allow(clippy::type_complexity)]
    logs: Arc<TokioMutex<Vec<(LogLevel, String, Option<serde_json::Value>)>>>,
    config: Arc<TokioMutex<Option<LoggingConfig>>>,
}

impl Default for MockLoggingService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockLoggingService {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(TokioMutex::new(Vec::new())),
            config: Arc::new(TokioMutex::new(None)),
        }
    }

    pub async fn get_logs(&self) -> Vec<(LogLevel, String, Option<serde_json::Value>)> {
        self.logs.lock().await.clone()
    }

    pub async fn get_config(&self) -> Option<LoggingConfig> {
        self.config.lock().await.clone()
    }

    pub async fn clear_logs(&self) {
        self.logs.lock().await.clear();
    }

    pub async fn has_log_level(&self, level: LogLevel) -> bool {
        self.logs.lock().await.iter().any(|(log_level, _, _)| *log_level == level)
    }
}

#[async_trait]
impl LoggingService for MockLoggingService {
    async fn configure_logging(&self, config: LoggingConfig) -> Result<(), String> {
        *self.config.lock().await = Some(config);
        Ok(())
    }

    async fn log(&self, level: LogLevel, message: &str) -> Result<(), String> {
        self.logs.lock().await.push((level, message.to_string(), None));
        Ok(())
    }

    async fn log_with_context(&self, level: LogLevel, message: &str, context: serde_json::Value) -> Result<(), String> {
        self.logs.lock().await.push((level, message.to_string(), Some(context)));
        Ok(())
    }
}

// ============================================================================
// Mock ConfigurationService
// ============================================================================

pub struct MockConfigurationService {
    config: Arc<TokioMutex<UserPreferences>>,
    julia_path: Arc<TokioMutex<Option<String>>>,
    root_folder: Arc<TokioMutex<Option<String>>>,
}

impl Default for MockConfigurationService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockConfigurationService {
    pub fn new() -> Self {
        Self {
            config: Arc::new(TokioMutex::new(UserPreferences::default())),
            julia_path: Arc::new(TokioMutex::new(None)),
            root_folder: Arc::new(TokioMutex::new(None)),
        }
    }

    pub async fn set_config(&self, config: UserPreferences) {
        *self.config.lock().await = config;
    }

    pub async fn set_julia_path(&self, path: Option<String>) {
        *self.julia_path.lock().await = path;
    }

    pub async fn set_root_folder(&self, folder: Option<String>) {
        *self.root_folder.lock().await = folder;
    }
}

#[async_trait]
impl ConfigurationService for MockConfigurationService {
    async fn load_config(&self) -> Result<UserPreferences, String> {
        let config_guard = self.config.lock().await;
        Ok(config_guard.clone())
    }

    async fn save_config(&self, config: &UserPreferences) -> Result<(), String> {
        let mut config_guard = self.config.lock().await;
        *config_guard = config.clone();
        Ok(())
    }


    async fn get_root_folder(&self) -> Result<Option<String>, String> {
        let config_guard = self.config.lock().await;
        Ok(config_guard.last_opened_folder.clone())
    }

    async fn set_root_folder(&self, folder: Option<String>) -> Result<(), String> {
        let mut config_guard = self.config.lock().await;
        config_guard.last_opened_folder = folder;
        Ok(())
    }

    async fn get_user_email(&self) -> Result<Option<String>, String> {
        let config_guard = self.config.lock().await;
        Ok(config_guard.user_email.clone())
    }

    async fn set_user_email(&self, email: Option<String>) -> Result<(), String> {
        let mut config_guard = self.config.lock().await;
        config_guard.user_email = email;
        Ok(())
    }

}


// ============================================================================
// Mock InstallationService
// ============================================================================

pub struct MockInstallationService {
    is_installed: Arc<TokioMutex<bool>>,
    is_installing: Arc<TokioMutex<bool>>,
    version: Arc<TokioMutex<Option<String>>>,
    executable_path: Arc<TokioMutex<Option<String>>>,
}

impl Default for MockInstallationService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockInstallationService {
    pub fn new() -> Self {
        Self {
            is_installed: Arc::new(TokioMutex::new(false)),
            is_installing: Arc::new(TokioMutex::new(false)),
            version: Arc::new(TokioMutex::new(None)),
            executable_path: Arc::new(TokioMutex::new(None)),
        }
    }

    pub async fn set_installed(&self, installed: bool) {
        *self.is_installed.lock().await = installed;
    }

    pub async fn set_installing(&self, installing: bool) {
        *self.is_installing.lock().await = installing;
    }

    pub async fn set_version(&self, version: Option<String>) {
        *self.version.lock().await = version;
    }

    pub async fn set_executable_path(&self, path: Option<String>) {
        *self.executable_path.lock().await = path;
    }
}

#[async_trait]
impl InstallationService for MockInstallationService {
    async fn check_julia_installation(&self) -> Result<bool, String> {
        Ok(*self.is_installed.lock().await)
    }

    async fn install_julia(&self, _julia_version: Option<&str>) -> Result<(), String> {
        *self.is_installing.lock().await = true;
        // Simulate installation
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        *self.is_installing.lock().await = false;
        *self.is_installed.lock().await = true;
        Ok(())
    }

    async fn get_julia_version(&self) -> Result<Option<String>, String> {
        Ok(self.version.lock().await.clone())
    }

    async fn is_installation_in_progress(&self) -> bool {
        *self.is_installing.lock().await
    }

    async fn get_julia_executable_path(&self) -> Result<Option<String>, String> {
        Ok(self.executable_path.lock().await.clone())
    }

    async fn cleanup_old_julia_versions(&self, _current_version: &str) -> Result<(), String> {
        // Mock implementation - just return success
        Ok(())
    }

    async fn get_detected_installations(&self) -> Result<Vec<crate::types::JuliaInstallation>, String> {
        Ok(vec![])
    }

    async fn detect_installations(&self) -> Result<Vec<crate::types::JuliaInstallation>, String> {
        Ok(vec![])
    }

    async fn validate_installation(&self, _installation: crate::types::JuliaInstallation) -> Result<bool, String> {
        Ok(true)
    }

    async fn repair_installation(&self, installation: &crate::types::JuliaInstallation) -> Result<crate::types::JuliaInstallation, String> {
        Ok(installation.clone())
    }
}

// Sysimage mocks removed

// MockAuthService removed - BackendAuthService trait removed

// MockSubscriptionService removed - SubscriptionService trait removed

// Mock DebugService removed - debug functionality removed in open-source version
