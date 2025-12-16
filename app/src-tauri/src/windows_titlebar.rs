#[cfg(target_os = "windows")]
use windows::{
    Win32::Foundation::HWND,
    Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE, DWMWA_USE_IMMERSIVE_DARK_MODE},
};

#[cfg(target_os = "windows")]
use log::debug;


#[cfg(target_os = "windows")]
pub fn set_dark_title_bar(window_handle: HWND) -> Result<(), Box<dyn std::error::Error>> {
    debug!("[Windows] Setting dark title bar for window handle: {:?}", window_handle);
    
    // Enable immersive dark mode for the title bar
    // This overrides the system theme setting
    unsafe {
        // Windows APIs expect a 4-byte integer (BOOL), not Rust bool (u8)
        let use_dark_mode: i32 = 1;

        // Some Windows 10 builds use attribute 19, newer builds use 20
        // Try the enum first, then fallback to explicit 19
        let try_attr = |attr: DWMWINDOWATTRIBUTE| {
            DwmSetWindowAttribute(
                window_handle,
                attr,
                &use_dark_mode as *const i32 as *const _,
                std::mem::size_of::<i32>() as u32,
            )
        };

        let result_primary = try_attr(DWMWA_USE_IMMERSIVE_DARK_MODE);
        if result_primary.is_ok() {
            debug!("[Windows] Successfully set dark title bar");
            return Ok(());
        }

        // Fallback for older Windows 10 builds (attribute value 19)
        let result_fallback = try_attr(DWMWINDOWATTRIBUTE(19));
        if result_fallback.is_ok() {
            debug!("[Windows] Successfully set dark title bar (fallback attr 19)");
            Ok(())
        } else {
            let error_msg = format!(
                "Failed to set dark title bar: primary={:?}, fallback={:?}",
                result_primary, result_fallback
            );
            debug!("[Windows] {}", error_msg);
            Err(error_msg.into())
        }
    }
}

/// Non-Windows platforms: no-op function
#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn set_dark_title_bar(_window_handle: *mut std::ffi::c_void) -> Result<(), Box<dyn std::error::Error>> {
    // No-op for non-Windows platforms
    Ok(())
}
