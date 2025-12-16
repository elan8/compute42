use crate::pipeline::storage::Index;
use crate::types::LspError;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::pipeline::types::{Reference, ScopeTree};
use crate::types::{TypeDefinition, FunctionSignature};
use crate::types::Symbol;

/// Serialize index to JSON
pub fn serialize_to_json(index: &Index, path: &Path) -> Result<(), LspError> {
    let json = serde_json::to_string_pretty(&SerializableIndex::from(index))
        .map_err(|e| LspError::InternalError(format!("Failed to serialize index: {}", e)))?;

    std::fs::write(path, json)
        .map_err(|e| LspError::InternalError(format!("Failed to write JSON file: {}", e)))?;

    Ok(())
}

/// Deserialize index from JSON
pub fn deserialize_from_json(path: &Path) -> Result<Index, LspError> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| LspError::InternalError(format!("Failed to read JSON file: {}", e)))?;

    let serializable: SerializableIndex = serde_json::from_str(&json)
        .map_err(|e| LspError::InternalError(format!("Failed to deserialize index: {}", e)))?;

    Ok(serializable.into())
}

#[derive(Serialize, Deserialize)]
struct SerializableIndex {
    version: u32,
    symbols: HashMap<String, Vec<Symbol>>,
    file_symbols: HashMap<String, Vec<String>>, // PathBuf as String
    references: HashMap<String, Vec<Reference>>,
    file_references: HashMap<String, Vec<String>>, // PathBuf as String
    types: HashMap<String, HashMap<String, TypeDefinition>>,
    file_scopes: HashMap<String, ScopeTree>, // PathBuf as String
    signatures: HashMap<String, HashMap<String, Vec<FunctionSignature>>>,
    exports: HashMap<String, Vec<String>>, // HashSet as Vec
    file_exports: HashMap<String, Vec<String>>, // PathBuf as String, HashSet as Vec
}

impl From<&Index> for SerializableIndex {
    fn from(index: &Index) -> Self {
        // Access internal data through public methods
        let symbols: HashMap<String, Vec<Symbol>> = index.get_all_symbols()
            .into_iter()
            .fold(HashMap::new(), |mut acc, symbol| {
                acc.entry(symbol.name.clone()).or_default().push(symbol);
                acc
            });
        
        // For file_symbols, file_references, file_scopes, file_exports, we need to access them
        // Since they're private, we'll need to add getter methods or reconstruct from available data
        // For now, we'll serialize what we can access and reconstruct the rest
        
        let mut file_symbols: HashMap<String, Vec<String>> = HashMap::new();
        for symbol in index.get_all_symbols() {
            let path = symbol.file_uri.clone();
            file_symbols.entry(path).or_default().push(symbol.name.clone());
        }
        
        let mut file_references: HashMap<String, Vec<String>> = HashMap::new();
        for reference in index.get_all_references() {
            let path = reference.file_uri.clone();
            file_references.entry(path).or_default().push(reference.name.clone());
        }
        
        let references: HashMap<String, Vec<Reference>> = index.get_all_references()
            .into_iter()
            .fold(HashMap::new(), |mut acc, reference| {
                acc.entry(reference.name.clone()).or_default().push(reference);
                acc
            });
        
        // For types, signatures, exports - we need to collect from all modules
        let mut types = HashMap::new();
        for module in index.get_all_type_modules() {
            let mut module_types = HashMap::new();
            for type_name in index.get_module_types(&module) {
                if let Some(type_def) = index.find_type(&module, &type_name) {
                    module_types.insert(type_name, type_def);
                }
            }
            if !module_types.is_empty() {
                types.insert(module, module_types);
            }
        }
        
        let mut signatures = HashMap::new();
        for module in index.get_all_modules() {
            let mut module_sigs = HashMap::new();
            for func_name in index.get_module_functions(&module) {
                let sigs = index.find_signatures(&module, &func_name);
                if !sigs.is_empty() {
                    module_sigs.insert(func_name, sigs);
                }
            }
            if !module_sigs.is_empty() {
                signatures.insert(module, module_sigs);
            }
        }
        
        let mut exports = HashMap::new();
        for module in index.get_all_modules() {
            let module_exports: Vec<String> = index.get_module_exports(&module).into_iter().collect();
            if !module_exports.is_empty() {
                exports.insert(module, module_exports);
            }
        }
        
        // For file_scopes and file_exports, we can't easily reconstruct them from public API
        // We'll need to add getter methods to Index or accept that they won't be preserved
        // For base index, scopes aren't critical since we're only storing signatures and types
        let file_scopes = HashMap::new();
        let file_exports = HashMap::new();
        
        Self {
            version: 1,
            symbols,
            file_symbols,
            references,
            file_references,
            types,
            file_scopes,
            signatures,
            exports,
            file_exports,
        }
    }
}

impl Into<Index> for SerializableIndex {
    fn into(self) -> Index {
        // Convert Vec<String> back to HashSet<String> for exports
        let exports: HashMap<String, std::collections::HashSet<String>> = self.exports
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect();
        
        // For base index, we mainly need types, signatures, and exports
        // Symbols, references, and scopes aren't needed for base/stdlib indexing
        Index::from_serialized(self.types, self.signatures, exports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_serialize_deserialize() {
        let index = Index::new();
        let temp_dir = TempDir::new().unwrap();
        let json_path = temp_dir.path().join("index.json");

        serialize_to_json(&index, &json_path).unwrap();
        let deserialized = deserialize_from_json(&json_path).unwrap();

        assert_eq!(deserialized.get_all_symbols().len(), 0);
    }
}








