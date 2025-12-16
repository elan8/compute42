// Re-export functions from julia_pipeline for backward compatibility
// These functions are now implemented in julia_pipeline.rs
use crate::pipeline::julia_pipeline::JuliaPipeline;
use crate::pipeline::storage::Index;
use crate::types::LspError;
use std::path::{Path, PathBuf};

/// Extract Base/stdlib metadata (signatures, types, exports)
/// 
/// This function:
/// 1. Discovers Base and stdlib files
/// 2. Extracts signatures, types, and exports to create a lightweight Index
/// 
/// Note: This extracts only signatures, types, and exports (not symbols/references/scopes).
/// The Index can be merged into the main workspace Index for type inference and semantic analysis.
/// 
/// This is a convenience wrapper around JuliaPipeline::index_base().
#[deprecated(note = "Use JuliaPipeline::index_base() instead")]
pub fn index_base(julia_executable: &Path) -> Result<Index, LspError> {
    let pipeline = JuliaPipeline::new();
    pipeline.index_base(julia_executable)
}

/// Save base index to file
/// 
/// Saves:
/// - base_index.json: Index format (for type inference and semantic analysis)
/// 
/// This is a convenience wrapper around JuliaPipeline::save_base_index().
#[deprecated(note = "Use JuliaPipeline::save_base_index() instead")]
pub fn save_base_index(
    index: &Index,
    output_path: Option<PathBuf>,
) -> Result<PathBuf, LspError> {
    let pipeline = JuliaPipeline::new();
    pipeline.save_base_index(index, output_path)
}

/// Check if base_index.json exists and is recent (to skip re-indexing)
/// 
/// Returns true if base_index.json exists and is within the last 7 days
/// 
/// **Deprecated**: Cache checking is now handled automatically in `JuliaPipeline::run()`.
/// This method is kept for backward compatibility only.
#[deprecated(note = "Cache checking is now handled automatically in JuliaPipeline::run(). Use run() instead.")]
pub fn should_skip_base_indexing() -> bool {
    // Deprecated: This method is no longer used. Cache checking is handled in run().
    // Return false to indicate indexing should proceed if this is somehow still called.
    false
}

// Note: Package indexing is now handled by PackagePipeline in package_pipeline.rs
// The Index is only created for workspace files using WorkspacePipeline in workspace_pipeline.rs

