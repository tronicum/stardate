use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;
use serde_json::{Map, Value};
use spex_core::Point;
use std::collections::{BTreeMap, HashMap};
use std::cmp::Ordering;

use crate::Graph;

/// Per-node layout metadata alongside the rendered points: the "machine
/// readable" companion to the point cloud, letting a viewer (or any other
/// consumer) map a 3D position back to the original node's label/metric/metadata.
#[derive(Clone, Debug, Serialize)]
pub struct LayoutNodeInfo {
    pub id: String,
    pub label: String,
    pub parent: Option<String>,
    pub center: [f64; 3],
    pub metric: Option<f64>,
    pub metadata: Map<String, Value>,
}

pub struct LayoutResult {
    pub points: Vec<Point>,
    pub nodes: Vec<LayoutNodeInfo>,
}

const RADIUS_STEP: f64 = 8.0;
const HEIGHT_STEP: f64 = 4.0;
const ANGLE_JITTER: f64 = 0.6;
const BLOB_POINTS: usize = 300;
const BLOB_RADIUS: f64 = 1.5;
const EDGE_POINTS: usize = 60;
const EDGE_JITTER: f64 = 0.15;
const NEUTRAL_GRAY: [u8; 3] = [140, 140, 140];
/// Fixed colors for the diff/temporal viewer (`graph_diff::merge_for_viz`'s
/// `diff_status` metadata tag) — distinct from the metric heat gradient so
/// "what changed" reads as a category, not a magnitude.
const DIFF_ADDED_COLOR: [u8; 3] = [60, 220, 90];
const DIFF_REMOVED_COLOR: [u8; 3] = [220, 50, 50];
const DIFF_CHANGED_COLOR: [u8; 3] = [255, 170, 30];
const DIFF_UNCHANGED_COLOR: [u8; 3] = [90, 110, 160];
/// Alternating radial offset applied to every other sibling ("zigzag" a
/// ring's blobs slightly in/out) so a crowded ring — narrow angular slices,
/// e.g. from a capped high-fanout parent — gets a little breathing room
/// between neighbors without changing the angular partition (so it can't
/// affect which descendants belong to which branch).
const RING_STAGGER: f64 = 2.5;

/// Max children rendered individually before the rest get collapsed into one
/// synthetic "+N more" sibling (see [`collapse_high_fanout`]). Chosen so
/// children physically fit around a depth-1 ring without overlapping: at
/// depth 1 the ring's circumference is `2*pi*RADIUS_STEP` ~= 50 units, and
/// each blob needs ~3 units of arc to avoid touching its neighbors -> ~16-17;
/// 20 gives a little headroom.
const MAX_CHILDREN_SHOWN: usize = 20;

fn fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn lerp_color(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    [
        (a[0] as f64 + (b[0] as f64 - a[0] as f64) * t).round() as u8,
        (a[1] as f64 + (b[1] as f64 - a[1] as f64) * t).round() as u8,
        (a[2] as f64 + (b[2] as f64 - a[2] as f64) * t).round() as u8,
    ]
}

/// blue (low) -> yellow (mid) -> red (high). Shared by the 3D layout (blob
/// color) and the terminal view (`display::format_tree`'s ANSI color), so
/// both media use exactly the same color language for the same data.
pub(crate) fn heat_color(t: f64) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        lerp_color([40, 110, 255], [255, 220, 0], t / 0.5)
    } else {
        lerp_color([255, 220, 0], [255, 40, 40], (t - 0.5) / 0.5)
    }
}

/// `(min, range)` over whatever nodes have a metric, with `range` defaulting
/// to 1.0 when there's no real spread (or no metrics at all) — used to
/// normalize a metric into `heat_color`'s 0..1 input.
pub(crate) fn metric_min_range<'a>(metrics: impl Iterator<Item = &'a Option<f64>>) -> (f64, f64) {
    let (min, max) = metrics
        .filter_map(|m| *m)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), m| (lo.min(m), hi.max(m)));
    let range = if max > min { max - min } else { 1.0 };
    (min, range)
}

/// A node as seen by the layout engine: owns its data (rather than borrowing
/// from the input `Graph`) so that [`collapse_high_fanout`] can fabricate
/// synthetic "+N more" nodes that don't exist in the original graph.
#[derive(Clone)]
struct WorkingNode {
    id: String,
    label: String,
    parent: Option<String>,
    metric: Option<f64>,
    metadata: Map<String, Value>,
}

struct LayoutNode {
    id: String,
    label: String,
    parent: Option<String>,
    metric: Option<f64>,
    metadata: Map<String, Value>,
    center: [f64; 3],
    color: [u8; 3],
}

/// Lays out a tree radially in 3D (root at the center, each depth level a wider,
/// higher ring, siblings splitting their parent's angular slice) and expands each
/// node into a small point-cluster "blob" plus sparse point-trails along
/// parent-child edges, so the result is a dense-enough point cloud for the
/// existing octree tiler/viewer pipeline to render — no new visual primitives.
pub fn build_points(graph: &Graph) -> Vec<Point> {
    build(graph).points
}

/// Same as [`build_points`], but also returns each node's layout position and
/// original metadata — the human/machine-readable companion to the points.
pub fn build(graph: &Graph) -> LayoutResult {
    let mut by_id: HashMap<String, WorkingNode> = graph
        .nodes
        .iter()
        .map(|n| {
            (
                n.id.clone(),
                WorkingNode {
                    id: n.id.clone(),
                    label: n.label.clone(),
                    parent: n.parent.clone(),
                    metric: n.metric,
                    metadata: n.metadata.clone(),
                },
            )
        })
        .collect();

    let mut children: HashMap<Option<String>, Vec<String>> = HashMap::new();
    for n in &graph.nodes {
        children.entry(n.parent.clone()).or_default().push(n.id.clone());
    }

    collapse_high_fanout(&mut by_id, &mut children);

    let (metric_min, metric_range) = metric_min_range(by_id.values().map(|n| &n.metric));

    let mut layout: BTreeMap<String, LayoutNode> = BTreeMap::new();

    let roots = children.get(&None).cloned().unwrap_or_default();
    for root_id in roots {
        place(&root_id, 0, 0.0, std::f64::consts::TAU, 0.0, &by_id, &children, metric_min, metric_range, &mut layout);
    }

    let mut points = Vec::new();
    for ln in layout.values() {
        scatter_blob(ln, &mut points);
    }
    for ln in layout.values() {
        if let Some(parent_id) = ln.parent.as_deref() {
            if let Some(parent_ln) = layout.get(parent_id) {
                scatter_edge(parent_ln, ln, &mut points);
            }
        }
    }

    let nodes = layout
        .values()
        .map(|ln| LayoutNodeInfo {
            id: ln.id.clone(),
            label: ln.label.clone(),
            parent: ln.parent.clone(),
            center: ln.center,
            metric: ln.metric,
            metadata: ln.metadata.clone(),
        })
        .collect();

    LayoutResult { points, nodes }
}

/// For any parent with more than [`MAX_CHILDREN_SHOWN`] children, keeps the
/// heaviest (by `metric`, descending; no-metric children sort last) and
/// collapses the rest into one synthetic sibling — so a node with hundreds of
/// children (as real process trees can have) renders as a bounded, legible
/// fan-out instead of an unreadable ring, no matter which adapter produced it.
fn collapse_high_fanout(by_id: &mut HashMap<String, WorkingNode>, children: &mut HashMap<Option<String>, Vec<String>>) {
    let parent_keys: Vec<Option<String>> = children.keys().cloned().collect();
    for parent_key in parent_keys {
        let entry = children.get_mut(&parent_key).expect("key came from children.keys()");
        if entry.len() <= MAX_CHILDREN_SHOWN {
            continue;
        }

        entry.sort_by(|a, b| {
            let ma = by_id.get(a).and_then(|n| n.metric).unwrap_or(f64::NEG_INFINITY);
            let mb = by_id.get(b).and_then(|n| n.metric).unwrap_or(f64::NEG_INFINITY);
            mb.partial_cmp(&ma).unwrap_or(Ordering::Equal)
        });
        let collapsed_ids = entry.split_off(MAX_CHILDREN_SHOWN - 1);

        let (metric_sum, metric_count) = collapsed_ids.iter().fold((0.0, 0usize), |(sum, cnt), cid| match by_id.get(cid).and_then(|n| n.metric) {
            Some(m) => (sum + m, cnt + 1),
            None => (sum, cnt),
        });

        let synthetic_id = format!("{}__more", parent_key.as_deref().unwrap_or("root"));
        let mut metadata = Map::new();
        metadata.insert("collapsedCount".to_string(), Value::from(collapsed_ids.len()));
        by_id.insert(
            synthetic_id.clone(),
            WorkingNode {
                id: synthetic_id.clone(),
                label: format!("+{} more", collapsed_ids.len()),
                parent: parent_key.clone(),
                metric: if metric_count > 0 { Some(metric_sum) } else { None },
                metadata,
            },
        );
        entry.push(synthetic_id);
        // Collapsed children stay in `by_id` (harmless) but are no longer
        // reachable through `children`, so `place()`'s recursion — which only
        // walks via `children` — never visits them or their own descendants.
    }
}

#[allow(clippy::too_many_arguments)]
fn place(
    id: &str,
    depth: u32,
    angle_start: f64,
    angle_end: f64,
    radius_offset: f64,
    by_id: &HashMap<String, WorkingNode>,
    children: &HashMap<Option<String>, Vec<String>>,
    metric_min: f64,
    metric_range: f64,
    out: &mut BTreeMap<String, LayoutNode>,
) {
    let node = match by_id.get(id) {
        Some(n) => n,
        None => return,
    };

    let mut rng = StdRng::seed_from_u64(fnv1a(id));
    let jitter = (rng.gen::<f64>() - 0.5) * ANGLE_JITTER;
    let angle = (angle_start + angle_end) / 2.0 + jitter;
    let radius = depth as f64 * RADIUS_STEP + radius_offset;
    let center = [radius * angle.cos(), radius * angle.sin(), depth as f64 * HEIGHT_STEP];

    let color = match node.metadata.get("diff_status").and_then(|v| v.as_str()) {
        Some("added") => DIFF_ADDED_COLOR,
        Some("removed") => DIFF_REMOVED_COLOR,
        Some("changed") => DIFF_CHANGED_COLOR,
        Some("unchanged") => DIFF_UNCHANGED_COLOR,
        _ => match node.metric {
            Some(m) => heat_color((m - metric_min) / metric_range),
            None => NEUTRAL_GRAY,
        },
    };

    out.insert(
        id.to_string(),
        LayoutNode {
            id: node.id.clone(),
            label: node.label.clone(),
            parent: node.parent.clone(),
            metric: node.metric,
            metadata: node.metadata.clone(),
            center,
            color,
        },
    );

    let kids = children.get(&Some(id.to_string())).cloned().unwrap_or_default();
    let n = kids.len();
    if n == 0 {
        return;
    }
    let span = angle_end - angle_start;
    for (i, child_id) in kids.into_iter().enumerate() {
        let a0 = angle_start + span * (i as f64) / (n as f64);
        let a1 = angle_start + span * ((i + 1) as f64) / (n as f64);
        let child_radius_offset = if i % 2 == 0 { 0.0 } else { RING_STAGGER };
        place(&child_id, depth + 1, a0, a1, child_radius_offset, by_id, children, metric_min, metric_range, out);
    }
}

fn random_in_sphere(rng: &mut StdRng, radius: f64) -> [f64; 3] {
    loop {
        let p = [rng.gen::<f64>() * 2.0 - 1.0, rng.gen::<f64>() * 2.0 - 1.0, rng.gen::<f64>() * 2.0 - 1.0];
        let d2 = p[0] * p[0] + p[1] * p[1] + p[2] * p[2];
        if d2 <= 1.0 {
            return [p[0] * radius, p[1] * radius, p[2] * radius];
        }
    }
}

fn scatter_blob(ln: &LayoutNode, out: &mut Vec<Point>) {
    let mut rng = StdRng::seed_from_u64(fnv1a(&ln.id) ^ 0x5bd1_e995);
    for _ in 0..BLOB_POINTS {
        let offset = random_in_sphere(&mut rng, BLOB_RADIUS);
        out.push(Point {
            position: [ln.center[0] + offset[0], ln.center[1] + offset[1], ln.center[2] + offset[2]],
            color: ln.color,
        });
    }
}

fn scatter_edge(parent: &LayoutNode, child: &LayoutNode, out: &mut Vec<Point>) {
    let mut rng = StdRng::seed_from_u64(fnv1a(&child.id) ^ 0x9e37_79b9);
    let dim = |c: u8| ((c as f64) * 0.5).round() as u8;
    let color = [dim(child.color[0]), dim(child.color[1]), dim(child.color[2])];
    for i in 0..EDGE_POINTS {
        let t = (i as f64 + 0.5) / EDGE_POINTS as f64;
        let base = [
            parent.center[0] + (child.center[0] - parent.center[0]) * t,
            parent.center[1] + (child.center[1] - parent.center[1]) * t,
            parent.center[2] + (child.center[2] - parent.center[2]) * t,
        ];
        let offset = random_in_sphere(&mut rng, EDGE_JITTER);
        out.push(Point {
            position: [base[0] + offset[0], base[1] + offset[1], base[2] + offset[2]],
            color,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GraphNode;

    fn node(id: &str, parent: Option<&str>, metric: Option<f64>) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            label: id.to_string(),
            parent: parent.map(|p| p.to_string()),
            metric,
            metadata: Map::new(),
        }
    }

    #[test]
    fn chain_produces_distinct_blob_centers_and_edges() {
        let graph = Graph {
            nodes: vec![
                node("a", None, None),
                node("b", Some("a"), Some(1.0)),
                node("c", Some("b"), Some(5.0)),
            ],
            ..Default::default()
        };
        let points = build_points(&graph);

        // 3 blobs * BLOB_POINTS + 2 edges * EDGE_POINTS
        assert_eq!(points.len(), 3 * BLOB_POINTS + 2 * EDGE_POINTS);

        // Root "a" should sit at the origin (depth 0 => radius 0).
        let near_origin = points.iter().filter(|p| {
            let d = (p.position[0].powi(2) + p.position[1].powi(2) + p.position[2].powi(2)).sqrt();
            d < BLOB_RADIUS * 1.5
        });
        assert!(near_origin.count() >= BLOB_POINTS);
    }

    #[test]
    fn siblings_zigzag_radius_to_reduce_ring_crowding() {
        let mut nodes = vec![node("root", None, None)];
        for i in 0..6 {
            nodes.push(node(&format!("child{i}"), Some("root"), None));
        }
        let graph = Graph { nodes, ..Default::default() };
        let result = build(&graph);

        let radius_of = |id: &str| {
            let n = result.nodes.iter().find(|n| n.id == id).unwrap();
            (n.center[0].powi(2) + n.center[1].powi(2)).sqrt()
        };

        // Even-indexed children sit on the base ring; odd-indexed ones are staggered outward.
        let base = radius_of("child0");
        assert!((radius_of("child2") - base).abs() < 1e-9);
        assert!((radius_of("child4") - base).abs() < 1e-9);
        assert!((radius_of("child1") - (base + RING_STAGGER)).abs() < 1e-9);
        assert!((radius_of("child3") - (base + RING_STAGGER)).abs() < 1e-9);
    }

    #[test]
    fn heat_color_gradient_is_monotonically_hotter() {
        let cold = heat_color(0.0);
        let mid = heat_color(0.5);
        let hot = heat_color(1.0);
        // redness increases and overall blueness drops, from cold to hot
        assert!(cold[2] > mid[2]);
        assert!(mid[0] >= cold[0]);
        assert!(hot[0] >= mid[0]);
        assert!(hot[2] < cold[2]);
    }

    #[test]
    fn deterministic_across_runs() {
        let graph = Graph {
            nodes: vec![node("root", None, None), node("child", Some("root"), Some(2.0))],
            ..Default::default()
        };
        let p1 = build_points(&graph);
        let p2 = build_points(&graph);
        assert_eq!(p1.len(), p2.len());
        for (a, b) in p1.iter().zip(p2.iter()) {
            assert_eq!(a.position, b.position);
            assert_eq!(a.color, b.color);
        }
    }

    #[test]
    fn high_fanout_gets_capped_and_collapsed() {
        let mut nodes = vec![node("root", None, None)];
        for i in 0..30 {
            // descending metric so we can predict exactly which 19 survive
            nodes.push(node(&format!("child{i}"), Some("root"), Some((30 - i) as f64)));
        }
        let graph = Graph { nodes, ..Default::default() };
        let result = build(&graph);

        let root_children: Vec<&LayoutNodeInfo> = result.nodes.iter().filter(|n| n.parent.as_deref() == Some("root")).collect();
        // 19 real (heaviest) children + 1 synthetic "+more" node
        assert_eq!(root_children.len(), MAX_CHILDREN_SHOWN);

        let synthetic = root_children.iter().find(|n| n.id == "root__more").expect("synthetic node present");
        assert_eq!(synthetic.label, "+11 more");
        assert_eq!(synthetic.metadata.get("collapsedCount").and_then(Value::as_u64), Some(11));

        // The 19 heaviest children (metric 30..12) survived; the 11 lightest (11..1) did not.
        for i in 0..19 {
            assert!(result.nodes.iter().any(|n| n.id == format!("child{i}")), "child{i} should survive (heaviest)");
        }
        for i in 19..30 {
            assert!(!result.nodes.iter().any(|n| n.id == format!("child{i}")), "child{i} should be collapsed");
        }
    }
}
