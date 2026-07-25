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

/// `spex mesh-model` end to end against the real LDraw library: the bundle
/// validates, the numbers are the ones the repo already established, and two
/// runs are byte-identical.
///
/// Network-gated like every other live-fetch test here — but unlike them it
/// is worth running by hand after any change to `spex-mesh` or `spex-ldraw`,
/// because it is the only check that the manifest agrees with the bytes
/// beside it on *real* geometry rather than a fixture.
#[test]
#[ignore = "real live network fetch against ldraw.org, not run by default"]
fn mesh_bundle_matches_its_schema_and_is_deterministic() {
    let dir = std::env::temp_dir().join(format!("spex-mesh-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let scene = repo_root().join("ldraw-scenes/monolith.ldr");
    let cache = repo_root().join(".ldraw-cache");

    let run = |out: &Path| {
        let status = Command::new(spex_bin())
            .arg("mesh-model")
            .arg(&scene)
            .arg("-o")
            .arg(out)
            .arg("--cache-dir")
            .arg(&cache)
            .status()
            .expect("running spex mesh-model");
        assert!(status.success(), "spex mesh-model failed");
    };
    let a = dir.join("a");
    let b = dir.join("b");
    run(&a);
    run(&b);

    validate(&a.join("mesh.json"), "mesh.schema.json");

    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(a.join("mesh.json")).unwrap()).unwrap();
    assert_eq!(manifest["parts"].as_array().unwrap().len(), 2, "3010 and 3710");
    assert_eq!(manifest["instanceEncoding"]["count"], 9);
    assert_eq!(
        manifest["instanceEncoding"]["maxTranslationErrorMm"], 0.0,
        "the monolith stacks on whole LDU, so quantisation is exact"
    );
    assert_eq!(manifest["orientations"].as_array().unwrap().len(), 1, "all nine share one");

    // 75.2 mm, not the 73.6 mm the .ldr header quotes. Both are real and they
    // measure different things: 73.6 is the STACK (7*24 + 2*8 = 184 LDU),
    // 75.2 is the RENDERED extent, which includes the topmost stud's 4 LDU.
    // A bounding box is the second one. The spec's own acceptance criterion
    // had quoted the first.
    let bounds = &manifest["bounds"];
    let height = bounds["max"][1].as_f64().unwrap() - bounds["min"][1].as_f64().unwrap();
    assert!((height - 75.2).abs() < 0.01, "monolith height {height} mm");

    // Ten bytes per instance is the whole reason instances are not JSON.
    assert_eq!(std::fs::metadata(a.join("instances.bin")).unwrap().len(), 90);

    for f in ["mesh.json", "instances.bin"] {
        assert_eq!(
            std::fs::read(a.join(f)).unwrap(),
            std::fs::read(b.join(f)).unwrap(),
            "{f} must be byte-identical across runs"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
