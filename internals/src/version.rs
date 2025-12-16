// Centralized version management for internals

// Application version and build information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get the build date from environment variable
pub fn get_build_date() -> &'static str {
    option_env!("BUILD_DATE").unwrap_or("unknown")
}

/// Get the Julia version
pub fn get_julia_version() -> &'static str {
    "1.12.1"
}
