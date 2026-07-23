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

    let nodes = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let mut metadata = Map::new();
            metadata.insert("name".to_string(), Value::from(e.name.clone()));
            if let Some(version) = &e.version {
                metadata.insert("version".to_string(), Value::from(version.clone()));
            }
            GraphNode {
                id: format!("pkg-{i}"),
                label: e.name.clone(),
                parent: e.parent.map(|p| format!("pkg-{p}")),
                metric: Some(subtree_size[i]),
                metadata,
            }
        })
        .collect();

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
}
