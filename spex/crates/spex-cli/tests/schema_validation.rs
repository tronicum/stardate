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
