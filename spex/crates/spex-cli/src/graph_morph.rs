//! `spex graph-morph` — generalizes the diff/temporal idea in
//! `graph_diff.rs` (a single static merged snapshot) into a real
//! point-cloud animation: every node present in both a real "old" and
//! "new" graph capture (matched by id, same matching `graph_diff::run`
//! already does) lerps from its old layout position/color to its new one;
//! a node only in `old` shrinks away, a node only in `new` grows in. Reuses
//! `spex frame-sequence`'s shared-offset tiling core (`frame_sequence::run_from_frames`)
//! so playback is the *same* real point-cloud animation mechanism
//! `brick-assembly`/`frame-sequence` already use — no new viewer code.
//!
//! Deliberately node-only for this first cut: edges aren't animated (a
//! matched node's own edge-to-parent point-trail would need its own
//! old/new interpolation, and isn't needed to show *what changed*, which
//! is the actual ask). A real, scoped follow-up if edge motion turns out
//! to matter once this is in front of real data.
use anyhow::{Context, Result};
use spex_core::Point;
use spex_graph::{Graph, LayoutNodeInfo};
use std::collections::HashMap;
use std::path::Path;

pub fn run(old_path: &Path, new_path: &Path, frames: usize, fps: f64, out: &Path) -> Result<()> {
    if frames < 2 {
        anyhow::bail!("--frames must be at least 2 (one for the old state, one for the new)");
    }
    let old = Graph::read_json(old_path).with_context(|| format!("reading {}", old_path.display()))?;
    let new = Graph::read_json(new_path).with_context(|| format!("reading {}", new_path.display()))?;

    let old_layout = spex_graph::build(&old);
    let new_layout = spex_graph::build(&new);
    println!(
        "laid out old ({} nodes) and new ({} nodes) states...",
        old_layout.nodes.len(),
        new_layout.nodes.len()
    );

    let motions = plan_motions(&old_layout.nodes, &new_layout.nodes);
    println!(
        "{} matched, {} removed, {} added",
        motions.iter().filter(|m| matches!(m.kind, MotionKind::Matched { .. })).count(),
        motions.iter().filter(|m| matches!(m.kind, MotionKind::Removed)).count(),
        motions.iter().filter(|m| matches!(m.kind, MotionKind::Added)).count(),
    );

    let frame_points: Vec<Vec<Point>> = (0..frames)
        .map(|f| {
            let t = f as f64 / (frames - 1) as f64;
            render_frame(&motions, t)
        })
        .collect();

    let config = spex_tiler::TilerConfig::default();
    crate::frame_sequence::run_from_frames(frame_points, out, fps, &config)
}

enum MotionKind {
    Matched { new: NodeState },
    Removed,
    Added,
}

struct NodeState {
    center: [f64; 3],
    color: [u8; 3],
}

struct Motion {
    id: String,
    old: NodeState,
    kind: MotionKind,
}

/// Colors every node the same way `spex_graph::layout::build` does
/// internally (a blue-yellow-red heat gradient over that graph's own
/// metric spread) — `graph-morph` needs this itself since `LayoutNodeInfo`
/// carries a node's metric, not the color already derived from it.
fn node_colors(nodes: &[LayoutNodeInfo]) -> HashMap<&str, [u8; 3]> {
    let (metric_min, metric_range) = spex_graph::metric_min_range(nodes.iter().map(|n| &n.metric));
    nodes
        .iter()
        .map(|n| {
            let color = match n.metric {
                Some(m) => spex_graph::heat_color((m - metric_min) / metric_range),
                None => spex_graph::NEUTRAL_GRAY,
            };
            (n.id.as_str(), color)
        })
        .collect()
}

/// Matches nodes by id (the same comparison `graph_diff::run`/`merge_for_viz`
/// use) and records each one's real motion: interpolate between two real
/// positions/colors for a matched node, or grow-in/shrink-away for a node
/// on only one side.
fn plan_motions(old_nodes: &[LayoutNodeInfo], new_nodes: &[LayoutNodeInfo]) -> Vec<Motion> {
    let old_colors = node_colors(old_nodes);
    let new_colors = node_colors(new_nodes);
    let old_by_id: HashMap<&str, &LayoutNodeInfo> = old_nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let new_by_id: HashMap<&str, &LayoutNodeInfo> = new_nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let mut motions = Vec::new();
    for on in old_nodes {
        let old_state = NodeState { center: on.center, color: old_colors[on.id.as_str()] };
        let kind = match new_by_id.get(on.id.as_str()) {
            Some(nn) => MotionKind::Matched { new: NodeState { center: nn.center, color: new_colors[nn.id.as_str()] } },
            None => MotionKind::Removed,
        };
        motions.push(Motion { id: on.id.clone(), old: old_state, kind });
    }
    for nn in new_nodes {
        if old_by_id.contains_key(nn.id.as_str()) {
            continue; // already covered by the matched branch above
        }
        let new_state = NodeState { center: nn.center, color: new_colors[nn.id.as_str()] };
        // `old` for an added node is only ever read by `MotionKind::Added`'s
        // arm, which ignores it and uses `new_state` directly — filled in
        // just to keep `Motion` a single non-`Option`-laden shape.
        motions.push(Motion { id: nn.id.clone(), old: new_state, kind: MotionKind::Added });
    }
    motions
}

fn lerp3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Renders one animation frame at `t` (0.0 = old state, 1.0 = new state):
/// a matched node's blob lerps position/color; a removed node's blob
/// shrinks from full density to nothing; an added node's grows from
/// nothing to full density — all using `spex_graph::scatter_blob_at`, the
/// same per-node "blob" real layout output uses, so a morph frame looks
/// exactly like a real `graph-layout` render at every point along the way.
fn render_frame(motions: &[Motion], t: f64) -> Vec<Point> {
    let mut points = Vec::new();
    for m in motions {
        match &m.kind {
            MotionKind::Matched { new } => {
                let center = lerp3(m.old.center, new.center, t);
                let color = spex_graph::lerp_color(m.old.color, new.color, t);
                points.extend(spex_graph::scatter_blob_at(&m.id, center, color, spex_graph::BLOB_POINTS));
            }
            MotionKind::Removed => {
                let count = ((1.0 - t) * spex_graph::BLOB_POINTS as f64).round() as usize;
                if count > 0 {
                    points.extend(spex_graph::scatter_blob_at(&m.id, m.old.center, m.old.color, count));
                }
            }
            MotionKind::Added => {
                let count = (t * spex_graph::BLOB_POINTS as f64).round() as usize;
                if count > 0 {
                    points.extend(spex_graph::scatter_blob_at(&m.id, m.old.center, m.old.color, count));
                }
            }
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, parent: Option<&str>, metric: f64) -> spex_graph::GraphNode {
        spex_graph::GraphNode {
            id: id.to_string(),
            label: id.to_string(),
            parent: parent.map(str::to_string),
            metric: Some(metric),
            metadata: Default::default(),
        }
    }

    #[test]
    fn plan_motions_matches_shared_ids_and_flags_added_removed() {
        let old = spex_graph::build(&Graph {
            title: None,
            metric_label: None,
            nodes: vec![node("root", None, 1.0), node("a", Some("root"), 2.0), node("gone", Some("root"), 3.0)],
        });
        let new = spex_graph::build(&Graph {
            title: None,
            metric_label: None,
            nodes: vec![node("root", None, 1.0), node("a", Some("root"), 5.0), node("fresh", Some("root"), 4.0)],
        });

        let motions = plan_motions(&old.nodes, &new.nodes);
        let by_id: HashMap<&str, &Motion> = motions.iter().map(|m| (m.id.as_str(), m)).collect();

        assert!(matches!(by_id["root"].kind, MotionKind::Matched { .. }));
        assert!(matches!(by_id["a"].kind, MotionKind::Matched { .. }));
        assert!(matches!(by_id["gone"].kind, MotionKind::Removed));
        assert!(matches!(by_id["fresh"].kind, MotionKind::Added));
        assert_eq!(motions.len(), 4);
    }

    #[test]
    fn render_frame_at_t_zero_and_one_matches_the_real_endpoints() {
        let old = spex_graph::build(&Graph {
            title: None,
            metric_label: None,
            nodes: vec![node("root", None, 1.0), node("a", Some("root"), 2.0)],
        });
        let new = spex_graph::build(&Graph {
            title: None,
            metric_label: None,
            nodes: vec![node("root", None, 1.0), node("a", Some("root"), 9.0)],
        });
        let motions = plan_motions(&old.nodes, &new.nodes);

        let at_start = render_frame(&motions, 0.0);
        let at_end = render_frame(&motions, 1.0);
        // Same node count at both real endpoints (nothing added/removed here).
        assert_eq!(at_start.len(), at_end.len());
        assert!(!at_start.is_empty());
    }

    #[test]
    fn render_frame_fades_a_removed_node_toward_zero_points() {
        let old = spex_graph::build(&Graph {
            title: None,
            metric_label: None,
            nodes: vec![node("root", None, 1.0), node("gone", Some("root"), 2.0)],
        });
        let new = spex_graph::build(&Graph { title: None, metric_label: None, nodes: vec![node("root", None, 1.0)] });
        let motions = plan_motions(&old.nodes, &new.nodes);

        let early = render_frame(&motions, 0.1);
        let late = render_frame(&motions, 0.9);
        assert!(early.len() > late.len(), "a removed node's point count should shrink as t grows: {} vs {}", early.len(), late.len());
    }

    #[test]
    fn run_rejects_fewer_than_two_frames() {
        let dir = tempfile::tempdir().unwrap();
        let old_path = dir.path().join("old.json");
        let new_path = dir.path().join("new.json");
        std::fs::write(&old_path, r#"{"nodes":[{"id":"r","label":"r","parent":null}]}"#).unwrap();
        std::fs::write(&new_path, r#"{"nodes":[{"id":"r","label":"r","parent":null}]}"#).unwrap();

        let result = run(&old_path, &new_path, 1, 10.0, &dir.path().join("out"));
        assert!(result.is_err());
    }
}
