use crate::pipeline::sources::Document;
use crate::pipeline::storage::Index;
use crate::pipeline::query::SymbolQuery;
use crate::types::{CompletionItem, CompletionItemKind, CompletionList, Position, Symbol, SymbolKind};

/// Stateless completion provider - takes index and document as parameters
pub struct CompletionProvider;

#[derive(Debug)]
enum CompletionContext {
    AfterDot { prefix: String },
    General { prefix: String },
}

impl CompletionProvider {
    pub fn complete(
        index: &Index,
        document: &Document,
        position: Position,
    ) -> Option<CompletionList> {
        let context = Self::extract_context(document, position)?;
        let symbol_query = SymbolQuery::new(index);
        let (symbols, prefix) = match &context {
            CompletionContext::General { prefix } => {
                let syms = if prefix.is_empty() {
                    index.get_all_symbols()
                } else {
                    symbol_query.find_by_prefix(prefix)
                };
                (syms, prefix.clone())
            }
            CompletionContext::AfterDot { prefix } => {
                (symbol_query.find_by_prefix(prefix), prefix.clone())
            }
        };
        let keyword_items = Self::julia_keyword_items_filtered(&prefix);
        let symbol_items = Self::symbols_to_completion_items(symbols);
        let has_matches = !keyword_items.is_empty() || !symbol_items.is_empty();
        let mut items = Vec::new();
        let mut seen = std::collections::HashSet::new();
        if has_matches {
            for kw in keyword_items.into_iter() {
                if seen.insert(kw.label.clone()) {
                    items.push(kw);
                }
            }
            for s in symbol_items.into_iter() {
                if seen.insert(s.label.clone()) {
                    items.push(s);
                }
            }
        } else {
            // fallback: top 20 general symbols plus all keywords
            let mut fallback = Self::julia_keyword_items();
            let mut general_symbols = index.get_all_symbols();
            general_symbols.truncate(20);
            let mut general_items = Self::symbols_to_completion_items(general_symbols);
            fallback.append(&mut general_items);
            for item in fallback.into_iter() {
                if seen.insert(item.label.clone()) {
                    items.push(item);
                }
            }
        }
        Some(CompletionList {
            is_incomplete: false,
            items,
        })
    }
    
    fn extract_context(document: &Document, position: Position) -> Option<CompletionContext> {
        let line_text = document.get_line(position.line as usize)?;
        let line_text_prev = if position.line > 0 { document.get_line((position.line-1) as usize) } else { None };
        let fname = document.uri();
        use log::debug;
        let line_text_prev_log = line_text_prev.as_deref().unwrap_or("<none>");
        debug!("extract_context: file='{}' line={} line_text='{}' line_text_prev='{}'", fname, position.line, line_text, line_text_prev_log);
        let text_before_cursor = &line_text[..position.character.min(line_text.len() as u32) as usize];
        debug!("extract_context: text_before_cursor='{}'", text_before_cursor);
        if let Some(dot_pos) = text_before_cursor.rfind('.') {
            let prefix = text_before_cursor[dot_pos + 1..].to_string();
            debug!("extract_context: AfterDot context, prefix='{}'", prefix);
            return Some(CompletionContext::AfterDot { prefix });
        }
        let prefix = extract_word_before_cursor(text_before_cursor);
        debug!("extract_context: General context, extract_word_before_cursor returned '{}'", prefix);
        Some(CompletionContext::General { prefix })
    }
    
    fn symbols_to_completion_items(symbols: Vec<Symbol>) -> Vec<CompletionItem> {
        symbols.into_iter().map(|s| CompletionItem {
            label: s.name.clone(),
            kind: symbol_kind_to_completion_kind(s.kind),
            detail: s.signature.clone(),
            documentation: s.doc_comment.clone(),
            insert_text: Some(s.name),
        }).collect()
    }

    fn julia_keyword_items() -> Vec<CompletionItem> {
        const KEYWORDS: &[&str] = &[
            "function", "struct", "mutable struct", "module", "using", "import",
            "for", "while", "if", "elseif", "else", "return", "let", "const",
            "macro", "where", "do", "try", "catch", "finally", "begin", "quote",
        ];

        KEYWORDS
            .iter()
            .map(|k| CompletionItem {
                label: (*k).to_string(),
                kind: CompletionItemKind::Constant,
                detail: None,
                documentation: None,
                insert_text: Some((*k).to_string()),
            })
            .collect()
    }

    fn julia_keyword_items_filtered(prefix: &str) -> Vec<CompletionItem> {
        use log::debug;
        if prefix.is_empty() {
            let items = Self::julia_keyword_items();
            debug!("Keyword filter: prefix empty, returning all {} keywords", items.len());
            return items;
        }
        let lower = prefix.to_lowercase();
        let pre_len = lower.len();
        let items: Vec<_> = Self::julia_keyword_items()
            .into_iter()
            .filter(|item| item.label.to_lowercase().get(0..pre_len) == Some(&*lower))
            .collect();
        debug!("Keyword filter: for prefix '{:?}', returning {} candidates: {:?}",
            prefix, items.len(), items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
        items
    }
}

fn symbol_kind_to_completion_kind(kind: SymbolKind) -> CompletionItemKind {
    match kind {
        SymbolKind::Function => CompletionItemKind::Function,
        SymbolKind::Variable => CompletionItemKind::Variable,
        SymbolKind::Module => CompletionItemKind::Module,
        SymbolKind::Type => CompletionItemKind::Type,
        SymbolKind::Constant => CompletionItemKind::Constant,
        SymbolKind::Macro => CompletionItemKind::Macro,
    }
}

fn extract_word_before_cursor(text: &str) -> String {
    use log::debug;
    debug!("extract_word_before_cursor: input='{}'", text);
    let res = text.chars().rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars().rev().collect();
    debug!("extract_word_before_cursor: result='{}'", res);
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::parser::JuliaParser;

    #[test]
    fn test_completion_context_extraction() {
        let parser = JuliaParser::new();
        let code = "my_function(1, 2)";
        let _tree = parser.parse(code).unwrap();
        let doc = Document::new("test.jl".to_string(), code.to_string());
        
        // Test general context
        let position = Position { line: 0, character: 2 };
        let context = CompletionProvider::extract_context(&doc, position);
        assert!(matches!(context, Some(CompletionContext::General { prefix }) if prefix == "my"));
    }

    #[test]
    fn test_completion_items_conversion() {
        let symbol = Symbol {
            name: "test_function".to_string(),
            kind: SymbolKind::Function,
            range: crate::types::Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 13 },
            },
            scope_id: 0,
            doc_comment: Some("Test function".to_string()),
            signature: Some("test_function(x, y)".to_string()),
            file_uri: "test.jl".to_string(),
        };

        let items = CompletionProvider::symbols_to_completion_items(vec![symbol]);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "test_function");
        assert_eq!(items[0].kind, CompletionItemKind::Function);
        assert_eq!(items[0].detail, Some("test_function(x, y)".to_string()));
        assert_eq!(items[0].documentation, Some("Test function".to_string()));
    }
}
