//! Graph-aware "packet path" support for `spex ascii --animate`
//! (crates/spex-cli/src/ascii.rs). Given a tileset directory's optional
//! `nodes.json` (written only by the graph pipeline — `spex graph-layout`;
//! absent for a plain point-cloud tileset from `spex convert`), computes the
//! same tree-sweep the browser viewer animates via its "Animate packet"
//! checkbox, so a terminal/HTML ASCII animation can show an equivalent
//! marker + hop readout with no browser involved.
//!
//! `build_full_sweep_path` is a direct port of
//! `viewer/src/packetAnimation.ts`'s `buildFullSweepPath`: a full
//! depth-first sweep of the tree, heaviest subtree first (by `metric`),
//! backtracking to the branch point after each child so the *whole* tree is
//! covered rather than just one branch. (The viewer's older
//! `buildPrimaryPath` — a simpler always-first-child walk — was replaced by
//! this in the viewer itself; see that file's `git log`. Porting the
//! current algorithm keeps this feature in lock-step with what a browser
//! actually shows today, rather than reproducing a since-superseded one.)
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::Path;

/// One entry of a tileset's `nodes.json` — see `spec/nodes.schema.json`.
/// Deserialize-only mirror of `spex_graph::layout::LayoutNodeInfo` (that
/// type only derives `Serialize`; it's the writer, this is a separate
/// reader living in spex-cli rather than a cross-crate dependency change).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NodeLabel {
    pub id: String,
    pub label: String,
    pub parent: Option<String>,
    pub center: [f64; 3],
    pub metric: Option<f64>,
    #[serde(default)]
    pub metadata: Map<String, Value>,
}

/// Loads `<tileset_dir>/nodes.json` if present. `Ok(None)` means "plain
/// point-cloud tileset, no graph data" — the caller's cue to fall back to
/// today's raw-points-only animation — kept distinct from a real read/parse
/// error (a `nodes.json` that exists but is malformed should surface, not
/// silently degrade).
pub fn load_nodes(tileset_dir: &Path) -> Result<Option<Vec<NodeLabel>>> {
    let path = tileset_dir.join("nodes.json");
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let nodes: Vec<NodeLabel> = serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))?;
    Ok(Some(nodes))
}

/// Full depth-first sweep of the tree from its root — see module docs.
/// Returns `[]` if there's no root or fewer than two nodes (mirrors
/// `buildFullSweepPath`'s early-return; nothing meaningful to animate).
pub fn build_full_sweep_path(nodes: &[NodeLabel]) -> Vec<NodeLabel> {
    if nodes.len() < 2 {
        return Vec::new();
    }

    let mut children_of: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, n) in nodes.iter().enumerate() {
        if let Some(parent) = &n.parent {
            children_of.entry(parent.as_str()).or_default().push(i);
        }
    }
    for siblings in children_of.values_mut() {
        // Heaviest subtree first, same as the viewer's `(b.metric ?? -Infinity)
        // - (a.metric ?? -Infinity)` comparator. `sort_by` (not
        // `sort_unstable_by`) so ties keep their original relative order,
        // matching JS's stable `Array.sort`.
        siblings.sort_by(|&a, &b| {
            let ma = nodes[a].metric.unwrap_or(f64::NEG_INFINITY);
            let mb = nodes[b].metric.unwrap_or(f64::NEG_INFINITY);
            mb.partial_cmp(&ma).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    let Some(root) = nodes.iter().position(|n| n.parent.is_none()) else {
        return Vec::new();
    };

    let mut path_indices = Vec::new();
    visit(root, nodes, &children_of, &mut path_indices);

    if path_indices.len() >= 2 {
        path_indices.into_iter().map(|i| nodes[i].clone()).collect()
    } else {
        Vec::new()
    }
}

fn visit(idx: usize, nodes: &[NodeLabel], children_of: &HashMap<&str, Vec<usize>>, path: &mut Vec<usize>) {
    path.push(idx);
    let Some(children) = children_of.get(nodes[idx].id.as_str()) else {
        return;
    };
    for (i, &child) in children.iter().enumerate() {
        visit(child, nodes, children_of, path);
        if i < children.len() - 1 {
            path.push(idx); // walk back to the branch point before the next sibling
        }
    }
}

/// The packet's interpolated state for one animation frame.
#[derive(Debug, Clone, PartialEq)]
pub struct PacketState {
    pub position: [f64; 3],
    /// 1-based index of the current hop (path segment) out of `hop_count`.
    pub hop_index: usize,
    pub hop_count: usize,
    pub from_label: String,
    pub to_label: String,
}

/// Computes one `PacketState` per animation frame, spreading exactly one
/// full traversal of `path` across `frame_count` frames — paired with
/// `ascii.rs`'s turntable orbit the same way: `frame_count` frames is one
/// full 360-degree camera loop, so this makes it one full packet sweep too,
/// completing in lockstep with the camera rather than running on an
/// unrelated clock. Within that per-frame budget, travel time per hop is
/// scaled by the hop's real 3D distance (mirroring the viewer's
/// `updatePacket`'s distance/`packetSpeed` timing in `main.ts`) so a long
/// edge visually takes proportionally longer to cross than a short one.
/// Returns `[]` if `path` has fewer than two nodes or `frame_count` is 0.
pub fn packet_states(path: &[NodeLabel], frame_count: usize) -> Vec<PacketState> {
    if path.len() < 2 || frame_count == 0 {
        return Vec::new();
    }

    // `.max(1e-3)` mirrors the viewer's `|| 0.001` guard against a
    // zero-length hop (two nodes at the same position) stalling the sweep.
    let segment_lengths: Vec<f64> = path.windows(2).map(|w| distance(w[0].center, w[1].center).max(1e-3)).collect();
    let total: f64 = segment_lengths.iter().sum();
    let mut cumulative = vec![0.0; segment_lengths.len() + 1];
    for (i, len) in segment_lengths.iter().enumerate() {
        cumulative[i + 1] = cumulative[i] + len;
    }

    (0..frame_count)
        .map(|f| {
            let phase = f as f64 / frame_count as f64;
            let traveled = phase * total;
            // Last segment whose start is <= traveled. A linear scan is
            // fine here: even a large real demo's tree sweep is at most a
            // few thousand hops, walked once per frame.
            let mut seg = 0;
            while seg + 1 < segment_lengths.len() && cumulative[seg + 1] <= traveled {
                seg += 1;
            }
            let t = ((traveled - cumulative[seg]) / segment_lengths[seg]).clamp(0.0, 1.0);
            let a = &path[seg];
            let b = &path[seg + 1];
            PacketState {
                position: lerp(a.center, b.center, t),
                hop_index: seg + 1,
                hop_count: segment_lengths.len(),
                from_label: a.label.clone(),
                to_label: b.label.clone(),
            }
        })
        .collect()
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

fn lerp(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small hand-built tree, shaped like the kind of data
    /// `spex graph-layout` would write to `nodes.json`:
    /// ```text
    ///        root (metric 0)
    ///        /          \
    ///   heavy(5)      light(1)
    ///       |
    ///     leaf(2)
    /// ```
    /// `heavy` outweighs `light`, so the sweep visits `heavy`'s subtree
    /// (down to `leaf`) before backtracking through `root` to `light`.
    fn sample_tree() -> Vec<NodeLabel> {
        fn node(id: &str, parent: Option<&str>, metric: f64) -> NodeLabel {
            NodeLabel {
                id: id.to_string(),
                label: format!("{id}-label"),
                parent: parent.map(str::to_string),
                center: match id {
                    "root" => [0.0, 0.0, 0.0],
                    "heavy" => [10.0, 0.0, 0.0],
                    "leaf" => [10.0, 10.0, 0.0],
                    "light" => [0.0, 10.0, 0.0],
                    _ => unreachable!(),
                },
                metric: Some(metric),
                metadata: Map::new(),
            }
        }
        vec![node("root", None, 0.0), node("heavy", Some("root"), 5.0), node("light", Some("root"), 1.0), node("leaf", Some("heavy"), 2.0)]
    }

    #[test]
    fn full_sweep_visits_the_heaviest_branch_first_then_backtracks() {
        let path = build_full_sweep_path(&sample_tree());
        let ids: Vec<&str> = path.iter().map(|n| n.id.as_str()).collect();
        // root -> heavy -> leaf -> (back to) root -> light
        assert_eq!(ids, vec!["root", "heavy", "leaf", "root", "light"]);
    }

    #[test]
    fn full_sweep_of_fewer_than_two_nodes_is_empty() {
        assert_eq!(build_full_sweep_path(&[]), Vec::new());
        let one = vec![NodeLabel { id: "solo".into(), label: "solo".into(), parent: None, center: [0.0; 3], metric: None, metadata: Map::new() }];
        assert_eq!(build_full_sweep_path(&one), Vec::new());
    }

    #[test]
    fn full_sweep_with_no_root_is_empty() {
        // Every node has a parent — a malformed/cyclic input, not a real
        // `spex graph-layout` output, but shouldn't panic.
        let nodes = vec![
            NodeLabel { id: "a".into(), label: "a".into(), parent: Some("b".into()), center: [0.0; 3], metric: None, metadata: Map::new() },
            NodeLabel { id: "b".into(), label: "b".into(), parent: Some("a".into()), center: [1.0, 0.0, 0.0], metric: None, metadata: Map::new() },
        ];
        assert_eq!(build_full_sweep_path(&nodes), Vec::new());
    }

    #[test]
    fn packet_states_starts_at_the_root_on_frame_zero() {
        let path = build_full_sweep_path(&sample_tree());
        let states = packet_states(&path, 12);
        assert_eq!(states.len(), 12);
        assert_eq!(states[0].position, path[0].center);
        assert_eq!(states[0].hop_index, 1);
        assert_eq!(states[0].hop_count, path.len() - 1);
    }

    #[test]
    fn packet_states_hop_index_increases_monotonically_across_the_sweep() {
        let path = build_full_sweep_path(&sample_tree());
        let states = packet_states(&path, 40);
        let mut last = 1;
        for s in &states {
            assert!(s.hop_index >= last, "hop index should never go backwards");
            assert!(s.hop_index <= s.hop_count);
            last = s.hop_index;
        }
        // With enough frames relative to the (short) path, every hop should
        // actually get visited at least once, not skipped over entirely.
        let last_hop_seen = states.last().unwrap().hop_index;
        assert_eq!(last_hop_seen, states[0].hop_count, "the sweep should reach its final hop by the last frame");
    }

    #[test]
    fn packet_states_interpolates_within_a_segment() {
        // Two nodes 10 units apart on the X axis — a single hop.
        let a = NodeLabel { id: "a".into(), label: "A".into(), parent: None, center: [0.0, 0.0, 0.0], metric: None, metadata: Map::new() };
        let b = NodeLabel { id: "b".into(), label: "B".into(), parent: Some("a".into()), center: [10.0, 0.0, 0.0], metric: None, metadata: Map::new() };
        let path = vec![a, b];
        let states = packet_states(&path, 4); // phases 0.0, 0.25, 0.5, 0.75
        assert_eq!(states[0].position, [0.0, 0.0, 0.0]);
        assert_eq!(states[1].position, [2.5, 0.0, 0.0]);
        assert_eq!(states[2].position, [5.0, 0.0, 0.0]);
        assert_eq!(states[3].position, [7.5, 0.0, 0.0]);
        assert!(states.iter().all(|s| s.hop_index == 1 && s.hop_count == 1));
        assert!(states.iter().all(|s| s.from_label == "A" && s.to_label == "B"));
    }

    #[test]
    fn packet_states_of_too_short_a_path_is_empty() {
        assert_eq!(packet_states(&[], 10), Vec::new());
        assert_eq!(packet_states(&sample_tree()[..1], 10), Vec::new());
    }
}
