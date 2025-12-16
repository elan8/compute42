// Type conversion utilities between languageserver crate types and internals types

use crate::types::{
    LspCompletionItem, LspDiagnostic, LspHover, LspLocation, LspMarkedString, LspPosition, LspRange,
};
use languageserver::types::{
    CompletionItem, CompletionItemKind, Diagnostic, DiagnosticSeverity, HoverResult, Location, Position, Range,
};

/// Convert languageserver Position to internals LspPosition
pub fn position_to_lsp(position: Position) -> LspPosition {
    LspPosition {
        line: position.line,
        character: position.character,
    }
}

/// Convert internals LspPosition to languageserver Position
pub fn lsp_position_to_position(lsp_pos: LspPosition) -> Position {
    Position {
        line: lsp_pos.line,
        character: lsp_pos.character,
    }
}

/// Convert languageserver Range to internals LspRange
pub fn range_to_lsp(range: Range) -> LspRange {
    LspRange {
        start: position_to_lsp(range.start),
        end: position_to_lsp(range.end),
    }
}

/// Convert internals LspRange to languageserver Range
#[allow(dead_code)]
pub fn lsp_range_to_range(lsp_range: LspRange) -> Range {
    Range {
        start: lsp_position_to_position(lsp_range.start),
        end: lsp_position_to_position(lsp_range.end),
    }
}

/// Convert languageserver CompletionItemKind to internals u32
pub fn completion_item_kind_to_lsp(kind: CompletionItemKind) -> u32 {
    match kind {
        CompletionItemKind::Function => 3,
        CompletionItemKind::Variable => 6,
        CompletionItemKind::Module => 9,
        CompletionItemKind::Type => 22,
        CompletionItemKind::Constant => 21,
        CompletionItemKind::Macro => 15,
    }
}

/// Convert languageserver CompletionItem to internals LspCompletionItem
pub fn completion_item_to_lsp(item: CompletionItem) -> LspCompletionItem {
    LspCompletionItem {
        label: item.label,
        kind: Some(completion_item_kind_to_lsp(item.kind)),
        detail: item.detail,
        documentation: item.documentation,
        insert_text: item.insert_text,
        insert_text_format: None, // Not supported in languageserver crate yet
        text_edit: None,          // Not supported in languageserver crate yet
        additional_text_edits: None,
        command: None,
        data: None,
        sort_text: None,
        filter_text: None,
    }
}

/// Convert languageserver HoverResult to internals LspHover
#[allow(dead_code)]
pub fn hover_result_to_lsp(hover: HoverResult) -> LspHover {
    LspHover {
        contents: vec![LspMarkedString {
            language: Some("julia".to_string()),
            value: hover.contents,
        }],
        range: hover.range.map(range_to_lsp),
    }
}

/// Convert languageserver Location to internals LspLocation
pub fn location_to_lsp(location: Location) -> LspLocation {
    LspLocation {
        uri: location.uri,
        range: range_to_lsp(location.range),
    }
}

/// Convert internals LspLocation to languageserver Location
#[allow(dead_code)]
pub fn lsp_location_to_location(lsp_location: LspLocation) -> Location {
    Location {
        uri: lsp_location.uri,
        range: lsp_range_to_range(lsp_location.range),
    }
}

/// Convert languageserver Diagnostic to internals LspDiagnostic
pub fn diagnostic_to_lsp(diagnostic: Diagnostic) -> LspDiagnostic {
    LspDiagnostic {
        range: range_to_lsp(diagnostic.range),
        severity: diagnostic.severity.map(|s| match s {
            DiagnosticSeverity::Error => 1,
            DiagnosticSeverity::Warning => 2,
            DiagnosticSeverity::Information => 3,
            DiagnosticSeverity::Hint => 4,
        }),
        code: diagnostic.code,
        source: diagnostic.source,
        message: diagnostic.message,
        tags: None,
        related_information: None,
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_conversion() {
        let pos = Position { line: 5, character: 10 };
        let lsp_pos = position_to_lsp(pos);
        assert_eq!(lsp_pos.line, 5);
        assert_eq!(lsp_pos.character, 10);

        let back_to_pos = lsp_position_to_position(lsp_pos);
        assert_eq!(back_to_pos.line, 5);
        assert_eq!(back_to_pos.character, 10);
    }

    #[test]
    fn test_range_conversion() {
        let range = Range {
            start: Position { line: 1, character: 2 },
            end: Position { line: 3, character: 4 },
        };
        let lsp_range = range_to_lsp(range);
        assert_eq!(lsp_range.start.line, 1);
        assert_eq!(lsp_range.start.character, 2);
        assert_eq!(lsp_range.end.line, 3);
        assert_eq!(lsp_range.end.character, 4);

        let back_to_range = lsp_range_to_range(lsp_range);
        assert_eq!(back_to_range.start.line, 1);
        assert_eq!(back_to_range.start.character, 2);
        assert_eq!(back_to_range.end.line, 3);
        assert_eq!(back_to_range.end.character, 4);
    }

    #[test]
    fn test_completion_item_conversion() {
        let item = CompletionItem {
            label: "test_function".to_string(),
            kind: CompletionItemKind::Function,
            detail: Some("Test function".to_string()),
            documentation: Some("Documentation".to_string()),
            insert_text: Some("test_function()".to_string()),
        };
        let lsp_item = completion_item_to_lsp(item);
        assert_eq!(lsp_item.label, "test_function");
        assert_eq!(lsp_item.kind, Some(3)); // Function kind
        assert_eq!(lsp_item.detail, Some("Test function".to_string()));
        assert_eq!(lsp_item.documentation, Some("Documentation".to_string()));
        assert_eq!(lsp_item.insert_text, Some("test_function()".to_string()));
    }

    #[test]
    fn test_hover_conversion() {
        let hover = HoverResult {
            contents: "Hover content".to_string(),
            range: Some(Range {
                start: Position { line: 1, character: 0 },
                end: Position { line: 1, character: 5 },
            }),
        };
        let lsp_hover = hover_result_to_lsp(hover);
        assert_eq!(lsp_hover.contents.len(), 1);
        assert_eq!(lsp_hover.contents[0].value, "Hover content");
        assert!(lsp_hover.range.is_some());
        let range = lsp_hover.range.unwrap();
        assert_eq!(range.start.line, 1);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 1);
        assert_eq!(range.end.character, 5);
    }

    #[test]
    fn test_location_conversion() {
        let location = Location {
            uri: "file:///test.jl".to_string(),
            range: Range {
                start: Position { line: 10, character: 5 },
                end: Position { line: 10, character: 15 },
            },
        };
        let lsp_location = location_to_lsp(location);
        assert_eq!(lsp_location.uri, "file:///test.jl");
        assert_eq!(lsp_location.range.start.line, 10);
        assert_eq!(lsp_location.range.start.character, 5);
        assert_eq!(lsp_location.range.end.line, 10);
        assert_eq!(lsp_location.range.end.character, 15);

        let back_to_location = lsp_location_to_location(lsp_location);
        assert_eq!(back_to_location.uri, "file:///test.jl");
        assert_eq!(back_to_location.range.start.line, 10);
        assert_eq!(back_to_location.range.start.character, 5);
        assert_eq!(back_to_location.range.end.line, 10);
        assert_eq!(back_to_location.range.end.character, 15);
    }
}

