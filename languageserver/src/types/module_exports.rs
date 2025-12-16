use crate::pipeline::storage::Index;
use std::collections::{HashMap, HashSet};
use tree_sitter::{Node, Tree};

/// Represents an imported module with its available symbols
#[derive(Debug, Clone)]
pub struct ImportedModule {
    /// Module name (e.g., "DataFrames")
    pub name: String,
    /// Alias if imported with `as` (e.g., "DF" in `using DataFrames as DF`)
    pub alias: Option<String>,
    /// Specific symbols imported (if using selective import like `using Module: symbol1, symbol2`)
    /// If None, all exports are available
    pub specific_symbols: Option<HashSet<String>>,
    /// Whether this is an `import` (qualified only) or `using` (unqualified available)
    pub is_qualified_only: bool,
}

/// Tracks which modules are imported and what symbols they export
#[derive(Debug, Clone)]
pub struct ImportContext {
    /// Map of module name -> ImportedModule
    imported_modules: HashMap<String, ImportedModule>,
    /// Map of symbol name -> module name (for unqualified lookups)
    symbol_to_module: HashMap<String, String>,
    /// Set of all available unqualified symbols
    available_symbols: HashSet<String>,
}

impl ImportContext {
    /// Create a new empty import context
    pub fn new() -> Self {
        Self {
            imported_modules: HashMap::new(),
            symbol_to_module: HashMap::new(),
            available_symbols: HashSet::new(),
        }
    }

    /// Build import context from a parsed tree using Index
    pub fn from_tree_with_index(
        tree: &Tree,
        text: &str,
        index: &Index,
    ) -> Self {
        let mut context = Self::new();
        context.parse_imports(tree.root_node(), text);
        
        // Populate available symbols from imported modules using Index
        context.populate_from_index(index);
        
        context
    }

    /// Parse import statements from the AST
    fn parse_imports(&mut self, node: Node, text: &str) {
        match node.kind() {
            "using_statement" => {
                self.parse_using_statement(node, text, false);
            }
            "import_statement" => {
                self.parse_using_statement(node, text, true);
            }
            _ => {}
        }

        // Recursively process children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.parse_imports(child, text);
            }
        }
    }

    /// Parse a using or import statement
    fn parse_using_statement(&mut self, node: Node, text: &str, is_qualified_only: bool) {
        // Collect all module names from comma-separated list
        let mut module_names = Vec::new();
        let mut alias: Option<String> = None;
        let mut specific_symbols: Option<HashSet<String>> = None;
        let mut after_colon = false;
        let mut after_as = false;

        // Walk through the statement to extract module names, alias, and specific symbols
        // Handle comma-separated module lists
        let mut current_module_parts = Vec::new();
        let mut in_symbol_list = false;
        
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "identifier" => {
                        if let Ok(name) = child.utf8_text(text.as_bytes()) {
                            if name == "using" || name == "import" {
                                continue;
                            }
                            
                            if after_as {
                                // This is the alias (applies to the last module)
                                alias = Some(name.to_string());
                                after_as = false;
                            } else if after_colon || in_symbol_list {
                                // This is a specific symbol to import
                                if specific_symbols.is_none() {
                                    specific_symbols = Some(HashSet::new());
                                }
                                if let Some(ref mut symbols) = specific_symbols {
                                    symbols.insert(name.to_string());
                                }
                            } else {
                                // This is part of a module name
                                current_module_parts.push(name.to_string());
                            }
                        }
                    }
                    "field_access" | "field_expression" => {
                        // Qualified module name like "Package.Module"
                        if let Ok(qualified) = child.utf8_text(text.as_bytes()) {
                            // If we have accumulated parts, save them as a module first
                            if !current_module_parts.is_empty() {
                                let module_name = current_module_parts.join(".");
                                if !module_name.is_empty() {
                                    module_names.push(module_name);
                                }
                                current_module_parts.clear();
                            }
                            // Add the qualified name as a module
                            module_names.push(qualified.to_string());
                        }
                    }
                    "operator" => {
                        if let Ok(op) = child.utf8_text(text.as_bytes()) {
                            if op == ":" {
                                // Start of specific symbol list
                                after_colon = true;
                                in_symbol_list = true;
                                // Save current module before symbol list
                                if !current_module_parts.is_empty() {
                                    let module_name = current_module_parts.join(".");
                                    if !module_name.is_empty() {
                                        module_names.push(module_name);
                                    }
                                    current_module_parts.clear();
                                }
                            }
                        }
                    }
                    "keyword" => {
                        if let Ok(kw) = child.utf8_text(text.as_bytes()) {
                            if kw == "as" {
                                // Next identifier will be the alias
                                after_as = true;
                                // Save current module before alias
                                if !current_module_parts.is_empty() {
                                    let module_name = current_module_parts.join(".");
                                    if !module_name.is_empty() {
                                        module_names.push(module_name);
                                    }
                                    current_module_parts.clear();
                                }
                            }
                        }
                    }
                    "," => {
                        if in_symbol_list {
                            // Comma in symbol list - already handled by identifier processing
                            continue;
                        } else {
                            // Comma separator between modules
                            // Save current module and start a new one
                            if !current_module_parts.is_empty() {
                                let module_name = current_module_parts.join(".");
                                if !module_name.is_empty() {
                                    module_names.push(module_name);
                                }
                                current_module_parts.clear();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Don't forget the last module if there's no trailing comma
        if !current_module_parts.is_empty() {
            let module_name = current_module_parts.join(".");
            if !module_name.is_empty() {
                module_names.push(module_name);
            }
        }

        // If we didn't find any modules using the comma-separated approach,
        // fall back to the old method (for single module statements or edge cases)
        if module_names.is_empty() {
            let mut module_name = String::new();
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    match child.kind() {
                        "identifier" => {
                            if let Ok(name) = child.utf8_text(text.as_bytes()) {
                                if name != "using" && name != "import" && !after_colon {
                                    if module_name.is_empty() {
                                        module_name = name.to_string();
                                    } else {
                                        module_name.push('.');
                                        module_name.push_str(name);
                                    }
                                }
                            }
                        }
                        "field_access" | "field_expression" => {
                            if let Ok(qualified) = child.utf8_text(text.as_bytes()) {
                                if module_name.is_empty() {
                                    module_name = qualified.to_string();
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            if !module_name.is_empty() {
                module_names.push(module_name);
            }
        }

        // Register each module
        for module_name in module_names {
            let imported = ImportedModule {
                name: module_name.clone(),
                alias: alias.clone(), // Alias applies to all modules in the list (Julia behavior)
                specific_symbols: specific_symbols.clone(),
                is_qualified_only,
            };
            
            self.imported_modules.insert(module_name, imported);
        }
    }

    /// Populate available symbols from Index based on imported modules
    fn populate_from_index(&mut self, index: &Index) {
        for (module_name, imported) in &self.imported_modules {
            if imported.is_qualified_only {
                // For `import Module`, symbols are only available as Module.symbol
                continue;
            }

            // If specific symbols are listed, only add those
            if let Some(ref specific) = imported.specific_symbols {
                for symbol in specific {
                    // Check if this symbol exists in the index
                    if Self::symbol_exists_in_index(index, module_name, symbol) {
                        self.available_symbols.insert(symbol.clone());
                        self.symbol_to_module.insert(symbol.clone(), module_name.clone());
                    }
                }
            } else {
                // All exports are available - add all functions and types from this module
                // Get all functions from this module
                for func_name in index.get_module_functions(module_name) {
                    self.available_symbols.insert(func_name.clone());
                    self.symbol_to_module.insert(func_name, module_name.clone());
                }
                
                // Get all types from this module
                for type_name in index.get_module_types(module_name) {
                    self.available_symbols.insert(type_name.clone());
                    self.symbol_to_module.insert(type_name, module_name.clone());
                }
            }
        }
    }

    /// Check if a symbol exists in the index for a given module
    fn symbol_exists_in_index(index: &Index, module: &str, symbol: &str) -> bool {
        // Check functions (signatures)
        if !index.find_signatures(module, symbol).is_empty() {
            return true;
        }
        // Check types
        if index.find_type(module, symbol).is_some() {
            return true;
        }
        false
    }

    /// Check if a symbol is available (either locally defined or from imported module)
    pub fn is_symbol_available(&self, symbol: &str) -> bool {
        self.available_symbols.contains(symbol)
    }

    /// Get the module name for an unqualified symbol
    pub fn get_module_for_symbol(&self, symbol: &str) -> Option<&String> {
        self.symbol_to_module.get(symbol)
    }

    /// Check if a module is imported
    pub fn is_module_imported(&self, module: &str) -> bool {
        self.imported_modules.contains_key(module)
    }

    /// Get imported module info
    pub fn get_imported_module(&self, module: &str) -> Option<&ImportedModule> {
        self.imported_modules.get(module)
    }

    /// Get all imported module names
    pub fn imported_modules(&self) -> Vec<&String> {
        self.imported_modules.keys().collect()
    }

    /// Check if a qualified symbol (Module.symbol) is valid
    pub fn is_qualified_symbol_valid(&self, module: &str, symbol: &str, index: Option<&Index>) -> bool {
        // Check if module is imported
        if !self.is_module_imported(module) {
            return false;
        }

        // If we have an index, check if the symbol exists
        if let Some(idx) = index {
            if Self::symbol_exists_in_index(idx, module, symbol) {
                return true;
            }
            // If module is imported but symbol not in index, be conservative
            // The module might not be fully indexed, so assume it might exist
            return true;
        }

        // Without index, assume it's valid if module is imported
        true
    }

    /// Check if an unqualified symbol could come from an imported module (using Index)
    pub fn could_symbol_come_from_imports(&self, symbol: &str, index: Option<&Index>) -> bool {
        // Check if it's in our available symbols
        if self.is_symbol_available(symbol) {
            return true;
        }

        // Check all imported modules (for modules that export everything)
        for (module_name, imported) in &self.imported_modules {
            if imported.is_qualified_only {
                continue; // import Module only allows Module.symbol
            }
            
            // If specific symbols are listed, we already checked available_symbols
            if let Some(ref specific) = imported.specific_symbols {
                // Only check if symbol is in the specific list
                if specific.contains(symbol) {
                    // Check index if available
                    if let Some(idx) = index {
                        if Self::symbol_exists_in_index(idx, module_name, symbol) {
                            return true;
                        }
                    } else {
                        // No index - assume it exists if in specific list
                        return true;
                    }
                }
                continue;
            }
            
            // Module exports everything - check index if available
            if let Some(idx) = index {
                if Self::symbol_exists_in_index(idx, module_name, symbol) {
                    return true;
                }
            } else {
                // No index - assume it might exist
                return true;
            }
        }
        
        false
    }
}

impl Default for ImportContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Module exports tracker that maps module names to their exported symbols
#[derive(Debug, Clone)]
pub struct ModuleExports {
    /// Map of module name -> set of exported symbol names
    exports: HashMap<String, HashSet<String>>,
}

impl ModuleExports {
    /// Create a new empty module exports tracker
    pub fn new() -> Self {
        Self {
            exports: HashMap::new(),
        }
    }


    /// Add exported symbol for a module
    pub fn add_export(&mut self, module: &str, symbol: &str) {
        self.exports
            .entry(module.to_string())
            .or_default()
            .insert(symbol.to_string());
    }

    /// Check if a module exports a symbol
    pub fn is_exported(&self, module: &str, symbol: &str) -> bool {
        self.exports
            .get(module)
            .map(|symbols| symbols.contains(symbol))
            .unwrap_or(false)
    }

    /// Get all exports for a module
    pub fn get_module_exports(&self, module: &str) -> Option<&HashSet<String>> {
        self.exports.get(module)
    }
}

impl Default for ModuleExports {
    fn default() -> Self {
        Self::new()
    }
}


