use std::sync::Arc;

use internals::actor_system::ActorSystem;

#[derive(Clone)]
pub struct AppState {
    pub actor_system: Arc<ActorSystem>,
}

impl AppState {
    pub fn new(actor_system: Arc<ActorSystem>) -> Self {
        Self { 
            actor_system,
        }
    }
}


