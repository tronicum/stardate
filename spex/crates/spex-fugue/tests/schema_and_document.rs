//! M67 AC1 and AC3 — the schema validates the real document, and the document
//! agrees with the Rust the tests are written against.
//!
//! Two directions, and both are needed. The schema check says the JSON is
//! well-formed; the equality check says it is the *same music* as
//! `act_one_subject()`, which is what every assertion in `model.rs` is really
//! about. Without the second, the subject could be edited in one place and go
//! on passing every musical test in the other.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn act_one_document() -> serde_json::Value {
    let path = repo_root().join("shows/die-geschichtliche-matrix.show.json");
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

#[test]
fn the_real_documents_audio_block_satisfies_the_schema() {
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("spec/fugue.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&schema).expect("fugue.schema.json compiles");
    let audio = act_one_document()["audio"].clone();
    let errors: Vec<String> = validator.iter_errors(&audio).map(|e| format!("{e}")).collect();
    assert!(errors.is_empty(), "schema errors: {errors:#?}");
}

#[test]
fn the_document_and_the_rust_carry_the_same_subject() {
    let audio = act_one_document()["audio"].clone();
    let spec: spex_fugue::FugueSpec = serde_json::from_value(audio).expect("parses into the model");
    assert_eq!(spec.subject, spex_fugue::act_one_subject());
    assert_eq!(spec.countersubject, spex_fugue::act_one_countersubject());
    assert_eq!(spec.mode, spex_fugue::act_one_key());
}

#[test]
fn the_plan_puts_the_exposition_where_the_screenplay_needs_it() {
    // The spec's own bar table: four entries at bars 5, 7, 11 and 14, so the
    // exposition is complete at bar 17 — which is the end of Act I. The bars
    // belong to the film, and this is the assertion that says so.
    let spec: spex_fugue::FugueSpec =
        serde_json::from_value(act_one_document()["audio"].clone()).unwrap();
    let exposition = spec
        .plan
        .iter()
        .find_map(|s| match s {
            spex_fugue::Section::Exposition { entries } => Some(entries),
            _ => None,
        })
        .expect("there is an exposition");
    assert_eq!(exposition.len(), 4, "four voices");
    let bars: Vec<f64> = exposition.iter().map(|e| e.at_bar).collect();
    assert_eq!(bars, vec![5.0, 7.0, 11.0, 14.0]);
    use spex_fugue::model::Transposition::*;
    let t: Vec<_> = exposition.iter().map(|e| e.transposition).collect();
    assert_eq!(t, vec![Tonic, Dominant, Tonic, Dominant]);
    // Alto, soprano, tenor, bass — the order the screenplay's shots name.
    let v: Vec<u32> = exposition.iter().map(|e| e.voice).collect();
    assert_eq!(v, vec![1, 0, 2, 3]);
    // The last entry starts at 14 and the subject is two bars, so the
    // exposition ends at 16 and the act at 17: one bar of air, deliberately.
    let subject_bars = spex_fugue::theory::line_beats(&spec.subject) / spec.meter[0] as f64;
    assert_eq!(subject_bars, 2.0);
    assert!(bars[3] + subject_bars <= 17.0);
}

#[test]
fn every_planned_section_lands_inside_the_canonical_eighty_four_bars() {
    let spec: spex_fugue::FugueSpec =
        serde_json::from_value(act_one_document()["audio"].clone()).unwrap();
    use spex_fugue::Section::*;
    for s in &spec.plan {
        let end = match s {
            Exposition { entries } => entries.iter().map(|e| e.at_bar).fold(0.0, f64::max) + 2.0,
            Episode { at_bar, bars, .. }
            | Stretto { at_bar, bars, .. }
            | Pedal { at_bar, bars, .. }
            | Cadence { at_bar, bars, .. } => at_bar + bars,
            Entry { at_bar, augmentation, .. } => at_bar + 2.0 * augmentation.unwrap_or(1.0),
        };
        assert!(end <= 84.0, "section runs past the cut: {s:?} ends at bar {end}");
    }
}
