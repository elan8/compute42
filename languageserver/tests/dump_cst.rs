/// Test to dump CST structure for debugging macro parsing
use languageserver::pipeline::parser;
use languageserver::pipeline::sources::file::FileSource;
use std::path::PathBuf;
use tree_sitter::Node;

fn dump_node(node: &Node, text: &str, depth: usize, output: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    let kind = node.kind();
    let start = node.start_position();
    let end = node.end_position();
    let node_text = node.utf8_text(text.as_bytes()).unwrap_or("").to_string();
    let preview = if node_text.len() > 60 {
        format!("{}...", &node_text[..60])
    } else {
        node_text.clone()
    };
    
    output.push(format!(
        "{}{} [{}:{} - {}:{}] {}",
        indent, kind, start.row, start.column, end.row, end.column, preview
    ));
    
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            dump_node(&child, text, depth + 1, output);
        }
    }
}

fn find_and_dump_macros(node: &Node, text: &str, depth: usize, output: &mut Vec<String>) {
    if node.kind() == "macro_definition" {
        let indent = "  ".repeat(depth);
        let start = node.start_position();
        let end = node.end_position();
        let node_text = node.utf8_text(text.as_bytes()).unwrap_or("").to_string();
        
        output.push(format!("{}MACRO DEFINITION [{}:{} - {}:{}]", 
            indent, start.row, start.column, end.row, end.column));
        output.push(format!("{}Full text: {}", indent, node_text));
        output.push("".to_string());
        
        // Dump children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                dump_node(&child, text, depth + 1, output);
            }
        }
        output.push("".to_string());
    }
    
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            find_and_dump_macros(&child, text, depth, output);
        }
    }
}

#[test]
fn dump_cst_for_operators() {
    // Test with a simple operator definition
    let code = r#"(>)(a, b) = (b < a)
(&)(x) = x
"#;
    
    let source = FileSource::from_content(PathBuf::from("test_ops.jl"), code.to_string());
    let parsed = match parser::parse(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to parse: {}", e);
            return;
        }
    };
    
    let mut output = Vec::new();
    output.push("CST dump for operator definitions:".to_string());
    output.push("=".repeat(80));
    output.push("".to_string());
    
    let root = parsed.tree.root_node();
    dump_node(&root, &parsed.text, 0, &mut output);
    
    let output_path = "operator_cst_dump.txt";
    if let Err(e) = std::fs::write(output_path, output.join("\n")) {
        eprintln!("Failed to write output: {}", e);
        return;
    }
    
    println!("Operator CST dumped to {}", output_path);
}

#[test]
fn dump_cst_for_macros() {
    // Read a Julia file that contains macros
    let file_path = r"C:\Users\jeroe\AppData\Local\com.juliajunction.dev\julia\julia-1.12.1\share\julia\base\error.jl";
    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read file {}: {}", file_path, e);
            return;
        }
    };
    
    // Parse it
    let source = FileSource::from_content(PathBuf::from(file_path), content.clone());
    let parsed = match parser::parse(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to parse: {}", e);
            return;
        }
    };
    
    // Dump the CST
    let mut output = Vec::new();
    output.push(format!("CST dump for: {}", file_path));
    output.push("=".repeat(80));
    output.push("".to_string());
    
    let root = parsed.tree.root_node();
    dump_node(&root, &parsed.text, 0, &mut output);
    
    // Write to file
    let output_path = "cst_dump.txt";
    if let Err(e) = std::fs::write(output_path, output.join("\n")) {
        eprintln!("Failed to write output: {}", e);
        return;
    }
    
    println!("CST dumped to {}", output_path);
    
    // Also find and dump macro definitions specifically
    let mut macro_output = Vec::new();
    macro_output.push("=".repeat(80));
    macro_output.push("MACRO DEFINITIONS FOUND:".to_string());
    macro_output.push("=".repeat(80));
    macro_output.push("".to_string());
    
    find_and_dump_macros(&root, &parsed.text, 0, &mut macro_output);
    
    let macro_output_path = "macro_dump.txt";
    if let Err(e) = std::fs::write(macro_output_path, macro_output.join("\n")) {
        eprintln!("Failed to write macro output: {}", e);
        return;
    }
    
    println!("Macro definitions dumped to {}", macro_output_path);
}

