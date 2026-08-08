//! Proves `spec/*.schema.json` describe what spex *actually* produces, not
//! just what CLAUDE.md/spec/README.md claim: runs the real CLI end to end
//! (pstree-demo -> graph-layout) and validates every generated file against
//! its schema. Black-box (spawns the built binary) since spex-cli has no lib
//! target for tests to import from directly.

use std::path::{Path, PathBuf};
use std::process::Command;

fn spex_bin() -> &'static str {
    env!("CARGO_BIN_EXE_spex")
}

fn repo_root() -> PathBuf {
    // crates/spex-cli -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn generated_files_match_their_schemas() {
    let dir = std::env::temp_dir().join(format!("spex-schema-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let graph_path = dir.join("graph.json");
    let tileset_dir = dir.join("tileset");

    let status = Command::new(spex_bin())
        .args(["pstree-demo", "-o"])
        .arg(&graph_path)
        .status()
        .expect("running spex pstree-demo");
    assert!(status.success(), "spex pstree-demo failed");

    let status = Command::new(spex_bin())
        .arg("graph-layout")
        .arg(&graph_path)
        .arg("-o")
        .arg(&tileset_dir)
        .status()
        .expect("running spex graph-layout");
    assert!(status.success(), "spex graph-layout failed");

    validate(&graph_path, "graph.schema.json");
    validate(&tileset_dir.join("tileset.json"), "tileset.schema.json");
    validate(&tileset_dir.join("nodes.json"), "nodes.schema.json");
    validate(&tileset_dir.join("meta.json"), "meta.schema.json");

    let _ = std::fs::remove_dir_all(&dir);
}

fn validate(instance_path: &Path, schema_file: &str) {
    let schema_path = repo_root().join("spec").join(schema_file);
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&schema_path).unwrap_or_else(|e| panic!("reading {}: {e}", schema_path.display())))
            .unwrap_or_else(|e| panic!("parsing {}: {e}", schema_path.display()));
    let instance: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(instance_path).unwrap_or_else(|e| panic!("reading {}: {e}", instance_path.display())))
            .unwrap_or_else(|e| panic!("parsing {}: {e}", instance_path.display()));

    let validator = jsonschema::validator_for(&schema).unwrap_or_else(|e| panic!("compiling {schema_file}: {e}"));
    let errors: Vec<String> = validator.iter_errors(&instance).map(|e| e.to_string()).collect();
    assert!(
        errors.is_empty(),
        "{} does not match {schema_file}:\n{}",
        instance_path.display(),
        errors.join("\n")
    );
}

/// A node with `extraParents` (issue #24 — DAG/shared-dependency merging)
/// is real, valid `spex` output (e.g. from `brew-deps` on a formula with a
/// genuinely shared transitive dependency), but neither `pstree-demo` above
/// nor any other fixture used elsewhere in this file ever produces one — so
/// without this test, `graph.schema.json`/`nodes.schema.json`'s
/// `additionalProperties: false` could silently drift out of sync with the
/// real field `GraphNode`/`LayoutNodeInfo` serialize and nothing here would
/// catch it. Hand-writes a small graph.json with one shared node (standing
/// in for what `brew_deps::build_nodes` produces for a real re-occurring
/// package) and runs it through the real `spex graph-layout` binary.
#[test]
fn graph_with_extra_parents_matches_schemas() {
    let dir = std::env::temp_dir().join(format!("spex-schema-test-extra-parents-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let graph_path = dir.join("graph.json");
    let tileset_dir = dir.join("tileset");

    // "shared" stands in for a real package like openssl@3: reachable as a
    // direct dep of "a" (its one real parent, driving its one real 3D
    // position) AND as a dep of "b" (recorded as an extra structural edge).
    std::fs::write(
        &graph_path,
        r#"{
  "title": "extraParents schema fixture",
  "nodes": [
    { "id": "root", "label": "root", "parent": null },
    { "id": "a", "label": "a", "parent": "root" },
    { "id": "b", "label": "b", "parent": "root" },
    { "id": "shared", "label": "shared", "parent": "a", "extraParents": ["b"] }
  ]
}"#,
    )
    .unwrap();

    let status = Command::new(spex_bin())
        .arg("graph-layout")
        .arg(&graph_path)
        .arg("-o")
        .arg(&tileset_dir)
        .status()
        .expect("running spex graph-layout");
    assert!(status.success(), "spex graph-layout failed");

    validate(&graph_path, "graph.schema.json");
    validate(&tileset_dir.join("tileset.json"), "tileset.schema.json");
    validate(&tileset_dir.join("nodes.json"), "nodes.schema.json");
    validate(&tileset_dir.join("meta.json"), "meta.schema.json");

    // Confirm extraParents actually round-tripped into nodes.json (not just
    // schema-valid, but really carrying the data through graph-layout).
    let nodes_json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(tileset_dir.join("nodes.json")).unwrap()).unwrap();
    let shared_node = nodes_json.as_array().unwrap().iter().find(|n| n["id"] == "shared").expect("shared node present in nodes.json");
    assert_eq!(shared_node["extraParents"], serde_json::json!(["b"]));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn frame_sequence_output_matches_schemas() {
    let dir = std::env::temp_dir().join(format!("spex-schema-test-sequence-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Two tiny real point clouds standing in for two real animation frames
    // (the same shape `spex brick-assembly`'s own frame files
    // take) — frame 1 is just frame 0 shifted along x, so a correct shared
    // offset is actually exercised, not just a degenerate zero-shift case.
    let frame0 = dir.join("frame0.xyz");
    let frame1 = dir.join("frame1.xyz");
    std::fs::write(&frame0, "0 0 0 255 0 0\n1 0 0 0 255 0\n0 1 0 0 0 255\n").unwrap();
    std::fs::write(&frame1, "10 0 0 255 0 0\n11 0 0 0 255 0\n10 1 0 0 0 255\n").unwrap();

    let out_dir = dir.join("sequence-out");
    let status = Command::new(spex_bin())
        .arg("frame-sequence")
        .arg(&frame0)
        .arg(&frame1)
        .arg("-o")
        .arg(&out_dir)
        .arg("--fps")
        .arg("6")
        .status()
        .expect("running spex frame-sequence");
    assert!(status.success(), "spex frame-sequence failed");

    validate(&out_dir.join("sequence.json"), "sequence.schema.json");
    validate(&out_dir.join("frame-000").join("tileset.json"), "tileset.schema.json");
    validate(&out_dir.join("frame-001").join("tileset.json"), "tileset.schema.json");

    let _ = std::fs::remove_dir_all(&dir);
}

/// `spex heritage-index`'s output isn't exercised here (a real live SPARQL
/// fetch — same reasoning `heritage-index` itself documents for staying out
/// of `scripts/walkthrough.sh`), but the two real JSON shapes it/the
/// curation file produce are schema-checked against a small hand-written
/// fixture matching what a real `HeritageSite`/`Curation` actually
/// serializes to (see `crates/spex-heritage/src/list.rs`'s and
/// `curation.rs`'s own round-trip unit tests for the Rust-side proof).
#[test]
fn heritage_site_list_and_curation_file_match_heritage_schema() {
    let site_list = serde_json::json!([
        {
            "id": "Q188694",
            "name": "Butrint",
            "state_parties": ["Albania"],
            "inscribed_year": 1992,
            "criteria": ["iii"],
            "category": "Cultural",
            "lat": 39.757066,
            "lon": 20.016628,
            "source": "wikidata:Q188694"
        },
        {
            "id": "Q351",
            "name": "Yellowstone National Park",
            "state_parties": ["United States of America"],
            "inscribed_year": 1978,
            "criteria": ["vii", "viii", "ix", "x"],
            "category": "Natural",
            "lat": 44.6,
            "lon": -110.5,
            "source": "wikidata:Q351"
        }
    ]);
    validate_value(&site_list, "heritage.schema.json");

    let curation = serde_json::json!({
        "sites": {
            "Q188694": { "buildable": true, "justification": "compact fortified acropolis, simple massing" }
        },
        "excluded": {
            "Q131593": "Auschwitz-Birkenau: real historical genocide site, never rendered as a toy/game object"
        }
    });
    validate_value(&curation, "heritage.schema.json");
}

fn validate_value(instance: &serde_json::Value, schema_file: &str) {
    let schema_path = repo_root().join("spec").join(schema_file);
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&schema_path).unwrap_or_else(|e| panic!("reading {}: {e}", schema_path.display())))
            .unwrap_or_else(|e| panic!("parsing {}: {e}", schema_path.display()));

    let validator = jsonschema::validator_for(&schema).unwrap_or_else(|e| panic!("compiling {schema_file}: {e}"));
    let errors: Vec<String> = validator.iter_errors(instance).map(|e| e.to_string()).collect();
    assert!(errors.is_empty(), "instance does not match {schema_file}:\n{}", errors.join("\n"));
}

/// Unlike `demos/` (gitignored, machine-local), `scripts/heritage-data/` is
/// actually committed (see `docs/FUGEN-ENGINE.md` M73) — so, unlike
/// `real_decix_trace_demo_matches_schemas_too` below, this always runs in
/// CI, validating the real committed snapshot + curation file rather than
/// only the hand-written fixture above.
#[test]
fn real_committed_heritage_snapshot_and_curation_match_schema() {
    let dir = repo_root().join("scripts/heritage-data");
    let mut found_snapshot = false;
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("reading {}: {e}", dir.display())) {
        let path = entry.unwrap().path();
        let is_snapshot = path.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("wikidata-whs-") && n.ends_with(".json")).unwrap_or(false);
        if is_snapshot {
            validate(&path, "heritage.schema.json");
            found_snapshot = true;
        }
    }
    assert!(found_snapshot, "no committed scripts/heritage-data/wikidata-whs-*.json snapshot found");
    validate(&dir.join("curation.json"), "heritage.schema.json");
}

#[test]
#[ignore = "manual spot-check against real local demo data, not a committed fixture"]
fn real_decix_trace_demo_matches_schemas_too() {
    let base = repo_root().join("demos/decix-trace");
    if !base.exists() {
        eprintln!("skipping: {} not present (run scripts/walkthrough.sh)", base.display());
        return;
    }
    validate(&base.join("graph.json"), "graph.schema.json");
    validate(&base.join("tileset/tileset.json"), "tileset.schema.json");
    validate(&base.join("tileset/nodes.json"), "nodes.schema.json");
    validate(&base.join("tileset/meta.json"), "meta.schema.json");
}
