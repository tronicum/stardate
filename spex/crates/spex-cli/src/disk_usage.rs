use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use spex_graph::{Graph, GraphNode};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Runs `du -a -k <path>` and converts the real filesystem tree into a
/// `spex_graph::Graph` — parent/child from path nesting, size in KB driving
/// color. Real disk usage on this machine; the fan-out cap in
/// `spex_graph::layout` keeps directories with huge file counts (e.g. a
/// build/`node_modules` dir) legible without any special handling here.
pub fn run(path: &Path) -> Result<Graph> {
    let path_str = path.to_str().context("path is not valid UTF-8")?;
    let output = Command::new("du")
        .args(["-a", "-k", path_str])
        .output()
        .context("running `du` (is it on PATH?)")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let entries = parse_du(&stdout);
    if entries.is_empty() {
        bail!(
            "`du` produced no output for {} (stderr: {})",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut graph = build_graph(entries);
    graph.title = Some(format!("disk usage: {}", path.display()));
    graph.metric_label = Some("size (KB)".to_string());
    Ok(graph)
}

/// Parses `du -a -k` output: `<size_kb>\t<path>` per line. Splits on the
/// first tab (not generic whitespace) so paths containing spaces survive intact.
fn parse_du(output: &str) -> Vec<(f64, String)> {
    output
        .lines()
        .filter_map(|line| {
            let (size_str, path) = line.split_once('\t')?;
            let kb: f64 = size_str.trim().parse().ok()?;
            Some((kb, path.to_string()))
        })
        .collect()
}

fn build_graph(entries: Vec<(f64, String)>) -> Graph {
    let paths: HashSet<&str> = entries.iter().map(|(_, p)| p.as_str()).collect();

    // A path is a directory (for display only) if some other captured path's parent is it.
    let mut has_children: HashSet<&str> = HashSet::new();
    for (_, p) in &entries {
        if let Some(parent) = Path::new(p).parent().and_then(|pp| pp.to_str()) {
            if paths.contains(parent) {
                has_children.insert(parent);
            }
        }
    }

    let nodes = entries
        .iter()
        .map(|(kb, p)| {
            let label = Path::new(p).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.clone());
            let parent = Path::new(p)
                .parent()
                .and_then(|pp| pp.to_str())
                .filter(|pp| paths.contains(pp))
                .map(|s| s.to_string());

            let mut metadata = Map::new();
            metadata.insert("path".to_string(), Value::from(p.clone()));
            metadata.insert("sizeKb".to_string(), Value::from(*kb));
            metadata.insert("isDir".to_string(), Value::from(has_children.contains(p.as_str())));

            GraphNode {
                id: p.clone(),
                label,
                parent,
                metric: Some(*kb),
                metadata,
            }
        })
        .collect();

    Graph { nodes, ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "4\tdemo/a.txt\n8\tdemo/sub/b.txt\n8\tdemo/sub\n12\tdemo\n";

    #[test]
    fn parses_size_and_path_split_on_tab() {
        let entries = parse_du(SAMPLE);
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0], (4.0, "demo/a.txt".to_string()));
        assert_eq!(entries[3], (12.0, "demo".to_string()));
    }

    #[test]
    fn builds_tree_with_root_and_dir_flags() {
        let graph = build_graph(parse_du(SAMPLE));

        let root = graph.nodes.iter().find(|n| n.id == "demo").unwrap();
        assert_eq!(root.parent, None); // "demo"'s real parent isn't in the captured set
        assert_eq!(root.metadata.get("isDir").and_then(Value::as_bool), Some(true));

        let file = graph.nodes.iter().find(|n| n.id == "demo/a.txt").unwrap();
        assert_eq!(file.parent.as_deref(), Some("demo"));
        assert_eq!(file.label, "a.txt");
        assert_eq!(file.metadata.get("isDir").and_then(Value::as_bool), Some(false));

        let sub = graph.nodes.iter().find(|n| n.id == "demo/sub").unwrap();
        assert_eq!(sub.parent.as_deref(), Some("demo"));
        let nested = graph.nodes.iter().find(|n| n.id == "demo/sub/b.txt").unwrap();
        assert_eq!(nested.parent.as_deref(), Some("demo/sub"));
    }

    #[test]
    fn handles_paths_with_spaces() {
        let entries = parse_du("16\tmy dir/file with spaces.txt\n");
        assert_eq!(entries[0].1, "my dir/file with spaces.txt");
    }
}
