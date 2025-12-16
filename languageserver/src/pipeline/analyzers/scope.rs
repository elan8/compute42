use crate::pipeline::types::{ParsedItem, ScopeTree, ScopeNode};
use crate::types::{LspError, Range, Position};
use tree_sitter::Node;

/// Analyze a parsed item to build scope hierarchy
pub fn analyze(parsed: &ParsedItem) -> Result<ScopeTree, LspError> {
    let root = parsed.tree.root_node();
    let text = parsed.text.as_str();
    let file_uri = parsed.path.to_string_lossy().to_string();

    let root_scope = ScopeNode {
        id: 0,
        parent_id: None,
        range: node_to_range(root),
        file_uri: file_uri.clone(),
        children: Vec::new(),
    };

    let mut scope_tree = ScopeTree { root: root_scope };
    let mut next_scope_id = 1;

    build_scope_tree(&root, text, &file_uri, 0, &mut scope_tree.root, &mut next_scope_id)?;

    Ok(scope_tree)
}

fn build_scope_tree(
    node: &Node,
    _text: &str,
    file_uri: &str,
    parent_id: u32,
    parent_scope: &mut ScopeNode,
    next_scope_id: &mut u32,
) -> Result<(), LspError> {
    match node.kind() {
        "function_definition" | "module_definition" => {
            let scope_id = *next_scope_id;
            *next_scope_id += 1;

            let mut child_scope = ScopeNode {
                id: scope_id,
                parent_id: Some(parent_id),
                range: node_to_range(*node),
                file_uri: file_uri.to_string(),
                children: Vec::new(),
            };

            // Recursively build children scopes
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    build_scope_tree(&child, _text, file_uri, scope_id, &mut child_scope, next_scope_id)?;
                }
            }

            parent_scope.children.push(child_scope);
            return Ok(());
        }
        _ => {}
    }

    // Continue walking children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            build_scope_tree(&child, _text, file_uri, parent_id, parent_scope, next_scope_id)?;
        }
    }

    Ok(())
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
    fn test_analyze_scope_tree() {
        let code = r#"
function outer()
    function inner()
    end
end
"#;
        let parsed = parse_code(code);
        let scope_tree = analyze(&parsed).unwrap();

        assert_eq!(scope_tree.root.id, 0);
        assert_eq!(scope_tree.root.children.len(), 1); // outer function
        assert_eq!(scope_tree.root.children[0].children.len(), 1); // inner function
    }
}


















