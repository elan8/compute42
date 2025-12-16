use crate::types::ImportContext;
use crate::pipeline::storage::Index;
use crate::pipeline::query::symbol::SymbolQuery;
use crate::types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tree_sitter::Node;
use std::collections::{HashMap, HashSet};

use super::utils;

/// Check for undefined variable references
pub(super) fn check_undefined_variables(
    tree: &tree_sitter::Tree,
    text: &str,
    index: &Index,
    import_context: Option<&ImportContext>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let root = tree.root_node();
    let mut defined_symbols = HashSet::new();
    let mut symbol_definitions = HashMap::new();
    // Map from function node ID to its parameter names (for scoped checking)
    let mut function_scopes: HashMap<usize, HashSet<String>> = HashMap::new();
    
    // Debug: Dump CST for function definitions
    // Disabled: semantic_debug_functions.txt writing
    // #[cfg(debug_assertions)]
    // {
    //     use std::fs;
    //     use std::path::PathBuf;
    //     use super::debug;
    //     let debug_output = debug::dump_function_definitions_cst(root, text);
    //     let debug_path = PathBuf::from("temp").join("semantic_debug_functions.txt");
    //     if let Some(parent) = debug_path.parent() {
    //         let _ = fs::create_dir_all(parent);
    //     }
    //     let _ = fs::write(&debug_path, debug_output);
    //     log::trace!("Function definitions CST dumped to: {:?}", debug_path);
    // }
    
    // First pass: collect all defined symbols and function scopes
    use super::definitions;
    definitions::collect_definitions(root, text, index, &mut defined_symbols, &mut symbol_definitions, &mut function_scopes);
    
    // Debug: Log collected definitions
    #[cfg(debug_assertions)]
    {
        log::trace!("Collected {} defined symbols: {:?}", defined_symbols.len(), defined_symbols);
        log::trace!("Collected {} function scopes", function_scopes.len());
        for (node_id, params) in &function_scopes {
            log::trace!("  Function node {} has parameters: {:?}", node_id, params);
        }
    }
    
    // Second pass: check identifier references (with dependency awareness and scoping)
    check_identifier_references(
        root,
        text,
        &defined_symbols,
        &symbol_definitions,
        &function_scopes,
        index,
        import_context,
        diagnostics,
    );
}

/// Check identifier references against defined symbols (with dependency support and scoping)
pub(super) fn check_identifier_references(
    node: Node,
    text: &str,
    defined_symbols: &HashSet<String>,
    symbol_definitions: &HashMap<String, Range>,
    function_scopes: &HashMap<usize, HashSet<String>>,
    index: &Index,
    import_context: Option<&ImportContext>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Skip symbol literals (e.g., :bill_length_mm) - these are not variables
    // Also skip if this identifier is part of a quoted_identifier (symbol literal)
    if matches!(node.kind(), "quoted_identifier" | "symbol") {
        return;
    }
    // Check if this identifier is inside a quote_expression (symbol literal like :bill_length_mm)
    // In tree-sitter-julia, :symbol is parsed as quote_expression containing : and identifier
    let mut check_parent = node.parent();
    while let Some(p) = check_parent {
        if p.kind() == "quoted_identifier" || p.kind() == "symbol" || p.kind() == "quote_expression" {
            // This identifier is part of a symbol literal, skip it
            return;
        }
        check_parent = p.parent();
    }
    
    // Skip if this is a definition node (we already collected it)
    if matches!(node.kind(), "assignment" | "function_definition" | "using_statement" | "import_statement") {
        // Still check children, but skip the definition itself
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                check_identifier_references(
                    child,
                    text,
                    defined_symbols,
                    symbol_definitions,
                    function_scopes,
                    index,
                    import_context,
                    diagnostics,
                );
            }
        }
        return;
    }
    
    // Check for qualified symbol access (Module.symbol)
    if node.kind() == "field_access" || node.kind() == "field_expression" {
        check_qualified_symbol(
            node,
            text,
            defined_symbols,
            function_scopes,
            index,
            import_context,
            diagnostics,
        );
        // Don't recurse into field access children - the field name (right side of .)
        // should not be checked as an undefined variable. Only check the left side.
        // For example, in df.species, we should check df but not species.
        if let Some(left_child) = node.child(0) {
            // Only check the left side (the variable/object being accessed)
            check_identifier_references(
                left_child,
                text,
                defined_symbols,
                symbol_definitions,
                function_scopes,
                index,
                import_context,
                diagnostics,
            );
        }
        return;
    }
    
    // Check if this is an identifier reference
    if node.kind() == "identifier" {
        // Skip if this identifier is part of a parameter definition
        // Check if we're inside a parameter_list, argument_list (in function signatures), or typed_parameter
        let mut current = node.parent();
        while let Some(parent) = current {
            // Check for parameter_list first - any identifier inside a parameter_list is a parameter definition
            if parent.kind() == "parameter_list" {
                // This identifier is in a parameter list - it's a parameter definition, not a reference
                // Skip checking it as undefined
                return;
            }
            // Check for argument_list - in function signatures, parameters are in argument_list
            // Structure: function_definition -> signature -> call_expression -> argument_list
            if parent.kind() == "argument_list" {
                // Check if this argument_list is part of a function signature
                let mut check_parent = parent.parent();
                while let Some(grandparent) = check_parent {
                    if grandparent.kind() == "call_expression" {
                        // Check if this call_expression is inside a signature
                        let mut check_grandparent = grandparent.parent();
                        while let Some(ggparent) = check_grandparent {
                            if ggparent.kind() == "signature" {
                                // This argument_list is part of a function signature, so identifiers are parameters
                                return;
                            }
                            if ggparent.kind() == "function_definition" {
                                break; // Reached function_definition, not in signature
                            }
                            check_grandparent = ggparent.parent();
                        }
                    }
                    if grandparent.kind() == "function_definition" {
                        break; // Reached function_definition
                    }
                    check_parent = grandparent.parent();
                }
            }
            // Check for typed_parameter - the identifier in a typed parameter (like x::Int) is a parameter definition
            if parent.kind() == "typed_parameter" {
                // This identifier is part of a typed parameter definition - always skip it
                // The identifier in typed_parameter is always the parameter name
                return;
            }
            // Check for assignment in parameter list (default parameter values like k=5)
            // If we're the left-hand side of an assignment that's in a parameter_list, skip it
            if parent.kind() == "assignment" {
                // Check if this assignment is in a parameter_list
                let mut check_assignment_parent = parent.parent();
                while let Some(assignment_parent) = check_assignment_parent {
                    if assignment_parent.kind() == "parameter_list" {
                        // This assignment is in a parameter list, so the identifier is a parameter
                        // Check if we're the left-hand side (the parameter name)
                        if let Some(lhs) = parent.child(0) {
                            if lhs.id() == node.id() || (lhs.kind() == "typed_parameter" && lhs.child(0).map(|c| c.id()) == Some(node.id())) {
                                return; // This is a parameter definition, skip it
                            }
                        }
                    }
                    if assignment_parent.kind() == "function_definition" {
                        break;
                    }
                    check_assignment_parent = assignment_parent.parent();
                }
            }
            // Stop checking if we hit a function_definition (we've checked the parameter list)
            if parent.kind() == "function_definition" {
                break;
            }
            current = parent.parent();
        }
        
        // First, check if this identifier is actually a symbol literal by looking at the source text
        // In Julia, :symbol is a symbol literal. Check if there's a colon immediately before this identifier
        let node_start = node.start_byte();
        if node_start > 0 {
            // Check the character immediately before the identifier
            let prev_char = text.as_bytes().get(node_start.saturating_sub(1));
            if prev_char == Some(&b':') {
                // This is a symbol literal (preceded by colon), skip it
                return;
            }
        }
        
        // Skip if this identifier is part of a keyword argument (name = value)
        // Keyword argument names should not be checked as undefined variables
        // Walk up the tree to check if we're inside a keyword_argument node
        let mut current = Some(node);
        while let Some(n) = current {
            if n.kind() == "keyword_argument" || n.kind() == "named_argument" {
                // This identifier is part of a keyword argument - skip it
                return;
            }
            // Check if we're inside a pair node (key => value) - the key is a symbol literal
            if n.kind() == "pair" {
                // Check if this identifier is the left side of the pair (the key)
                if let Some(first_child) = n.child(0) {
                    if first_child.id() == node.id() {
                        // This is the key in a pair - it's a symbol literal, skip it
                        return;
                    }
                }
            }
            // Check if we're in a Dict constructor - identifiers in Dict keys are symbol literals
            if n.kind() == "call_expression" {
                // Check if this is a Dict constructor
                if let Some(func_node) = n.child(0) {
                    if let Ok(func_name) = func_node.utf8_text(text.as_bytes()) {
                        if func_name == "Dict" {
                            // We're in a Dict constructor - check if this identifier is a key
                            // In Dict(:key => value), the identifier is the key (symbol literal)
                            let mut check_parent = node.parent();
                            while let Some(p) = check_parent {
                                if p.id() == n.id() {
                                    break;
                                }
                                if p.kind() == "pair" {
                                    // This identifier is a key in a pair within Dict - skip it
                                    return;
                                }
                                check_parent = p.parent();
                            }
                        }
                    }
                }
            }
            current = n.parent();
        }
        
        if let Ok(name) = node.utf8_text(text.as_bytes()) {
            // Skip Julia built-ins and common keywords
            if utils::is_builtin_or_keyword(name) {
                return;
            }
            
            // Check if symbol is defined in current function scope (parameter)
            // This includes both regular function definitions and short function syntax (assignments)
            // Also check all enclosing scopes for closure variables
            let mut is_in_function_scope = false;
            let mut current = node.parent();
            while let Some(parent) = current {
                // Check for regular function definitions
                if parent.kind() == "function_definition" {
                    if let Some(params) = function_scopes.get(&parent.id()) {
                        if params.contains(name) {
                            is_in_function_scope = true;
                            break;
                        }
                    }
                }
                // Check for short function syntax (assignment with call_expression on left)
                if parent.kind() == "assignment" {
                    if let Some(params) = function_scopes.get(&parent.id()) {
                        if params.contains(name) {
                            is_in_function_scope = true;
                            break;
                        }
                    }
                }
                current = parent.parent();
            }
            
            // Check if symbol is defined locally or in index
            // Note: defined_symbols contains ALL variable definitions from the entire file,
            // including variables from outer scopes (closure variables), so this should
            // catch variables like n_samples used in nested functions
            let symbol_query = SymbolQuery::new(index);
            let is_defined_locally = is_in_function_scope
                || defined_symbols.contains(name)
                || symbol_query.find_symbol(name).is_some();
            
            if is_defined_locally {
                return; // Symbol is defined, no issue
            }
            
            // Check if symbol could come from imported modules
            let could_be_from_imports = if let Some(import_ctx) = import_context {
                import_ctx.could_symbol_come_from_imports(name, Some(index))
            } else {
                false
            };
            
            if could_be_from_imports {
                return; // Symbol likely comes from an imported module
            }
            
            // Check Index for Base module (standard library)
            let is_base_symbol = !index.find_signatures("Base", name).is_empty()
                || index.find_type("Base", name).is_some();
            
            if is_base_symbol {
                return; // Symbol is from Base module
            }
            
            // Check if symbol exists in any module in the index
            // This handles functions from packages like DataFrames, CSV, Statistics, etc.
            // Also handles Base submodules (e.g., Filesystem for joinpath)
            // Also check for exported functions that might be stored with different names (e.g., _describe -> describe)
            // Use imported modules from import context if available, otherwise check all modules
            let preferred_modules: Vec<&str> = if let Some(import_ctx) = import_context {
                import_ctx.imported_modules().iter().map(|s| s.as_str()).collect()
            } else {
                Vec::new()
            };
            
            // Check preferred modules first (from "using" statements)
            let mut exists_in_preferred = false;
            for module in &preferred_modules {
                // Check if symbol is exported by this module (most reliable check)
                if index.is_exported(module, name) {
                    // Symbol is exported - check if it exists (might be in submodule or re-exported)
                    if !index.find_signatures(module, name).is_empty() || index.find_type(module, name).is_some() {
                        exists_in_preferred = true;
                        break;
                    }
                    // Check submodules for exported functions
                    let all_modules = index.get_all_modules();
                    for submodule in &all_modules {
                        if submodule.starts_with(&format!("{}.", module))
                            && !index.find_signatures(submodule, name).is_empty() {
                            exists_in_preferred = true;
                            break;
                        }
                    }
                    if exists_in_preferred {
                        break;
                    }
                }
                // Check direct name match
                if !index.find_signatures(module, name).is_empty() || index.find_type(module, name).is_some() {
                    exists_in_preferred = true;
                    break;
                }
                // Check if it's an exported function stored with underscore prefix (e.g., _describe -> describe)
                let underscored_name = format!("_{}", name);
                if !index.find_signatures(module, &underscored_name).is_empty() {
                    exists_in_preferred = true;
                    break;
                }
            }
            
            if exists_in_preferred {
                return; // Symbol exists in an imported module
            }
            
            // Check all other modules - prioritize modules that export this symbol
            let exporting_modules = index.find_modules_exporting(name);
            let all_modules = index.get_all_modules();
            let exists_in_any_module = exporting_modules.iter().any(|module| {
                // Check if exported symbol exists
                !index.find_signatures(module, name).is_empty() || index.find_type(module, name).is_some()
            }) || all_modules.iter().any(|module| {
                // Skip checking Base and preferred modules again (already checked above)
                if module == "Base" || preferred_modules.contains(&module.as_str()) || exporting_modules.contains(module) {
                    return false;
                }
                // Check direct name match
                if !index.find_signatures(module, name).is_empty() || index.find_type(module, name).is_some() {
                    return true;
                }
                // Check if it's an exported function stored with underscore prefix (e.g., _describe -> describe)
                let underscored_name = format!("_{}", name);
                if !index.find_signatures(module, &underscored_name).is_empty() {
                    return true;
                }
                false
            });
            
            if exists_in_any_module {
                return; // Symbol exists in some module (Base submodule or package)
            }
            
            // Be more conservative: if modules are imported, assume symbols could come from them
            // even if they're not in the index (packages might not be fully indexed)
            // This is a fallback for when the symbol isn't found in the index but the module is imported
            if let Some(import_ctx) = import_context {
                // Check if any imported module could provide this symbol
                // If a module is imported with `using`, all its exports are available
                for module_name in import_ctx.imported_modules() {
                    let imported = import_ctx.get_imported_module(module_name).unwrap();
                    // Skip if it's qualified-only (import Module)
                    if imported.is_qualified_only {
                        continue;
                    }
                    // If specific symbols are listed, check if this symbol is in the list
                    if let Some(ref specific) = imported.specific_symbols {
                        if specific.contains(name) {
                            // Symbol is explicitly imported - assume it's valid even if not in index
                            log::trace!("SemanticAnalyzer: Symbol '{}' not found in index, but is explicitly imported from '{}' - assuming valid", name, module_name);
                            return;
                        }
                    } else {
                        // Module exports everything - if module is imported, assume symbol might exist
                        // This is conservative but avoids false positives for partially indexed packages
                        log::trace!("SemanticAnalyzer: Symbol '{}' not found in index, but module '{}' is imported (exports all) - assuming valid", name, module_name);
                        return;
                    }
                }
            }
            
            // Symbol is not defined and not found in imports, Base, or any other module
            // Log for debugging before reporting as undefined
            let all_modules_list = index.get_all_modules();
            let sample_modules: Vec<&String> = all_modules_list.iter().take(20).collect();
            log::trace!("SemanticAnalyzer: Symbol '{}' not found - checked locally: {}, imports: {}, Base: {}, exists_in_any_module: {}, total_modules: {}, sample_modules: {:?}",
                name, is_defined_locally, could_be_from_imports, is_base_symbol, exists_in_any_module, all_modules_list.len(), sample_modules);
            
            // Check if the symbol exists in any of the imported modules specifically
            if let Some(import_ctx) = import_context {
                let imported_modules = import_ctx.imported_modules();
                log::trace!("SemanticAnalyzer: Checking imported modules for '{}': {:?}", name, imported_modules);
                for module_name in imported_modules {
                    let funcs = index.get_module_functions(module_name);
                    let types = index.get_module_types(module_name);
                    let is_exp = index.is_exported(module_name, name);
                    log::trace!("SemanticAnalyzer: Module '{}' - functions: {}, types: {}, exports '{}': {}", 
                        module_name, funcs.len(), types.len(), name, is_exp);
                    if !funcs.is_empty() || !types.is_empty() {
                        log::trace!("SemanticAnalyzer: Module '{}' has content - functions: {:?} (first 10)", 
                            module_name, funcs.iter().take(10).collect::<Vec<_>>());
                    }
                }
            }
            
            // Report as undefined
            let range = Range {
                start: Position::from(node.start_position()),
                end: Position::from(node.end_position()),
            };
            
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::Error),
                code: Some("undefined_variable".to_string()),
                source: Some("semantic".to_string()),
                message: format!("Undefined variable: `{}`", name),
                related_information: None,
            });
        }
    }
    
    // Recursively check children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            check_identifier_references(
                child,
                text,
                defined_symbols,
                symbol_definitions,
                function_scopes,
                index,
                import_context,
                diagnostics,
            );
        }
    }
}

/// Check qualified symbol access (Module.symbol or variable.field)
pub(super) fn check_qualified_symbol(
    node: Node,
    text: &str,
    defined_symbols: &HashSet<String>,
    _function_scopes: &HashMap<usize, HashSet<String>>,
    index: &Index,
    import_context: Option<&ImportContext>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Extract module and symbol from qualified access
    let Some(node_text) = node.utf8_text(text.as_bytes()).ok() else {
        return;
    };
    
    let parts: Vec<&str> = node_text.split('.').collect();
    
    if parts.len() < 2 {
        return; // Not a valid qualified access
    }
    
    // Last part is the symbol, everything before is the module path
    let Some(symbol_name) = parts.last() else {
        return;
    };
    let symbol_name = symbol_name.to_string();
    let module_name = parts[..parts.len() - 1].join(".");
    
    // First check if the left-hand side is a defined variable
    // If it is, this is field access on a variable, not module access
    let symbol_query = SymbolQuery::new(index);
    let is_variable = defined_symbols.contains(&module_name)
        || symbol_query.find_symbol(&module_name).is_some();
    
    if is_variable {
        // This is field access on a variable (e.g., df.bill_length_mm)
        // Don't check for module imports - it's a valid field access
        return;
    }
    
    // If it's not a variable, treat it as module access (e.g., DataFrames.DataFrame, Download.download)
    // Check if symbol exists in the module first
    let symbol_exists = !index.find_signatures(&module_name, &symbol_name).is_empty()
        || index.find_type(&module_name, &symbol_name).is_some();
    
    // If symbol exists, it's valid (even if module not explicitly imported - might be stdlib or auto-imported)
    if symbol_exists {
        return;
    }
    
    // Check if module is imported
    let module_imported = import_context
        .map(|ctx| ctx.is_module_imported(&module_name))
        .unwrap_or(false);
    
    // Check if it's a stdlib module (Base, Core, Downloads, etc.)
    let is_stdlib = utils::is_stdlib_module(&module_name);
    
    // Check if module exists in index (has some functions/types)
    let module_has_content = !index.get_module_functions(&module_name).is_empty()
        || !index.get_module_types(&module_name).is_empty();
    
    // Only report error if:
    // 1. Module is imported (so we know it should be available) AND module has content in index
    // 2. Symbol doesn't exist in index
    // 3. It's not a stdlib module (stdlib modules might not be fully indexed)
    // Be conservative: only report errors for modules that are both imported AND indexed
    if module_imported && module_has_content && !symbol_exists && !is_stdlib {
        // Module is imported and indexed but symbol not found - this is likely an error
        let range = Range {
            start: Position::from(node.start_position()),
            end: Position::from(node.end_position()),
        };
        
        diagnostics.push(Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::Error),
            code: Some("undefined_symbol_in_module".to_string()),
            source: Some("semantic".to_string()),
            message: format!("Symbol `{}` not found in module `{}`", symbol_name, module_name),
            related_information: None,
        });
    }
    // If module is not imported or not indexed, be conservative and don't report errors
    // Packages might not be fully indexed, so we can't verify symbols
}

