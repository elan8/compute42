use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::types::LspError;
use crate::pipeline::storage::Index;
use crate::pipeline::parser::JuliaParser;
use crate::pipeline::sources::indexing::extract_docstrings_with_function_names;
use serde::{Serialize, Deserialize};

/// Documentation entry with module, name, and docstring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocEntry {
    /// Module name (e.g., "Base", "Base.Filesystem", "Statistics")
    pub module: String,
    /// Function/symbol name (e.g., "joinpath", "select")
    pub name: String,
    /// Documentation string
    pub docstring: String,
}

/// Registry for Base/stdlib documentation loaded from pre-extracted JSON file
/// Stores entries with module, name, and docstring (no duplication)
pub struct BaseDocsRegistry {
    /// All documentation entries (stored once per function)
    entries: Vec<DocEntry>,
    /// Index: bare name -> Vec<entry indices> (for lookup by name only)
    by_name: HashMap<String, Vec<usize>>,
    /// Index: qualified name "module.name" -> entry index (for exact lookup)
    by_qualified: HashMap<String, usize>,
}

impl BaseDocsRegistry {
    /// Create a new BaseDocsRegistry by loading documentation from a JSON file
    /// The JSON file should be an array of DocEntry objects: [{ "module": "Base", "name": "joinpath", "docstring": "..." }, ...]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, LspError> {
        log::trace!("BaseDocsRegistry: Loading from {:?}", path.as_ref());
        
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| LspError::InternalError(format!("Failed to read base docs file: {}", e)))?;
        
        // Try to parse as new format (array of DocEntry) first
        let entries: Vec<DocEntry> = match serde_json::from_str(&content) {
            Ok(entries) => entries,
            Err(_) => {
                // Fallback: try to parse as old format (HashMap<String, String>) and convert
                log::warn!("BaseDocsRegistry: Old format detected, converting to new format");
                let old_docs: HashMap<String, String> = serde_json::from_str(&content)
                    .map_err(|e| LspError::InternalError(format!("Failed to parse base docs JSON: {}", e)))?;
                
                // Convert old format to new format
                let mut entries = Vec::new();
                for (key, docstring) in old_docs {
                    // Try to extract module and name from key
                    let (module, name) = if let Some(dot_pos) = key.rfind('.') {
                        let module_part = &key[..dot_pos];
                        let name_part = &key[dot_pos + 1..];
                        (module_part.to_string(), name_part.to_string())
                    } else {
                        // Bare name - assume Base module
                        ("Base".to_string(), key)
                    };
                    
                    entries.push(DocEntry {
                        module,
                        name,
                        docstring,
                    });
                }
                entries
            }
        };
        
        log::info!("BaseDocsRegistry: Loaded {} symbols", entries.len());
        
        Ok(Self::from_entries(entries))
    }
    
    /// Create BaseDocsRegistry from entries and build indexes
    fn from_entries(entries: Vec<DocEntry>) -> Self {
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_qualified: HashMap<String, usize> = HashMap::new();
        
        for (idx, entry) in entries.iter().enumerate() {
            // Index by bare name
            by_name.entry(entry.name.clone()).or_default().push(idx);
            
            // Index by qualified name
            let qualified = format!("{}.{}", entry.module, entry.name);
            by_qualified.insert(qualified, idx);
        }
        
        Self {
            entries,
            by_name,
            by_qualified,
        }
    }

    /// Create an empty registry (for when file doesn't exist)
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
            by_name: HashMap::new(),
            by_qualified: HashMap::new(),
        }
    }

    /// Get documentation for a symbol
    /// Returns None if symbol is not found
    /// Tries qualified name first (e.g., "Base.joinpath"), then bare name
    /// When multiple entries exist for a bare name, prefers "Base" module entries
    pub fn get_documentation(&self, symbol: &str) -> Option<String> {
        // Try qualified name first
        if let Some(&idx) = self.by_qualified.get(symbol) {
            if let Some(entry) = self.entries.get(idx) {
                log::trace!("BaseDocsRegistry: Found documentation for qualified key '{}'", symbol);
                return Some(entry.docstring.clone());
            }
        }
        
        // Try bare name - if multiple matches, prefer "Base" module, but return any match if no Base
        if let Some(indices) = self.by_name.get(symbol) {
            if indices.is_empty() {
                log::trace!("BaseDocsRegistry: No entries found for bare name '{}'", symbol);
                return None;
            }
            
            log::trace!("BaseDocsRegistry: Found {} entries for bare name '{}'", indices.len(), symbol);
            
            // First, try to find an entry with module "Base"
            for &idx in indices {
                if let Some(entry) = self.entries.get(idx) {
                    if entry.module == "Base" {
                        log::trace!("BaseDocsRegistry: Found documentation for bare name '{}' in Base module", symbol);
                        return Some(entry.docstring.clone());
                    }
                }
            }
            // If no Base entry found, use the first match (should work for package functions)
            // Prefer entries with longer module paths (more specific submodules)
            let mut candidates: Vec<_> = indices.iter()
                .filter_map(|&idx| self.entries.get(idx))
                .collect();
            candidates.sort_by(|a, b| b.module.len().cmp(&a.module.len()));
            
            if let Some(entry) = candidates.first() {
                log::trace!("BaseDocsRegistry: Found documentation for bare name '{}' (module: {}, first of {} matches, preferring most specific)", 
                    symbol, entry.module, indices.len());
                return Some(entry.docstring.clone());
            }
        } else {
            log::trace!("BaseDocsRegistry: No entries indexed for bare name '{}'", symbol);
            // Fallback: search all entries directly (in case indexing failed)
            for entry in &self.entries {
                if entry.name == symbol {
                    log::trace!("BaseDocsRegistry: Found documentation for '{}' via fallback search (module: {})", 
                        symbol, entry.module);
                    return Some(entry.docstring.clone());
                }
            }
        }
        
        None
    }
    
    /// Get documentation for a symbol in a specific module
    /// Returns None if not found in that module
    /// Also searches submodules (e.g., "DataFrames" will match "DataFrames.Selection")
    pub fn get_documentation_by_module(&self, module: &str, name: &str) -> Option<String> {
        // Try exact module match first
        let qualified = format!("{}.{}", module, name);
        if let Some(&idx) = self.by_qualified.get(&qualified) {
            if let Some(entry) = self.entries.get(idx) {
                return Some(entry.docstring.clone());
            }
        }
        
        // Also search submodules (e.g., "DataFrames" should match "DataFrames.Selection.select")
        let module_prefix = format!("{}.", module);
        for entry in &self.entries {
            if entry.name == name && entry.module.starts_with(&module_prefix) {
                log::trace!("BaseDocsRegistry: Found documentation for '{}' in submodule '{}'", name, entry.module);
                return Some(entry.docstring.clone());
            }
        }
        
        None
    }
    
    /// Get all entries for a specific module
    pub fn get_entries_by_module(&self, module: &str) -> Vec<&DocEntry> {
        self.entries.iter()
            .filter(|entry| entry.module == module)
            .collect()
    }

    /// Find documentation by searching for entries that end with the given symbol name
    /// This is useful for finding functions in submodules (e.g., "Base.Filesystem.joinpath" when searching for "joinpath")
    /// Returns the first matching documentation found, preferring more specific module paths
    pub fn find_documentation_by_suffix(&self, symbol_name: &str) -> Option<String> {
        // First try bare name lookup (fast path)
        if let Some(indices) = self.by_name.get(symbol_name) {
            if !indices.is_empty() {
                // Sort by module path length (prefer more specific paths like "Base.Filesystem" over "Base")
                let mut candidates: Vec<_> = indices.iter()
                    .filter_map(|&idx| self.entries.get(idx))
                    .collect();
                
                candidates.sort_by(|a, b| {
                    // Prefer longer module paths (more specific)
                    b.module.len().cmp(&a.module.len())
                });
                
                if let Some(entry) = candidates.first() {
                    log::trace!("BaseDocsRegistry: Found documentation for '{}' via suffix search (module: {})", 
                        symbol_name, entry.module);
                    return Some(entry.docstring.clone());
                }
            }
        }
        
        // Also search qualified names that end with the symbol name
        let suffix_pattern = format!(".{}", symbol_name);
        let mut candidates: Vec<&DocEntry> = Vec::new();
        
        for entry in &self.entries {
            let qualified = format!("{}.{}", entry.module, entry.name);
            if qualified.ends_with(&suffix_pattern) || entry.name == symbol_name {
                candidates.push(entry);
            }
        }
        
        if candidates.is_empty() {
            log::trace!("BaseDocsRegistry: No candidates found for '{}' (searched {} entries)", 
                symbol_name, self.entries.len());
            return None;
        }
        
        // Sort by module path length (prefer more specific)
        candidates.sort_by(|a, b| b.module.len().cmp(&a.module.len()));
        
        if let Some(entry) = candidates.first() {
            log::trace!("BaseDocsRegistry: Found documentation for '{}' via suffix search (module: {})", 
                symbol_name, entry.module);
            return Some(entry.docstring.clone());
        }
        
        None
    }

    /// Check if registry has any documentation loaded
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get number of symbols in registry
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Get all unique module names in the registry
    pub fn get_all_modules(&self) -> std::collections::HashSet<String> {
        self.entries.iter()
            .map(|entry| entry.module.clone())
            .collect()
    }

    /// Debug method: find all entries containing a substring in name or module
    /// Useful for diagnosing why certain functions aren't found
    pub fn find_entries_containing(&self, substring: &str) -> Vec<&DocEntry> {
        self.entries.iter()
            .filter(|entry| entry.name.contains(substring) || entry.module.contains(substring))
            .collect()
    }
    
    /// Save the registry to a JSON file
    /// Saves as an array of DocEntry objects: [{ "module": "Base", "name": "joinpath", "docstring": "..." }, ...]
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), LspError> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .map_err(|e| LspError::InternalError(format!("Failed to create directory: {}", e)))?;
        }
        
        // Serialize to JSON as array of DocEntry objects
        let json = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| LspError::InternalError(format!("Failed to serialize registry: {}", e)))?;
        
        fs::write(path.as_ref(), json)
            .map_err(|e| LspError::InternalError(format!("Failed to write JSON file: {}", e)))?;
        
        log::info!("BaseDocsRegistry: Saved {} symbols to {:?}", self.entries.len(), path.as_ref());
        Ok(())
    }
    
    /// Create a BaseDocsRegistry by parsing basedocs.jl directly
    /// This is faster and more accurate than parsing all source files
    pub fn from_basedocs_jl<P: AsRef<Path>>(basedocs_path: P) -> Result<Self, LspError> {
        use crate::pipeline::sources::base_docs_extraction::parse_basedocs_jl;
        
        let old_docs = parse_basedocs_jl(basedocs_path.as_ref())?;
        
        // Convert to new format
        let mut entries = Vec::new();
        for (key, docstring) in old_docs {
            // Try to extract module and name from key
            let (module, name) = if let Some(dot_pos) = key.rfind('.') {
                let module_part = &key[..dot_pos];
                let name_part = &key[dot_pos + 1..];
                (module_part.to_string(), name_part.to_string())
            } else {
                // Bare name - assume Base module
                ("Base".to_string(), key)
            };
            
            entries.push(DocEntry {
                module,
                name,
                docstring,
            });
        }
        
        log::info!("BaseDocsRegistry: Created from basedocs.jl with {} symbols", entries.len());
        
        Ok(Self::from_entries(entries))
    }
    
    /// Create a BaseDocsRegistry from an Index
    /// Extracts documentation from Base/stdlib modules
    /// NOTE: This is slower than from_basedocs_jl - prefer from_basedocs_jl when possible
    pub fn from_index(index: &Index) -> Self {
        let mut entries = Vec::new();
        
        let all_modules = index.get_all_modules();
        
        // Extract from Base, Core, and stdlib modules
        for module in all_modules {
            // Only process Base, Core, and stdlib modules (skip user modules)
            if !module.starts_with("Base") && module != "Core" && !module.contains(".") {
                // Check if it's a stdlib module (common ones)
                let is_stdlib = matches!(module.as_str(), 
                    "Statistics" | "LinearAlgebra" | "Random" | "Printf" | 
                    "Dates" | "DelimitedFiles" | "Distributed" | "SharedArrays" |
                    "SparseArrays" | "SuiteSparse" | "Test" | "UUIDs");
                if !is_stdlib {
                    continue;
                }
            }
            
            // Get all functions from this module
            let func_names = index.get_module_functions(&module);
            for func_name in func_names {
                let signatures = index.find_signatures(&module, &func_name);
                
                // Get doc from first signature that has one
                let mut doc: Option<String> = None;
                for sig in &signatures {
                    if doc.is_none() {
                        doc = sig.doc_comment.clone();
                    }
                }
                
                // Store entry (only once, no duplication)
                if let Some(doc_str) = doc {
                    entries.push(DocEntry {
                        module: module.clone(),
                        name: func_name.clone(),
                        docstring: doc_str,
                    });
                }
            }
            
            // Also extract types
            let type_names = index.get_module_types(&module);
            for type_name in type_names {
                if let Some(type_def) = index.find_type(&module, &type_name) {
                    if let Some(doc_str) = &type_def.doc_comment {
                        entries.push(DocEntry {
                            module: module.clone(),
                            name: type_name.clone(),
                            docstring: doc_str.clone(),
                        });
                    }
                }
            }
        }
        
        Self::from_entries(entries)
    }
    
    /// Create a BaseDocsRegistry by extracting docstrings directly from source files
    /// This uses the docstring-first approach: extract all docstrings and derive function names from them
    /// This is more reliable than matching docstrings to functions
    pub fn from_source_files<P: AsRef<Path>>(source_files: &[P]) -> Result<Self, LspError> {
        let mut entries = Vec::new();
        let parser = JuliaParser::new();
        
        log::info!("BaseDocsRegistry: Extracting docstrings from {} source files", source_files.len());
        
        for file_path in source_files {
            let file_path = file_path.as_ref();
            let content = fs::read_to_string(file_path)
                .map_err(|e| LspError::InternalError(format!("Failed to read file {:?}: {}", file_path, e)))?;
            
            let tree = parser.parse(&content)
                .map_err(|e| LspError::ParseError(format!("Failed to parse file {:?}: {}", file_path, e)))?;
            
            let root = tree.root_node();
            let file_docs = extract_docstrings_with_function_names(root, &content);
            
            // Infer module name from file path
            let module = Self::infer_module_from_path(file_path);
            
            // Convert to entries
            // IMPORTANT: For package files, always use the inferred module from path,
            // not the module prefix from the docstring (which might reference other modules like "Base.select")
            for (func_name, docstring) in file_docs {
                // Extract bare function name (strip any module prefix)
                let entry_name = if let Some(dot_pos) = func_name.rfind('.') {
                    // Extract just the function name part (e.g., "select" from "Base.select" or "DataFrames.select")
                    func_name[dot_pos + 1..].to_string()
                } else {
                    func_name
                };
                
                // Always use the inferred module from path for package files
                // This ensures DataFrames functions are stored with "DataFrames" module, not "Base"
                entries.push(DocEntry {
                    module: module.clone(),
                    name: entry_name,
                    docstring,
                });
            }
        }
        
        log::info!("BaseDocsRegistry: Extracted {} documentation entries from source files", entries.len());
        
        Ok(Self::from_entries(entries))
    }
    
    /// Infer module name from file path
    fn infer_module_from_path(path: &Path) -> String {
        let path_str = path.to_string_lossy();
        let path_lower = path_str.to_lowercase();
        
        // Check if path contains "base/" or "base\" (case-insensitive)
        if path_lower.contains("/base/") || path_lower.contains("\\base\\") {
            // Try to extract submodule from path (e.g., "base/filesystem.jl" -> "Base.Filesystem")
            if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                if !file_stem.is_empty() {
                    // Capitalize first letter
                    let mut chars = file_stem.chars();
                    if let Some(first) = chars.next() {
                        let capitalized = format!("{}{}", first.to_uppercase(), chars.as_str());
                        return format!("Base.{}", capitalized);
                    }
                }
            }
            return "Base".to_string();
        }
        
        // Check for Core
        if path_lower.contains("/core/") || path_lower.contains("\\core\\") {
            return "Core".to_string();
        }
        
        // Check for stdlib modules
        for stdlib_module in ["Statistics", "LinearAlgebra", "Random", "Printf"] {
            let module_lower = stdlib_module.to_lowercase();
            if path_lower.contains(&format!("/{}/", module_lower)) || 
               path_lower.contains(&format!("\\{}\\", module_lower)) {
                return stdlib_module.to_string();
            }
        }
        
        // Check for external packages in depot structure
        // Pattern: .../packages/{PackageName}/{slug}/src/{ModuleName}.jl
        // or: .../packages/{PackageName}/{slug}/src/{PackageName}.jl
        if let Some(packages_idx) = Self::find_packages_dir_in_path(path) {
            let components: Vec<_> = path.components().collect();
            if packages_idx + 1 < components.len() {
                // Get package name (component after "packages")
                if let Some(std::path::Component::Normal(package_name_os)) = components.get(packages_idx + 1) {
                    if let Some(package_name) = package_name_os.to_str() {
                        // Look for src/ directory
                        if Self::find_src_dir_in_path(path).is_some() {
                            // Get the file stem - this is likely the module name
                            if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                                if !file_stem.is_empty() {
                                    // Capitalize first letter
                                    let mut chars = file_stem.chars();
                                    if let Some(first) = chars.next() {
                                        let capitalized = format!("{}{}", first.to_uppercase(), chars.as_str());
                                        // If file name matches package name, use it; otherwise use file name as module
                                        if capitalized.eq_ignore_ascii_case(package_name) {
                                            return capitalized;
                                        } else {
                                            // Could be a submodule - try package.module format
                                            return format!("{}.{}", package_name, capitalized);
                                        }
                                    }
                                }
                            }
                            // Fallback: use package name as module name
                            return Self::capitalize_first(package_name);
                        }
                    }
                }
            }
        }
        
        // Fallback: try to parse module declaration from file
        if let Ok(content) = std::fs::read_to_string(path) {
            // Read first 50 lines looking for "module {Name}" declaration
            for line in content.lines().take(50) {
                let line_trimmed = line.trim();
                if line_trimmed.starts_with("module ") {
                    // Extract module name: "module ModuleName" or "module Package.ModuleName"
                    let parts: Vec<&str> = line_trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let module_name = parts[1].trim_end_matches(" #");
                        return module_name.to_string();
                    }
                }
            }
        }
        
        // Default to Base
        "Base".to_string()
    }
    
    /// Find the index of "packages" directory component in path
    fn find_packages_dir_in_path(path: &Path) -> Option<usize> {
        let components: Vec<_> = path.components().collect();
        for (idx, component) in components.iter().enumerate() {
            if let std::path::Component::Normal(name) = component {
                if name.to_string_lossy().eq_ignore_ascii_case("packages") {
                    return Some(idx);
                }
            }
        }
        None
    }
    
    /// Find the index of "src" directory component in path
    fn find_src_dir_in_path(path: &Path) -> Option<usize> {
        let components: Vec<_> = path.components().collect();
        for (idx, component) in components.iter().enumerate() {
            if let std::path::Component::Normal(name) = component {
                if name.to_string_lossy().eq_ignore_ascii_case("src") {
                    return Some(idx);
                }
            }
        }
        None
    }
    
    /// Capitalize first letter of a string
    fn capitalize_first(s: &str) -> String {
        let mut chars = s.chars();
        if let Some(first) = chars.next() {
            format!("{}{}", first.to_uppercase(), chars.as_str())
        } else {
            s.to_string()
        }
    }
}

impl Default for BaseDocsRegistry {
    fn default() -> Self {
        Self::empty()
    }
}

