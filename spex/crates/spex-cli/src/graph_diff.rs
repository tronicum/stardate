use anyhow::Result;
use serde_json::Value;
use spex_graph::{Graph, GraphNode};
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

/// Builds a single renderable `Graph` out of two captures, tagging every
/// node with a `diff_status` metadata field (`"added"`/`"removed"`/
/// `"changed"`/`"unchanged"`) that `spex-graph::layout` colors distinctly
/// (green/red/orange/dim-blue) instead of the usual metric heat gradient —
/// the viewer half of the diff/temporal idea `format_diff` (terminal-only)
/// couldn't cover on its own. Kept nodes use `new`'s position-driving data
/// (parent/label/metric); a removed node keeps `old`'s data so it still
/// renders at roughly its old place in the tree — if its old parent no
/// longer exists in the merged set it simply won't be reachable from any
/// root and the layout silently skips it, the same fallback the layout
/// already applies to any node whose parent is missing.
pub fn merge_for_viz(old: &Graph, new: &Graph) -> Graph {
    let old_by_id: HashMap<&str, &GraphNode> = old.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let new_by_id: HashMap<&str, &GraphNode> = new.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let mut nodes: Vec<GraphNode> = Vec::new();
    for n in &new.nodes {
        let mut metadata = n.metadata.clone();
        match old_by_id.get(n.id.as_str()) {
            None => {
                metadata.insert("diff_status".to_string(), Value::from("added"));
            }
            Some(old_n) if old_n.metric != n.metric => {
                metadata.insert("diff_status".to_string(), Value::from("changed"));
                metadata.insert("old_metric".to_string(), match old_n.metric {
                    Some(m) => Value::from(m),
                    None => Value::Null,
                });
            }
            Some(_) => {
                metadata.insert("diff_status".to_string(), Value::from("unchanged"));
            }
        }
        nodes.push(GraphNode { id: n.id.clone(), label: n.label.clone(), parent: n.parent.clone(), metric: n.metric, metadata });
    }
    for n in &old.nodes {
        if new_by_id.contains_key(n.id.as_str()) {
            continue;
        }
        let mut metadata = n.metadata.clone();
        metadata.insert("diff_status".to_string(), Value::from("removed"));
        nodes.push(GraphNode { id: n.id.clone(), label: n.label.clone(), parent: n.parent.clone(), metric: n.metric, metadata });
    }

    Graph {
        title: Some(format!(
            "diff: {} -> {}",
            old.title.as_deref().unwrap_or("(untitled)"),
            new.title.as_deref().unwrap_or("(untitled)")
        )),
        metric_label: new.metric_label.clone(),
        nodes,
    }
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

    #[test]
    fn merge_for_viz_tags_every_node_with_its_diff_status() {
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

        let merged = merge_for_viz(&old, &new);
        // Alice (unchanged), Bob (changed, kept from `new`), Carol (added) — no removed nodes here.
        assert_eq!(merged.nodes.len(), 3);
        let by_id: HashMap<&str, &GraphNode> = merged.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        assert_eq!(by_id["a"].metadata["diff_status"], Value::from("unchanged"));
        assert_eq!(by_id["b"].metadata["diff_status"], Value::from("changed"));
        assert_eq!(by_id["b"].metadata["old_metric"], Value::from(2.0));
        assert_eq!(by_id["b"].metric, Some(5.0)); // kept from `new`, not `old`
        assert_eq!(by_id["c"].metadata["diff_status"], Value::from("added"));
    }

    #[test]
    fn merge_for_viz_keeps_removed_nodes_with_their_old_data() {
        let old = Graph { title: None, metric_label: None, nodes: vec![node("a", "Alice", None), node("b", "Bob", Some(7.0))] };
        let new = Graph { title: None, metric_label: None, nodes: vec![node("a", "Alice", None)] };

        let merged = merge_for_viz(&old, &new);
        assert_eq!(merged.nodes.len(), 2);
        let removed = merged.nodes.iter().find(|n| n.id == "b").unwrap();
        assert_eq!(removed.metadata["diff_status"], Value::from("removed"));
        assert_eq!(removed.metric, Some(7.0));
    }
}
