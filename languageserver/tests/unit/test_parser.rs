#[cfg(test)]
mod tests {
    use languageserver::pipeline::parser::JuliaParser;
    
    #[test]
    fn test_parse_simple_function() {
        let code = include_str!("../fixtures/simple_function.jl");
        let parser = JuliaParser::new();
        let result = parser.parse(code);
        
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(tree.root_node().child_count() > 0);
    }
    
    #[test]
    fn test_parse_malformed_code() {
        let code = include_str!("../fixtures/malformed.jl");
        let parser = JuliaParser::new();
        let result = parser.parse(code);
        
        // Should parse but with error nodes
        assert!(result.is_ok());
        assert!(result.unwrap().root_node().has_error());
    }
}
