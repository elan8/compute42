// Core types and traits
pub mod types;
pub mod service_traits;

// App time tracking
pub mod app_time;

// Actor system
pub mod actor_system;
pub mod actor_error_handling;

// Actors (business logic only)
pub mod actors;

// Services (implementation logic)
pub mod services;

// Messages (actor communication)
pub mod messages;

// LSP functionality is now in services/lsp

// Utilities
pub mod version;

pub mod config;

// Core services (moved to services/support)

// Testing
pub mod mocks;
