use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use spex_graph::{Graph, GraphNode};
use std::collections::HashMap;
use std::process::Command;

/// Runs `cargo tree -p <package>` (from the current directory — must be run
/// inside a real Cargo workspace/project) and converts its box-drawing
/// dependency tree into a `spex_graph::Graph`. Real package metadata, no
/// network access needed (works off the already-resolved `Cargo.lock`) — a
/// second real package-manager tree alongside `brew-deps`.
pub fn run(package: &str) -> Result<Graph> {
    let output = Command::new("cargo")
        .args(["tree", "-p", package])
        .output()
        .context("running `cargo tree` (is this a Cargo project, and is `package` a real dependency in it?)")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        bail!("`cargo tree -p {package}` produced no output (stderr: {})", String::from_utf8_lossy(&output.stderr));
    }

    let entries = parse_tree(&stdout);
    if entries.is_empty() {
        bail!("could not parse any packages from `cargo tree -p {package}` output");
    }

    let subtree_size = compute_subtree_sizes(&entries);
    let nodes = build_nodes(&entries, &subtree_size);
    Ok(Graph {
        title: Some(format!("cargo dependency tree: {package}")),
        metric_label: Some("subtree size (crates)".to_string()),
        nodes,
    })
}

struct Entry {
    name: String,
    version: Option<String>,
    parent: Option<usize>,
    depth: usize,
}

/// Turns the positional (one entry per real line of `cargo tree` output)
/// parse into `GraphNode`s, merging real duplicate crates into one node
/// instead of rendering each re-occurrence as its own blob. `cargo tree`
/// doesn't dedup its own output either — the same real resolved crate (same
/// name *and* same version — Cargo's resolver picks one exact version per
/// unification group) legitimately gets re-expanded at every position in the
/// tree it's required from (e.g. `proc-macro2` pulled in by both
/// `clap_derive` and `quote`, where the second occurrence is elided with a
/// trailing `(*)` since its subtree was already printed once) — so without
/// this, one real crate would get two duplicate nodes in the graph.
///
/// Identity here is `(name, version)`, not just `name` — a version conflict
/// can legitimately put two *different* versions of the same crate name in
/// one real dependency tree, and those really are two distinct crates, not a
/// duplicate; only a repeat of the same name at the same version is the real
/// "reached from two branches" case issue #24 is about.
///
/// The *first* time an (name, version) pair is seen becomes the one real
/// node: its position in the entry list still drives `parent`. Every later
/// re-occurrence (typically the `(*)`-elided ones, which `parse_tree` already
/// leaves childless) reuses that node's id and contributes its own real
/// parent at that position to `extra_parents` instead of creating a second
/// node — same technique as `brew_deps::build_nodes`.
fn build_nodes(entries: &[Entry], subtree_size: &[f64]) -> Vec<GraphNode> {
    // canonical_of[i] = index of the first entry sharing entries[i]'s (name, version).
    let mut canonical_of: Vec<usize> = Vec::with_capacity(entries.len());
    let mut first_seen: HashMap<(&str, Option<&str>), usize> = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        let key = (e.name.as_str(), e.version.as_deref());
        let canonical = *first_seen.entry(key).or_insert(i);
        canonical_of.push(canonical);
    }
    let node_id = |idx: usize| format!("pkg-{}", canonical_of[idx]);

    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut node_pos: HashMap<usize, usize> = HashMap::new(); // canonical entry idx -> position in `nodes`

    for (i, e) in entries.iter().enumerate() {
        if canonical_of[i] != i {
            // A later real occurrence of an already-seen (name, version): not
            // a new node, but a genuine additional real parent for the
            // canonical one (skip self-references and exact duplicates of
            // the primary parent).
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
        if let Some(version) = &e.version {
            metadata.insert("version".to_string(), Value::from(version.clone()));
        }
        metadata.insert("depth".to_string(), Value::from(e.depth));
        node_pos.insert(i, nodes.len());
        nodes.push(GraphNode {
            id: node_id(i),
            label: e.name.clone(),
            parent: e.parent.map(node_id),
            // Subtree size (including self), computed from this
            // (canonical/first) occurrence's own position only — same
            // simplification `brew_deps::build_nodes` makes.
            metric: Some(subtree_size[i]),
            metadata,
            ..Default::default()
        });
    }
    nodes
}

/// Parses `cargo tree`'s box-drawing output. Same depth-by-prefix-chunk
/// technique as `brew_deps::parse_tree`, adjusted for cargo's line shape:
/// `name vX.Y.Z (annotation)` instead of a bare name, and `[build-dependencies]`/
/// `[dev-dependencies]` section-header lines (skipped — they don't
/// represent a package, and the real dependency lines beneath them are
/// already at the correct depth via the normal prefix-chunk count).
fn parse_tree(output: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
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
        let rest: String = chars[pos..].iter().collect::<String>().trim().to_string();
        if rest.is_empty() || rest.starts_with('[') {
            continue; // section header (e.g. "[build-dependencies]"), not a package
        }

        let (name, version) = parse_name_version(&rest);

        let parent = if depth >= 1 { stack.get(depth - 1).copied() } else { None };
        let idx = entries.len();
        entries.push(Entry { name, version, parent, depth });
        if depth == 0 {
            stack.clear();
        } else {
            stack.truncate(depth);
        }
        stack.push(idx);
    }
    entries
}

/// `"serde v1.0.229 (*)"` -> `("serde", Some("1.0.229"))`; the `(*)` /
/// `(proc-macro)` / `(build)` / local-path annotation in parens is dropped —
/// it's cargo's own elision/kind marker, not part of the package identity.
fn parse_name_version(text: &str) -> (String, Option<String>) {
    let before_paren = text.split(" (").next().unwrap_or(text).trim();
    match before_paren.split_once(" v") {
        Some((name, version)) => (name.to_string(), Some(version.to_string())),
        None => (before_paren.to_string(), None),
    }
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
spex-cli v0.1.0 (/Users/stefan/workspace/stardate/spex/crates/spex-cli)
├── anyhow v1.0.104
├── clap v4.6.4
│   ├── clap_builder v4.6.2
│   └── clap_derive v4.6.4 (proc-macro)
│       ├── proc-macro2 v1.0.107
│       │   [build-dependencies]
│       │   └── rustversion v1.0.23 (proc-macro)
│       └── quote v1.0.47
│           └── proc-macro2 v1.0.107 (*)
└── libc v0.2.189
";

    #[test]
    fn parses_name_version_and_nesting() {
        let entries = parse_tree(SAMPLE);
        assert_eq!(entries[0].name, "spex-cli");
        assert_eq!(entries[0].version, Some("0.1.0".to_string()));
        assert_eq!(entries[0].parent, None);

        let clap_idx = entries.iter().position(|e| e.name == "clap").unwrap();
        assert_eq!(entries[clap_idx].parent, Some(0));
        assert_eq!(entries[clap_idx].depth, 1);

        let derive_idx = entries.iter().position(|e| e.name == "clap_derive").unwrap();
        assert_eq!(entries[derive_idx].parent, Some(clap_idx));

        // The [build-dependencies] header line must not create a node, and
        // must not shift the depth of the real package line beneath it.
        assert!(entries.iter().all(|e| e.name != "[build-dependencies]"));
        let rustversion_idx = entries.iter().position(|e| e.name == "rustversion").unwrap();
        let proc_macro2_idx = entries.iter().position(|e| e.name == "proc-macro2" && e.parent == Some(derive_idx)).unwrap();
        assert_eq!(entries[rustversion_idx].parent, Some(proc_macro2_idx));
    }

    #[test]
    fn strips_star_annotation_from_elided_repeated_subtrees() {
        let entries = parse_tree(SAMPLE);
        let quote_idx = entries.iter().position(|e| e.name == "quote").unwrap();
        // "proc-macro2 v1.0.107 (*)" is nested under quote in the sample —
        // the "(*)" (cargo's "subtree already printed elsewhere" marker)
        // must be stripped from the version, not left dangling on it.
        let elided = entries.iter().find(|e| e.name == "proc-macro2" && e.parent == Some(quote_idx)).unwrap();
        assert_eq!(elided.version.as_deref(), Some("1.0.107"));
    }

    #[test]
    fn parse_name_version_handles_bare_names_without_a_version() {
        assert_eq!(parse_name_version("some-crate"), ("some-crate".to_string(), None));
    }

    #[test]
    fn subtree_sizes_reflect_nesting() {
        let entries = parse_tree(SAMPLE);
        let sizes = compute_subtree_sizes(&entries);
        assert_eq!(sizes[0], entries.len() as f64); // root: whole tree

        let libc_idx = entries.iter().position(|e| e.name == "libc").unwrap();
        assert_eq!(sizes[libc_idx], 1.0); // leaf
    }

    #[test]
    fn build_nodes_merges_duplicate_crate_into_one_node_with_extra_parents() {
        // "proc-macro2 v1.0.107" is real-world-shaped here: it's a direct
        // dep of `clap_derive` AND a dep of `quote`, and `cargo tree` itself
        // marks the second occurrence with "(*)" since it already printed
        // that subtree once (issue #24's cargo-tree analog of the brew-deps
        // `openssl@3` case). The graph we hand to the layout stage should
        // merge those into one real node.
        let entries = parse_tree(SAMPLE);
        let subtree_size = compute_subtree_sizes(&entries);
        let nodes = build_nodes(&entries, &subtree_size);

        // 10 real package lines ("[build-dependencies]" isn't a package),
        // one real duplicate ("proc-macro2 v1.0.107" appears twice) merged away.
        assert_eq!(nodes.len(), 9);

        let proc_macro2_nodes: Vec<&GraphNode> = nodes.iter().filter(|n| n.label == "proc-macro2").collect();
        assert_eq!(proc_macro2_nodes.len(), 1, "proc-macro2 v1.0.107 must appear as exactly one node, not two duplicates");
        let proc_macro2 = proc_macro2_nodes[0];

        let clap_derive = nodes.iter().find(|n| n.label == "clap_derive").unwrap();
        let quote = nodes.iter().find(|n| n.label == "quote").unwrap();

        // Its one real 3D position is still driven by its first real
        // occurrence's parent (direct child of clap_derive), which also
        // keeps its own real subtree (rustversion via [build-dependencies]).
        assert_eq!(proc_macro2.parent.as_deref(), Some(clap_derive.id.as_str()));
        // The second (elided, "(*)") real occurrence's parent (quote)
        // becomes a real extra structural edge instead of a second node.
        assert_eq!(proc_macro2.extra_parents, vec![quote.id.clone()]);

        let rustversion = nodes.iter().find(|n| n.label == "rustversion").unwrap();
        assert_eq!(rustversion.parent.as_deref(), Some(proc_macro2.id.as_str()));

        // Every other (genuinely non-duplicate) crate still gets its own node.
        assert!(nodes.iter().any(|n| n.label == "anyhow"));
        assert!(nodes.iter().any(|n| n.label == "libc"));
    }

    #[test]
    fn build_nodes_keeps_same_name_different_version_as_distinct_nodes() {
        // A real cargo dependency graph can legitimately resolve two
        // different versions of the same crate name side by side (a version
        // conflict) — those are two distinct crates and must not be merged
        // just because the name matches.
        const VERSION_CONFLICT_SAMPLE: &str = "\
root v0.1.0 (/tmp/root)
├── left v1.0.0
│   └── shared-lib v1.0.0
└── right v1.0.0
    └── shared-lib v2.0.0
";
        let entries = parse_tree(VERSION_CONFLICT_SAMPLE);
        let subtree_size = compute_subtree_sizes(&entries);
        let nodes = build_nodes(&entries, &subtree_size);

        let shared_nodes: Vec<&GraphNode> = nodes.iter().filter(|n| n.label == "shared-lib").collect();
        assert_eq!(shared_nodes.len(), 2, "shared-lib v1.0.0 and shared-lib v2.0.0 are distinct crates and must not be merged");
        for n in &shared_nodes {
            assert!(n.extra_parents.is_empty());
        }
    }
}
