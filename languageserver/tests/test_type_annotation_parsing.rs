// Test to understand tree-sitter AST structure for type annotations

#[cfg(test)]
mod tests {
    use languageserver::pipeline::parser::JuliaParser;
    use tree_sitter::Node;

    #[test]
    fn test_parse_type_annotation_ast() {
        // Test cases to understand AST structure
        let test_cases = vec![
            "x::Int64",
            "function f(x::Int64) end",
            "function f()::DataFrame end",
            "y::Vector{Int64}",
            "z::Union{Int64, Missing}",
        ];

        let parser = JuliaParser::new();
        let mut tree_parser = parser.create_parser().unwrap();

        for code in test_cases {
            println!("\n=== Parsing: {} ===", code);
            if let Some(tree) = tree_parser.parse(code, None) {
                print_ast_tree(tree.root_node(), code, 0);
            }
        }
    }

    fn print_ast_tree(node: Node, source: &str, depth: usize) {
        let indent = "  ".repeat(depth);
        let kind = node.kind();
        let text = node.utf8_text(source.as_bytes()).unwrap_or("");
        println!("{}{} [{}:{}] '{}'", 
            indent, 
            kind, 
            node.start_byte(), 
            node.end_byte(),
            if text.len() > 50 { &text[..50] } else { text }
        );

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                print_ast_tree(child, source, depth + 1);
            }
        }
    }
}






