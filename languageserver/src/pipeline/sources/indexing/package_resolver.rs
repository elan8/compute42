use crate::pipeline::sources::project_context::ManifestToml;
use crate::types::LspError;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Resolve package path using custom depot and manifest
pub fn resolve_package_path(
    depot_path: &Path,
    package_name: &str,
    manifest: Option<&ManifestToml>,
) -> Option<PathBuf> {
    log::trace!("resolve_package_path: Looking for package '{}' in depot {:?}", package_name, depot_path);
    
    // First check manifest for UUID and path
    if let Some(manifest) = manifest {
        if let Some(entries) = manifest.packages.get(package_name) {
            log::trace!("resolve_package_path: Found {} manifest entries for '{}'", entries.len(), package_name);
            for entry in entries {
                if let Some(uuid) = &entry.uuid {
                    // Packages in depot are at: {depot}/packages/{PackageName}/{UUID}/
                    let package_path = depot_path
                        .join("packages")
                        .join(package_name)
                        .join(uuid);
                    
                    log::trace!("resolve_package_path: Trying path {:?} (UUID: {})", package_path, uuid);
                    if package_path.exists() {
                        log::trace!("resolve_package_path: Found package '{}' at {:?}", package_name, package_path);
                        return Some(package_path);
                    } else {
                        log::trace!("resolve_package_path: Path {:?} does not exist", package_path);
                    }
                } else {
                    log::trace!("resolve_package_path: Manifest entry for '{}' has no UUID", package_name);
                }
            }
        } else {
            log::trace!("resolve_package_path: No manifest entries found for '{}'", package_name);
        }
    } else {
        log::trace!("resolve_package_path: No manifest available");
    }
    
    // Fallback: try to find package in depot by name (may have multiple versions)
    let packages_dir = depot_path.join("packages").join(package_name);
    log::trace!("resolve_package_path: Trying fallback - checking if {:?} exists", packages_dir);
    if packages_dir.exists() {
        // Find first UUID directory
        if let Ok(entries) = std::fs::read_dir(&packages_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    log::trace!("resolve_package_path: Found package '{}' via fallback at {:?}", package_name, entry.path());
                    return Some(entry.path());
                }
            }
        }
        log::trace!("resolve_package_path: Package directory exists but no UUID subdirectories found");
    } else {
        log::trace!("resolve_package_path: Package directory {:?} does not exist", packages_dir);
    }
    
    log::trace!("resolve_package_path: Could not resolve path for package '{}'", package_name);
    None
}

/// Check if directory entry should be skipped
pub fn should_skip_entry(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();
        matches!(
            name_str.as_ref(),
            ".git" | "node_modules" | "target" | ".vscode" | ".idea" | "__pycache__" | ".DS_Store"
        )
    } else {
        false
    }
}

/// Extract package slug from package path
/// Path format: packages/{PackageName}/{slug}/
pub fn extract_package_slug(package_path: &Path) -> Option<String> {
    package_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
}

/// Compute package slug from UUID and git-tree-sha1
/// This implements Julia's version_slug function from base/loading.jl
/// Algorithm:
/// 1. Compute CRC32C of UUID
/// 2. Compute CRC32C of SHA1 bytes using previous CRC as seed
/// 3. Convert UInt32 to base62 string of length 5
pub fn compute_package_slug(uuid_str: &str, git_tree_sha1: &str) -> Result<String, LspError> {
    // Parse UUID
    let uuid = Uuid::parse_str(uuid_str)
        .map_err(|e| LspError::InternalError(format!("Invalid UUID: {}", e)))?;
    
    // Parse git-tree-sha1 (hex string to bytes)
    let sha1_bytes = hex::decode(git_tree_sha1)
        .map_err(|e| LspError::InternalError(format!("Invalid git-tree-sha1: {}", e)))?;
    
    if sha1_bytes.len() != 20 {
        return Err(LspError::InternalError(format!(
            "git-tree-sha1 must be 20 bytes, got {}",
            sha1_bytes.len()
        )));
    }
    
    // Compute CRC32C of UUID
    // Julia's UUID.value is a UInt128, and _crc32c processes it as 16 bytes
    // However, Julia's byte order for UInt128 is different from Rust's Uuid::as_bytes()
    // We need to reverse the bytes to match Julia's representation
    let mut uuid_bytes = uuid.as_bytes().to_vec();
    uuid_bytes.reverse();
    let uuid_crc = crc32c::crc32c(&uuid_bytes);
    
    // Compute CRC32C of SHA1 bytes using previous CRC as seed
    // Julia's _crc32c with seed does: crc32c(sha1_bytes, uuid_crc)
    // This means: initialize CRC with uuid_crc, then process sha1_bytes
    // crc32c::crc32c_append does exactly this - computes CRC32C starting with a previous value
    let crc = crc32c::crc32c_append(uuid_crc, &sha1_bytes);
    
    // Convert to base62 slug of length 5
    let slug = u32_to_base62(crc, 5);
    
    Ok(slug)
}

/// Convert UInt32 to base62 string
/// Character set: A-Z, a-z, 0-9 (62 characters)
/// Julia's implementation writes digits from least significant to most significant
/// (first iteration writes the least significant digit)
fn u32_to_base62(mut x: u32, length: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let n = CHARS.len() as u32;
    
    let mut result = String::with_capacity(length);
    for _ in 0..length {
        let (quotient, remainder) = (x / n, x % n);
        // Julia writes in order: first remainder (least significant) is written first
        result.push(CHARS[remainder as usize] as char);
        x = quotient;
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_package_slug_dataframes() {
        // Test case from Manifest.toml: DataFrames package
        let uuid = "a93c6f00-e57d-5684-b7b6-d8193f3e46c0";
        let git_tree_sha1 = "d8928e9169ff76c6281f39a659f9bca3a573f24c";
        let expected_slug = "b4w9K";

        let computed_slug = compute_package_slug(uuid, git_tree_sha1)
            .expect("Failed to compute slug");

        assert_eq!(computed_slug, expected_slug, 
            "Computed slug '{}' does not match expected '{}'", computed_slug, expected_slug);
    }

    #[test]
    fn test_u32_to_base62() {
        // Test base62 conversion
        let result = u32_to_base62(0, 5);
        assert_eq!(result.len(), 5);
        
        // Test with a known value
        let result = u32_to_base62(123456, 5);
        assert_eq!(result.len(), 5);
        // Verify it only contains valid base62 characters
        assert!(result.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_extract_package_slug() {
        let path = PathBuf::from("packages/DataFrames/b4w9K");
        let slug = extract_package_slug(&path);
        assert_eq!(slug, Some("b4w9K".to_string()));

        let path = PathBuf::from("packages/TestPackage/abc123");
        let slug = extract_package_slug(&path);
        assert_eq!(slug, Some("abc123".to_string()));
    }

    #[test]
    fn test_compute_package_slug_invalid_uuid() {
        let result = compute_package_slug("invalid-uuid", "d8928e9169ff76c6281f39a659f9bca3a573f24c");
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_package_slug_invalid_sha1() {
        let uuid = "a93c6f00-e57d-5684-b7b6-d8193f3e46c0";
        let result = compute_package_slug(uuid, "invalid-sha1");
        assert!(result.is_err());
    }
}

