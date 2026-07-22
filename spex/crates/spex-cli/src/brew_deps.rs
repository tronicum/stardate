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

    let nodes = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let mut metadata = Map::new();
            metadata.insert("name".to_string(), Value::from(e.name.clone()));
            metadata.insert("depth".to_string(), Value::from(e.depth));
            GraphNode {
                id: format!("pkg-{i}"),
                label: e.name.clone(),
                parent: e.parent.map(|p| format!("pkg-{p}")),
                // Subtree size (including self): a rough "how much of the
                // dependency tree hangs off this package" weight, driving color.
                metric: Some(subtree_size[i]),
                metadata,
            }
        })
        .collect();
    Ok(Graph {
        title: Some(format!("brew dependency tree: {formula}")),
        metric_label: Some("subtree size (packages)".to_string()),
        nodes,
    })
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
