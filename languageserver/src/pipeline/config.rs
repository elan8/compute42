/// Configuration for pipeline execution
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Whether to extract symbols
    pub extract_symbols: bool,
    /// Whether to extract references
    pub extract_references: bool,
    /// Whether to extract types
    pub extract_types: bool,
    /// Whether to extract scopes
    pub extract_scopes: bool,
    /// Whether to extract signatures
    pub extract_signatures: bool,
    /// Whether to extract exports
    pub extract_exports: bool,
}

impl PipelineConfig {
    /// Create a config for full analysis (all analyzers, but type inference disabled)
    pub fn full() -> Self {
        Self {
            extract_symbols: true,
            extract_references: true,
            extract_types: true,
            extract_scopes: true,
            extract_signatures: true,
            extract_exports: true,
        }
    }

    /// Create a config for package indexing (signatures, types, and exports)
    /// 
    /// **Deprecated**: Package indexing now uses PackagePipeline which extracts docstrings only.
    /// This config is no longer used in production code.
    #[deprecated(note = "Package indexing now uses PackagePipeline. Use WorkspacePipeline for workspace files.")]
    pub fn package() -> Self {
        Self {
            extract_symbols: false,
            extract_references: false,
            extract_types: true,
            extract_scopes: false,
            extract_signatures: true,
            extract_exports: true,
        }
    }

    /// Create a config for Base/stdlib indexing (signatures and documentation only)
    /// 
    /// **Deprecated**: Base/stdlib indexing now uses JuliaPipeline which extracts docstrings only.
    /// This config is no longer used in production code.
    #[deprecated(note = "Base/stdlib indexing now uses JuliaPipeline. Use WorkspacePipeline for workspace files.")]
    pub fn base_indexing() -> Self {
        Self {
            extract_symbols: false, // Not needed for documentation-only indexing
            extract_references: false,
            extract_types: true, // Still extract type definitions
            extract_scopes: false,
            extract_signatures: true, // Extract function signatures with documentation
            extract_exports: true, // Extract exports to filter which functions to index
        }
    }
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self::full()
    }
}

