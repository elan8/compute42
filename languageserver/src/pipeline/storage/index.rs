use std::collections::HashMap;
use std::path::PathBuf;
use crate::pipeline::types::{AnalysisResult, Reference, ScopeTree};
use crate::types::{TypeDefinition, TypeDefinitionKind, FunctionSignature};
use crate::types::{Symbol, LspError};
// Legacy types removed - conversion methods no longer needed

/// Unified index combining symbols, references, types, scopes, and signatures
pub struct Index {
    /// Symbol name -> Vec<Symbol> (multiple symbols can have the same name)
    symbols: HashMap<String, Vec<Symbol>>,
    /// File path -> Vec<Symbol> (symbols per file)
    file_symbols: HashMap<PathBuf, Vec<String>>,
    /// Reference name -> Vec<Reference>
    references: HashMap<String, Vec<Reference>>,
    /// File path -> Vec<Reference> (references per file)
    file_references: HashMap<PathBuf, Vec<String>>,
    /// Module -> Type name -> TypeDefinition
    types: HashMap<String, HashMap<String, TypeDefinition>>,
    /// File path -> ScopeTree
    file_scopes: HashMap<PathBuf, ScopeTree>,
    /// Module -> Function name -> Vec<FunctionSignature> (multiple dispatch)
    signatures: HashMap<String, HashMap<String, Vec<FunctionSignature>>>,
    /// Module -> Set of exported symbol names
    exports: HashMap<String, std::collections::HashSet<String>>,
    /// File path -> Set of exports (for tracking which file exports what)
    file_exports: HashMap<PathBuf, std::collections::HashSet<String>>,
}

impl Index {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            file_symbols: HashMap::new(),
            references: HashMap::new(),
            file_references: HashMap::new(),
            types: HashMap::new(),
            file_scopes: HashMap::new(),
            signatures: HashMap::new(),
            exports: HashMap::new(),
            file_exports: HashMap::new(),
        }
    }

    /// Merge analysis results for a file into the index
    /// For dependencies: only indexes exported symbols (not internal functions)
    /// For workspace files: indexes all symbols
    pub fn merge_file(&mut self, file_path: &PathBuf, analysis: AnalysisResult) -> Result<(), LspError> {
        // Remove old data for this file
        self.remove_file(file_path);

        // Check if this is a dependency file (in packages/ directory) vs workspace file
        // Also treat Base/stdlib files as dependencies (they should only index exported symbols)
        let path_str = file_path.to_string_lossy();
        let is_base_file = path_str.contains("/base/") || path_str.contains("\\base\\");
        let is_stdlib_file = path_str.contains("/stdlib/") || path_str.contains("\\stdlib\\");
        let is_dependency = path_str.contains("packages/") || is_base_file || is_stdlib_file;
        
        // Infer module name from file path
        let module_name = Self::infer_module_name_from_path(file_path);
        let is_main_module_file = file_path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == &format!("{}.jl", module_name) || n == &format!("{}.jl", module_name.to_lowercase()))
            .unwrap_or(false);
        
        // Add exports first (so we can use them to filter signatures/types)
        if !analysis.exports.is_empty() {
            log::trace!("Index: Adding {} exports for module '{}' from file {:?}", 
                analysis.exports.len(), module_name, file_path);
            self.exports
                .entry(module_name.clone())
                .or_default()
                .extend(analysis.exports.iter().cloned());
            self.file_exports.insert(file_path.clone(), analysis.exports.clone());
        }
        
        // Get all exports for this module (including ones we just added)
        let all_module_exports: std::collections::HashSet<String> = self.exports
            .get(&module_name)
            .cloned()
            .unwrap_or_default();

        // Add symbols (only for workspace files, not dependencies)
        // Note: Symbols are needed for type inference, but we only extract them for workspace files
        // For Base/stdlib, we rely on signatures for type inference
        if !is_dependency {
            let mut file_symbol_names = Vec::new();
            for symbol in analysis.symbols {
                file_symbol_names.push(symbol.name.clone());
                self.symbols
                    .entry(symbol.name.clone())
                    .or_default()
                    .push(symbol);
            }
            if !file_symbol_names.is_empty() {
                self.file_symbols.insert(file_path.clone(), file_symbol_names);
            }
        }

        // Add references (only for workspace files, not dependencies)
        if !is_dependency {
            let mut file_reference_names = Vec::new();
            for reference in analysis.references {
                file_reference_names.push(reference.name.clone());
                self.references
                    .entry(reference.name.clone())
                    .or_default()
                    .push(reference);
            }
            if !file_reference_names.is_empty() {
                self.file_references.insert(file_path.clone(), file_reference_names);
            }
        }
        
        // Add types
        // For dependencies: only index if exported (or module definition)
        // For workspace files: index all types
        for type_def in analysis.types {
            let is_module_def = type_def.kind == TypeDefinitionKind::Abstract && 
                               type_def.module == type_def.name;
            
            // Always index module definitions (needed for module lookups)
            // For dependencies: only index exported types
            // For workspace files: index all types
            let should_index = is_module_def || 
                              !is_dependency ||
                              all_module_exports.contains(&type_def.name);
            
            if should_index {
                if is_module_def {
                    log::trace!("Index: Adding module type '{}' from file {:?}", type_def.name, file_path);
                }
                self.types
                    .entry(type_def.module.clone())
                    .or_default()
                    .insert(type_def.name.clone(), type_def);
            } else {
                log::trace!("Index: Skipping type '{}' in module '{}' from file {:?} (not exported, dependency)", 
                    type_def.name, type_def.module, file_path);
            }
        }

        // Add scopes (always store, even if empty, as it's needed for scope-aware queries)
        self.file_scopes.insert(file_path.clone(), analysis.scopes);
        
        // Add signatures
        // For Base/stdlib: ONLY index functions with docstrings (they're the documented public API)
        // For other dependencies: index if exported or has documentation
        // For workspace files: index all functions
        for sig in analysis.signatures {
            // Check if function is exported - use the signature's module, not the inferred module
            // This is important because the signature's module might be more accurate (e.g., from qualified names)
            let sig_module_exports: std::collections::HashSet<String> = self.exports
                .get(&sig.module)
                .cloned()
                .unwrap_or_default();
            
            // Also check the inferred module name as a fallback (for cases where module inference was wrong)
            let is_exported = sig_module_exports.contains(&sig.name) || 
                             sig_module_exports.contains(&format!("_{}", sig.name)) ||
                             all_module_exports.contains(&sig.name) || 
                             all_module_exports.contains(&format!("_{}", sig.name));
            
            // Check if function has documentation - functions with docstrings are likely public API
            let has_documentation = sig.doc_comment.as_ref()
                .map(|doc| !doc.trim().is_empty())
                .unwrap_or(false);
            
            // Check if this is a Base/stdlib file
            let is_base_or_stdlib = path_str.contains("/base/") || path_str.contains("\\base\\") ||
                                   path_str.contains("/stdlib/") || path_str.contains("\\stdlib\\") ||
                                   sig.module == "Base" || sig.module.starts_with("Base.");
            
            // For Base/stdlib: ONLY index functions with docstrings (no internal functions)
            // For other dependencies: index if exported, in main module file, or has documentation
            // For workspace files: index all functions
            let should_index = if is_base_or_stdlib {
                // Base/stdlib: only index documented functions
                has_documentation
            } else if is_dependency {
                // Other dependencies: index if exported, in main module file, or has documentation
                is_main_module_file || is_exported || has_documentation
            } else {
                // Workspace files: index all functions
                true
            };
            
            if should_index {
                if sig.module != "Main" && !sig.module.is_empty() {
                    let reason = if is_base_or_stdlib {
                        "Base/stdlib with documentation"
                    } else if !is_dependency {
                        "workspace file"
                    } else if is_main_module_file {
                        "main module file"
                    } else if is_exported {
                        "exported"
                    } else {
                        "has documentation"
                    };
                    log::trace!("Index: Adding function signature '{}' in module '{}' from file {:?} (reason: {}, dependency: {})", 
                        sig.name, sig.module, file_path, reason, is_dependency);
                }
                self.signatures
                    .entry(sig.module.clone())
                    .or_default()
                    .entry(sig.name.clone())
                    .or_default()
                    .push(sig);
            } else {
                let skip_reason = if is_base_or_stdlib {
                    "Base/stdlib function without documentation"
                } else {
                    "not exported, no docs, dependency file"
                };
                log::trace!("Index: Skipping function signature '{}' in module '{}' from file {:?} ({})", 
                    sig.name, sig.module, file_path, skip_reason);
            }
        }

        Ok(())
    }
    
    /// Infer module name from file path (helper for exports)
    fn infer_module_name_from_path(path: &std::path::Path) -> String {
        // Try to extract from path components
        // Common patterns:
        // - packages/DataFrames/{uuid}/src/DataFrames.jl -> DataFrames
        // - packages/CSV/{uuid}/src/CSV.jl -> CSV
        // - .../base/array.jl -> Base
        // - .../stdlib/Test/src/Test.jl -> Test
        let path_str = path.to_string_lossy();
        
        // Check if this is a Base file (case-insensitive for robustness)
        let path_lower = path_str.to_lowercase();
        if path_lower.contains("/base/") || path_lower.contains("\\base\\") {
            return "Base".to_string();
        }
        
        // Check if this is a stdlib file
        if path_str.contains("/stdlib/") || path_str.contains("\\stdlib\\") {
            // Extract stdlib module name from path: .../stdlib/ModuleName/src/...
            if let Some(stdlib_pos) = path_str.find("/stdlib/") {
                let after_stdlib = &path_str[stdlib_pos + 8..]; // Skip "/stdlib/"
                if let Some(slash_pos) = after_stdlib.find('/') {
                    let module_name = &after_stdlib[..slash_pos];
                    // Capitalize first letter
                    let mut chars = module_name.chars();
                    if let Some(first) = chars.next() {
                        return format!("{}{}", first.to_uppercase(), chars.as_str());
                    }
                }
            } else if let Some(stdlib_pos) = path_str.find("\\stdlib\\") {
                let after_stdlib = &path_str[stdlib_pos + 8..]; // Skip "\\stdlib\\"
                if let Some(slash_pos) = after_stdlib.find('\\') {
                    let module_name = &after_stdlib[..slash_pos];
                    // Capitalize first letter
                    let mut chars = module_name.chars();
                    if let Some(first) = chars.next() {
                        return format!("{}{}", first.to_uppercase(), chars.as_str());
                    }
                }
            }
        }
        
        // Check if path contains "packages/{PackageName}/"
        if let Some(packages_pos) = path_str.find("packages/") {
            let after_packages = &path_str[packages_pos + 9..]; // Skip "packages/"
            if let Some(slash_pos) = after_packages.find('/') {
                let package_name = &after_packages[..slash_pos];
                return package_name.to_string();
            }
        }
        
        // Fallback: try to use filename without extension
        if let Some(file_stem) = path.file_stem() {
            if let Some(stem_str) = file_stem.to_str() {
                // Capitalize first letter (Julia module convention)
                let mut chars = stem_str.chars();
                if let Some(first) = chars.next() {
                    return format!("{}{}", first.to_uppercase(), chars.as_str());
                }
            }
        }
        
        "Main".to_string()
    }

    /// Remove all data for a file
    pub fn remove_file(&mut self, file_path: &PathBuf) {
        // Remove symbols
        if let Some(symbol_names) = self.file_symbols.remove(file_path) {
            for name in symbol_names {
                if let Some(symbols) = self.symbols.get_mut(&name) {
                    symbols.retain(|s| s.file_uri != file_path.to_string_lossy());
                    if symbols.is_empty() {
                        self.symbols.remove(&name);
                    }
                }
            }
        }

        // Remove references
        if let Some(reference_names) = self.file_references.remove(file_path) {
            for name in reference_names {
                if let Some(references) = self.references.get_mut(&name) {
                    references.retain(|r| r.file_uri != file_path.to_string_lossy());
                    if references.is_empty() {
                        self.references.remove(&name);
                    }
                }
            }
        }

        // Remove scopes
        self.file_scopes.remove(file_path);
        
        // Remove exports for this file (but keep module exports if they exist in other files)
        // We only remove the file's contribution, not the entire module's exports
        // This is important because exports might be collected in PASS 0 before this file is processed
        self.file_exports.remove(file_path);
        // Note: We don't remove from module exports here because:
        // 1. Exports might be in multiple files (main file + other files)
        // 2. Exports are collected in PASS 0 before files are processed
        // 3. Module exports are cumulative across all files
    }

    /// Get all symbols
    pub fn get_all_symbols(&self) -> Vec<Symbol> {
        self.symbols.values().flatten().cloned().collect()
    }

    /// Find symbols by name
    pub fn find_symbols(&self, name: &str) -> Vec<Symbol> {
        self.symbols.get(name).cloned().unwrap_or_default()
    }

    /// Find symbols with prefix
    pub fn find_symbols_with_prefix(&self, prefix: &str) -> Vec<Symbol> {
        self.symbols
            .iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .flat_map(|(_, symbols)| symbols.clone())
            .collect()
    }

    /// Get all references
    pub fn get_all_references(&self) -> Vec<Reference> {
        self.references.values().flatten().cloned().collect()
    }

    /// Find references by name
    pub fn find_references(&self, name: &str) -> Vec<Reference> {
        self.references.get(name).cloned().unwrap_or_default()
    }

    /// Find type definition
    pub fn find_type(&self, module: &str, name: &str) -> Option<TypeDefinition> {
        self.types.get(module)?.get(name).cloned()
    }

    /// Find function signatures
    pub fn find_signatures(&self, module: &str, name: &str) -> Vec<FunctionSignature> {
        self.signatures
            .get(module)
            .and_then(|m| m.get(name))
            .cloned()
            .unwrap_or_default()
    }

    /// Get all function names in a module
    pub fn get_module_functions(&self, module: &str) -> Vec<String> {
        self.signatures
            .get(module)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all type names in a module
    pub fn get_module_types(&self, module: &str) -> Vec<String> {
        self.types
            .get(module)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all modules that have function signatures
    pub fn get_all_modules(&self) -> Vec<String> {
        self.signatures.keys().cloned().collect()
    }

    /// Get all modules that have type definitions
    pub fn get_all_type_modules(&self) -> Vec<String> {
        self.types.keys().cloned().collect()
    }

    /// Find function signature by module and name (TypeRegistry-compatible interface)
    /// Returns the signatures as a reference for compatibility with TypeRegistry API
    pub fn find_function(&self, module: &str, name: &str) -> Option<Vec<FunctionSignature>> {
        let sigs = self.find_signatures(module, name);
        if sigs.is_empty() {
            None
        } else {
            Some(sigs)
        }
    }

    /// Find function signature by qualified name (e.g., "CSV.read", "Download.download") - TypeRegistry-compatible
    pub fn find_function_by_qualified_name(&self, qualified_name: &str) -> Option<Vec<FunctionSignature>> {
        if let Some((module, name)) = qualified_name.split_once('.') {
            log::trace!("Index: Looking up qualified function '{}' in module '{}'", name, module);
            let result = self.find_function(module, name);
            if result.is_none() {
                // Try submodules (e.g., "Downloads.download" might be in "Downloads" or a submodule)
                // Also try variations like "Download" vs "Downloads"
                let module_variants = vec![
                    module.to_string(),
                    format!("{}.{}", module, name), // Try nested module
                    if module.ends_with('s') { module.strip_suffix('s').unwrap_or(module).to_string() } 
                    else { format!("{}s", module) }, // Try plural/singular
                ];
                
                for variant in module_variants {
                    if let Some(found) = self.find_function(&variant, name) {
                        log::trace!("Index: Found function '{}' in variant module '{}'", name, variant);
                        return Some(found);
                    }
                }
                
                log::trace!("Index: Function '{}' not found in module '{}'. Available modules: {:?}", 
                    name, module, self.signatures.keys().take(20).collect::<Vec<_>>());
            }
            result
        } else {
            // Try Base module
            self.find_function("Base", qualified_name)
        }
    }

    /// Get return type for a function call (TypeRegistry-compatible)
    /// If name is not qualified, searches across all modules
    /// If preferred_modules is provided (from "using" statements), those are checked first
    /// Otherwise, Base is checked first, then all other modules
    pub fn get_return_type(&self, qualified_name: &str, preferred_modules: Option<&[&str]>) -> Option<crate::types::TypeExpr> {
        // If qualified (contains '.'), use direct lookup
        if qualified_name.contains('.') {
            log::trace!("Index: get_return_type called for qualified name '{}'", qualified_name);
            if let Some(signatures) = self.find_function_by_qualified_name(qualified_name) {
                log::trace!("Index: Found {} signature(s) for '{}'", signatures.len(), qualified_name);
                return signatures.first()?.return_type.clone();
            }
            log::trace!("Index: No signatures found for qualified name '{}'", qualified_name);
            return None;
        }
        
        // Unqualified name - search across modules
        // If preferred modules are provided (from "using" statements), check those first
        if let Some(preferred) = preferred_modules {
            if !preferred.is_empty() {
                log::trace!("Index: Checking preferred modules first for return type of '{}': {:?}", qualified_name, preferred);
                for module in preferred {
                    let signatures = self.find_signatures(module, qualified_name);
                    if let Some(sig) = signatures.first() {
                        if let Some(ref return_type) = sig.return_type {
                            log::trace!("Index: Found return type for '{}' in preferred module '{}'", qualified_name, module);
                            return Some(return_type.clone());
                        }
                    }
                    
                    // Also search submodules (e.g., DataFrames.AbstractDataFrame, DataFrames.DataFrame)
                    // This handles cases where functions are stored in submodules but exported by the package
                    for (stored_module, sigs_map) in &self.signatures {
                        if stored_module.starts_with(&format!("{}.", module)) {
                            if let Some(signatures) = sigs_map.get(qualified_name) {
                                if let Some(sig) = signatures.first() {
                                    if let Some(ref return_type) = sig.return_type {
                                        log::trace!("Index: Found return type for '{}' in submodule '{}' of package '{}'", qualified_name, stored_module, module);
                                        return Some(return_type.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Check Base first (always available)
        let signatures = self.find_signatures("Base", qualified_name);
        if let Some(sig) = signatures.first() {
            if let Some(ref return_type) = sig.return_type {
                log::trace!("Index: Found return type for '{}' in Base module", qualified_name);
                return Some(return_type.clone());
            }
        }
        
        // Search all other modules
        for (module, sigs_map) in &self.signatures {
            // Skip Base (already checked) and preferred modules (already checked)
            if module == "Base" || (preferred_modules.is_some() && preferred_modules.unwrap().contains(&module.as_str())) {
                continue;
            }
            if let Some(signatures) = sigs_map.get(qualified_name) {
                if let Some(sig) = signatures.first() {
                    if let Some(ref return_type) = sig.return_type {
                        log::trace!("Index: Found return type for '{}' in module '{}'", qualified_name, module);
                        return Some(return_type.clone());
                    }
                }
            }
        }
        
        None
    }

    /// Get return type for a function call with argument types (multiple dispatch)
    pub fn get_return_type_with_args(&self, qualified_name: &str, arg_types: &[crate::types::TypeExpr]) -> Option<crate::types::TypeExpr> {
        if let Some(signatures) = self.find_function_by_qualified_name(qualified_name) {
            if let Some(best_match) = self.find_best_match(&signatures, arg_types) {
                return best_match.return_type.clone();
            }
            // Fallback to first signature if no match found
            signatures.first()?.return_type.clone()
        } else {
            None
        }
    }

    /// Find the best matching function signature based on argument types
    /// Uses basic type compatibility: exact match > compatible match > Any
    fn find_best_match<'a>(&self, signatures: &'a [FunctionSignature], arg_types: &[crate::types::TypeExpr]) -> Option<&'a FunctionSignature> {
        if signatures.is_empty() || arg_types.is_empty() {
            return signatures.first();
        }

        let mut best_match: Option<(&'a FunctionSignature, usize)> = None;

        for sig in signatures {
            if sig.parameters.len() != arg_types.len() {
                continue;
            }

            let mut match_score = 0;
            let mut all_match = true;

            for (param, arg_type) in sig.parameters.iter().zip(arg_types.iter()) {
                if let Some(ref param_type) = param.param_type {
                    if self.types_compatible(arg_type, param_type) {
                        if arg_type == param_type {
                            match_score += 2; // Exact match
                        } else {
                            match_score += 1; // Compatible match
                        }
                    } else {
                        all_match = false;
                        break;
                    }
                } else {
                    // Parameter type is Any or unknown, lower score
                    match_score += 0;
                }
            }

            if all_match {
                match best_match {
                    Some((_, best_score)) if match_score > best_score => {
                        best_match = Some((sig, match_score));
                    }
                    None => {
                        best_match = Some((sig, match_score));
                    }
                    _ => {}
                }
            }
        }

        best_match.map(|(sig, _)| sig)
    }

    /// Check if two types are compatible (basic compatibility check)
    fn types_compatible(&self, arg_type: &crate::types::TypeExpr, param_type: &crate::types::TypeExpr) -> bool {
        match (arg_type, param_type) {
            // Exact match
            (crate::types::TypeExpr::Concrete(a), crate::types::TypeExpr::Concrete(b)) if a == b => true,
            (crate::types::TypeExpr::Generic(a_name, a_params), crate::types::TypeExpr::Generic(b_name, b_params)) 
                if a_name == b_name && a_params.len() == b_params.len() => {
                a_params.iter().zip(b_params.iter())
                    .all(|(a, b)| self.types_compatible(a, b))
            }
            // Any accepts everything
            (_, crate::types::TypeExpr::Any) => true,
            // Unknown is compatible with anything
            (crate::types::TypeExpr::Unknown, _) => true,
            (_, crate::types::TypeExpr::Unknown) => true,
            // Union types: check if arg_type is in the union
            (arg, crate::types::TypeExpr::Union(union_types)) => {
                union_types.iter().any(|ut| self.types_compatible(arg, ut))
            }
            _ => false,
        }
    }


    /// Get scope tree for a file
    pub fn get_file_scopes(&self, file_path: &PathBuf) -> Option<&ScopeTree> {
        self.file_scopes.get(file_path)
    }

    /// Find all symbols in a specific file
    pub fn find_symbols_in_file(&self, file_path: &PathBuf) -> Vec<Symbol> {
        let file_uri = file_path.to_string_lossy().to_string();
        if let Some(symbol_names) = self.file_symbols.get(file_path) {
            symbol_names
                .iter()
                .flat_map(|name| {
                    self.symbols
                        .get(name)
                        .map(|symbols| {
                            symbols
                                .iter()
                                .filter(|s| s.file_uri == file_uri)
                                .cloned()
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect()
        } else {
            Vec::new()
        }
    }


    /// Find symbol by name (first match)
    pub fn find_symbol(&self, name: &str) -> Option<Symbol> {
        self.symbols.get(name)?.first().cloned()
    }

    /// Get documentation for a function by module and name
    /// Returns the first non-empty docstring from any signature
    pub fn get_function_documentation(&self, module: &str, name: &str) -> Option<String> {
        let signatures = self.find_signatures(module, name);
        for sig in signatures {
            if let Some(ref doc) = sig.doc_comment {
                if !doc.trim().is_empty() {
                    return Some(doc.clone());
                }
            }
        }
        None
    }

    /// Get documentation for a type by module and name
    pub fn get_type_documentation(&self, module: &str, name: &str) -> Option<String> {
        self.find_type(module, name)?.doc_comment.clone()
    }

    /// Get documentation for a symbol by qualified name (e.g., "CSV.read" or "DataFrames.DataFrame")
    /// Tries function first, then type
    /// For unqualified names, searches across all modules (not just Base) to find documentation
    pub fn get_documentation(&self, qualified_name: &str) -> Option<String> {
        if let Some((module, name)) = qualified_name.split_once('.') {
            // Qualified name - try function first
            if let Some(doc) = self.get_function_documentation(module, name) {
                return Some(doc);
            }
            // Then try type
            if let Some(doc) = self.get_type_documentation(module, name) {
                return Some(doc);
            }
        } else {
            // Unqualified name - search across all modules
            // This handles cases where functions might be in submodules (e.g., Base.Filesystem.joinpath)
            // First try Base module (most common case)
            if let Some(doc) = self.get_function_documentation("Base", qualified_name) {
                return Some(doc);
            }
            if let Some(doc) = self.get_type_documentation("Base", qualified_name) {
                return Some(doc);
            }
            
            // Then search all other modules for the function name
            // This ensures we find functions in submodules (e.g., Base.Filesystem.joinpath)
            for (module, sigs_map) in &self.signatures {
                if module == "Base" {
                    continue; // Already checked above
                }
                if let Some(signatures) = sigs_map.get(qualified_name) {
                    // Find first non-empty docstring
                    for sig in signatures {
                        if let Some(ref doc) = sig.doc_comment {
                            if !doc.trim().is_empty() {
                                log::trace!("Index: Found documentation for '{}' in module '{}'", qualified_name, module);
                                return Some(doc.clone());
                            }
                        }
                    }
                }
            }
            
            // Also search types across all modules
            for (module, types_map) in &self.types {
                if module == "Base" {
                    continue; // Already checked above
                }
                if let Some(type_def) = types_map.get(qualified_name) {
                    if let Some(ref doc) = type_def.doc_comment {
                        if !doc.trim().is_empty() {
                            log::trace!("Index: Found type documentation for '{}' in module '{}'", qualified_name, module);
                            return Some(doc.clone());
                        }
                    }
                }
            }
        }
        None
    }

    /// Search for documentation by function name across all modules
    /// Returns the first matching documentation found
    /// If preferred_modules is provided, those modules are checked first (from "using" statements)
    /// Otherwise, falls back to searching all modules
    pub fn find_documentation_by_name(&self, name: &str, preferred_modules: Option<&[&str]>) -> Option<String> {
        log::trace!("Index: Searching for '{}' across all modules", name);
        
        // If preferred modules are provided (from "using" statements), check those first
        if let Some(preferred) = preferred_modules {
            if !preferred.is_empty() {
                log::trace!("Index: Checking preferred modules first: {:?}", preferred);
                for module in preferred {
                    if let Some(doc) = self.get_function_documentation(module, name) {
                        log::trace!("Index: Found documentation for '{}' in preferred module '{}'", name, module);
                        return Some(doc);
                    }
                }
            }
        }
        
        // Search all modules (including preferred ones if they weren't found above)
        // This ensures we don't miss documentation if preferred modules don't have it
        for (module, sigs_map) in &self.signatures {
            if let Some(signatures) = sigs_map.get(name) {
                // Find first non-empty docstring
                for sig in signatures {
                    if let Some(ref doc) = sig.doc_comment {
                        if !doc.trim().is_empty() {
                            log::trace!("Index: Found documentation for '{}' in module '{}'", name, module);
                            return Some(doc.clone());
                        }
                    }
                }
            }
        }
        
        log::trace!("Index: No documentation found for '{}'", name);
        None
    }

    /// Merge another index into this index
    pub fn merge(&mut self, other: Index) {
        // Merge symbols
        for (name, mut symbols) in other.symbols {
            self.symbols
                .entry(name)
                .or_default()
                .append(&mut symbols);
        }

        // Merge file symbols
        for (path, mut names) in other.file_symbols {
            self.file_symbols
                .entry(path)
                .or_default()
                .append(&mut names);
        }

        // Merge references
        for (name, mut references) in other.references {
            self.references
                .entry(name)
                .or_default()
                .append(&mut references);
        }

        // Merge file references
        for (path, mut names) in other.file_references {
            self.file_references
                .entry(path)
                .or_default()
                .append(&mut names);
        }

        // Merge types
        for (module, types_map) in other.types {
            let module_types = self.types.entry(module).or_default();
            for (name, type_def) in types_map {
                module_types.insert(name, type_def);
            }
        }

        // Merge scopes (replace if exists)
        for (path, scope_tree) in other.file_scopes {
            self.file_scopes.insert(path, scope_tree);
        }

        // Merge signatures
        for (module, sigs_map) in other.signatures {
            let module_sigs = self.signatures.entry(module).or_default();
            for (name, mut signatures) in sigs_map {
                module_sigs
                    .entry(name)
                    .or_default()
                    .append(&mut signatures);
            }
        }
        
        // Merge exports
        for (module, mut exports) in other.exports {
            self.exports
                .entry(module)
                .or_default()
                .extend(exports.drain());
        }
        
        // Merge file exports
        for (path, exports) in other.file_exports {
            self.file_exports.insert(path, exports);
        }
    }
    
    /// Get all exports for a module
    pub fn get_module_exports(&self, module: &str) -> std::collections::HashSet<String> {
        self.exports.get(module).cloned().unwrap_or_default()
    }
    
    /// Check if a symbol is exported by a module
    pub fn is_exported(&self, module: &str, symbol: &str) -> bool {
        self.exports
            .get(module)
            .map(|exports| exports.contains(symbol))
            .unwrap_or(false)
    }
    
    /// Get all modules that export a given symbol
    pub fn find_modules_exporting(&self, symbol: &str) -> Vec<String> {
        self.exports
            .iter()
            .filter(|(_, exports)| exports.contains(symbol))
            .map(|(module, _)| module.clone())
            .collect()
    }
    
    /// Add exports for a module (used during initial export collection pass)
    pub fn add_exports(&mut self, module: String, exports: std::collections::HashSet<String>, file_path: PathBuf) {
        self.exports
            .entry(module.clone())
            .or_default()
            .extend(exports.iter().cloned());
        self.file_exports.insert(file_path, exports);
    }
    
    /// Pre-populate exports for a module (convenience method for pre-loading exports from external sources like exports.jl)
    /// This is useful when you want to mark symbols as exported before processing files,
    /// so they won't be filtered out during indexing.
    pub fn preload_exports(&mut self, module: &str, exports: std::collections::HashSet<String>) {
        self.exports
            .entry(module.to_string())
            .or_default()
            .extend(exports.iter().cloned());
        log::trace!("Index: Pre-loaded {} exports for module '{}'", exports.len(), module);
    }
    
    /// Promote submodule functions to parent module
    /// 
    /// This makes all exported functions from submodules (e.g., Flux.Losses) available
    /// at the parent module level (e.g., Flux), which matches Julia's behavior when
    /// you do `using Flux` - submodule functions become available at the top level.
    /// 
    /// For example, `Flux.Losses.crossentropy` will also be available as `Flux.crossentropy`.
    pub fn promote_submodule_functions(&mut self) {
        // Collect all submodules (modules with dots in their name)
        let submodules: Vec<(String, String)> = self.signatures
            .keys()
            .filter_map(|module| {
                if let Some(dot_pos) = module.find('.') {
                    let parent = module[..dot_pos].to_string();
                    Some((parent, module.clone()))
                } else {
                    None
                }
            })
            .collect();
        
        if submodules.is_empty() {
            return;
        }
        
        log::trace!("Index: Promoting functions from {} submodules to parent modules", submodules.len());
        
        // For each submodule, copy exported functions to parent module
        for (parent_module, submodule) in submodules {
            // Get exports for the submodule
            let submodule_exports = self.exports
                .get(&submodule)
                .cloned()
                .unwrap_or_default();
            
            // Collect functions to promote (to avoid borrow checker issues)
            let functions_to_promote: Vec<(String, Vec<FunctionSignature>)> = {
                if let Some(submodule_sigs) = self.signatures.get(&submodule) {
                    submodule_sigs
                        .iter()
                        .filter_map(|(func_name, signatures)| {
                            // Only promote if the function is exported from the submodule
                            // or if the submodule has no explicit exports (assume all are exported)
                            let should_promote = submodule_exports.is_empty() || 
                                               submodule_exports.contains(func_name);
                            if should_promote {
                                Some((func_name.clone(), signatures.clone()))
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            };
            
            // Now add the functions to the parent module
            if !functions_to_promote.is_empty() {
                let parent_sigs = self.signatures
                    .entry(parent_module.clone())
                    .or_default();
                
                let mut promoted_count = 0;
                for (func_name, signatures) in functions_to_promote {
                    // Add the function to parent module (avoid duplicates)
                    if !parent_sigs.contains_key(&func_name) {
                        parent_sigs.insert(func_name.clone(), signatures);
                        promoted_count += 1;
                    }
                }
                
                if promoted_count > 0 {
                    log::trace!("Index: Promoted {} functions from '{}' to '{}'", 
                        promoted_count, submodule, parent_module);
                }
            }
        }
    }
    
    /// Reconstruct index from serialized data (for persistence)
    /// This is used to load a cached base index
    pub fn from_serialized(
        types: HashMap<String, HashMap<String, TypeDefinition>>,
        signatures: HashMap<String, HashMap<String, Vec<FunctionSignature>>>,
        exports: HashMap<String, std::collections::HashSet<String>>,
    ) -> Self {
        let mut index = Self::new();
        index.types = types;
        index.signatures = signatures;
        index.exports = exports;
        index
    }
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::types::AnalysisResult;

    #[test]
    fn test_index_new() {
        let index = Index::new();
        assert_eq!(index.get_all_symbols().len(), 0);
    }

    #[test]
    fn test_merge_file() {
        let mut index = Index::new();
        let mut analysis = AnalysisResult::new();
        
        let symbol = Symbol {
            name: "test".to_string(),
            kind: crate::types::SymbolKind::Function,
            range: crate::types::Range {
                start: crate::types::Position { line: 0, character: 0 },
                end: crate::types::Position { line: 0, character: 10 },
            },
            scope_id: 0,
            doc_comment: None,
            signature: None,
            file_uri: "test.jl".to_string(),
        };
        analysis.symbols.push(symbol);

        let file_path = PathBuf::from("test.jl");
        index.merge_file(&file_path, analysis).unwrap();

        assert_eq!(index.get_all_symbols().len(), 1);
        assert_eq!(index.find_symbols("test").len(), 1);
    }

    #[test]
    fn test_remove_file() {
        let mut index = Index::new();
        let mut analysis = AnalysisResult::new();
        
        let symbol = Symbol {
            name: "test".to_string(),
            kind: crate::types::SymbolKind::Function,
            range: crate::types::Range {
                start: crate::types::Position { line: 0, character: 0 },
                end: crate::types::Position { line: 0, character: 10 },
            },
            scope_id: 0,
            doc_comment: None,
            signature: None,
            file_uri: "test.jl".to_string(),
        };
        analysis.symbols.push(symbol);

        let file_path = PathBuf::from("test.jl");
        index.merge_file(&file_path, analysis).unwrap();
        assert_eq!(index.get_all_symbols().len(), 1);

        index.remove_file(&file_path);
        assert_eq!(index.get_all_symbols().len(), 0);
    }

    #[test]
    fn test_merge_index() {
        let mut index1 = Index::new();
        let mut analysis1 = AnalysisResult::new();
        
        let symbol1 = Symbol {
            name: "func1".to_string(),
            kind: crate::types::SymbolKind::Function,
            range: crate::types::Range {
                start: crate::types::Position { line: 0, character: 0 },
                end: crate::types::Position { line: 0, character: 10 },
            },
            scope_id: 0,
            doc_comment: None,
            signature: None,
            file_uri: "file1.jl".to_string(),
        };
        analysis1.symbols.push(symbol1);

        let file_path1 = PathBuf::from("file1.jl");
        index1.merge_file(&file_path1, analysis1).unwrap();

        let mut index2 = Index::new();
        let mut analysis2 = AnalysisResult::new();
        
        let symbol2 = Symbol {
            name: "func2".to_string(),
            kind: crate::types::SymbolKind::Function,
            range: crate::types::Range {
                start: crate::types::Position { line: 0, character: 0 },
                end: crate::types::Position { line: 0, character: 10 },
            },
            scope_id: 0,
            doc_comment: None,
            signature: None,
            file_uri: "file2.jl".to_string(),
        };
        analysis2.symbols.push(symbol2);

        let file_path2 = PathBuf::from("file2.jl");
        index2.merge_file(&file_path2, analysis2).unwrap();

        index1.merge(index2);
        assert_eq!(index1.get_all_symbols().len(), 2);
    }
}

