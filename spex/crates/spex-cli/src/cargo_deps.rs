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

    let nodes = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let mut metadata = Map::new();
            metadata.insert("name".to_string(), Value::from(e.name.clone()));
            if let Some(version) = &e.version {
                metadata.insert("version".to_string(), Value::from(version.clone()));
            }
            metadata.insert("depth".to_string(), Value::from(e.depth));
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
}
