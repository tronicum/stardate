//! `spex-wasm` — a proof-of-concept WASM build of `spex-graph`'s real
//! radial layout algorithm, so a browser can lay out a real `graph.json`
//! entirely client-side: no `spex serve`, no Rust process, no network
//! round-trip beyond loading this module once. Exposes exactly one
//! function; the parsing and layout math underneath are the *same* real
//! `spex_graph::build()` every other view of a graph already goes
//! through — not a reimplementation, so a WASM-laid-out graph and a
//! `spex graph-layout`-produced tileset agree exactly on where every node
//! ends up.
//!
//! Not wired into the real viewer (`viewer/`) — this is deliberately a
//! standalone PoC crate proving the layout core itself is WASM-portable
//! (pure computation, no filesystem/network/OS-randomness dependency at
//! runtime), not a finished feature. A real integration would still need
//! a browser-side octree tiler or a much simpler direct-point-cloud
//! render path, neither of which this PoC attempts.
use spex_graph::Graph;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[derive(serde::Serialize)]
struct LayoutOutput<'a> {
    /// `[x, y, z, r, g, b]` per point — flat arrays serialize/deserialize
    /// across the WASM boundary with far less overhead than an array of
    /// objects, and this is the same shape a typed-array-based renderer
    /// on the JS side would want directly.
    points: Vec<[f64; 6]>,
    nodes: &'a [spex_graph::LayoutNodeInfo],
}

/// The real logic, kept `JsValue`-free on purpose: `JsValue`'s methods
/// (e.g. `from_str`) link against JS glue that only exists inside an
/// actual wasm-bindgen JS runtime — calling them from a native `cargo
/// test` run aborts the process. Keeping this function plain
/// `Result<String, String>` lets its tests run natively; only the thin
/// `#[wasm_bindgen]` wrapper below ever touches `JsValue`, right at the
/// real FFI boundary where it belongs.
fn layout_graph_impl(graph_json: &str) -> Result<String, String> {
    let graph: Graph = serde_json::from_str(graph_json).map_err(|e| format!("parsing graph.json: {e}"))?;
    let result = spex_graph::build(&graph);

    let points: Vec<[f64; 6]> = result
        .points
        .iter()
        .map(|p| {
            [
                p.position[0],
                p.position[1],
                p.position[2],
                p.color[0] as f64,
                p.color[1] as f64,
                p.color[2] as f64,
            ]
        })
        .collect();

    serde_json::to_string(&LayoutOutput { points, nodes: &result.nodes }).map_err(|e| format!("serializing layout result: {e}"))
}

/// Lays out a real `graph.json` (passed as a JSON string, the same format
/// `spex graph-layout` reads from disk) and returns
/// `{points: [[x,y,z,r,g,b], ...], nodes: [...]}` as a JSON string.
#[wasm_bindgen]
pub fn layout_graph(graph_json: &str) -> Result<String, JsValue> {
    layout_graph_impl(graph_json).map_err(|e| JsValue::from_str(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_graph_lays_out_a_real_small_tree_and_round_trips_as_json() {
        let graph_json = r#"{
            "title": "poc",
            "nodes": [
                {"id": "root", "label": "root", "parent": null, "metric": 1.0},
                {"id": "a", "label": "a", "parent": "root", "metric": 2.0},
                {"id": "b", "label": "b", "parent": "root", "metric": 3.0}
            ]
        }"#;

        let out = layout_graph_impl(graph_json).expect("real small graph should lay out cleanly");
        let value: serde_json::Value = serde_json::from_str(&out).unwrap();
        let points = value["points"].as_array().unwrap();
        let nodes = value["nodes"].as_array().unwrap();

        assert_eq!(nodes.len(), 3, "3 real nodes in, 3 laid-out nodes out");
        assert!(!points.is_empty(), "a real 3-node tree should scatter real blob points");
        assert_eq!(points[0].as_array().unwrap().len(), 6, "[x,y,z,r,g,b] per point");
    }

    #[test]
    fn layout_graph_rejects_invalid_json_with_a_real_error_not_a_panic() {
        let result = layout_graph_impl("not real json");
        assert!(result.is_err());
    }
}
