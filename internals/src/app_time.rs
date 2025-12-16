// App start time tracking
use std::sync::OnceLock;
use std::time::Instant;

static APP_START_TIME: OnceLock<Instant> = OnceLock::new();

/// Initialize the app start time (should be called once at app startup)
pub fn init_app_start_time() {
    APP_START_TIME.get_or_init(|| Instant::now());
}

/// Get the app start time
pub fn get_app_start_time() -> Instant {
    *APP_START_TIME.get_or_init(|| Instant::now())
}

