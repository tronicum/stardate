use anyhow::Result;
use spex_graph::Graph;
use std::collections::HashMap;
use std::path::Path;

/// Smallest real slice of "diff/temporal mode": compares two captured
/// `graph.json` snapshots of the same kind of tree (e.g. two `ps-tree`
/// captures a few seconds apart, or `disk-usage` before/after a build) by
/// node id — which nodes appeared, disappeared, or changed metric. No new
/// data model needed; this is pure comparison over the existing `Graph`.
pub fn run(old_path: &Path, new_path: &Path) -> Result<String> {
    let old = Graph::read_json(old_path)?;
    let new = Graph::read_json(new_path)?;
    Ok(format_diff(&old, &new))
}

pub fn format_diff(old: &Graph, new: &Graph) -> String {
    let old_by_id: HashMap<&str, &spex_graph::GraphNode> = old.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let new_by_id: HashMap<&str, &spex_graph::GraphNode> = new.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let mut added: Vec<&str> = new_by_id.keys().filter(|id| !old_by_id.contains_key(*id)).copied().collect();
    let mut removed: Vec<&str> = old_by_id.keys().filter(|id| !new_by_id.contains_key(*id)).copied().collect();
    let mut changed: Vec<(&str, Option<f64>, Option<f64>)> = old_by_id
        .iter()
        .filter_map(|(id, on)| new_by_id.get(id).filter(|nn| nn.metric != on.metric).map(|nn| (*id, on.metric, nn.metric)))
        .collect();
    added.sort_unstable();
    removed.sort_unstable();
    changed.sort_by_key(|(id, ..)| *id);

    let mut out = String::new();
    out.push_str(&format!(
        "diff: {} -> {}\n",
        old.title.as_deref().unwrap_or("(untitled)"),
        new.title.as_deref().unwrap_or("(untitled)")
    ));
    out.push_str(&format!("{} added, {} removed, {} changed\n\n", added.len(), removed.len(), changed.len()));
    for id in &added {
        out.push_str(&format!("+ {} ({id})\n", new_by_id[id].label));
    }
    for id in &removed {
        out.push_str(&format!("- {} ({id})\n", old_by_id[id].label));
    }
    for (id, old_m, new_m) in &changed {
        let delta = match (old_m, new_m) {
            (Some(o), Some(n)) => n - o,
            _ => 0.0,
        };
        let sign = if delta >= 0.0 { "+" } else { "" };
        out.push_str(&format!("~ {} ({id}): {old_m:?} -> {new_m:?} ({sign}{delta:.2})\n", new_by_id[id].label));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;
    use spex_graph::GraphNode;

    fn node(id: &str, label: &str, metric: Option<f64>) -> GraphNode {
        GraphNode { id: id.to_string(), label: label.to_string(), parent: None, metric, metadata: Map::new() }
    }

    #[test]
    fn detects_added_removed_and_changed_nodes() {
        let old = Graph {
            title: Some("before".to_string()),
            metric_label: None,
            nodes: vec![node("a", "Alice", Some(1.0)), node("b", "Bob", Some(2.0))],
        };
        let new = Graph {
            title: Some("after".to_string()),
            metric_label: None,
            nodes: vec![node("a", "Alice", Some(1.0)), node("b", "Bob", Some(5.0)), node("c", "Carol", Some(3.0))],
        };

        let diff = format_diff(&old, &new);
        assert!(diff.contains("1 added, 0 removed, 1 changed"), "{diff}");
        assert!(diff.contains("+ Carol (c)"), "{diff}");
        assert!(diff.contains("~ Bob (b): Some(2.0) -> Some(5.0) (+3.00)"), "{diff}");
        assert!(!diff.contains("Alice"), "unchanged node shouldn't be listed: {diff}");
    }

    #[test]
    fn detects_removed_nodes() {
        let old = Graph { title: None, metric_label: None, nodes: vec![node("a", "Alice", None), node("b", "Bob", None)] };
        let new = Graph { title: None, metric_label: None, nodes: vec![node("a", "Alice", None)] };

        let diff = format_diff(&old, &new);
        assert!(diff.contains("0 added, 1 removed, 0 changed"), "{diff}");
        assert!(diff.contains("- Bob (b)"), "{diff}");
    }
}
