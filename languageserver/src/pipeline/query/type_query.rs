use crate::pipeline::storage::Index;
use crate::types::TypeDefinition;

/// Query type definitions from the index
pub struct TypeQuery<'a> {
    index: &'a Index,
}

impl<'a> TypeQuery<'a> {
    pub fn new(index: &'a Index) -> Self {
        Self { index }
    }

    /// Find type definition by module and name
    pub fn find_type(&self, module: &str, name: &str) -> Option<TypeDefinition> {
        self.index.find_type(module, name)
    }

    /// Find all types in a module
    pub fn find_in_module(&self, module: &str) -> Vec<TypeDefinition> {
        self.index
            .find_type(module, "")
            .into_iter()
            .collect() // Simplified - would need to iterate all types in module
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::types::AnalysisResult;
    use crate::types::{TypeDefinition, TypeDefinitionKind};
    use std::path::PathBuf;

    #[test]
    fn test_find_type() {
        let mut index = Index::new();
        let mut analysis = AnalysisResult::new();
        
        let type_def = TypeDefinition {
            module: "Test".to_string(),
            name: "MyType".to_string(),
            kind: TypeDefinitionKind::Struct,
            doc_comment: None,
            file_uri: "test.jl".to_string(),
            range: crate::types::Range {
                start: crate::types::Position { line: 0, character: 0 },
                end: crate::types::Position { line: 0, character: 10 },
            },
        };
        analysis.types.push(type_def);

        let file_path = PathBuf::from("test.jl");
        index.merge_file(&file_path, analysis).unwrap();

        let query = TypeQuery::new(&index);
        let result = query.find_type("Test", "MyType");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "MyType");
    }
}

