use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use spex_graph::{Graph, GraphNode};
use std::collections::HashMap;
use std::process::Command;

/// Runs `npm ls --json --all` (from the current directory — must be a real
/// npm project with `node_modules` installed) and converts its real,
/// already-structured JSON dependency tree into a `spex_graph::Graph`. A
/// third real package-manager tree alongside `brew-deps`/`cargo-deps`, but
/// parsed from real JSON instead of scraping box-drawing tree art, since
/// `npm ls` (unlike `brew`/`cargo`) supports a `--json` mode directly.
pub fn run() -> Result<Graph> {
    let output = Command::new("npm")
        .args(["ls", "--json", "--all"])
        .output()
        .context("running `npm ls` (is this an npm project, and is npm on PATH?)")?;

    // `npm ls` can exit non-zero on real-world dependency-resolution
    // warnings (peer dep mismatches, extraneous packages) even when its
    // JSON output is perfectly valid, so the JSON is parsed regardless of
    // exit status — only a genuinely unparseable/empty output is an error.
    if output.stdout.trim_ascii().is_empty() {
        bail!("`npm ls --json` produced no output (stderr: {})", String::from_utf8_lossy(&output.stderr));
    }
    let root: Value = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("parsing `npm ls --json` output (stderr: {})", String::from_utf8_lossy(&output.stderr)))?;

    let project_name = root.get("name").and_then(Value::as_str).unwrap_or("npm-project").to_string();
    let project_version = root.get("version").and_then(Value::as_str).map(str::to_string);

    let mut entries = vec![Entry { name: project_name.clone(), version: project_version, parent: None }];
    if let Some(deps) = root.get("dependencies").and_then(Value::as_object) {
        walk(deps, 0, &mut entries);
    }

    let subtree_size = compute_subtree_sizes(&entries);
    let nodes = build_nodes(&entries, &subtree_size);

    Ok(Graph {
        title: Some(format!("npm dependency tree: {project_name}")),
        metric_label: Some("subtree size (packages)".to_string()),
        nodes,
    })
}

struct Entry {
    name: String,
    version: Option<String>,
    parent: Option<usize>,
}

/// Turns the positional (one entry per real `dependencies` object walked)
/// parse into `GraphNode`s, merging real duplicate packages into one node
/// instead of rendering each re-occurrence as its own blob. `npm ls --json`
/// doesn't dedup its own output — the same real resolved package (same name
/// *and* same resolved version) legitimately gets re-expanded at every
/// position in the tree it's required from (e.g. a common transitive dep
/// pulled in by two unrelated top-level packages), so without this, one real
/// installed package would get two duplicate nodes in the graph.
///
/// Identity here is `(name, version)`, not just `name` — unlike Homebrew
/// formulas, npm legitimately resolves the *same* package name to two
/// *different* versions at different tree positions (peer-dep/semver-range
/// conflicts), and those really are two distinct installed packages, not a
/// duplicate; only a repeat of the same name at the same resolved version is
/// the real "reached from two branches" case issue #24 is about.
///
/// The *first* time an (name, version) pair is seen becomes the one real
/// node: its position in the entry list still drives `parent`. Every later
/// re-occurrence of the same (name, version) reuses that node's id and
/// contributes its own real parent at that position to `extra_parents`
/// instead of creating a second node — same technique as
/// `brew_deps::build_nodes`.
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

/// Recursively walks `npm ls --json`'s real `dependencies` object shape
/// (`{"pkg-name": {"version": "...", "dependencies": {...}}}`), same
/// positional-index-as-id scheme as `brew_deps`/`cargo_deps` since the same
/// package name can legitimately appear more than once at different tree
/// positions (different resolved versions, or just re-required elsewhere).
fn walk(deps: &Map<String, Value>, parent: usize, entries: &mut Vec<Entry>) {
    for (name, info) in deps {
        let version = info.get("version").and_then(Value::as_str).map(str::to_string);
        let idx = entries.len();
        entries.push(Entry { name: name.clone(), version, parent: Some(parent) });
        if let Some(child_deps) = info.get("dependencies").and_then(Value::as_object) {
            walk(child_deps, idx, entries);
        }
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
    size_of(0, &children, &mut sizes);
    sizes
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "name": "spex-viewer",
        "version": "0.1.0",
        "dependencies": {
            "three": { "version": "0.185.1" },
            "typescript": {
                "version": "7.0.2",
                "dependencies": {
                    "@typescript/typescript-darwin-arm64": { "version": "7.0.2" }
                }
            }
        }
    }"#;

    fn parse_sample() -> Vec<Entry> {
        let root: Value = serde_json::from_str(SAMPLE).unwrap();
        let mut entries = vec![Entry {
            name: root["name"].as_str().unwrap().to_string(),
            version: root["version"].as_str().map(str::to_string),
            parent: None,
        }];
        walk(root["dependencies"].as_object().unwrap(), 0, &mut entries);
        entries
    }

    #[test]
    fn walks_real_npm_ls_json_shape_into_a_flat_parented_list() {
        let entries = parse_sample();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].name, "spex-viewer");
        assert_eq!(entries[0].parent, None);

        let three_idx = entries.iter().position(|e| e.name == "three").unwrap();
        assert_eq!(entries[three_idx].parent, Some(0));
        assert_eq!(entries[three_idx].version.as_deref(), Some("0.185.1"));

        let ts_idx = entries.iter().position(|e| e.name == "typescript").unwrap();
        assert_eq!(entries[ts_idx].parent, Some(0));

        let nested_idx = entries.iter().position(|e| e.name == "@typescript/typescript-darwin-arm64").unwrap();
        assert_eq!(entries[nested_idx].parent, Some(ts_idx));
    }

    #[test]
    fn subtree_sizes_reflect_nesting() {
        let entries = parse_sample();
        let sizes = compute_subtree_sizes(&entries);
        assert_eq!(sizes[0], 4.0); // root: whole tree

        let three_idx = entries.iter().position(|e| e.name == "three").unwrap();
        assert_eq!(sizes[three_idx], 1.0); // leaf

        let ts_idx = entries.iter().position(|e| e.name == "typescript").unwrap();
        assert_eq!(sizes[ts_idx], 2.0); // typescript + its one nested dep
    }

    // "inflight" here is real-world-shaped: it's a common transitive dep
    // (e.g. of glob) that `npm ls --json --all` legitimately re-expands at
    // every position it's required from, same as issue #24's brew-deps
    // `openssl@3` case. `debug` appears twice at two *different* resolved
    // versions — a real npm peer-dep/semver-range split — and must stay two
    // distinct nodes, not get merged just because the name matches.
    const DUPLICATE_SAMPLE: &str = r#"{
        "name": "spex-viewer",
        "version": "0.1.0",
        "dependencies": {
            "glob": {
                "version": "7.2.3",
                "dependencies": {
                    "inflight": { "version": "1.0.6" }
                }
            },
            "rimraf": {
                "version": "3.0.2",
                "dependencies": {
                    "inflight": { "version": "1.0.6" }
                }
            },
            "eslint": {
                "version": "8.57.0",
                "dependencies": {
                    "debug": { "version": "4.3.4" }
                }
            },
            "chokidar": {
                "version": "3.6.0",
                "dependencies": {
                    "debug": { "version": "3.2.7" }
                }
            }
        }
    }"#;

    fn parse_duplicate_sample() -> Vec<Entry> {
        let root: Value = serde_json::from_str(DUPLICATE_SAMPLE).unwrap();
        let mut entries = vec![Entry {
            name: root["name"].as_str().unwrap().to_string(),
            version: root["version"].as_str().map(str::to_string),
            parent: None,
        }];
        walk(root["dependencies"].as_object().unwrap(), 0, &mut entries);
        entries
    }

    #[test]
    fn build_nodes_merges_same_name_and_version_into_one_node_with_extra_parents() {
        let entries = parse_duplicate_sample();
        assert_eq!(entries.len(), 9); // root + glob + rimraf + eslint + chokidar + inflight*2 + debug*2

        let subtree_size = compute_subtree_sizes(&entries);
        let nodes = build_nodes(&entries, &subtree_size);

        // 9 real entries, one real duplicate ("inflight@1.0.6" appears
        // twice) merged away.
        assert_eq!(nodes.len(), 8);

        let inflight_nodes: Vec<&GraphNode> = nodes.iter().filter(|n| n.label == "inflight").collect();
        assert_eq!(inflight_nodes.len(), 1, "inflight@1.0.6 must appear as exactly one node, not two duplicates");
        let inflight = inflight_nodes[0];

        let glob = nodes.iter().find(|n| n.label == "glob").unwrap();
        let rimraf = nodes.iter().find(|n| n.label == "rimraf").unwrap();

        // Its one real 3D position is still driven by its first real
        // occurrence's parent (glob, the first dependency walked).
        assert_eq!(inflight.parent.as_deref(), Some(glob.id.as_str()));
        // The second real occurrence's parent (rimraf) becomes a real extra
        // structural edge instead of a second node.
        assert_eq!(inflight.extra_parents, vec![rimraf.id.clone()]);
    }

    #[test]
    fn build_nodes_keeps_same_name_different_version_as_distinct_nodes() {
        let entries = parse_duplicate_sample();
        let subtree_size = compute_subtree_sizes(&entries);
        let nodes = build_nodes(&entries, &subtree_size);

        // "debug" appears at two different resolved versions (4.3.4 under
        // eslint, 3.2.7 under chokidar) — a real npm version split, not a
        // duplicate, so both must remain their own real node.
        let debug_nodes: Vec<&GraphNode> = nodes.iter().filter(|n| n.label == "debug").collect();
        assert_eq!(debug_nodes.len(), 2, "debug@4.3.4 and debug@3.2.7 are distinct packages and must not be merged");
        for n in &debug_nodes {
            assert!(n.extra_parents.is_empty());
        }
    }
}
