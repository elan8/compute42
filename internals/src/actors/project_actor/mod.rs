use actix::prelude::*;
use log::debug;
use uuid::Uuid;

use crate::messages::filesystem::{ReadProjectToml, WriteProjectToml};

pub struct ProjectActor;

impl Default for ProjectActor {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectActor {
    pub fn new() -> Self {
        Self
    }
}

impl Actor for ProjectActor {
    type Context = Context<Self>;
}

impl Handler<ReadProjectToml> for ProjectActor {
    type Result = Result<serde_json::Value, String>;

    fn handle(&mut self, msg: ReadProjectToml, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[ProjectActor] Read Project.toml for {}", msg.project_path);
        let project_toml_path = std::path::Path::new(&msg.project_path).join("Project.toml");
        if !project_toml_path.exists() {
            return Err("Project.toml file does not exist".to_string());
        }
        let content = std::fs::read_to_string(&project_toml_path)
            .map_err(|e| format!("Failed to read Project.toml: {}", e))?;
        let parsed: toml::Value = content
            .parse()
            .map_err(|e| format!("Failed to parse Project.toml: {}", e))?;
        let mut config = serde_json::json!({});
        for key in ["name", "version", "description", "authors", "uuid", "license"].iter() {
            if let Some(v) = parsed.get(*key) {
                config[*key] = serde_json::to_value(v).unwrap_or(serde_json::Value::Null);
            }
        }
        if let Some(deps) = parsed.get("deps").and_then(|v| v.as_table()) {
            config["deps"] = serde_json::to_value(deps).unwrap_or(serde_json::Value::Null);
        }
        if let Some(compat) = parsed.get("compat").and_then(|v| v.as_table()) {
            config["compat"] = serde_json::to_value(compat).unwrap_or(serde_json::Value::Null);
        }
        if let Some(sources) = parsed.get("sources").and_then(|v| v.as_table()) {
            config["sources"] = serde_json::to_value(sources).unwrap_or(serde_json::Value::Null);
        }
        Ok(config)
    }
}

#[derive(serde::Deserialize)]
struct ProjectTomlWriteConfigSerde {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub uuid: Option<String>,
    pub license: Option<String>,
    pub project_path: String,
}

impl Handler<WriteProjectToml> for ProjectActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: WriteProjectToml, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[ProjectActor] Write Project.toml");
        let cfg: ProjectTomlWriteConfigSerde = serde_json::from_value(msg.config)
            .map_err(|e| format!("Failed to parse config: {}", e))?;
        let project_toml_path = std::path::Path::new(&cfg.project_path).join("Project.toml");
        let project_exists = project_toml_path.exists();

        let mut toml_table = if project_exists {
            let content = std::fs::read_to_string(&project_toml_path)
                .map_err(|e| format!("Failed to read Project.toml: {}", e))?;
            content
                .parse::<toml::Table>()
                .map_err(|e| format!("Failed to parse existing Project.toml: {}", e))?
        } else {
            toml::Table::new()
        };

        if let Some(name) = &cfg.name {
            toml_table.insert("name".to_string(), toml::Value::String(name.clone()));
        }
        if let Some(version) = &cfg.version {
            toml_table.insert("version".to_string(), toml::Value::String(version.clone()));
        }
        if let Some(description) = &cfg.description {
            if !description.trim().is_empty() {
                toml_table.insert("description".to_string(), toml::Value::String(description.clone()));
            }
        }
        if let Some(authors) = &cfg.authors {
            if !authors.trim().is_empty() {
                let authors_array: Vec<toml::Value> = authors
                    .split('\n')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| toml::Value::String(s.trim().to_string()))
                    .collect();
                toml_table.insert("authors".to_string(), toml::Value::Array(authors_array));
            }
        }
        
        // Handle UUID: generate new one if empty, otherwise use provided value
        let uuid_to_use = if let Some(uuid) = &cfg.uuid {
            if uuid.trim().is_empty() {
                // Generate new UUID v4 following Julia Pkg standard
                let new_uuid = Uuid::new_v4();
                debug!("[ProjectActor] Generated new UUID: {}", new_uuid);
                new_uuid.to_string()
            } else {
                uuid.clone()
            }
        } else {
            // No UUID provided, generate new one
            let new_uuid = Uuid::new_v4();
            debug!("[ProjectActor] Generated new UUID: {}", new_uuid);
            new_uuid.to_string()
        };
        toml_table.insert("uuid".to_string(), toml::Value::String(uuid_to_use));
        
        if let Some(license) = &cfg.license {
            if !license.trim().is_empty() {
                toml_table.insert("license".to_string(), toml::Value::String(license.clone()));
            }
        }

        let toml_string = toml::to_string(&toml_table)
            .map_err(|e| format!("Failed to serialize Project.toml: {}", e))?;
        std::fs::write(&project_toml_path, toml_string)
            .map_err(|e| format!("Failed to write Project.toml: {}", e))
    }
}


