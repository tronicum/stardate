//! M60 — the real show document, checked against both the schema and the
//! model.
//!
//! Two readers have to agree about `show.json`: `spec/show.schema.json`, which
//! is what any outside tool validates against, and `spex_show::model`, which is
//! what this project actually plays. A document that satisfies only one of them
//! is a format with two definitions. So every check here runs against the real
//! file in `shows/`, through a real JSON Schema validator, and through serde.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // crates/spex-show -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn schema() -> serde_json::Value {
    let path = repo_root().join("spec/show.schema.json");
    serde_json::from_str(&std::fs::read_to_string(&path).expect("reading show.schema.json"))
        .expect("show.schema.json is not valid JSON")
}

fn assert_valid(schema: &serde_json::Value, doc: &serde_json::Value, what: &str) {
    let compiled = jsonschema::validator_for(schema).expect("compiling show.schema.json");
    let errors: Vec<String> = compiled.iter_errors(doc).map(|e| format!("{} at {}", e, e.instance_path())).collect();
    assert!(errors.is_empty(), "{what} does not satisfy show.schema.json:\n  {}", errors.join("\n  "));
}

/// AC2: a minimal hand-written document — one movement, two shots, every
/// optional field left out — validates.
///
/// The real document exercises nearly every field, which is exactly why it
/// cannot answer this question: a schema that over-requires would still accept
/// it. This one is the smallest thing an author could reasonably write.
#[test]
fn a_minimal_document_validates() {
    let doc: serde_json::Value = serde_json::from_str(
        r#"{
          "version": 1,
          "id": "minimal",
          "title": "Minimal",
          "archiveSignature": "IA-2026-000",
          "baseDurationBars": 4,
          "seed": 0,
          "scenes": [],
          "movements": [{
            "id": "m1", "title": "One", "romanNumeral": "I",
            "shots": [
              { "id": "S01", "title": "A", "weight": 1, "scaling": "fixed",
                "durationBars": 2, "tier": 1,
                "camera": { "mode": "keyed", "keys": [{ "t": 0, "value": {} }] } },
              { "id": "S02", "title": "B", "weight": 1, "scaling": "stretch",
                "durationBars": 2, "tier": 1,
                "camera": { "mode": "orbit",
                  "orbit": { "center": [0,0,0], "radius": 100, "height": 20,
                             "startDeg": 0, "endDeg": 180 } } }
            ]
          }]
        }"#,
    )
    .unwrap();

    assert_valid(&schema(), &doc, "the minimal document");
    let show = spex_show::from_str(&doc.to_string()).expect("the model rejects the minimal document");
    assert_eq!(show.movements[0].shots.len(), 2);
}

/// `show-resolved.schema.json` has no producer until M61, and a schema that
/// has never had a valid instance is a schema nobody has checked. This is the
/// smallest resolved show that satisfies it — written by hand, and therefore
/// also the contract M61's resolver has to hit.
///
/// The numbers are real: two bars at 84 bpm 4/4 is 40/7 s, and the second shot
/// starts where the first ends.
#[test]
fn a_minimal_resolved_document_validates() {
    let schema_path = repo_root().join("spec/show-resolved.schema.json");
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&schema_path).unwrap()).unwrap();

    let doc: serde_json::Value = serde_json::from_str(
        r#"{
          "version": 1,
          "generator": "spex-show 0.1.0",
          "id": "minimal",
          "title": "Minimal",
          "archiveSignature": "IA-2026-000",
          "tempo": { "bpm": 84, "beatsPerBar": 4,
                     "barSeconds": 2.857142857142857, "beatSeconds": 0.7142857142857143 },
          "targetSec": 5.714285714285714,
          "durationSec": 5.714285714285714,
          "endless": false,
          "seed": 0,
          "scenes": [{ "id": "brick", "prefix": "brick", "bundle": "bundles/brick", "instanceCount": 1 }],
          "shots": [
            { "id": "S01", "title": "A", "movementId": "m1", "tier": 1,
              "startSec": 0, "endSec": 2.857142857142857, "durationSec": 2.857142857142857,
              "startBar": 0, "durationBars": 1,
              "camera": { "mode": "keyed",
                "keys": [{ "timeSec": 0, "easing": "linear", "value": { "position": [0, 5, 300] } }] } },
            { "id": "S02", "title": "B", "movementId": "m1", "tier": 1,
              "startSec": 2.857142857142857, "endSec": 5.714285714285714,
              "durationSec": 2.857142857142857, "startBar": 1, "durationBars": 1,
              "camera": { "mode": "keyed",
                "keys": [{ "timeSec": 2.857142857142857, "easing": "linear", "value": { "fovDeg": 28 } }] },
              "tracks": [
                { "kind": "dissolve",
                  "target": { "glob": "brick/*", "scene": "brick", "instances": [0] },
                  "keys": [{ "timeSec": 2.857142857142857, "value": 1, "easing": "step" }] }
              ],
              "cues": [{ "timeSec": 4, "kind": "marker", "shotId": "S02", "payload": {} }] }
          ]
        }"#,
    )
    .unwrap();

    assert_valid(&schema, &doc, "the minimal resolved document");
}

#[test]
fn the_real_act_one_document_satisfies_the_schema_and_the_model() {
    let path = repo_root().join("shows/die-geschichtliche-matrix.show.json");
    let text = std::fs::read_to_string(&path).expect("reading the Act I document");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("the document is not valid JSON");
    assert_valid(&schema(), &doc, "shows/die-geschichtliche-matrix.show.json");

    let show = spex_show::from_str(&text).expect("the model rejects the Act I document");
    assert_eq!(show.archive_signature, "IA-2026-002");
    assert_eq!(show.movements.len(), 1);
    assert_eq!(show.movements[0].shots.len(), 6);
}

/// The screenplay says Act I is bars 0..17 and ends at 0:48.571. If the
/// document's own shots do not add up to that, one of the two is wrong — and
/// silently disagreeing with the screenplay is the failure this whole format
/// exists to prevent.
#[test]
fn act_one_is_seventeen_bars_and_ends_where_the_screenplay_says() {
    let path = repo_root().join("shows/die-geschichtliche-matrix.show.json");
    let show = spex_show::load(&path).unwrap();

    let bars = show.movements[0].duration_bars();
    assert_eq!(bars, 17.0, "Act I should be 17 bars");
    assert_eq!(bars, show.base_duration_bars, "baseDurationBars should be what the document actually contains");

    // 17 x 20/7 = 48.571428...
    let end = show.base_duration_sec();
    assert!((end - 340.0 / 7.0).abs() < 1e-9, "Act I ends at {end}, not 48.571…");

    // The screenplay's per-shot bar counts, in order.
    let got: Vec<f64> = show.movements[0].shots.iter().map(|s| s.duration_bars).collect();
    assert_eq!(got, vec![2.0, 2.0, 3.0, 4.0, 3.0, 3.0]);
}

/// Every track target must start with a scene prefix the document defines,
/// and must be able to reach a real instance id.
///
/// A mesh bundle's own `instanceIds` are `<part>/<n>` — `3010/0`, `3710/7` —
/// so a scene-prefixed id is `monolith/3010/0`: **two** separators, not one.
/// A `*` does not cross a separator (that is what makes `flag/dk/tile-*`
/// meaningful), so `monolith/*` matches nothing at all, silently, and the
/// shot simply does not animate. This was a real defect in the first draft of
/// the Act I document, found by running `spex mesh-model` on the real scenes
/// and reading the ids rather than assuming them.
#[test]
fn every_track_target_names_a_real_scene_prefix_and_can_reach_an_instance() {
    let path = repo_root().join("shows/die-geschichtliche-matrix.show.json");
    let show = spex_show::load(&path).unwrap();
    let prefixes: Vec<&str> = show.scenes.iter().map(|s| s.prefix.as_str()).collect();

    let target_of = |t: &spex_show::Track| -> Option<String> {
        match t {
            spex_show::Track::Transform { target, .. }
            | spex_show::Track::Dissolve { target, .. }
            | spex_show::Track::Material { target, .. }
            | spex_show::Track::PointCloud { target, .. } => Some(target.clone()),
            // A post or HUD track addresses the frame, not the geometry.
            spex_show::Track::Post { .. } | spex_show::Track::Hud { .. } => None,
        }
    };

    for (movement, shot) in show.shots() {
        for target in shot.tracks.iter().filter_map(target_of) {
            let (prefix, rest) = target.split_once('/').unwrap_or((target.as_str(), ""));
            assert!(
                prefixes.contains(&prefix),
                "{}/{}: target {target:?} starts with {prefix:?}, which is no scene's prefix",
                movement.id,
                shot.id
            );
            assert!(
                !rest.contains('*') || rest.contains("**") || !rest.ends_with('*'),
                "{}/{}: target {target:?} uses a single `*` after the scene prefix, which cannot \
                 cross the `/` in a `<part>/<n>` instance id and so matches nothing",
                movement.id,
                shot.id
            );
        }
    }
}

/// A shot that names a scene the document never defines would fail at build
/// time, in the middle of compiling bundles, with a message about a missing
/// file. `validate` catches it while it is still a typo.
#[test]
fn every_scene_a_shot_names_is_defined() {
    let path = repo_root().join("shows/die-geschichtliche-matrix.show.json");
    // `load` already runs validate() and would have bailed; this asserts the
    // specific property rather than trusting that it was among the checks.
    let show = spex_show::load(&path).unwrap();
    let defined: Vec<&str> = show.scenes.iter().map(|s| s.id.as_str()).collect();
    for (movement, shot) in show.shots() {
        for scene in &shot.scenes {
            assert!(
                defined.contains(&scene.as_str()),
                "{}/{} references undefined scene {scene:?}",
                movement.id,
                shot.id
            );
        }
    }
}
