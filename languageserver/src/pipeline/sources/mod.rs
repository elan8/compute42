pub mod workspace;
pub mod package;
pub mod file;
pub mod base;
pub mod document;
pub mod project_context;
pub mod base_docs;
pub mod base_docs_extraction;
pub mod indexing;

pub use workspace::WorkspaceSource;
pub use package::PackageSource;
pub use file::FileSource;
pub use base::BaseSource;
pub use document::Document;
pub use project_context::ProjectContext;
pub use base_docs::BaseDocsRegistry;
// PackageIndexer removed - was using TypeRegistry and is not used anywhere










