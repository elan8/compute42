use crate::pipeline::storage::Index;
use crate::pipeline::types::Reference;

/// Query references from the index
pub struct ReferenceQuery<'a> {
    index: &'a Index,
}

impl<'a> ReferenceQuery<'a> {
    pub fn new(index: &'a Index) -> Self {
        Self { index }
    }

    /// Find all references to a symbol by name
    pub fn find_references(&self, symbol_name: &str) -> Vec<Reference> {
        self.index.find_references(symbol_name)
    }

    /// Find all references in a file
    pub fn find_in_file(&self, file_path: &std::path::Path) -> Vec<Reference> {
        self.index
            .get_all_references()
            .into_iter()
            .filter(|reference| reference.file_uri == file_path.to_string_lossy())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::types::{AnalysisResult, Reference, ReferenceKind};
    use std::path::PathBuf;

    #[test]
    fn test_find_references() {
        let mut index = Index::new();
        let mut analysis = AnalysisResult::new();
        
        let reference = Reference {
            name: "test".to_string(),
            range: crate::types::Range {
                start: crate::types::Position { line: 0, character: 0 },
                end: crate::types::Position { line: 0, character: 10 },
            },
            file_uri: "test.jl".to_string(),
            kind: ReferenceKind::Variable,
        };
        analysis.references.push(reference);

        let file_path = PathBuf::from("test.jl");
        index.merge_file(&file_path, analysis).unwrap();

        let query = ReferenceQuery::new(&index);
        let results = query.find_references("test");
        assert_eq!(results.len(), 1);
    }
}

















