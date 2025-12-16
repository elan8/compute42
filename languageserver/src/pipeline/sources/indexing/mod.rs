mod package_resolver;
mod file_indexer;
mod ast_walker;
mod signature_extraction;
mod docstring_extraction;
mod type_extraction;

pub use package_resolver::{resolve_package_path, should_skip_entry, compute_package_slug, extract_package_slug};
// index_file and walk_node removed - were only used by PackageIndexer which used TypeRegistry
pub use signature_extraction::extract_function_signature;
pub use docstring_extraction::{extract_docstring, extract_docstrings_with_function_names};
pub use type_extraction::{extract_struct_definition, extract_abstract_definition};

// PackageIndexer removed - was using TypeRegistry and is not used anywhere
// Package indexing is now handled by the pipeline system using Index


