use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use spex_graph::{Graph, GraphNode};
use std::collections::HashMap;
use std::process::Command;

/// Runs `brew deps --tree <formula>` and converts Homebrew's indented
/// dependency tree into a `spex_graph::Graph`. Real package metadata (no
/// installation or local system state involved) — a package manager is a
/// natural second "low-level" tree alongside traceroute and pstree.
pub fn run(formula: &str) -> Result<Graph> {
    let output = Command::new("brew")
        .args(["deps", "--tree", formula])
        .env("HOMEBREW_NO_ENV_HINTS", "1")
        .output()
        .context("running `brew deps --tree` (is Homebrew installed and on PATH?)")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        bail!(
            "`brew deps --tree {formula}` produced no output (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let entries = parse_tree(&stdout);
    if entries.is_empty() {
        bail!("could not parse any packages from `brew deps --tree {formula}` output");
    }

    let subtree_size = compute_subtree_sizes(&entries);
    let nodes = build_nodes(&entries, &subtree_size);

    Ok(Graph {
        title: Some(format!("brew dependency tree: {formula}")),
        metric_label: Some("subtree size (packages)".to_string()),
        nodes,
    })
}

/// Turns the positional (one entry per real output line) parse into
/// `GraphNode`s, merging real duplicate package names into one node instead
/// of rendering each re-occurrence as its own blob. `brew deps --tree`
/// doesn't dedup its own output — it re-expands the same real package at
/// every position it's reachable from (e.g. `openssl@3` under two unrelated
/// branches) — so without this, one real package would get two duplicate
/// nodes in the graph.
///
/// The *first* time a name is seen becomes the one real node: its position
/// in the entry list still drives `parent` (and therefore this node's one
/// real 3D position downstream, via `layout::place`'s single-parent walk —
/// unchanged by this). Every later re-occurrence of the same name reuses
/// that node's id and contributes its own real parent at that position to
/// `extra_parents` instead of creating a second node — same "one real
/// position, extra structure recorded alongside it" precedent as
/// `sql_schema.rs`'s extra-FK handling and `molecule.rs`'s `ring_bond_to`.
fn build_nodes(entries: &[Entry], subtree_size: &[f64]) -> Vec<GraphNode> {
    // canonical_of[i] = index of the first entry sharing entries[i]'s name.
    let mut canonical_of: Vec<usize> = Vec::with_capacity(entries.len());
    let mut first_seen: HashMap<&str, usize> = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        let canonical = *first_seen.entry(e.name.as_str()).or_insert(i);
        canonical_of.push(canonical);
    }
    let node_id = |idx: usize| format!("pkg-{}", canonical_of[idx]);

    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut node_pos: HashMap<usize, usize> = HashMap::new(); // canonical entry idx -> position in `nodes`

    for (i, e) in entries.iter().enumerate() {
        if canonical_of[i] != i {
            // A later real occurrence of an already-seen package: not a new
            // node, but a genuine additional real parent for the canonical
            // one (skip self-references and exact duplicates of the primary parent).
            let Some(p) = e.parent else { continue };
            let extra_parent_id = node_id(p);
            let pos = node_pos[&canonical_of[i]];
            let canonical_node = &mut nodes[pos];
            if extra_parent_id != canonical_node.id
                && canonical_node.parent.as_deref() != Some(extra_parent_id.as_str())
                && !canonical_node.extra_parents.contains(&extra_parent_id)
            {
                canonical_node.extra_parents.push(extra_parent_id);
            }
            continue;
        }

        let mut metadata = Map::new();
        metadata.insert("name".to_string(), Value::from(e.name.clone()));
        metadata.insert("depth".to_string(), Value::from(e.depth));
        node_pos.insert(i, nodes.len());
        nodes.push(GraphNode {
            id: node_id(i),
            label: e.name.clone(),
            parent: e.parent.map(node_id),
            // Subtree size (including self): a rough "how much of the
            // dependency tree hangs off this package" weight, driving color.
            // Computed from this (canonical/first) occurrence's own position
            // only — a later occurrence's own subtree, if it has one, isn't
            // folded into this number, same simplification `parent` above
            // already makes for position.
            metric: Some(subtree_size[i]),
            extra_parents: Vec::new(),
            metadata,
        });
    }
    nodes
}

struct Entry {
    name: String,
    parent: Option<usize>,
    depth: usize,
}

/// Parses Homebrew's `--tree` box-drawing output. Depth is the number of
/// complete 4-character prefix chunks ("├── ", "└── ", "│   ", or "    ")
/// before the package name; a stack of the most-recently-seen node at each
/// depth gives each line's parent.
fn parse_tree(output: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() || line.starts_with("Warning:") || line.starts_with("==>") {
            continue;
        }

        if !line.starts_with(' ') && !line.starts_with('├') && !line.starts_with('└') && !line.starts_with('│') {
            let name = line.trim().to_string();
            if name.is_empty() {
                continue;
            }
            let idx = entries.len();
            entries.push(Entry { name, parent: None, depth: 0 });
            stack.clear();
            stack.push(idx);
            continue;
        }

        let chars: Vec<char> = line.chars().collect();
        let mut pos = 0usize;
        let mut depth = 0usize;
        while pos + 4 <= chars.len() {
            let chunk: String = chars[pos..pos + 4].iter().collect();
            match chunk.as_str() {
                "│   " | "    " => {
                    pos += 4;
                    depth += 1;
                }
                "├── " | "└── " => {
                    pos += 4;
                    depth += 1;
                    break;
                }
                _ => break,
            }
        }
        let name: String = chars[pos..].iter().collect::<String>().trim().to_string();
        if name.is_empty() {
            continue;
        }

        let parent = if depth >= 1 { stack.get(depth - 1).copied() } else { None };
        let idx = entries.len();
        entries.push(Entry { name, parent, depth });
        stack.truncate(depth);
        stack.push(idx);
    }
    entries
}

fn compute_subtree_sizes(entries: &[Entry]) -> Vec<f64> {
    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        if let Some(p) = e.parent {
            children.entry(p).or_default().push(i);
        }
    }

    fn size_of(i: usize, children: &HashMap<usize, Vec<usize>>, out: &mut [f64]) -> f64 {
        if out[i] > 0.0 {
            return out[i];
        }
        let mut total = 1.0;
        if let Some(kids) = children.get(&i) {
            for &k in kids {
                total += size_of(k, children, out);
            }
        }
        out[i] = total;
        total
    }

    let mut sizes = vec![0.0; entries.len()];
    for (i, e) in entries.iter().enumerate() {
        if e.parent.is_none() {
            size_of(i, &children, &mut sizes);
        }
    }
    sizes
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
neovim
├── libuv
├── lpeg
├── luajit
├── luv
│   └── libuv
├── tree-sitter
├── unibilium
├── utf8proc
└── gettext
    ├── json-c
    └── libunistring
";

    #[test]
    fn parses_nested_tree_with_duplicate_names() {
        // `parse_tree` itself stays positional/per-line — one `Entry` per
        // real line of `brew deps --tree` output, duplicates and all.
        // Deduping same-name packages into one `GraphNode` happens one
        // level up, in `build_nodes` (see
        // `build_nodes_merges_duplicate_package_into_one_node_with_extra_parents`
        // below) — this test documents the raw parse this merge step
        // consumes, not the final graph.
        let entries = parse_tree(SAMPLE);
        assert_eq!(entries.len(), 12);
        assert_eq!(entries[0].name, "neovim");
        assert_eq!(entries[0].parent, None);

        // Two distinct "libuv" entries at different tree positions.
        let libuv_indices: Vec<usize> = entries.iter().enumerate().filter(|(_, e)| e.name == "libuv").map(|(i, _)| i).collect();
        assert_eq!(libuv_indices.len(), 2);
        assert_eq!(entries[libuv_indices[0]].parent, Some(0)); // direct child of neovim
        assert_eq!(entries[libuv_indices[0]].depth, 1);

        let luv_idx = entries.iter().position(|e| e.name == "luv").unwrap();
        assert_eq!(entries[libuv_indices[1]].parent, Some(luv_idx)); // nested under luv
        assert_eq!(entries[libuv_indices[1]].depth, 2);

        let gettext_idx = entries.iter().position(|e| e.name == "gettext").unwrap();
        let json_c_idx = entries.iter().position(|e| e.name == "json-c").unwrap();
        let libunistring_idx = entries.iter().position(|e| e.name == "libunistring").unwrap();
        assert_eq!(entries[json_c_idx].parent, Some(gettext_idx));
        assert_eq!(entries[libunistring_idx].parent, Some(gettext_idx));
    }

    #[test]
    fn build_nodes_merges_duplicate_package_into_one_node_with_extra_parents() {
        // "libuv" is real-world-shaped here: it's a direct dep of neovim
        // AND a dep of luv, which `brew deps --tree` re-expands at both
        // positions rather than deduping itself (issue #24). The graph we
        // hand to the layout stage should merge those into one real node.
        let entries = parse_tree(SAMPLE);
        let subtree_size = compute_subtree_sizes(&entries);
        let nodes = build_nodes(&entries, &subtree_size);

        // 12 real lines, one real duplicate ("libuv" appears twice) merged away.
        assert_eq!(nodes.len(), 11);

        let libuv_nodes: Vec<&GraphNode> = nodes.iter().filter(|n| n.label == "libuv").collect();
        assert_eq!(libuv_nodes.len(), 1, "libuv must appear as exactly one node, not two duplicates");
        let libuv = libuv_nodes[0];

        let neovim = nodes.iter().find(|n| n.label == "neovim").unwrap();
        let luv = nodes.iter().find(|n| n.label == "luv").unwrap();

        // Its one real 3D position is still driven by its first real
        // occurrence's parent (direct child of neovim).
        assert_eq!(libuv.parent.as_deref(), Some(neovim.id.as_str()));
        // The second real occurrence's parent (luv) becomes a real extra
        // structural edge instead of a second node.
        assert_eq!(libuv.extra_parents, vec![luv.id.clone()]);

        // Every other (genuinely non-duplicate) package still gets its own node.
        assert!(nodes.iter().any(|n| n.label == "gettext"));
        assert!(nodes.iter().any(|n| n.label == "json-c"));
        assert!(nodes.iter().any(|n| n.label == "libunistring"));
    }

    #[test]
    fn subtree_sizes_reflect_nesting() {
        let entries = parse_tree(SAMPLE);
        let sizes = compute_subtree_sizes(&entries);
        assert_eq!(sizes[0], 12.0); // root: whole tree

        let gettext_idx = entries.iter().position(|e| e.name == "gettext").unwrap();
        assert_eq!(sizes[gettext_idx], 3.0); // gettext + json-c + libunistring

        let lpeg_idx = entries.iter().position(|e| e.name == "lpeg").unwrap();
        assert_eq!(sizes[lpeg_idx], 1.0); // leaf
    }
}
