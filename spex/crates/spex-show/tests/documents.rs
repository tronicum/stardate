//! M60 — the real show document, checked against both the schema and the
//! model.
//!
//! Two readers have to agree about `show.json`: `spec/show.schema.json`, which
//! is what any outside tool validates against, and `spex_show::model`, which is
//! what this project actually plays. A document that satisfies only one of them
//! is a format with two definitions. So every check here runs against the real
//! file in `shows/`, through a real JSON Schema validator, and through serde.

use std::path::{Path, PathBuf};

/// Resolves the one cross-file `$ref` this format has, from disk rather than
/// from the network. Anything else is an error rather than a silent skip.
struct SiblingSchemas {
    fugue: serde_json::Value,
}

impl jsonschema::Retrieve for SiblingSchemas {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        if uri.as_str().ends_with("fugue.schema.json") {
            Ok(self.fugue.clone())
        } else {
            Err(format!("no local copy of {uri}").into())
        }
    }
}

fn repo_root() -> PathBuf {
    // crates/spex-show -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn schema() -> serde_json::Value {
    let path = repo_root().join("spec/show.schema.json");
    serde_json::from_str(&std::fs::read_to_string(&path).expect("reading show.schema.json"))
        .expect("show.schema.json is not valid JSON")
}

/// `show.schema.json` refers to `fugue.schema.json` by `$ref`, and a validator
/// with no retriever tries to fetch that over HTTP and fails. Registering the
/// sibling file as a resource is the honest fix: the two schemas really are one
/// format described in two files, and a test that dodged the reference by
/// loosening `audio` to "any object" would be validating nothing where the
/// music is.
fn assert_valid(schema: &serde_json::Value, doc: &serde_json::Value, what: &str) {
    let fugue: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("spec/fugue.schema.json"))
            .expect("reading fugue.schema.json"),
    )
    .expect("fugue.schema.json is not valid JSON");
    let compiled = jsonschema::options()
        .with_retriever(SiblingSchemas { fugue })
        .build(schema)
        .expect("compiling show.schema.json");
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
          "beatAligned": true,
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
                  "target": { "glob": "brick/**", "scene": "brick", "instances": [0] },
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
fn the_real_document_satisfies_the_schema_and_the_model() {
    let path = repo_root().join("shows/die-geschichtliche-matrix.show.json");
    let text = std::fs::read_to_string(&path).expect("reading the piece");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("the document is not valid JSON");
    assert_valid(&schema(), &doc, "shows/die-geschichtliche-matrix.show.json");

    let show = spex_show::from_str(&text).expect("the model rejects the document");
    assert_eq!(show.archive_signature, "IA-2026-002");
    // Four movements, and the shot counts the screenplay's own table gives.
    // Asserted per movement rather than as a total, because a shot moving
    // between acts is exactly the mistake a total would not notice.
    let shots: Vec<usize> = show.movements.iter().map(|m| m.shots.len()).collect();
    assert_eq!(shots, vec![6, 5, 6, 6], "shots per movement");
}

/// The screenplay gives every act a bar count, and the whole piece 84 bars =
/// 240.000 s. If the document's own shots do not add up to that, one of the
/// two is wrong — and silently disagreeing with the screenplay is the failure
/// this whole format exists to prevent.
///
/// **This test asserted 17 for two acts.** It was written when the document
/// was Act I alone, and from the moment Act II landed it failed on a number
/// that was not wrong — which is worse than useless, because three tests being
/// red is three tests nobody reads, and the two beside it were catching real
/// things the whole time. The numbers below are per act and are the
/// screenplay's, so the next act to land fails this on purpose, once, and in
/// the one place where the two documents are meant to be compared.
#[test]
fn every_act_is_as_long_as_the_screenplay_says_and_they_sum_to_two_forty() {
    let path = repo_root().join("shows/die-geschichtliche-matrix.show.json");
    let show = spex_show::load(&path).unwrap();

    let bars: Vec<f64> = show.movements.iter().map(|m| m.duration_bars()).collect();
    assert_eq!(bars, vec![17.0, 20.0, 20.0, 21.0], "bars per act");

    // 78 authored, 84 declared. THE SIX-BAR GAP IS THE ATLAS (bars 57-63),
    // which depends on M73/M74/M75 and is not built — so the resolver
    // water-fills those six bars across the stretch shots that do exist. The
    // gap is asserted rather than tolerated: it is the one place in the
    // repository where "a movement of this piece does not exist yet" is a
    // number, and the day the Atlas lands this fails and says so.
    let total: f64 = bars.iter().sum();
    assert_eq!(total, 78.0, "authored bars");
    assert_eq!(show.base_duration_bars, 84.0, "declared bars");
    assert_eq!(show.base_duration_bars - total, 6.0, "the unbuilt ATLAS, bars 57-63");

    // 84 x 20/7 is exactly 240, and nothing else nearby is.
    let end = show.base_duration_sec();
    assert!((end - 240.0).abs() < 1e-9, "the piece ends at {end}, not 240.000");

    // Act I's per-shot bar counts, in order — the one act whose shape the
    // screenplay states shot by shot rather than in a table.
    let got: Vec<f64> = show.movements[0].shots.iter().map(|s| s.duration_bars).collect();
    assert_eq!(got, vec![2.0, 2.0, 3.0, 4.0, 3.0, 3.0]);
}

/// The piece has four voices, and the director HUD should say four.
///
/// `player.ts` labels a voice entry `"${voice} ${range}"` and keeps the
/// distinct labels it has seen, so a single `voiceEntry` cue that spells the
/// same voice differently makes the instrument report a fifth voice in a
/// four-voice fugue. That is exactly what the document did: A1-S03 wrote
/// `"alto"` and A3-S01 and all four of A3-S05's stretto entries wrote `"alt"`,
/// so the HUD listed voice 1 twice as soon as Act III began.
///
/// It costs nothing to be wrong about and nothing to check, which is the
/// argument for checking it rather than for being careful.
#[test]
fn each_voice_has_exactly_one_name() {
    use std::collections::BTreeMap;

    let path = repo_root().join("shows/die-geschichtliche-matrix.show.json");
    let show = spex_show::load(&path).unwrap();

    let mut names: BTreeMap<u64, Vec<String>> = BTreeMap::new();
    for (_, shot) in show.shots() {
        for cue in &shot.cues {
            if cue.payload.get("event").and_then(|v| v.as_str()) != Some("voiceEntry") {
                continue;
            }
            let voice = cue.payload.get("voice").and_then(|v| v.as_u64()).expect("a voiceEntry names a voice");
            let range = cue.payload.get("range").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let seen = names.entry(voice).or_default();
            if !seen.contains(&range) {
                seen.push(range);
            }
        }
    }

    for (voice, spellings) in &names {
        assert_eq!(
            spellings.len(),
            1,
            "voice {voice} is spelled {spellings:?} — the HUD would list it more than once"
        );
    }
    assert_eq!(names.len(), 4, "the fugue is in four voices: {names:?}");
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
            | spex_show::Track::Color { target, .. }
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

/// Keyframes authored at the same shot-local `t` must resolve to the same
/// absolute second.
///
/// A1-S03 briefly did not. Its crossfade and its edge arrival were marked
/// `snapToBeat` and its rotation was not, so the resolver moved two of the
/// three onto beat 51 and left the third 0.343 s earlier — the brick began
/// turning before the outlines it was meant to reveal had arrived. Nothing
/// failed; the numbers were simply different, and only reading the resolved
/// output showed it. That is the failure mode beat snapping has, so this is
/// the check it gets.
#[test]
fn keys_authored_at_the_same_moment_resolve_to_the_same_moment() {
    use std::collections::BTreeMap;

    let path = repo_root().join("shows/die-geschichtliche-matrix.show.json");
    let show = spex_show::load(&path).unwrap();
    let resolved = spex_show::resolve(
        &show,
        &spex_show::ResolveOptions { target_sec: 240.0, seed: show.seed, endless: false },
    )
    .unwrap();

    let key_times = |t: &spex_show::ResolvedTrack| -> Vec<f64> {
        match t {
            spex_show::ResolvedTrack::Transform { keys, .. } => keys.iter().map(|k| k.time_sec).collect(),
            spex_show::ResolvedTrack::Color { keys, .. } => keys.iter().map(|k| k.time_sec).collect(),
            spex_show::ResolvedTrack::Dissolve { keys, .. }
            | spex_show::ResolvedTrack::Material { keys, .. }
            | spex_show::ResolvedTrack::Post { keys, .. }
            | spex_show::ResolvedTrack::Hud { keys, .. }
            | spex_show::ResolvedTrack::PointCloud { keys, .. } => keys.iter().map(|k| k.time_sec).collect(),
        }
    };
    let src_times = |t: &spex_show::Track| -> Vec<f64> {
        match t {
            spex_show::Track::Transform { keys, .. } => keys.iter().map(|k| k.t).collect(),
            spex_show::Track::Color { keys, .. } => keys.iter().map(|k| k.t).collect(),
            spex_show::Track::Dissolve { keys, .. }
            | spex_show::Track::Material { keys, .. }
            | spex_show::Track::Post { keys, .. }
            | spex_show::Track::Hud { keys, .. }
            | spex_show::Track::PointCloud { keys, .. } => keys.iter().map(|k| k.t).collect(),
        }
    };

    for (_, shot) in show.shots() {
        let Some(rshot) = resolved.shots.iter().find(|s| s.id == shot.id) else { continue };
        // Authored t (as an exact bit pattern, since these are literals in the
        // document) -> the absolute seconds it resolved to.
        let mut by_t: BTreeMap<u64, Vec<f64>> = BTreeMap::new();
        for (src, res) in shot.tracks.iter().zip(rshot.tracks.iter()) {
            for (t, sec) in src_times(src).into_iter().zip(key_times(res)) {
                by_t.entry(t.to_bits()).or_default().push(sec);
            }
        }
        for (bits, secs) in by_t {
            let t = f64::from_bits(bits);
            let first = secs[0];
            assert!(
                secs.iter().all(|s| (s - first).abs() < 1e-9),
                "{}: keys authored at t={t} resolved to {secs:?} — one of them is missing \
                 `snapToBeat`, so the beat they were meant to share is not shared",
                shot.id
            );
        }
    }
}
