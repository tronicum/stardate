//! Graph-aware "packet path" support for `spex ascii --animate`
//! (crates/spex-cli/src/ascii.rs). Given a tileset directory's optional
//! `nodes.json` (written only by the graph pipeline — `spex graph-layout`;
//! absent for a plain point-cloud tileset from `spex convert`), computes the
//! same tree-sweep(s) the browser viewer animates via its "Animate packet"
//! checkbox, so a terminal/HTML ASCII animation can show equivalent
//! marker(s) + hop readout with no browser involved.
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
//!
//! issue #26: `build_concurrent_sweep_paths` is the multi-marker sibling —
//! one marker per the root's direct children, each independently sweeping
//! only its own subtree, all animating concurrently — a port of the
//! viewer's `buildConcurrentSweepPaths`. See its doc comment for the
//! design and `concurrent_packet_states` for the shared-speed timing model
//! that lets subtrees of different real sizes finish their own loop at
//! different times instead of being forced to sync on one shared frame
//! count.
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

/// Builds the `parent -> children` index (children sorted heaviest-`metric`
/// first) that every sweep — single- or multi-marker — walks. Shared by
/// `build_full_sweep_path` and `build_concurrent_sweep_paths` so both stay
/// in lock-step on ordering instead of risking two copies of the same sort
/// drifting apart.
fn children_of(nodes: &[NodeLabel]) -> HashMap<&str, Vec<usize>> {
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
    children_of
}

/// Full depth-first sweep of the tree from its root — see module docs.
/// Returns `[]` if there's no root or fewer than two nodes (mirrors
/// `buildFullSweepPath`'s early-return; nothing meaningful to animate).
///
/// Still used directly for the single-marker case: `build_concurrent_sweep_paths`
/// produces a byte-identical single path when the root has exactly one
/// child (see its doc comment).
pub fn build_full_sweep_path(nodes: &[NodeLabel]) -> Vec<NodeLabel> {
    if nodes.len() < 2 {
        return Vec::new();
    }
    let children_of = children_of(nodes);
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

/// issue #26: multiple concurrent packets instead of one marker sweeping the
/// *entire* tree. One path per the root's direct children (already bounded
/// by `spex_graph::layout`'s own fan-out safeguard, `MAX_CHILDREN_SHOWN` = 20
/// — this never has to re-enforce that cap itself), each independently
/// DFS-sweeping only its own subtree via the exact same `visit` the
/// single-marker sweep uses — called once per root child instead of once for
/// the whole tree, per the design in the tracking issue. Every path starts
/// with the shared root — the one point all the children genuinely share —
/// and then descends into exactly one child's subtree, never crossing into a
/// sibling's: that's what makes the markers read as concurrent, independent
/// activity (a process forking several children "at once", several packets
/// in flight) rather than N unrelated partial tours.
///
/// Degenerate cases, both handled by the general algorithm with no special
/// casing:
/// - A root with exactly one child (a plain chain — a single top-level
///   process, traceroute hops) produces exactly one path, byte-identical to
///   `build_full_sweep_path`'s output for the same tree (there's only one
///   child to recurse into, so no backtracking-to-root ever happens either
///   way) — a chain demo renders exactly one marker, indistinguishable from
///   the pre-#26 single-packet animation.
/// - A root with zero children (a single-node graph), or fewer than two
///   nodes at all, returns `[]` — zero markers, not a panic.
pub fn build_concurrent_sweep_paths(nodes: &[NodeLabel]) -> Vec<Vec<NodeLabel>> {
    if nodes.len() < 2 {
        return Vec::new();
    }
    let children_of = children_of(nodes);
    let Some(root) = nodes.iter().position(|n| n.parent.is_none()) else {
        return Vec::new();
    };
    let Some(root_children) = children_of.get(nodes[root].id.as_str()) else {
        return Vec::new(); // root has no children: zero markers
    };

    root_children
        .iter()
        .map(|&child| {
            let mut path_indices = vec![root];
            visit(child, nodes, &children_of, &mut path_indices);
            path_indices.into_iter().map(|i| nodes[i].clone()).collect()
        })
        .collect()
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

/// `.max(1e-3)` per hop mirrors the viewer's `|| 0.001` guard against a
/// zero-length hop (two nodes at the same position) stalling the sweep.
fn segment_lengths(path: &[NodeLabel]) -> Vec<f64> {
    path.windows(2).map(|w| distance(w[0].center, w[1].center).max(1e-3)).collect()
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
///
/// A thin wrapper around [`packet_states_with_period`] with `period_frames
/// == frame_count` — one full sweep exactly fills the animation, the single-
/// marker case's original timing. See [`concurrent_packet_states`] for why
/// issue #26's multiple markers each get their own, generally different,
/// period instead — production code (`ascii.rs`) now always goes through
/// `concurrent_packet_states`, even for a single-marker chain, since its
/// period there is derived to equal `frame_count` exactly for that case
/// (see that function's doc comment); this wrapper is kept public as the
/// base single-sweep timing model and exercised directly by this module's
/// own tests, so `#[allow(dead_code)]` rather than deleting real, still-
/// meaningful public API.
#[allow(dead_code)]
pub fn packet_states(path: &[NodeLabel], frame_count: usize) -> Vec<PacketState> {
    packet_states_with_period(path, frame_count, frame_count)
}

/// Same interpolation as [`packet_states`], but `path`'s full traversal
/// loops every `period_frames` frames rather than exactly once across all
/// `frame_count` frames — e.g. `period_frames == frame_count / 2` completes
/// two full loops within the animation. Still produces exactly `frame_count`
/// states (or `[]` if `path` has fewer than two nodes, `frame_count == 0`,
/// or `period_frames == 0`), so every caller can zip it 1:1 against the same
/// per-frame camera/grid sequence [`packet_states`] already does.
fn packet_states_with_period(path: &[NodeLabel], frame_count: usize, period_frames: usize) -> Vec<PacketState> {
    if path.len() < 2 || frame_count == 0 || period_frames == 0 {
        return Vec::new();
    }

    let segment_lengths = segment_lengths(path);
    let total: f64 = segment_lengths.iter().sum();
    let mut cumulative = vec![0.0; segment_lengths.len() + 1];
    for (i, len) in segment_lengths.iter().enumerate() {
        cumulative[i + 1] = cumulative[i] + len;
    }

    (0..frame_count)
        .map(|f| {
            let phase = (f % period_frames) as f64 / period_frames as f64;
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

/// issue #26's concurrent-marker timing model. `nodes` is a tileset's whole
/// `nodes.json` (not a pre-swept path); returns one `Vec<PacketState>` per
/// marker (see [`build_concurrent_sweep_paths`]), each exactly `frame_count`
/// long, indexed identically to the marker's position in that function's
/// return value — `[]` if there's nothing to animate (fewer than two nodes,
/// a childless root, or `frame_count == 0`).
///
/// Timing model (see the tracking issue and the PR description for the full
/// reasoning): every marker moves at the same *real-world* speed — a
/// reference "distance per frame" derived from the classic single-sweep
/// path's total real distance spread over `frame_count` (i.e. the exact pace
/// the pre-#26 single packet already moved at, so #26 doesn't change how
/// fast an individual hop feels). Each marker's own subtree total distance
/// then determines its own loop period at that shared speed
/// (`distance / reference_speed`, rounded to whole frames, floored at 1) —
/// a marker sweeping a bigger subtree takes proportionally longer to
/// complete one loop and simply loops more slowly, rather than every marker
/// being force-fit to finish exactly on `frame_count`'s last frame
/// regardless of its own real size. A root with exactly one child produces
/// one marker whose own total distance equals the reference sweep's total
/// distance, so its period is exactly `frame_count` — [`packet_states`]'s
/// original single-marker timing, unchanged in that degenerate case.
pub fn concurrent_packet_states(nodes: &[NodeLabel], frame_count: usize) -> Vec<Vec<PacketState>> {
    if frame_count == 0 {
        return Vec::new();
    }
    let paths = build_concurrent_sweep_paths(nodes);
    if paths.is_empty() {
        return Vec::new();
    }

    let reference_distance: f64 = segment_lengths(&build_full_sweep_path(nodes)).iter().sum();
    let reference_speed = if reference_distance > 0.0 { reference_distance / frame_count as f64 } else { 1.0 };

    paths
        .iter()
        .map(|path| {
            let distance: f64 = segment_lengths(path).iter().sum();
            let period = ((distance / reference_speed).round() as usize).max(1);
            packet_states_with_period(path, frame_count, period)
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

    fn node(id: &str, parent: Option<&str>, center: [f64; 3], metric: f64) -> NodeLabel {
        NodeLabel { id: id.to_string(), label: format!("{id}-label"), parent: parent.map(str::to_string), center, metric: Some(metric), metadata: Map::new() }
    }

    // issue #26: multiple concurrent markers, one per root child.

    #[test]
    fn concurrent_sweep_produces_one_path_per_root_child_each_prefixed_by_the_shared_root() {
        // sample_tree()'s root has two children (heavy, light) — real
        // branching, e.g. a process that forked two children.
        let paths = build_concurrent_sweep_paths(&sample_tree());
        assert_eq!(paths.len(), 2, "root has two children, so two markers");

        let ids: Vec<Vec<&str>> = paths.iter().map(|p| p.iter().map(|n| n.id.as_str()).collect()).collect();
        // Heaviest child first (same ordering `build_full_sweep_path` uses),
        // each path starting at the shared root then descending only into
        // its own subtree — heavy's path never mentions light, and vice
        // versa.
        assert_eq!(ids[0], vec!["root", "heavy", "leaf"]);
        assert_eq!(ids[1], vec!["root", "light"]);
    }

    #[test]
    fn concurrent_sweep_of_a_chain_degrades_to_exactly_one_marker_matching_the_full_sweep() {
        // A plain chain: root has exactly one child, e.g. `ps-tree`'s single
        // top-level process or a traceroute's single next-hop chain.
        let chain = vec![node("root", None, [0.0, 0.0, 0.0], 0.0), node("mid", Some("root"), [1.0, 0.0, 0.0], 0.0), node("leaf", Some("mid"), [2.0, 0.0, 0.0], 0.0)];
        let paths = build_concurrent_sweep_paths(&chain);
        assert_eq!(paths.len(), 1, "a chain's root has exactly one child, so exactly one marker");
        assert_eq!(paths[0], build_full_sweep_path(&chain), "a single-child root's marker path must match the pre-#26 single sweep exactly");
    }

    #[test]
    fn concurrent_sweep_of_a_single_node_graph_has_zero_markers() {
        let solo = vec![node("solo", None, [0.0, 0.0, 0.0], 0.0)];
        assert_eq!(build_concurrent_sweep_paths(&solo), Vec::<Vec<NodeLabel>>::new());
    }

    #[test]
    fn concurrent_sweep_with_no_root_is_empty_not_a_panic() {
        // Mirrors `full_sweep_with_no_root_is_empty`: malformed/cyclic
        // input, not real `spex graph-layout` output, but must not panic.
        let nodes = vec![
            NodeLabel { id: "a".into(), label: "a".into(), parent: Some("b".into()), center: [0.0; 3], metric: None, metadata: Map::new() },
            NodeLabel { id: "b".into(), label: "b".into(), parent: Some("a".into()), center: [1.0, 0.0, 0.0], metric: None, metadata: Map::new() },
        ];
        assert_eq!(build_concurrent_sweep_paths(&nodes), Vec::<Vec<NodeLabel>>::new());
    }

    #[test]
    fn concurrent_packet_states_produces_one_series_per_marker_all_starting_at_the_shared_root() {
        let states = concurrent_packet_states(&sample_tree(), 24);
        assert_eq!(states.len(), 2, "two root children, two marker state series");
        for series in &states {
            assert_eq!(series.len(), 24, "every marker gets a state for every frame, even mid-loop");
            assert_eq!(series[0].position, [0.0, 0.0, 0.0], "every marker starts at the shared root on frame 0");
        }
        // The two markers are genuinely independent: by frame 1 they've
        // already diverged into different subtrees.
        assert_ne!(states[0][1].position, states[1][1].position);
    }

    #[test]
    fn concurrent_packet_states_gives_a_bigger_subtree_a_longer_loop_period() {
        // heavy's subtree (root -> heavy -> leaf, total real distance 20)
        // is twice light's (root -> light, total real distance 10) in
        // sample_tree(). At a shared real-world speed, light's marker
        // should complete about twice as many loops as heavy's in the same
        // frame budget — checked here by counting how many frames land
        // exactly back at the shared root (every marker's path starts at
        // the root, so phase == 0 puts the marker's position exactly back
        // there; hop_index alone can't detect this for a single-hop path
        // like light's, since it never has a second hop to "decrease" from).
        let states = concurrent_packet_states(&sample_tree(), 240);
        let root_position = [0.0, 0.0, 0.0];
        let loop_starts = |series: &[PacketState]| series.iter().filter(|s| s.position == root_position).count();
        let heavy_loops = loop_starts(&states[0]);
        let light_loops = loop_starts(&states[1]);
        assert!(light_loops > heavy_loops, "light's shorter subtree should loop more often than heavy's in the same frame budget (light={light_loops}, heavy={heavy_loops})");
    }

    #[test]
    fn concurrent_packet_states_of_a_chain_matches_the_original_single_packet_timing() {
        // Continuity check for the degenerate single-marker case: a chain's
        // one marker should move at exactly the same pace `packet_states`
        // always used (period == frame_count), not some other derived speed.
        let chain = vec![node("root", None, [0.0, 0.0, 0.0], 0.0), node("mid", Some("root"), [10.0, 0.0, 0.0], 0.0), node("leaf", Some("mid"), [10.0, 10.0, 0.0], 0.0)];
        let concurrent = concurrent_packet_states(&chain, 16);
        let legacy = packet_states(&build_full_sweep_path(&chain), 16);
        assert_eq!(concurrent.len(), 1);
        assert_eq!(concurrent[0], legacy, "a single-marker chain must animate identically to the pre-#26 single sweep");
    }

    #[test]
    fn concurrent_packet_states_of_a_single_node_graph_is_empty_not_a_panic() {
        let solo = vec![node("solo", None, [0.0, 0.0, 0.0], 0.0)];
        assert_eq!(concurrent_packet_states(&solo, 10), Vec::<Vec<PacketState>>::new());
    }

    #[test]
    fn concurrent_packet_states_of_zero_frames_is_empty() {
        assert_eq!(concurrent_packet_states(&sample_tree(), 0), Vec::<Vec<PacketState>>::new());
    }
}
