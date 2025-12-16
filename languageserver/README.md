# Julia Language Server

A high-performance Julia language server implementation built with Rust, featuring pro-active parsing, intelligent caching, and comprehensive LSP capabilities.

## Features

- **Tree-sitter Based Parsing**: Fast and accurate Julia syntax parsing
- **Project-Aware**: Understands Julia projects via Project.toml and Manifest.toml
- **Workspace Indexing**: Pro-actively indexes entire projects for fast symbol lookup
- **Smart Caching**: Multi-layered caching for optimal performance
- **Comprehensive LSP Support**:
  - Hover information (with type inference and Julia docs)
  - Find references (project-wide)
  - Go to definition (cross-file support)
  - Diagnostics (syntax errors)
  - Code completion

## Architecture

### Core Components

#### 1. Project Context Manager (`core/project_context.rs`)

Manages Julia project metadata and dependencies:

```rust
let context = ProjectContext::new(project_root)?;
println!("Project: {}", context.project_name().unwrap());
println!("Dependencies: {:?}", context.dependencies());
```

**Features:**
- Parses Project.toml and Manifest.toml
- Resolves package dependency paths
- Tracks project metadata

#### 2. Pipeline Architecture (`pipeline/`)

Indexes Julia files using a unified pipeline architecture:

```rust
use crate::pipeline::{sources::WorkspaceSource, Pipeline, config::PipelineConfig};

let workspace_source = WorkspaceSource::new(project_root.clone());
let source_items = workspace_source.discover()?;
let pipeline = Pipeline::new(PipelineConfig::full());
let index = pipeline.run(source_items)?;

println!("Indexed {} symbols", index.get_all_symbols().len());
```

**Features:**
- Discovers and parses all `.jl` files
- Unified Index combining symbols, references, types, scopes, and signatures
- Two-pass type inference using Index from Pass 1
- Supports workspace, package, and base/stdlib indexing
- Skips common directories (.git, node_modules, etc.)

**Performance:** < 2 seconds for 1000 files

#### 3. Cache Manager (`core/cache.rs`)

Multi-layered LRU caching system:

```rust
let cache_manager = CacheManager::new();

// Document cache - tracks parsed documents
cache_manager.document_cache.update(uri, timestamp);

// Symbol cache - caches symbol lookups
cache_manager.symbol_cache.put(symbol_name, symbols);

// Type inference cache - caches Julia type inference results
cache_manager.type_inference_cache.put(key, result);

// Docs cache - caches Julia documentation
cache_manager.docs_cache.put(symbol_name, docs);

// Hover cache - caches hover results
cache_manager.hover_cache.put(uri, line, char, hover_result);
```

**Cache Statistics:**
```rust
let stats = cache_manager.stats();
println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
```

**Target:** > 80% hit rate for repeated operations

#### 4. Diagnostics Provider (`features/diagnostics.rs`)

Tree-sitter based syntax error detection:

```rust
let diagnostics = DiagnosticsProvider::compute_diagnostics(&document);

for diagnostic in diagnostics {
    println!("{}: {}", diagnostic.severity, diagnostic.message);
}
```

**Features:**
- Detects syntax errors in real-time
- Provides helpful error messages with context
- Identifies missing `end` keywords, unmatched parentheses, etc.

**Performance:** < 100ms per file

### LSP Providers

All LSP providers are stateless and take their dependencies as parameters:

#### Hover Provider

```rust
let hover_result = HoverProvider::hover(
    &symbol_table,
    julia_lsp.as_ref(),
    &document,
    position
).await?;
```

**Provides:**
- Symbol type information (from tree-sitter)
- Type inference (via Julia process)
- Julia documentation
- Function signatures
- Variable assignments

**Performance:** < 50ms (cached), < 200ms (uncached)

#### Find References

```rust
let locations = ReferencesProvider::find_references(
    &symbol_table,
    &reference_table,
    &document,
    position,
    include_declaration
)?;
```

**Features:**
- Project-wide reference search
- Cross-file support
- Optional inclusion of declaration

**Performance:** < 100ms single file, < 500ms project-wide

#### Go to Definition

```rust
let locations = DefinitionProvider::find_definition(
    &symbol_table,
    &document,
    position
)?;
```

**Features:**
- Project-wide symbol resolution
- Supports user-defined symbols
- Future: package dependency support

### Embedded LSP Service

High-level API for use in the internals actor system:

```rust
let config = LspConfig::new(julia_executable)
    .with_project_root(project_root);

let mut service = EmbeddedLspService::new(config);

// Open and index project
service.open_project(project_root)?;

// Update documents
service.update_document(uri, content)?;

// Get hover info
let hover = service.hover(&uri, line, character).await;

// Get diagnostics
let diagnostics = service.get_diagnostics(&uri);

// Find references
let references = service.find_references(&uri, line, character, true);
```

**Features:**
- Manages all core components
- Automatic workspace indexing
- Cache invalidation on document changes
- Julia LSP process management

## Usage Example

```rust
use languageserver::embedded::{EmbeddedLspService, LspConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create service
    let config = LspConfig::new(PathBuf::from("julia"))
        .with_project_root(PathBuf::from("/path/to/project"));
    
    let mut service = EmbeddedLspService::new(config);
    
    // Initialize Julia LSP
    service.initialize_julia_lsp().await?;
    
    // Open and index project
    service.open_project(PathBuf::from("/path/to/project"))?;
    
    // Open a document
    let uri = PathBuf::from("/path/to/project/src/main.jl");
    let content = std::fs::read_to_string(&uri)?;
    service.update_document(uri.clone(), content)?;
    
    // Get diagnostics
    let diagnostics = service.get_diagnostics(&uri);
    println!("Found {} diagnostics", diagnostics.len());
    
    // Get hover info
    if let Some(hover) = service.hover(&uri, 5, 10).await {
        println!("Hover: {}", hover);
    }
    
    // Find references
    if let Some(refs) = service.find_references(&uri, 5, 10, true) {
        println!("Found {} references", refs.len());
    }
    
    Ok(())
}
```

## Testing

The library has comprehensive test coverage:

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test lib

# Run tests with output
cargo test -- --nocapture
```

**Test Organization:**
- `languageserver/tests/unit/` - Unit tests for core modules
- `languageserver/tests/integration/` - Integration tests
- `languageserver/tests/fixtures/` - Test Julia files

**Current Coverage:** 48 tests covering:
- Project context parsing
- Workspace indexing
- Cache functionality
- Diagnostics provider
- All LSP providers (hover, definition, references, completion)
- Julia LSP integration

## Performance Metrics

| Operation | Target | Current Status |
|-----------|--------|----------------|
| Workspace Indexing | < 2s for 1000 files | ✅ Achieved |
| Cache Hit Rate | > 80% | ✅ Monitored |
| Hover (cached) | < 50ms | ✅ Achieved |
| Hover (uncached) | < 200ms | ✅ Achieved |
| Find References (single file) | < 100ms | ✅ Achieved |
| Find References (project) | < 500ms | ✅ Achieved |
| Diagnostics | < 100ms per file | ✅ Achieved |

## Caching Strategy

### Cache Types

1. **Document Cache**: Tracks last modified timestamps
2. **Symbol Cache**: Caches symbol lookup results
3. **Type Inference Cache**: Caches Julia type inference (expensive operation)
4. **Docs Cache**: Caches Julia documentation (expensive operation)
5. **Hover Cache**: Caches complete hover results

### Cache Invalidation

- **File changes**: All caches for that file are invalidated
- **Project changes**: Workspace reindexing triggers cache refresh
- **LRU eviction**: Least recently used entries are evicted when capacity is reached

### Cache Warming

When a project is opened:
1. Workspace is indexed
2. All symbols are extracted
3. Frequently accessed files are prioritized

## Integration with Internals

The languageserver is used by the internals library via the `lsp_native` service:

```rust
// In internals/src/services/lsp_native/lsp_service_impl.rs
impl LspServiceTrait for LspServiceImpl {
    async fn get_diagnostics(&self, uri: String) -> Result<Vec<LspDiagnostic>, String> {
        let service = self.service.read().await;
        let path = PathBuf::from(uri);
        
        let diagnostics = service.get_diagnostics(&path);
        Ok(diagnostics.into_iter().map(diagnostic_to_lsp).collect())
    }
    
    // ... other methods
}
```

## Future Enhancements

### Planned Features

1. **Semantic Diagnostics**: Beyond syntax errors (type checking, undefined variables)
2. **Package Dependency Support**: Jump to definitions in installed packages
3. **Incremental Parsing**: Only reparse changed sections
4. **Background Indexing**: Index workspace in background threads
5. **Signature Help**: Parameter hints for function calls
6. **Code Actions**: Quick fixes for common issues
7. **Document Formatting**: Auto-format Julia code
8. **Rename Refactoring**: Project-wide symbol renaming

### Performance Optimizations

1. **Parallel Indexing**: Use multiple threads for workspace indexing
2. **Persistent Cache**: Save cache to disk between sessions
3. **Smart Reparsing**: Only reparse when necessary
4. **Lazy Loading**: Load package dependencies on-demand

## Dependencies

- **tree-sitter**: Julia syntax parsing
- **tree-sitter-julia**: Julia grammar for tree-sitter
- **dashmap**: Concurrent hashmap for symbol tables
- **ropey**: Efficient text buffer
- **toml**: Project.toml parsing
- **walkdir**: File system traversal
- **lru**: LRU cache implementation
- **tokio**: Async runtime

## Migration from TreeSitterSyntaxService

The diagnostics functionality has been migrated from `internals/src/services/syntax/tree_sitter_service.rs` to the languageserver. The internals Syntax service is now deprecated and will be removed in a future version.

**Migration Path:**
1. ✅ DiagnosticsProvider created in languageserver
2. ✅ Integrated with EmbeddedLspService
3. ✅ Type conversions added to lsp_native
4. ⏳ Deprecate TreeSitterSyntaxService (pending)
5. ⏳ Remove old Syntax service (future release)

## Contributing

When adding new features:

1. Follow the stateless provider pattern
2. Add comprehensive unit tests
3. Add integration tests for cross-file scenarios
4. Update cache invalidation logic if needed
5. Document performance implications
6. Run `cargo check` and `cargo test` before committing

## License

See workspace license.

