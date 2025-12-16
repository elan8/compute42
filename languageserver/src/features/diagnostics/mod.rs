mod extractor;
mod error_analysis;
mod syntax_analyzer;
mod semantic;
mod message_enhancer;
pub mod incremental;
#[cfg(debug_assertions)]
mod cst_debug;

use crate::types::ImportContext;
use crate::pipeline::sources::Document;
use crate::pipeline::storage::Index;
use crate::types::Diagnostic;
use extractor::extract_diagnostics_from_tree;
use semantic::SemanticAnalyzer;
use message_enhancer::MessageEnhancer;

/// Diagnostics provider for Julia code analysis
pub struct DiagnosticsProvider;

impl DiagnosticsProvider {
    /// Compute diagnostics for a document (basic version without index)
    pub fn compute_diagnostics(document: &Document) -> Vec<Diagnostic> {
        Self::compute_diagnostics_with_context(document, None, None, None)
    }
    
    /// Compute diagnostics for a document with index
    pub fn compute_diagnostics_with_context(
        document: &Document,
        index: Option<&Index>,
        depot_path: Option<&std::path::Path>,
        manifest: Option<&crate::pipeline::sources::project_context::ManifestToml>,
    ) -> Vec<Diagnostic> {
        let Some(tree) = document.tree() else {
            log::warn!("DiagnosticsProvider: No parse tree available for document");
            return Vec::new();
        };
        
        let text = document.text();
        let mut diagnostics = Vec::new();
        
        // In debug mode, dump the CST to a file for offline analysis
        #[cfg(debug_assertions)]
        {
            use cst_debug::dump_cst_to_file;
            // Create a sanitized filename from the document URI
            // Handle both file:/// and file:// URIs, and Windows paths
            let uri = document.uri();
            let file_name = if uri.starts_with("file://") {
                // Extract path from file:// URI
                let path_part = uri.strip_prefix("file://").unwrap_or(uri);
                // Remove leading slash if present (file:///C:/path vs file://C:/path)
                let path_part = path_part.strip_prefix('/').unwrap_or(path_part);
                // URL decode if needed, then get filename
                std::path::Path::new(path_part)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
            } else {
                uri.split('/').next_back().unwrap_or("unknown")
            };
            // Sanitize filename for filesystem
            let sanitized = file_name
                .replace(":", "_")
                .replace("\\", "_")
                .replace("/", "_")
                .replace("<", "_")
                .replace(">", "_")
                .replace("|", "_")
                .replace("\"", "_")
                .replace("?", "_")
                .replace("*", "_");
            let cst_file_name = format!("{}.cst.txt", sanitized);
            dump_cst_to_file(tree, &text, &cst_file_name);
        }
        
        // Syntax diagnostics from tree-sitter CST
        // Rely entirely on tree-sitter's parse tree for syntax errors
        extract_diagnostics_from_tree(tree, &text, &mut diagnostics);
        
        // Semantic diagnostics (if index is available)
        if let Some(index) = index {
            // Create import context from the document's import statements
            let import_context = document.tree().map(|tree| ImportContext::from_tree_with_index(tree, &text, index));
            
            // Use the enhanced analyzer with import context
            let semantic_diagnostics = SemanticAnalyzer::analyze_with_imports(
                document,
                index,
                import_context.as_ref(),
                depot_path,
                manifest,
            );
            diagnostics.extend(semantic_diagnostics);
        }
        
        // Enhance messages
        for diagnostic in &mut diagnostics {
            MessageEnhancer::enhance(diagnostic, &text, index);
            MessageEnhancer::refine_severity(diagnostic, &text);
        }
        
        log::trace!(
            "DiagnosticsProvider: Found {} diagnostics for {}",
            diagnostics.len(),
            document.uri()
        );
        
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::parser::JuliaParser;
    use crate::types::DiagnosticSeverity;
    
    #[test]
    fn test_valid_code_no_diagnostics() {
        let parser = JuliaParser::new();
        let code = "function test() return 42 end";
        
        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();
        
        let diagnostics = DiagnosticsProvider::compute_diagnostics(&doc);
        assert_eq!(diagnostics.len(), 0);
    }
    
    #[test]
    fn test_syntax_error_missing_end() {
        let parser = JuliaParser::new();
        let code = "function test() return 42";
        
        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();
        
        let diagnostics = DiagnosticsProvider::compute_diagnostics(&doc);
        assert!(diagnostics.len() > 0);
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::Error));
    }
    
    #[test]
    fn test_syntax_error_invalid_syntax() {
        let parser = JuliaParser::new();
        let code = "function test(";
        
        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();
        
        let diagnostics = DiagnosticsProvider::compute_diagnostics(&doc);
        assert!(diagnostics.len() > 0);
    }
    
    #[test]
    fn test_multiple_errors() {
        let parser = JuliaParser::new();
        let code = "function test(\nif x";
        
        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();
        
        let diagnostics = DiagnosticsProvider::compute_diagnostics(&doc);
        // Should have multiple errors
        assert!(diagnostics.len() > 0);
    }
    
    #[test]
    fn test_diagnostic_contains_helpful_message() {
        let parser = JuliaParser::new();
        let code = "function test() return 42";
        
        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();
        
        let diagnostics = DiagnosticsProvider::compute_diagnostics(&doc);
        assert!(diagnostics.len() > 0);
        
        // Message should be helpful
        let message = &diagnostics[0].message;
        assert!(!message.is_empty());
        assert!(message.len() > 10); // Should have some substance
    }

    #[test]
    fn test_missing_end_reports_specific_message_and_small_range() {
        let parser = JuliaParser::new();
        let code = "function test(x)\n  x + 1\n"; // missing end

        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();

        // Debug: print tree structure
        if let Some(tree) = doc.tree() {
            eprintln!("\n=== CST Tree Structure for missing end ===");
            print_tree_debug(&tree.root_node(), &code, 0);
        }

        let diagnostics = DiagnosticsProvider::compute_diagnostics(&doc);
        
        eprintln!("\n=== Diagnostics for missing end ===");
        for (i, d) in diagnostics.iter().enumerate() {
            eprintln!("  Diagnostic[{}]: code={:?}, message='{}', range={:?}", 
                i, d.code, d.message, d.range);
        }
        
        assert!(diagnostics.iter().any(|d| d.message.contains("Missing 'end'")));
        // Ensure the first diagnostic does not mark the whole file
        let d = &diagnostics[0];
        assert!(d.range.end.line - d.range.start.line <= 1);
    }

    #[test]
    fn test_unexpected_end_reports_helpful_message() {
        let parser = JuliaParser::new();
        let code = "end"; // unexpected end at top

        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();

        // Debug: print tree structure
        if let Some(tree) = doc.tree() {
            eprintln!("\n=== CST Tree Structure for 'end' ===");
            print_tree_debug(&tree.root_node(), &code, 0);
        }

        let diagnostics = DiagnosticsProvider::compute_diagnostics(&doc);
        
        eprintln!("\n=== Diagnostics for 'end' ===");
        for (i, d) in diagnostics.iter().enumerate() {
            eprintln!("  Diagnostic[{}]: code={:?}, message='{}'", i, d.code, d.message);
        }
        
        assert!(diagnostics.iter().any(|d| d.message.contains("Unexpected 'end'")));
    }
    
    fn print_tree_debug(node: &tree_sitter::Node, text: &str, depth: usize) {
        let indent = "  ".repeat(depth);
        let node_text = node.utf8_text(text.as_bytes()).unwrap_or_default();
        let node_text_preview = if node_text.len() > 30 {
            format!("{}...", &node_text[..30])
        } else {
            node_text.to_string()
        };
        eprintln!("{}{} [{}:{}] '{}' (error: {}, missing: {})", 
            indent, 
            node.kind(), 
            node.start_position().row, 
            node.start_position().column,
            node_text_preview,
            node.is_error(),
            node.is_missing()
        );
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                print_tree_debug(&child, text, depth + 1);
            }
        }
    }

    #[test]
    fn test_unmatched_parenthesis() {
        let parser = JuliaParser::new();
        let code = "f(x = (1 + 2"; // missing closing )

        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();

        let diagnostics = DiagnosticsProvider::compute_diagnostics(&doc);
        assert!(diagnostics.iter().any(|d| d.message.contains("parenthesis")));
    }

    #[test]
    fn test_unterminated_string() {
        let parser = JuliaParser::new();
        let code = "s = \"hello"; // missing closing quote

        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();

        let diagnostics = DiagnosticsProvider::compute_diagnostics(&doc);
        // generic message mentions delimiters
        assert!(diagnostics.iter().any(|d| d.message.to_lowercase().contains("string") || d.message.to_lowercase().contains("delimiter")));
    }

    #[test]
    fn test_invalid_assignment_hint() {
        let parser = JuliaParser::new();
        let code = "1 = x"; // invalid assignment target

        let mut doc = Document::new("test.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();

        let diagnostics = DiagnosticsProvider::compute_diagnostics(&doc);
        // should at least flag a syntax error
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_missing_end_does_not_flag_valid_trailing_block() {
        let parser = JuliaParser::new();
        // function without end near the top, later a valid for-block that is closed
        let code = r#"
function test(x)
    x + 1

using Printf

for i in 1:5
    println(i)
end
        "#;

        let mut doc = Document::new("demo.jl".to_string(), code.to_string());
        let mut parser_instance = parser.create_parser().unwrap();
        doc.parse(&mut parser_instance).unwrap();

        let diagnostics = DiagnosticsProvider::compute_diagnostics(&doc);

        // Should include a missing_end near the function header (line 1 in this snippet)
        assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("missing_end") && d.range.start.line <= 2));

        // Should NOT include an error at the valid 'for' header line (line ~6 here)
        let for_line = 6u32; // zero-based: line with 'for i in 1:5' in this snippet
        assert!(diagnostics.iter().all(|d| d.range.start.line != for_line));
    }
}

