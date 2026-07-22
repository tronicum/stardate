//! A reusable "does the whole pipeline actually work" harness, exercised
//! here with a small illustrative fixture: a simulated TCP/UDP packet's
//! journey Berlin -> Tegernsee -> Neuss (real coordinates, illustrative
//! latency — see demos/berlin-tegernsee-neuss/graph.json for the same data
//! as a standalone demo). `run_full_pipeline` is written to be reusable for
//! testing *other* pipelines too: build a small `spex_graph::Graph` fixture
//! for whatever you're testing, call it, assert on the `PipelineArtifacts`
//! it returns — no need to reinvent "spawn the CLI, read the files back"
//! each time.
use serde_json::Map;
use spex_graph::{Graph, GraphNode};
use std::path::Path;
use std::process::Command;

fn spex_bin() -> &'static str {
    env!("CARGO_BIN_EXE_spex")
}

/// What came out of running a graph through the real CLI end to end
/// (graph-layout, graph-print, ascii). The shape to assert against,
/// whatever fixture a future test wants to exercise.
struct PipelineArtifacts {
    tileset_point_count: u64,
    terminal_text: String,
    ascii_text: String,
}

/// Runs `graph` through the real `spex` binary in a scratch directory —
/// black-box (spawns the built binary), since spex-cli has no lib target
/// for tests to call into directly. A template for testing any other
/// adapter/pipeline: build a small `Graph` fixture, call this, assert on
/// the result.
fn run_full_pipeline(graph: &Graph, work_dir: &Path) -> PipelineArtifacts {
    std::fs::create_dir_all(work_dir).unwrap();
    let graph_path = work_dir.join("graph.json");
    let tileset_dir = work_dir.join("tileset");
    graph.write_json(&graph_path).expect("writing graph.json");

    let status = Command::new(spex_bin())
        .arg("graph-layout")
        .arg(&graph_path)
        .arg("-o")
        .arg(&tileset_dir)
        .status()
        .expect("running spex graph-layout");
    assert!(status.success(), "spex graph-layout failed");

    let tileset_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(tileset_dir.join("tileset.json")).unwrap()).unwrap();
    let tileset_point_count = tileset_json["pointCount"].as_u64().unwrap();

    let print_output = Command::new(spex_bin()).arg("graph-print").arg(&graph_path).output().expect("running spex graph-print");
    assert!(print_output.status.success(), "spex graph-print failed");
    let terminal_text = String::from_utf8(print_output.stdout).unwrap();

    let ascii_output = Command::new(spex_bin())
        .arg("ascii")
        .arg(&tileset_dir)
        .arg("--width")
        .arg("60")
        .output()
        .expect("running spex ascii");
    assert!(ascii_output.status.success(), "spex ascii failed");
    let ascii_text = String::from_utf8(ascii_output.stdout).unwrap();

    PipelineArtifacts { tileset_point_count, terminal_text, ascii_text }
}

#[allow(clippy::too_many_arguments)]
fn city_node(id: &str, label: &str, parent: Option<&str>, metric: Option<f64>, city: &str, lat: f64, lon: f64) -> GraphNode {
    let mut metadata = Map::new();
    metadata.insert("city".to_string(), serde_json::Value::from(city));
    metadata.insert("lat".to_string(), serde_json::Value::from(lat));
    metadata.insert("lon".to_string(), serde_json::Value::from(lon));
    GraphNode {
        id: id.to_string(),
        label: label.to_string(),
        parent: parent.map(|p| p.to_string()),
        metric,
        metadata,
    }
}

/// A deliberately tiny, fully deterministic fixture — good for exactly this
/// kind of "does the whole thing work" smoke test (unlike a real `trace`/
/// `ps-tree` capture, nothing here depends on network conditions or this
/// machine's current state).
fn berlin_tegernsee_neuss_journey() -> Graph {
    Graph {
        title: Some("simulated packet journey: Berlin -> Tegernsee -> Neuss".to_string()),
        metric_label: Some("simulated one-way latency (ms) - illustrative, not measured".to_string()),
        nodes: vec![
            city_node("berlin", "Berlin", None, None, "Berlin", 52.52, 13.405),
            city_node("tegernsee", "Tegernsee", Some("berlin"), Some(8.2), "Tegernsee", 47.7167, 11.75),
            city_node("neuss", "Neuss", Some("tegernsee"), Some(9.6), "Neuss", 51.2, 6.6833),
        ],
    }
}

#[test]
fn berlin_tegernsee_neuss_journey_survives_the_whole_pipeline() {
    let dir = std::env::temp_dir().join(format!("spex-journey-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    let graph = berlin_tegernsee_neuss_journey();
    let artifacts = run_full_pipeline(&graph, &dir);

    // 3 nodes * 300 blob points + 2 edges * 60 trail points (spex-graph's
    // layout constants) — if this ever drifts, it's a real signal something
    // in the layout changed, not just test noise.
    assert_eq!(artifacts.tileset_point_count, 3 * 300 + 2 * 60);

    for city in ["Berlin", "Tegernsee", "Neuss"] {
        assert!(artifacts.terminal_text.contains(city), "terminal view missing {city}:\n{}", artifacts.terminal_text);
    }
    assert!(
        artifacts.terminal_text.contains("8.20") && artifacts.terminal_text.contains("9.60"),
        "terminal footer should show the latency range:\n{}",
        artifacts.terminal_text
    );

    let non_blank_cells = artifacts.ascii_text.chars().filter(|c| *c != ' ' && *c != '\n').count();
    assert!(non_blank_cells > 0, "ascii render should draw something, got:\n{}", artifacts.ascii_text);

    let _ = std::fs::remove_dir_all(&dir);
}
