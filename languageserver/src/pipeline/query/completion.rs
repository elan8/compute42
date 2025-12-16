use crate::pipeline::storage::Index;
use crate::types::{CompletionItem, CompletionItemKind, SymbolKind};

/// Query for completion suggestions
pub struct CompletionQuery<'a> {
    index: &'a Index,
}

impl<'a> CompletionQuery<'a> {
    pub fn new(index: &'a Index) -> Self {
        Self { index }
    }

    /// Get completion items for a prefix
    pub fn complete(&self, prefix: &str) -> Vec<CompletionItem> {
        let symbols = self.index.find_symbols_with_prefix(prefix);
        
        symbols
            .into_iter()
            .map(|symbol| CompletionItem {
                label: symbol.name.clone(),
                kind: symbol_kind_to_completion_kind(symbol.kind),
                detail: symbol.signature.clone(),
                documentation: symbol.doc_comment.clone(),
                insert_text: Some(symbol.name),
            })
            .collect()
    }

    /// Get completion items for a file context
    pub fn complete_in_file(&self, file_path: &std::path::Path, prefix: &str) -> Vec<CompletionItem> {
        let symbols = self.index
            .get_all_symbols()
            .into_iter()
            .filter(|symbol| {
                symbol.file_uri == file_path.to_string_lossy()
                    && symbol.name.starts_with(prefix)
            })
            .collect::<Vec<_>>();

        symbols
            .into_iter()
            .map(|symbol| CompletionItem {
                label: symbol.name.clone(),
                kind: symbol_kind_to_completion_kind(symbol.kind),
                detail: symbol.signature.clone(),
                documentation: symbol.doc_comment.clone(),
                insert_text: Some(symbol.name),
            })
            .collect()
    }
}

fn symbol_kind_to_completion_kind(kind: SymbolKind) -> CompletionItemKind {
    match kind {
        SymbolKind::Function => CompletionItemKind::Function,
        SymbolKind::Variable => CompletionItemKind::Variable,
        SymbolKind::Type => CompletionItemKind::Type,
        SymbolKind::Module => CompletionItemKind::Module,
        SymbolKind::Constant => CompletionItemKind::Constant,
        SymbolKind::Macro => CompletionItemKind::Macro,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::types::AnalysisResult;
    use crate::types::Symbol;
    use std::path::PathBuf;

    #[test]
    fn test_complete() {
        let mut index = Index::new();
        let mut analysis = AnalysisResult::new();
        
        let symbol = Symbol {
            name: "test_function".to_string(),
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

        let query = CompletionQuery::new(&index);
        let completions = query.complete("test_");
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].label, "test_function");
    }
}


