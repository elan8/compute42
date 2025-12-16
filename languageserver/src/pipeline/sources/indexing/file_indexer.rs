// This file is no longer used - index_file was only used by PackageIndexer which has been removed
// Package indexing is now handled by the pipeline system using Index
// Keeping this file temporarily for reference but it should be deleted

// use crate::pipeline::parser::JuliaParser;
// use crate::types::LspError;
// use std::path::Path;
// use super::ast_walker::walk_node;

// /// Index a single Julia file
// pub fn index_file(
//     file_path: &Path,
//     module_name: &str,
//     parser: &JuliaParser,
//     find_first_child_of_type: &dyn for<'a> Fn(tree_sitter::Node<'a>, &'a str) -> Result<tree_sitter::Node<'a>, LspError>,
// ) -> Result<TypeRegistry, LspError> {
//     let content = std::fs::read_to_string(file_path)
//         .map_err(|e| LspError::InternalError(format!("Failed to read file: {}", e)))?;
//     
//     let mut parser_instance = parser.create_parser()?;
//     let tree = parser_instance.parse(&content, None)
//         .ok_or_else(|| LspError::ParseError("Failed to parse file".to_string()))?;
//     
//     let mut registry = TypeRegistry::new();
//     let root = tree.root_node();
//     
//     // Walk AST to extract function signatures and type definitions
//     walk_node(root, &content, module_name, &mut registry, find_first_child_of_type)?;
//     
//     Ok(registry)
// }
