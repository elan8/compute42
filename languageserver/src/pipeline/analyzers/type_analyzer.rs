use crate::pipeline::types::ParsedItem;
use crate::types::{TypeDefinition, TypeDefinitionKind};
use crate::types::{LspError, Range, Position};
use tree_sitter::Node;

/// Analyze a parsed item to extract type definitions
pub fn analyze(parsed: &ParsedItem) -> Result<Vec<TypeDefinition>, LspError> {
    let mut types = Vec::new();
    let root = parsed.tree.root_node();
    let text = parsed.text.as_str();

    // Infer module name from file path (same logic as signature analyzer)
    let default_module = infer_module_name_from_path(&parsed.path);

    walk_node(&root, text, &parsed.path.to_string_lossy(), &default_module, &mut types)?;

    Ok(types)
}

fn walk_node(
    node: &Node,
    text: &str,
    file_uri: &str,
    module_name: &str,
    types: &mut Vec<TypeDefinition>,
) -> Result<(), LspError> {
    match node.kind() {
        "struct_definition" => {
            if let Some(mut type_def) = extract_struct_definition(node, text, file_uri)? {
                type_def.module = module_name.to_string();
                types.push(type_def);
            }
        }
        "abstract_definition" => {
            if let Some(mut type_def) = extract_abstract_definition(node, text, file_uri)? {
                type_def.module = module_name.to_string();
                types.push(type_def);
            }
        }
        "module_definition" => {
            // Extract module definitions as types so module names like DataFrames, CSV can be recognized
            if let Some(mut type_def) = extract_module_definition(node, text, file_uri)? {
                // For module definitions, use the module name from the file (not the inferred one)
                // But if it matches the inferred name, use the inferred name to avoid nesting
                let nested_module = if module_name == type_def.name {
                    module_name.to_string()
                } else if module_name.is_empty() || module_name == "Main" {
                    type_def.name.clone()
                } else {
                    format!("{}.{}", module_name, type_def.name)
                };
                
                type_def.module = nested_module.clone();
                types.push(type_def);
                
                // Walk children with new module context
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        walk_node(&child, text, file_uri, &nested_module, types)?;
                    }
                }
                return Ok(());
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_node(&child, text, file_uri, module_name, types)?;
        }
    }

    Ok(())
}

fn extract_struct_definition(
    node: &Node,
    text: &str,
    file_uri: &str,
) -> Result<Option<TypeDefinition>, LspError> {
    if let Some(type_head) = find_first_child_of_type(node, "type_head") {
        if let Some(name_node) = find_first_child_of_type(&type_head, "identifier") {
            let name = name_node.utf8_text(text.as_bytes())
                .map_err(|e| LspError::ParseError(format!("Failed to extract struct name: {}", e)))?
                .to_string();

            let range = node_to_range(name_node);
            let doc_comment = extract_doc_comment(node, text)?;

            return Ok(Some(TypeDefinition {
                module: String::new(), // Will be set by caller if needed
                name,
                kind: TypeDefinitionKind::Struct,
                doc_comment,
                file_uri: file_uri.to_string(),
                range,
            }));
        }
    }

    Ok(None)
}

fn extract_abstract_definition(
    node: &Node,
    text: &str,
    file_uri: &str,
) -> Result<Option<TypeDefinition>, LspError> {
    if let Some(type_head) = find_first_child_of_type(node, "type_head") {
        if let Some(name_node) = find_first_child_of_type(&type_head, "identifier") {
            let name = name_node.utf8_text(text.as_bytes())
                .map_err(|e| LspError::ParseError(format!("Failed to extract abstract type name: {}", e)))?
                .to_string();

            let range = node_to_range(name_node);
            let doc_comment = extract_doc_comment(node, text)?;

            return Ok(Some(TypeDefinition {
                module: String::new(), // Will be set by caller if needed
                name,
                kind: TypeDefinitionKind::Abstract,
                doc_comment,
                file_uri: file_uri.to_string(),
                range,
            }));
        }
    }

    Ok(None)
}

fn extract_module_definition(
    node: &Node,
    text: &str,
    file_uri: &str,
) -> Result<Option<TypeDefinition>, LspError> {
    // Extract module name from module_definition -> identifier
    if let Some(name_node) = find_first_child_of_type(node, "identifier") {
        let name = name_node.utf8_text(text.as_bytes())
            .map_err(|e| LspError::ParseError(format!("Failed to extract module name: {}", e)))?
            .to_string();

        let range = node_to_range(name_node);
        let doc_comment = extract_doc_comment(node, text)?;

        // Store module as a type - module name will be set by walk_node based on context
        return Ok(Some(TypeDefinition {
            module: String::new(), // Will be set by walk_node
            name,
            kind: TypeDefinitionKind::Abstract, // Use Abstract as closest match (modules are like abstract types)
            doc_comment,
            file_uri: file_uri.to_string(),
            range,
        }));
    }

    Ok(None)
}

/// Infer module name from file path (same logic as signature analyzer)
/// For package files like "CSV/src/CSV.jl", extract "CSV"
/// For files like "Base/filesystem.jl", extract "Base"
fn infer_module_name_from_path(path: &std::path::Path) -> String {
    // Try to extract from path components
    // Common patterns:
    // - packages/CSV/{uuid}/src/CSV.jl -> CSV
    // - packages/DataFrames/{uuid}/src/DataFrames.jl -> DataFrames
    // - Base/filesystem.jl -> Base
    
    let path_str = path.to_string_lossy();
    
    // Check if path contains "packages/{PackageName}/"
    if let Some(packages_pos) = path_str.find("packages/") {
        let after_packages = &path_str[packages_pos + 9..]; // Skip "packages/"
        if let Some(slash_pos) = after_packages.find('/') {
            let package_name = &after_packages[..slash_pos];
            return package_name.to_string();
        }
    }
    
    // Check if path contains "Base/" or similar stdlib paths
    for stdlib_module in ["Base", "Core", "Statistics", "LinearAlgebra"] {
        if path_str.contains(&format!("{}/", stdlib_module)) {
            return stdlib_module.to_string();
        }
    }
    
    // Fallback: try to use filename without extension
    if let Some(file_stem) = path.file_stem() {
        if let Some(stem_str) = file_stem.to_str() {
            // Capitalize first letter (Julia module convention)
            let mut chars = stem_str.chars();
            if let Some(first) = chars.next() {
                return format!("{}{}", first.to_uppercase(), chars.as_str());
            }
        }
    }
    
    "Main".to_string()
}

fn find_first_child_of_type<'a>(node: &'a Node<'a>, kind: &str) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == kind {
                return Some(child);
            }
        }
    }
    None
}

fn node_to_range(node: Node) -> Range {
    let start_pos = node.start_position();
    let end_pos = node.end_position();

    Range {
        start: Position {
            line: start_pos.row as u32,
            character: start_pos.column as u32,
        },
        end: Position {
            line: end_pos.row as u32,
            character: end_pos.column as u32,
        },
    }
}

fn extract_doc_comment(node: &Node, text: &str) -> Result<Option<String>, LspError> {
    let start_byte = node.start_byte();
    let before_text = &text[..start_byte.min(text.len())];

    if let Some(doc_start) = before_text.rfind("\"\"\"") {
        if let Some(doc_end) = text[doc_start + 3..].find("\"\"\"") {
            let doc = text[doc_start + 3..doc_start + 3 + doc_end].trim().to_string();
            if !doc.is_empty() {
                return Ok(Some(doc));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::parser;
    use crate::pipeline::sources::file::FileSource;
    use std::path::PathBuf;

    fn parse_code(code: &str) -> ParsedItem {
        let source = FileSource::from_content(PathBuf::from("test.jl"), code.to_string());
        parser::parse(&source).unwrap()
    }

    #[test]
    fn test_analyze_struct() {
        let code = "struct MyStruct x::Int end";
        let parsed = parse_code(code);
        let types = analyze(&parsed).unwrap();

        assert_eq!(types.len(), 1);
        assert_eq!(types[0].name, "MyStruct");
        assert_eq!(types[0].kind, TypeDefinitionKind::Struct);
    }

    #[test]
    fn test_analyze_abstract() {
        let code = "abstract type MyAbstract end";
        let parsed = parse_code(code);
        let types = analyze(&parsed).unwrap();

        assert_eq!(types.len(), 1);
        assert_eq!(types[0].name, "MyAbstract");
        assert_eq!(types[0].kind, TypeDefinitionKind::Abstract);
    }
}

