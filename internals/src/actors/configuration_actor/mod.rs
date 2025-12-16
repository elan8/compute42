mod persistence;

use actix::prelude::*;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::Mutex;
use log::{debug, error};
use serde_json::Value;

use crate::messages::configuration::*;
use crate::services::events::EventService;
use crate::types::{Configuration, UserPreferences};
use persistence::PersistenceHelper;

/// ConfigurationActor - manages application configuration
/// This replaces the mutex-based ConfigurationManager with a clean actor model
pub struct ConfigurationActor {
    // Actor state (no mutexes needed)
    config: Configuration,
    config_loaded: bool,
    user_preferences: Arc<Mutex<UserPreferences>>,
    persistence_helper: Arc<PersistenceHelper>,
    event_manager: EventService,
}

impl ConfigurationActor {
    /// Create a new ConfigurationActor instance
    pub fn new(
        event_manager: EventService,
    ) -> Self {
        // Create persistence helper
        let persistence_helper = Arc::new(
            PersistenceHelper::new()
                .expect("Failed to create persistence helper")
        );
        
        Self {
            config: Configuration::default(),
            config_loaded: false,
            user_preferences: Arc::new(Mutex::new(UserPreferences::default())),
            persistence_helper,
            event_manager,
        }
    }
    
    /// Load configuration from file or test mode
    async fn load_configuration(&self) -> Result<UserPreferences, String> {
        // Check if we're in test mode
        let is_test_mode = std::env::var("TESTMODE")
            .unwrap_or_else(|_| "false".to_string())
            .to_lowercase() == "true";
        
        if is_test_mode {
            // In test mode, load from environment variables or use defaults
            let config = Self::load_config_from_test_mode();
            let mut config_guard = self.user_preferences.lock().await;
            *config_guard = config.clone();
            
            Ok(config)
        } else {
            // Normal mode: load from file
            let config = self.load_config_from_file().await?;
            let mut config_guard = self.user_preferences.lock().await;
            *config_guard = config.clone();
            
            Ok(config)
        }
    }
    
    /// Load configuration from test mode (environment variables or defaults)
    fn load_config_from_test_mode() -> UserPreferences {
        let mut prefs = UserPreferences::default();
        
        // Load from environment variables if set
        if let Ok(last_opened_folder) = std::env::var("TEST_USER_LAST_OPENED_FOLDER") {
            if !last_opened_folder.is_empty() {
                prefs.last_opened_folder = Some(last_opened_folder);
            }
        }
        
        if let Ok(user_email) = std::env::var("TEST_USER_EMAIL") {
            if !user_email.is_empty() {
                prefs.user_email = Some(user_email);
            }
        }
        
        if let Ok(font_family) = std::env::var("TEST_EDITOR_FONT_FAMILY") {
            if !font_family.is_empty() {
                prefs.editor_font_family = Some(font_family);
            }
        }
        
        if let Ok(font_size) = std::env::var("TEST_EDITOR_FONT_SIZE") {
            if let Ok(size) = font_size.parse::<u16>() {
                prefs.editor_font_size = Some(size);
            }
        }
        
        if let Ok(terminal_font_family) = std::env::var("TEST_TERMINAL_FONT_FAMILY") {
            if !terminal_font_family.is_empty() {
                prefs.terminal_font_family = Some(terminal_font_family);
            }
        }
        
        if let Ok(terminal_font_size) = std::env::var("TEST_TERMINAL_FONT_SIZE") {
            if let Ok(size) = terminal_font_size.parse::<u16>() {
                prefs.terminal_font_size = Some(size);
            }
        }
        
        if let Ok(word_wrap) = std::env::var("TEST_EDITOR_WORD_WRAP") {
            if let Ok(wrap) = word_wrap.parse::<bool>() {
                prefs.editor_word_wrap = Some(wrap);
            }
        }
        
        if let Ok(tab_size) = std::env::var("TEST_EDITOR_TAB_SIZE") {
            if let Ok(size) = tab_size.parse::<u16>() {
                prefs.editor_tab_size = Some(size);
            }
        }
        
        if let Ok(line_numbers) = std::env::var("TEST_EDITOR_LINE_NUMBERS") {
            if let Ok(show) = line_numbers.parse::<bool>() {
                prefs.editor_line_numbers = Some(show);
            }
        }
        
        if let Ok(minimap) = std::env::var("TEST_EDITOR_MINIMAP") {
            if let Ok(show) = minimap.parse::<bool>() {
                prefs.editor_minimap = Some(show);
            }
        }
        
        if let Ok(color_scheme) = std::env::var("TEST_EDITOR_COLOR_SCHEME") {
            if !color_scheme.is_empty() {
                prefs.editor_color_scheme = Some(color_scheme);
            }
        }
        
        prefs
    }
    
    /// Set user preferences programmatically (for test mode)
    /// This method allows tests to set preferences directly without file I/O
    pub async fn set_test_preferences(&self, preferences: UserPreferences) -> Result<(), String> {
        let is_test_mode = std::env::var("TESTMODE")
            .unwrap_or_else(|_| "false".to_string())
            .to_lowercase() == "true";
        
        if !is_test_mode {
            return Err("set_test_preferences can only be called when TESTMODE=true".to_string());
        }
        
        let mut config_guard = self.user_preferences.lock().await;
        *config_guard = preferences.clone();
        
        Ok(())
    }
    
    /// Save configuration to file
    async fn save_configuration(&self, config: &UserPreferences) -> Result<(), String> {
        self.save_config_to_file(config).await?;
        let mut config_guard = self.user_preferences.lock().await;
        *config_guard = config.clone();
        Ok(())
    }
    
    /// Get current configuration
    async fn get_config(&self) -> UserPreferences {
        let config = self.user_preferences.lock().await;
        config.clone()
    }
    
    /// Load configuration from file
    async fn load_config_from_file(&self) -> Result<UserPreferences, String> {
        self.persistence_helper.load_config_from_file().await
    }
    
    /// Save configuration to file
    async fn save_config_to_file(&self, config: &UserPreferences) -> Result<(), String> {
        self.persistence_helper.save_config_to_file(config).await
    }
    
    /// Get root folder from configuration
    async fn get_root_folder_internal(&self) -> Result<Option<String>, String> {
        let config = self.get_config().await;
        debug!("ConfigurationActor: Getting root folder: {:?}", config.last_opened_folder);
        Ok(config.last_opened_folder)
    }
    
    /// Set root folder in configuration
    async fn set_root_folder_internal(&self, folder: Option<String>) -> Result<(), String> {
        debug!("ConfigurationActor: Setting root folder to: {:?}", folder);
        let mut config = self.get_config().await;
        config.last_opened_folder = folder;
        self.save_configuration(&config).await
    }
    
    /// Get user email from configuration
    async fn get_user_email_internal(&self) -> Result<Option<String>, String> {
        let config = self.get_config().await;
        Ok(config.user_email)
    }
    
    /// Set user email in configuration
    async fn set_user_email_internal(&self, email: Option<String>) -> Result<(), String> {
        let mut config = self.get_config().await;
        config.user_email = email;
        self.save_configuration(&config).await
    }
    
    /// Save configuration to external service
    async fn save_config(&mut self, config_value: Value) -> Result<(), String> {
        debug!("ConfigurationActor: Saving configuration");
        
        // Validate configuration
        let new_config: Configuration = serde_json::from_value(config_value.clone())
            .map_err(|e| format!("Invalid configuration format: {}", e))?;
        
        // Check if user_email is being updated in the incoming config
        let existing = self.load_configuration().await.unwrap_or_default();
        let user_email = if let Some(email_value) = config_value.get("user_email") {
            // Use the new user_email from the incoming config
            email_value.as_str().map(|s| s.to_string())
        } else {
            // Preserve existing user_email if not provided
            existing.user_email
        };
        
        let app_config: UserPreferences = UserPreferences {
            last_opened_folder: new_config.root_folder.clone(),
            user_email,
            editor_font_family: existing.editor_font_family.clone(),
            editor_font_size: existing.editor_font_size,
            terminal_font_family: existing.terminal_font_family.clone(),
            terminal_font_size: existing.terminal_font_size,
            editor_word_wrap: existing.editor_word_wrap,
            editor_tab_size: existing.editor_tab_size,
            editor_line_numbers: existing.editor_line_numbers,
            editor_minimap: existing.editor_minimap,
            editor_color_scheme: existing.editor_color_scheme.clone(),
        };
        self.save_configuration(&app_config).await?;
        
        // Update actor state
        self.config = new_config;
        
        // Optionally emit config saved event
        
        debug!("ConfigurationActor: Configuration saved successfully");
        Ok(())
    }
    
}

impl Actor for ConfigurationActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        ctx.set_mailbox_capacity(128);
        
        // Load configuration during initialization
        let persistence_helper = self.persistence_helper.clone();
        let actor_addr = ctx.address();
        let user_preferences = self.user_preferences.clone();
        
        actix::spawn(async move {
            // Check if we're in test mode
            let is_test_mode = std::env::var("TESTMODE")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase() == "true";
            
            let mut app_config = if is_test_mode {
                // In test mode, load from environment variables or use defaults
                ConfigurationActor::load_config_from_test_mode()
            } else {
                // Normal mode: load from file
                match persistence_helper.load_config_from_file().await {
                    Ok(config) => config,
                    Err(e) => {
                        error!("ConfigurationActor: Failed to load configuration: {}", e);
                        UserPreferences::default()
                    }
                }
            };
            
            // Check if config file exists - if not, this is first startup
            let config_file_exists = persistence_helper.config_file_exists().await;
            debug!("ConfigurationActor: Config file exists: {}", config_file_exists);
            
            // If config file doesn't exist or no root folder is set, try to initialize with demo folder
            if (!config_file_exists || app_config.last_opened_folder.is_none()) && !is_test_mode {
                debug!("ConfigurationActor: Config file doesn't exist or no root folder set, checking for demo folder");
                if let Some(demo_path) = get_demo_folder_path() {
                    debug!("ConfigurationActor: Demo folder path: {:?}", demo_path);
                    if demo_path.exists() && demo_path.is_dir() {
                        let project_toml = demo_path.join("Project.toml");
                        if project_toml.exists() {
                            let demo_path_str = demo_path.to_string_lossy().to_string();
                            debug!("ConfigurationActor: Initializing root folder with demo folder: {}", demo_path_str);
                            app_config.last_opened_folder = Some(demo_path_str);
                        } else {
                            debug!("ConfigurationActor: Demo folder exists but Project.toml not found at: {:?}", project_toml);
                        }
                    } else {
                        debug!("ConfigurationActor: Demo folder does not exist at: {:?}", demo_path);
                    }
                } else {
                    debug!("ConfigurationActor: Could not determine demo folder path");
                }
                
                // Always save the config file if it didn't exist (even if demo folder is not set yet)
                // This ensures the config file is created on first startup
                debug!("ConfigurationActor: Saving config file (creating if it doesn't exist)");
                if let Err(e) = persistence_helper.save_config_to_file(&app_config).await {
                    error!("ConfigurationActor: Failed to save initialized config: {}", e);
                } else {
                    debug!("ConfigurationActor: Successfully saved config file with root folder: {:?}", app_config.last_opened_folder);
                }
            }
            
            // Update user_preferences immediately to avoid race conditions
            // This ensures GetUserEmail will work correctly
            {
                let mut prefs_guard = user_preferences.lock().await;
                *prefs_guard = app_config.clone();
            }
            
            let config_value = serde_json::to_value(&app_config).unwrap_or_default();
            
            // Send a message to update the actor's state
            let _ = actor_addr.send(UpdateConfigState { config_value }).await;
        });
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("ConfigurationActor: Actor stopped");
    }
}

// Message handlers

impl Handler<UpdateConfigState> for ConfigurationActor {
    type Result = ();
    
    fn handle(&mut self, msg: UpdateConfigState, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("ConfigurationActor: Updating configuration state");
        
        // Parse and store the configuration
        if let Ok(config) = serde_json::from_value::<Configuration>(msg.config_value.clone()) {
            self.config = config;
            self.config_loaded = true;
        } else {
            error!("ConfigurationActor: Failed to parse configuration value");
        }
        
        // Note: user_preferences is already updated in started() before this message is sent
        // This handler only updates the Configuration struct for backward compatibility
    }
}

impl Handler<SaveConfig> for ConfigurationActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: SaveConfig, _ctx: &mut Context<Self>) -> Self::Result {
        let mut actor = self.clone();
        async move {
            let _ = actor.save_config(msg.config).await;
        }.into_actor(self).spawn(_ctx);
        Ok(())
    }
}


impl Handler<GetRootFolder> for ConfigurationActor {
    type Result = ResponseActFuture<Self, Result<Option<String>, String>>;
    
    fn handle(&mut self, _msg: GetRootFolder, _ctx: &mut Context<Self>) -> Self::Result {
        let actor = self.clone();
        
        Box::pin(async move {
            let result = actor.get_root_folder_internal().await;
            debug!("ConfigurationActor: GetRootFolder result: {:?}", result);
            result
        }.into_actor(self))
    }
}

impl Handler<SetRootFolder> for ConfigurationActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: SetRootFolder, _ctx: &mut Context<Self>) -> Self::Result {
        let actor = self.clone();
        let folder = msg.folder;
        
        Box::pin(async move {
            let result = actor.set_root_folder_internal(folder).await;
            debug!("ConfigurationActor: SetRootFolder result: {:?}", result);
            result
        }.into_actor(self))
    }
}


impl Handler<GetUserEmail> for ConfigurationActor {
    type Result = ResponseActFuture<Self, Result<Option<String>, String>>;
    
    fn handle(&mut self, _msg: GetUserEmail, _ctx: &mut Context<Self>) -> Self::Result {
        let actor = self.clone();
        
        Box::pin(async move {
            let result = actor.get_user_email_internal().await;
            debug!("ConfigurationActor: GetUserEmail result: {:?}", result);
            result
        }.into_actor(self))
    }
}

impl Handler<SetUserEmail> for ConfigurationActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: SetUserEmail, _ctx: &mut Context<Self>) -> Self::Result {
        let actor = self.clone();
        let email = msg.email;
        
        Box::pin(async move {
            let result = actor.set_user_email_internal(email).await;
            debug!("ConfigurationActor: SetUserEmail result: {:?}", result);
            result
        }.into_actor(self))
    }
}


/// Get demo folder path from resource directory
/// The demo folder is bundled as a Tauri resource at compile time
fn get_demo_folder_path() -> Option<PathBuf> {
    // Try to find the demo folder relative to the executable
    // In Tauri, resources are typically in the same directory as the executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // In development, resources might be in target/debug/resources or target/release/resources
            // In production, resources are in the same directory as the executable
            let mut possible_paths = vec![
                exe_dir.join("resources").join("demo"),  // Production path
                exe_dir.join("demo"),  // Alternative production path
            ];
            
            // Add development path if parent exists
            if let Some(parent) = exe_dir.parent() {
                possible_paths.push(parent.join("resources").join("demo"));  // Development path (target/debug/resources/demo)
            }
            
            for path in possible_paths {
                if path.exists() && path.is_dir() {
                    let project_toml = path.join("Project.toml");
                    if project_toml.exists() {
                        return Some(path);
                    }
                }
            }
            
            // Also try workspace root for development
            if let Ok(cwd) = std::env::current_dir() {
                let workspace_demo = cwd.join("demo");
                if workspace_demo.exists() && workspace_demo.is_dir() {
                    let project_toml = workspace_demo.join("Project.toml");
                    if project_toml.exists() {
                        return Some(workspace_demo);
                    }
                }
            }
        }
    }
    
    None
}

/// Get default font family based on the platform
fn get_default_font_family(is_editor: bool) -> String {
    if cfg!(target_os = "windows") {
        if is_editor {
            // For editor, prefer Fira Code with Consolas fallback
            "\"Fira Code\", \"Consolas\", \"Monaco\", monospace".to_string()
        } else {
            // For terminal, prefer Consolas (widely available on Windows)
            "\"Consolas\", \"Monaco\", \"Courier New\", monospace".to_string()
        }
    } else if cfg!(target_os = "macos") {
        if is_editor {
            // For editor, prefer Monaco with SF Mono fallback
            "\"Monaco\", \"SF Mono\", \"Consolas\", monospace".to_string()
        } else {
            // For terminal, prefer Monaco (native on macOS)
            "\"Monaco\", \"SF Mono\", \"Consolas\", monospace".to_string()
        }
    } else {
        // Linux - use common monospace fonts
        if is_editor {
            "\"Fira Code\", \"DejaVu Sans Mono\", \"Liberation Mono\", monospace".to_string()
        } else {
            "\"DejaVu Sans Mono\", \"Liberation Mono\", \"Courier New\", monospace".to_string()
        }
    }
}

impl Handler<GetFontSettings> for ConfigurationActor {
    type Result = ResponseActFuture<Self, Result<serde_json::Value, String>>;
    
    fn handle(&mut self, _msg: GetFontSettings, _ctx: &mut Context<Self>) -> Self::Result {
        let actor = self.clone();
        
        Box::pin(async move {
            let prefs = actor.load_configuration().await
                .map_err(|e| format!("Failed to load configuration: {}", e))?;
            
            // Get default values when None
            let editor_font_family = prefs.editor_font_family.unwrap_or_else(|| get_default_font_family(true));
            let terminal_font_family = prefs.terminal_font_family.unwrap_or_else(|| get_default_font_family(false));
            
            // Apply defaults when values are None
            let settings = serde_json::json!({
                "editor_font_family": editor_font_family,
                "editor_font_size": prefs.editor_font_size.unwrap_or(14),
                "terminal_font_family": terminal_font_family,
                "terminal_font_size": prefs.terminal_font_size.unwrap_or(13),
                "editor_word_wrap": prefs.editor_word_wrap.unwrap_or(false),
                "editor_tab_size": prefs.editor_tab_size.unwrap_or(4),
                "editor_line_numbers": prefs.editor_line_numbers.unwrap_or(true),
                "editor_minimap": prefs.editor_minimap.unwrap_or(true),
                "editor_color_scheme": prefs.editor_color_scheme.unwrap_or_else(|| "vs-dark".to_string()),
            });
            
            debug!("ConfigurationActor: GetFontSettings result: {:?}", settings);
            Ok(settings)
        }.into_actor(self))
    }
}

impl Handler<SetFontSettings> for ConfigurationActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: SetFontSettings, _ctx: &mut Context<Self>) -> Self::Result {
        let actor = self.clone();
        let updates = msg;
        
        Box::pin(async move {
            // Load current preferences
            let mut prefs = actor.load_configuration().await
                .map_err(|e| format!("Failed to load configuration: {}", e))?;
            
            // Update only provided fields
            if updates.editor_font_family.is_some() {
                prefs.editor_font_family = updates.editor_font_family;
            }
            if updates.editor_font_size.is_some() {
                prefs.editor_font_size = updates.editor_font_size;
            }
            if updates.terminal_font_family.is_some() {
                prefs.terminal_font_family = updates.terminal_font_family;
            }
            if updates.terminal_font_size.is_some() {
                prefs.terminal_font_size = updates.terminal_font_size;
            }
            if updates.editor_word_wrap.is_some() {
                prefs.editor_word_wrap = updates.editor_word_wrap;
            }
            if updates.editor_tab_size.is_some() {
                prefs.editor_tab_size = updates.editor_tab_size;
            }
            if updates.editor_line_numbers.is_some() {
                prefs.editor_line_numbers = updates.editor_line_numbers;
            }
            if updates.editor_minimap.is_some() {
                prefs.editor_minimap = updates.editor_minimap;
            }
            if updates.editor_color_scheme.is_some() {
                prefs.editor_color_scheme = updates.editor_color_scheme;
            }
            
            // Save updated preferences
            let result = actor.save_configuration(&prefs).await;
            debug!("ConfigurationActor: SetFontSettings result: {:?}", result);
            result.map_err(|e| format!("Failed to save settings: {}", e))
        }.into_actor(self))
    }
}

// Clone implementation for async operations
impl Clone for ConfigurationActor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            config_loaded: self.config_loaded,
            user_preferences: self.user_preferences.clone(),
            persistence_helper: self.persistence_helper.clone(),
            event_manager: self.event_manager.clone(),
        }
    }
}
