use actix::prelude::*;
use log::debug;

use crate::messages::state::*;
use crate::services::events::EventService;
use crate::types::Tab;

/// StateActor - manages tab state and file operations
/// This replaces the mutex-based StateManager with a clean actor model
pub struct StateActor {
    // Actor state (no mutexes needed)
    tabs: Vec<Tab>,
    current_tab_id: Option<String>,
    project_path: Option<String>,
    
    // External communication services only (no mutex-based managers)
    event_manager: EventService,
}

impl StateActor {
    /// Create a new StateActor instance
    pub fn new(
        event_manager: EventService,
    ) -> Self {
        
        Self {
            tabs: Vec::new(),
            current_tab_id: None,
            project_path: None,
            event_manager,
        }
    }
}

impl Actor for StateActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // debug!("StateActor: Actor started");
        ctx.set_mailbox_capacity(256);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("StateActor: Actor stopped");
    }
}

// Message handlers
impl Handler<AddTab> for StateActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: AddTab, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("StateActor: Adding tab with id {}", msg.tab.id);
        
        // Check if tab already exists
        if self.tabs.iter().any(|t| t.id == msg.tab.id) {
            return Err(format!("Tab with id {} already exists", msg.tab.id));
        }
        
        // Add tab to state
        self.tabs.push(msg.tab.clone());
        
        // Set as current tab if no current tab
        if self.current_tab_id.is_none() {
            self.current_tab_id = Some(msg.tab.id.clone());
        }
        
        debug!("StateActor: Tab added successfully");
        Ok(())
    }
}

impl Handler<RemoveTab> for StateActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: RemoveTab, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("StateActor: Removing tab with id {}", msg.tab_id);
        
        // Find tab index
        let tab_index = self.tabs.iter().position(|t| t.id == msg.tab_id)
            .ok_or_else(|| format!("Tab with id {} not found", msg.tab_id))?;
        
        // Remove tab
        let _removed_tab = self.tabs.remove(tab_index);
        
        // Update current tab if needed
        if self.current_tab_id.as_ref() == Some(&msg.tab_id) {
            self.current_tab_id = self.tabs.first().map(|t| t.id.clone());
        }
        
        debug!("StateActor: Tab removed successfully");
        Ok(())
    }
}

impl Handler<GetTabs> for StateActor {
    type Result = ResponseActFuture<Self, Result<Vec<Tab>, String>>;
    
    fn handle(&mut self, _msg: GetTabs, _ctx: &mut Context<Self>) -> Self::Result {
        let tabs = self.tabs.clone();
        Box::pin(
            async move {
                Ok(tabs)
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<UpdateTab> for StateActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, msg: UpdateTab, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("StateActor: Updating tab with id {}", msg.tab_id);
        
        // Find tab index
        let tab_index = self.tabs.iter().position(|t| t.id == msg.tab_id)
            .ok_or_else(|| format!("Tab with id {} not found", msg.tab_id))?;
        
        // Update tab
        self.tabs[tab_index] = msg.updated_tab.clone();
        
        Ok(())
    }
}

impl Handler<ClearTabs> for StateActor {
    type Result = Result<(), String>;
    
    fn handle(&mut self, _msg: ClearTabs, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("StateActor: Clearing all tabs");
        
        self.tabs.clear();
        self.current_tab_id = None;
        
        debug!("StateActor: All tabs cleared successfully");
        Ok(())
    }
}

impl Handler<UpdateTabContent> for StateActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: UpdateTabContent, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("StateActor: Updating content for tab {}", msg.tab_id);
        
        // Find tab and extract data before async block
        let tab = self.tabs.iter().find(|t| t.id == msg.tab_id);
        
        let (file_path, tab_id, content) = match tab {
            Some(t) => {
                let path = t.path.clone();
                (path, msg.tab_id.clone(), msg.content.clone())
            },
            None => return Box::pin(async move { Err(format!("Tab with id {} not found", msg.tab_id)) }.into_actor(self).map(|res, _actor, _| res)),
        };
        
        Box::pin(
            async move {
                // Check if content differs from file (if file exists)
                let is_dirty = match &file_path {
                    Some(path) => {
                        match std::fs::read_to_string(path) {
                            Ok(file_content) => {
                                // Content differs from file
                                content != file_content
                            },
                            Err(_) => {
                                // File doesn't exist or can't be read - assume dirty if content is not empty
                                !content.is_empty()
                            }
                        }
                    },
                    None => {
                        // No file path - can't determine, assume dirty if content changed
                        true
                    }
                };
                
                Ok((tab_id, content, is_dirty))
            }
            .into_actor(self)
            .map(|result, actor, _| {
                match result {
                    Ok((tab_id, content, is_dirty)) => {
                        // Update tab content and dirty status
                        if let Some(tab) = actor.tabs.iter_mut().find(|t| t.id == tab_id) {
                            tab.content = content;
                            tab.is_dirty = is_dirty;
                        }
                        Ok(())
                    },
                    Err(e) => Err(e),
                }
            })
        )
    }
}

impl Handler<SaveTabToFile> for StateActor {
    type Result = ResponseActFuture<Self, Result<(), String>>;
    
    fn handle(&mut self, msg: SaveTabToFile, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("StateActor: Saving tab {} to file", msg.tab_id);
        
        // Find tab and extract data before async block
        let tab = self.tabs.iter().find(|t| t.id == msg.tab_id);
        
        let (file_path, content, tab_id) = match tab {
            Some(t) => {
                match &t.path {
                    Some(path) => (path.clone(), t.content.clone(), msg.tab_id.clone()),
                    None => return Box::pin(async move { Err("Tab has no associated file path".to_string()) }.into_actor(self).map(|res, _actor, _| res)),
                }
            },
            None => return Box::pin(async move { Err(format!("Tab with id {} not found", msg.tab_id)) }.into_actor(self).map(|res, _actor, _| res)),
        };
        
        let tab_id_for_update = tab_id.clone();
        
        Box::pin(
            async move {
                // Write file synchronously (blocking)
                std::fs::write(&file_path, &content)
                    .map_err(|e| format!("Failed to write file: {}", e))?;
                
                Ok(tab_id_for_update)
            }
            .into_actor(self)
            .map(|result, actor, _| {
                match result {
                    Ok(tab_id_updated) => {
                        // Update tab state (mark as clean) after file write succeeds
                        if let Some(tab) = actor.tabs.iter_mut().find(|t| t.id == tab_id_updated) {
                            tab.is_dirty = false;
                        }
                        Ok(())
                    },
                    Err(e) => Err(e),
                }
            })
        )
    }
}

impl Handler<GetDirtyTabs> for StateActor {
    type Result = ResponseActFuture<Self, Result<Vec<Tab>, String>>;
    
    fn handle(&mut self, _msg: GetDirtyTabs, _ctx: &mut Context<Self>) -> Self::Result {
        let dirty_tabs: Vec<Tab> = self.tabs.iter()
            .filter(|t| t.is_dirty)
            .cloned()
            .collect();
        Box::pin(
            async move {
                Ok(dirty_tabs)
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

impl Handler<GetCurrentTab> for StateActor {
    type Result = ResponseActFuture<Self, Result<Option<Tab>, String>>;
    
    fn handle(&mut self, _msg: GetCurrentTab, _ctx: &mut Context<Self>) -> Self::Result {
        let current_tab_id = self.current_tab_id.clone();
        let tabs = self.tabs.clone();
        
        Box::pin(
            async move {
                let current_tab = if let Some(tab_id) = current_tab_id {
                    tabs.iter().find(|t| t.id == tab_id).cloned()
                } else {
                    None
                };
                Ok(current_tab)
            }
            .into_actor(self)
            .map(|res, _actor, _| res)
        )
    }
}

// Clone implementation for async operations
impl Clone for StateActor {
    fn clone(&self) -> Self {
        Self {
            tabs: self.tabs.clone(),
            current_tab_id: self.current_tab_id.clone(),
            project_path: self.project_path.clone(),
            event_manager: self.event_manager.clone(),
        }
    }
}
