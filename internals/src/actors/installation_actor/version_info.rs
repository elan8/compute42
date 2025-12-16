// Platform-specific Julia version information

use super::types::JuliaVersion;

/// Platform-specific Julia version information
pub fn get_julia_version_info(version: &str) -> JuliaVersion {
    // Use provided Julia version
    let version_path = version.split('.').take(2).collect::<Vec<_>>().join(".");

    #[cfg(target_os = "windows")]
    {
        #[cfg(target_arch = "x86_64")]
        {
            let url = format!(
                "https://julialang-s3.julialang.org/bin/winnt/x64/{}/julia-{}-win64.zip",
                version_path, version
            );
            JuliaVersion {
                version: version.to_string(),
                download_url: url,
                filename: format!("julia-{}-win64.zip", version),
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            let url = format!("https://julialang-s3.julialang.org/bin/winnt/aarch64/{}/julia-{}-win64-aarch64.zip", version_path, version);
            JuliaVersion {
                version: version.to_string(),
                download_url: url,
                filename: format!("julia-{}-win64-aarch64.zip", version),
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        #[cfg(target_arch = "x86_64")]
        {
            let url = format!(
                "https://julialang-s3.julialang.org/bin/mac/x64/{}/julia-{}-mac64.dmg",
                version_path, version
            );
            JuliaVersion {
                version: version.to_string(),
                download_url: url,
                filename: format!("julia-{}-mac64.dmg", version),
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            let url = format!(
                "https://julialang-s3.julialang.org/bin/mac/aarch64/{}/julia-{}-macaarch64.dmg",
                version_path, version
            );
            JuliaVersion {
                version: version.to_string(),
                download_url: url,
                filename: format!("julia-{}-macaarch64.dmg", version),
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        #[cfg(target_arch = "x86_64")]
        {
            let url = format!("https://julialang-s3.julialang.org/bin/linux/x64/{}/julia-{}-linux-x86_64.tar.gz", version_path, version);
            JuliaVersion {
                version: version.to_string(),
                download_url: url,
                filename: format!("julia-{}-linux-x86_64.tar.gz", version),
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            let url = format!("https://julialang-s3.julialang.org/bin/linux/aarch64/{}/julia-{}-linux-aarch64.tar.gz", version_path, version);
            JuliaVersion {
                version: version.to_string(),
                download_url: url,
                filename: format!("julia-{}-linux-aarch64.tar.gz", version),
            }
        }
    }
}

/// Get Julia installation directory
pub fn get_julia_installation_dir() -> std::path::PathBuf {
    let app_data_dir = dirs::data_local_dir().expect("Failed to get app data directory");
    app_data_dir.join("com.compute42.dev").join("julia")
}




















