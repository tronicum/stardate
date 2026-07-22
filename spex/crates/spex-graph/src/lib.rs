mod display;
mod layout;

pub use display::format_tree;
pub use layout::{build, build_points, LayoutNodeInfo, LayoutResult};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single node in a generic tree: the common intermediate format that any
/// input adapter (traceroute, pstree, dependency graphs, ...) can target, and
/// that any layout/output stage can consume without knowing the source domain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    /// `None` marks a root. A forest (multiple roots) is valid.
    pub parent: Option<String>,
    /// Generic numeric weight (e.g. RTT ms, subtree size) driving color/size in a layout.
    #[serde(default)]
    pub metric: Option<f64>,
    /// Free-form source-specific fields (ip, hostname, hop number, pid, ...).
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Graph {
    /// One-line human description of what this graph is, e.g. "traceroute to
    /// www.de-cix.net" — carried through to both the terminal view (header)
    /// and the browser view (persistent title), so a viewer never has to
    /// guess what they're looking at.
    #[serde(default)]
    pub title: Option<String>,
    /// What `metric` measures and its unit, e.g. "avg RTT (ms)" — without
    /// this, a bare number (or a color) means nothing to a human.
    #[serde(default)]
    pub metric_label: Option<String>,
    pub nodes: Vec<GraphNode>,
}

impl Graph {
    pub fn write_json(&self, path: &Path) -> Result<()> {
        let f = std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
        serde_json::to_writer_pretty(f, self)?;
        Ok(())
    }

    pub fn read_json(path: &Path) -> Result<Graph> {
        let data = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let graph: Graph = serde_json::from_str(&data).with_context(|| format!("parsing {}", path.display()))?;
        Ok(graph)
    }
}
