//! Validates the real, committed data files (`data/ankerstein-shapes.json`,
//! `data/ankerstein-sets.json`) against their formal schemas
//! (`spec/ankerstein-shapes.schema.json`, `spec/ankerstein-sets.schema.json`)
//! and against each other (every set's `shapeId` must resolve in the shape
//! catalog) — the same generically-applicable check
//! `crates/spex-cli/tests/schema_validation.rs` runs for the point-cloud/
//! graph pipeline's generated files, applied here to this crate's
//! hand-authored data instead of CLI output. Closes the gap noted in issue
//! #15: nothing previously loaded these two files directly in a test.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // crates/spex-ankerstein -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn validate(instance_path: &Path, schema_path: &Path) {
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(schema_path).unwrap_or_else(|e| panic!("reading {}: {e}", schema_path.display())))
            .unwrap_or_else(|e| panic!("parsing {}: {e}", schema_path.display()));
    let instance: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(instance_path).unwrap_or_else(|e| panic!("reading {}: {e}", instance_path.display())))
            .unwrap_or_else(|e| panic!("parsing {}: {e}", instance_path.display()));

    let validator = jsonschema::validator_for(&schema).unwrap_or_else(|e| panic!("compiling {}: {e}", schema_path.display()));
    let errors: Vec<String> = validator.iter_errors(&instance).map(|e| e.to_string()).collect();
    assert!(
        errors.is_empty(),
        "{} does not match {}:\n{}",
        instance_path.display(),
        schema_path.display(),
        errors.join("\n")
    );
}

#[test]
fn real_shape_catalog_matches_its_schema() {
    let root = repo_root();
    validate(&root.join("data/ankerstein-shapes.json"), &root.join("spec/ankerstein-shapes.schema.json"));
}

#[test]
fn real_set_catalog_matches_its_schema() {
    let root = repo_root();
    validate(&root.join("data/ankerstein-sets.json"), &root.join("spec/ankerstein-sets.schema.json"));
}

#[test]
fn real_sets_only_reference_shapes_that_actually_exist_in_the_catalog() {
    let root = repo_root();
    let shapes = spex_ankerstein::load_catalog(&root.join("data/ankerstein-shapes.json")).expect("loading data/ankerstein-shapes.json");
    let sets = spex_ankerstein::load_sets(&root.join("data/ankerstein-sets.json")).expect("loading data/ankerstein-sets.json");
    spex_ankerstein::validate_against_catalog(&sets, &shapes).expect("every set's shapeId should resolve in the shape catalog");
}

#[test]
fn every_real_set_has_a_non_empty_citation_and_a_quantity_matching_its_own_claims() {
    let root = repo_root();
    let sets = spex_ankerstein::load_sets(&root.join("data/ankerstein-sets.json")).expect("loading data/ankerstein-sets.json");
    for set in &sets {
        assert!(!set.source_citation.trim().is_empty(), "set {:?} is missing a real source citation", set.set_id);
        assert!(!set.contents.is_empty(), "set {:?} has no contents", set.set_id);
        for content in &set.contents {
            assert!(content.quantity >= 1, "set {:?} has a non-positive quantity for {:?}", set.set_id, content.shape_id);
        }
    }
}
