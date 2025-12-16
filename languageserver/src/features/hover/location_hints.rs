use crate::pipeline::sources::Document;
use crate::pipeline::storage::CacheManager;
use crate::pipeline::storage::Index;
use crate::types::Position;
use tree_sitter::{Node, Tree};
use super::helpers::{find_prior_assignment_in_scope, extract_assignment_value, type_of_value_like};

/// Build a location-sensitive hint by inspecting prior assignments and preceding if-branches.
pub async fn location_sensitivity_hint<'a>(
    node: Node<'a>,
    tree: &'a Tree,
    text: &str,
    symbol_name: &str,
    document: &Document,
    position: Position,
    cache: Option<&CacheManager>,
    index: &Index,
) -> Option<String> {
    // Consult cached file-level types first
    if let Some(cm) = cache {
        let mut types = cm.file_type_map.types_at(document.uri(), position);
        if !types.is_empty() {
            types.sort();
            types.dedup();
            if types.len() == 1 {
                return Some(format!("::{}", types[0]));
            }
            return Some(format!("::Union{{{}}}", types.join(",")));
        }
    }
    
    // Collect candidates: immediate prior assignment in scope
    let mut candidates: Vec<String> = Vec::new();
    if let Some(rhs) = find_prior_assignment_in_scope(node, text, symbol_name) {
        if let Some(t) = type_of_value_like(&rhs) {
            candidates.push(t);
        } else {
            candidates.push(format!("= {}", rhs));
        }
    }

    // Also look at immediately preceding if-statement sibling (union of branch assignments)
    if let Some(parent) = node.parent() {
        if let Some(if_sibling) = find_nearest_preceding_if(parent, node) {
            let branch_types = collect_branch_assignment_types(if_sibling, tree, text, symbol_name, index);
            for bt in branch_types {
                if !candidates.contains(&bt) {
                    candidates.push(bt);
                }
            }
        }
    }

    if candidates.is_empty() {
        // Type inference is not supported for Julia
        return None;
    }

    // Normalize: split forms like "= expr" are kept; type-only forms prefixed with ::
    let mut type_like: Vec<String> = Vec::new();
    let mut value_like: Vec<String> = Vec::new();
    for c in candidates {
        if c.starts_with("::") {
            type_like.push(c.trim_start_matches(':').trim_start_matches(':').to_string());
        } else if c.starts_with("=") {
            value_like.push(c);
        } else {
            type_like.push(c);
        }
    }
    if !type_like.is_empty() {
        type_like.sort();
        type_like.dedup();
        if type_like.len() == 1 {
            return Some(format!("::{}", type_like[0]));
        } else {
            return Some(format!("::Union{{{}}}", type_like.join(",")));
        }
    }
    // If only value-like info
    value_like.sort();
    value_like.dedup();
    if value_like.len() == 1 {
        return Some(value_like[0].clone());
    }
    Some(format!("= one of [{}]", value_like.join(", ")))
}

fn find_nearest_preceding_if<'a>(parent: Node<'a>, child: Node<'a>) -> Option<Node<'a>> {
    // Find index of child in parent, scan previous siblings for an if-statement
    let mut idx_of_child = None;
    for i in 0..parent.child_count() {
        if let Some(ch) = parent.child(i) {
            if ch.id() == child.id() {
                idx_of_child = Some(i);
                break;
            }
        }
    }
    let Some(idx) = idx_of_child else {
        return None;
    };
    let mut i = idx as i32 - 1;
    while i >= 0 {
        if let Some(sib) = parent.child(i as usize) {
            if sib.kind() == "if_statement" {
                return Some(sib);
            }
        }
        i -= 1;
    }
    None
}

fn collect_branch_assignment_types(
    if_node: Node,
    _tree: &Tree,
    text: &str,
    symbol_name: &str,
    _index: &Index,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    // Search all descendants; keep last assignment per branch naive
    let cursor = if_node.walk();
    let mut stack: Vec<Node> = vec![if_node];
    while let Some(n) = stack.pop() {
        if n.kind() == "assignment" {
            if let Some(lhs) = n.child(0) {
                if lhs.kind() == "identifier" {
                    if let Ok(name) = lhs.utf8_text(text.as_bytes()) {
                        if name == symbol_name {
                            // Type inference is not supported for Julia
                            if let Some(rhs) = extract_assignment_value(n, text) {
                                let vstr = format!("= {}", rhs);
                                if !out.contains(&vstr) {
                                    out.push(vstr);
                                }
                            }
                        }
                    }
                }
            }
        }
        for i in 0..n.child_count() {
            if let Some(ch) = n.child(i) {
                stack.push(ch);
            }
        }
    }
    drop(cursor);
    out
}

