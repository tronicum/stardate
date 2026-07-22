use crate::layout::{heat_color, metric_min_range};
use crate::{Graph, GraphNode};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::IsTerminal;

/// Renders a `Graph` as a human-readable ASCII tree (the terminal-view
/// counterpart to `Graph::write_json`'s machine-readable JSON): a header
/// naming what this is, the tree itself (each line colored by its node's
/// metric, using the same gradient as the browser view), and a footer
/// calling out the metric's range and its hottest node — so a demo is
/// understandable from the terminal alone, no browser required.
pub fn format_tree(graph: &Graph) -> String {
    let mut out = String::new();

    match &graph.title {
        Some(title) => out.push_str(&format!("{title}  ({} nodes)\n\n", graph.nodes.len())),
        None => out.push_str(&format!("{} nodes\n\n", graph.nodes.len())),
    }

    let use_color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let (metric_min, metric_range) = metric_min_range(graph.nodes.iter().map(|n| &n.metric));

    let mut children: HashMap<Option<&str>, Vec<&GraphNode>> = HashMap::new();
    for n in &graph.nodes {
        children.entry(n.parent.as_deref()).or_default().push(n);
    }

    let no_parent: Option<&str> = None;
    let roots = children.get(&no_parent).cloned().unwrap_or_default();
    let count = roots.len();
    for (i, root) in roots.into_iter().enumerate() {
        write_node(root, "", i == count - 1, &children, use_color, metric_min, metric_range, &mut out);
    }

    if let Some(footer) = format_footer(graph) {
        out.push('\n');
        out.push_str(&footer);
        out.push('\n');
    }

    out
}

#[allow(clippy::too_many_arguments)]
fn write_node(
    node: &GraphNode,
    prefix: &str,
    is_last: bool,
    children: &HashMap<Option<&str>, Vec<&GraphNode>>,
    use_color: bool,
    metric_min: f64,
    metric_range: f64,
    out: &mut String,
) {
    let connector = if is_last { "└── " } else { "├── " };
    out.push_str(prefix);
    out.push_str(connector);

    let content = node_content(node);
    match (use_color, node.metric) {
        (true, Some(m)) => {
            let [r, g, b] = heat_color((m - metric_min) / metric_range);
            out.push_str(&format!("\x1b[38;2;{r};{g};{b}m{content}\x1b[0m"));
        }
        _ => out.push_str(&content),
    }
    out.push('\n');

    let child_prefix = format!("{prefix}{}", if is_last { "    " } else { "│   " });
    let kids = children.get(&Some(node.id.as_str())).cloned().unwrap_or_default();
    let count = kids.len();
    for (i, kid) in kids.into_iter().enumerate() {
        write_node(kid, &child_prefix, i == count - 1, children, use_color, metric_min, metric_range, out);
    }
}

fn node_content(node: &GraphNode) -> String {
    let mut content = node.label.clone();
    if let Some(m) = node.metric {
        content.push_str(&format!("  [{m:.2}]"));
    }
    if !node.metadata.is_empty() {
        let fields: Vec<String> = node.metadata.iter().map(|(k, v)| format!("{k}={}", compact_value(v))).collect();
        content.push_str(&format!("  ({})", fields.join(", ")));
    }
    content
}

fn format_footer(graph: &Graph) -> Option<String> {
    let label = graph.metric_label.as_deref()?;
    let mut with_metric: Vec<(&GraphNode, f64)> = graph.nodes.iter().filter_map(|n| n.metric.map(|m| (n, m))).collect();
    if with_metric.is_empty() {
        return None;
    }
    with_metric.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
    let (min_node, min_val) = with_metric.first().copied().unwrap();
    let (max_node, max_val) = with_metric.last().copied().unwrap();

    if min_node.id == max_node.id {
        return Some(format!("{label}: {max_val:.2} ({})", max_node.label));
    }
    Some(format!(
        "{label}: {min_val:.2}\u{2013}{max_val:.2}  ·  highest: {} ({max_val:.2})",
        max_node.label
    ))
}

fn compact_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(items) if items.len() > 3 => format!("[{} items]", items.len()),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GraphNode;
    use serde_json::Map;

    #[test]
    fn formats_chain_with_header_metadata_and_footer() {
        let mut meta = Map::new();
        meta.insert("hop".to_string(), Value::from(1));
        let graph = Graph {
            title: Some("test trace".to_string()),
            metric_label: Some("avg RTT (ms)".to_string()),
            nodes: vec![
                GraphNode {
                    id: "root".to_string(),
                    label: "you".to_string(),
                    parent: None,
                    metric: None,
                    metadata: Map::new(),
                },
                GraphNode {
                    id: "a".to_string(),
                    label: "fritz.box".to_string(),
                    parent: Some("root".to_string()),
                    metric: Some(7.167),
                    metadata: meta,
                },
            ],
        };
        let text = format_tree(&graph);
        // No color codes expected here: cargo test's stdout isn't a terminal.
        assert_eq!(
            text,
            "test trace  (2 nodes)\n\n└── you\n    └── fritz.box  [7.17]  (hop=1)\n\navg RTT (ms): 7.17 (fritz.box)\n"
        );
    }

    #[test]
    fn formats_branching_tree_without_title_or_metric() {
        let graph = Graph {
            nodes: vec![
                GraphNode { id: "r".to_string(), label: "root".to_string(), parent: None, metric: None, metadata: Map::new() },
                GraphNode { id: "a".to_string(), label: "a".to_string(), parent: Some("r".to_string()), metric: None, metadata: Map::new() },
                GraphNode { id: "b".to_string(), label: "b".to_string(), parent: Some("r".to_string()), metric: None, metadata: Map::new() },
            ],
            ..Default::default()
        };
        let text = format_tree(&graph);
        assert_eq!(text, "3 nodes\n\n└── root\n    ├── a\n    └── b\n");
    }

    #[test]
    fn footer_shows_range_and_hottest_node() {
        let graph = Graph {
            metric_label: Some("size (KB)".to_string()),
            nodes: vec![
                GraphNode { id: "r".to_string(), label: "root".to_string(), parent: None, metric: Some(1.0), metadata: Map::new() },
                GraphNode { id: "big".to_string(), label: "big.bin".to_string(), parent: Some("r".to_string()), metric: Some(500.0), metadata: Map::new() },
                GraphNode { id: "small".to_string(), label: "small.txt".to_string(), parent: Some("r".to_string()), metric: Some(0.5), metadata: Map::new() },
            ],
            ..Default::default()
        };
        let footer_line = format_tree(&graph).lines().last().unwrap().to_string();
        assert_eq!(footer_line, "size (KB): 0.50\u{2013}500.00  ·  highest: big.bin (500.00)");
    }
}
