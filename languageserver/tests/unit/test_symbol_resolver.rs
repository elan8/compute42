#[cfg(test)]
mod tests {
    use languageserver::{SymbolKind, Position};
    use languageserver::pipeline::query::SymbolResolver;
    
    #[test]
    fn test_find_symbol_at_function_name() {
        let code = include_str!("../fixtures/simple_function.jl");
        let tree = languageserver::pipeline::parser::JuliaParser::new().parse(code).unwrap();
        let resolver = SymbolResolver::new(&tree, code);
        
        // Position at 'my_function' in the function definition (line 0, col 9)
        let position = Position { line: 0, character: 9 };
        let symbol = resolver.symbol_at_position(position).unwrap();
        
        assert_eq!(symbol.name, "my_function");
        assert_eq!(symbol.kind, SymbolKind::Function);
    }
    
    #[test]
    fn test_find_symbol_at_variable() {
        let code = include_str!("../fixtures/simple_function.jl");
        let tree = languageserver::pipeline::parser::JuliaParser::new().parse(code).unwrap();
        let resolver = SymbolResolver::new(&tree, code);
        
        // Position at 'my_variable'
        let position = Position { line: 4, character: 0 };
        let symbol = resolver.symbol_at_position(position).unwrap();
        
        assert_eq!(symbol.name, "my_variable");
        assert_eq!(symbol.kind, SymbolKind::Variable);
    }
}
