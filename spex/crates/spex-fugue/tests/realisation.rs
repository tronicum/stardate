//! M68 AC1–AC3 — measured on the score that actually comes out.
//!
//! Every check here runs against notes read back out of the **emitted MIDI
//! file**, not against the `Realisation` the generator was holding. That
//! distinction is the whole point of `emit::read_smf` existing: analysing the
//! in-memory structure would test the generator against itself and would pass
//! unchanged if the writer dropped every second note.

use spex_fugue::counterpoint::{self, Placed};
use std::path::PathBuf;

fn spec() -> spex_fugue::FugueSpec {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../shows/die-geschichtliche-matrix.show.json");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    serde_json::from_value(doc["audio"].clone()).unwrap()
}

/// The emitted file, read back into the same shape the rules take.
///
/// **Channel 10 is not a voice.** M71 put the Act IV pulse into the same file
/// as a General MIDI drum track, which is right — one artefact, and a DAW
/// shows the layer — and which means a reader that takes every channel hands
/// the counterpoint rules a kick drum. Note 36 against note 39 is a minor
/// third that never moves, so the range rule and the parallel rule both fired
/// on it immediately. Filtering here rather than in `read_smf` is deliberate:
/// the reader's job is to report what is in the file.
fn round_tripped() -> Vec<Placed> {
    let spec = spec();
    let r = spex_fugue::realise(&spec, 263_865);
    let bytes = spex_fugue::to_smf(&r);
    let notes = spex_fugue::read_smf(&bytes).expect("the file we just wrote is readable");
    let tpb = spex_fugue::emit::TICKS_PER_BEAT as f64;
    notes
        .into_iter()
        .filter(|(voice, _, _, _)| *voice < 4)
        .map(|(voice, tick, midi, dur)| Placed {
            voice,
            at_beat: tick as f64 / tpb,
            beats: dur as f64 / tpb,
            midi,
        })
        .collect()
}

#[test]
fn ac1_the_emitted_score_has_no_parallel_fifths_or_octaves() {
    let notes = round_tripped();
    assert!(!notes.is_empty(), "the generator produced nothing at all");
    let parallels = counterpoint::parallel_perfects(&notes);
    let shown: Vec<String> = parallels
        .iter()
        .take(10)
        .map(|(t, a, b)| format!("bar {:.2}: {} / {}", t / 4.0 + 1.0, counterpoint::VOICE_NAMES[*a], counterpoint::VOICE_NAMES[*b]))
        .collect();
    assert!(
        parallels.is_empty(),
        "{} parallel perfect consonances in the emitted score:\n  {}",
        parallels.len(),
        shown.join("\n  ")
    );
}

#[test]
fn ac2_the_exposition_is_four_entries_alternating_tonic_and_dominant() {
    use spex_fugue::model::Transposition::*;
    let spec = spec();
    let r = spex_fugue::realise(&spec, 263_865);
    // Only the exposition's own entries: the plan has later ones too.
    let exposition: Vec<_> = r.entries.iter().filter(|e| e.at_bar <= 16.0).collect();
    assert_eq!(exposition.len(), 4, "four voices, once each");
    assert_eq!(
        exposition.iter().map(|e| e.transposition).collect::<Vec<_>>(),
        vec![Tonic, Dominant, Tonic, Dominant]
    );
    assert_eq!(exposition.iter().map(|e| e.voice).collect::<Vec<_>>(), vec![1, 0, 2, 3]);
    assert_eq!(
        exposition.iter().map(|e| e.at_bar).collect::<Vec<_>>(),
        vec![5.0, 7.0, 11.0, 14.0]
    );
    // And each of the four really put notes on the page.
    for e in &exposition {
        let n = r.notes.iter().filter(|n| n.voice == e.voice && (n.at_beat - e.at_beat).abs() < 1e-9).count();
        assert!(n > 0, "{:?} entry at bar {} sounded nothing", e.transposition, e.at_bar);
    }
}

#[test]
fn ac3_the_stretto_entries_genuinely_overlap() {
    let spec = spec();
    let r = spex_fugue::realise(&spec, 263_865);
    let subject_beats = spex_fugue::theory::line_beats(&spec.subject);
    let stretto: Vec<_> = r.entries.iter().filter(|e| e.at_bar >= 53.0 && e.at_bar < 57.0).collect();
    assert!(stretto.len() >= 2, "a stretto needs at least two entries, got {}", stretto.len());
    for w in stretto.windows(2) {
        let gap = w[1].at_beat - w[0].at_beat;
        assert!(gap > 0.0, "entries must be in order");
        assert!(
            gap < subject_beats,
            "entries {gap} beats apart do not overlap a {subject_beats}-beat subject — \
             which is the only thing that makes a stretto a stretto"
        );
    }
}

#[test]
fn the_same_seed_realises_identically_and_a_different_one_does_not() {
    let spec = spec();
    let a = spex_fugue::to_smf(&spex_fugue::realise(&spec, 263_865));
    let b = spex_fugue::to_smf(&spex_fugue::realise(&spec, 263_865));
    assert_eq!(a, b, "the same seed must give byte-identical output");
    let c = spex_fugue::to_smf(&spex_fugue::realise(&spec, 1));
    assert_ne!(a, c, "a different seed that changes nothing is a seed that does nothing");
}

#[test]
fn every_note_outside_its_voices_range_is_a_recorded_relaxation() {
    // Not "there are none" — the design does not promise that, and promising
    // it would mean transposing the subject to make a range work.
    //
    // The subject spans a ninth: A3 to B4 at the tonic, fourteen semitones.
    // The tenor's stated range is C3–A4, twenty-one semitones, which is wider
    // — and it still does not fit, because D4 sits near the top of it: the
    // subject at the tonic goes two semitones over the ceiling, and an octave
    // down it goes three under the floor. There is no octave that works. The
    // same happens to the bass on the answer and to the soprano on the entry
    // at bar 21.
    //
    // The generator's rule is that **the subject does not move to make a rule
    // pass** — it is the thing the piece is about — so it places it at the
    // least-bad octave and records the breach. What this test checks is that
    // the record is complete: every note over the line has a relaxation
    // naming its voice.
    let spec = spec();
    let r = spex_fugue::realise(&spec, 263_865);
    let notes = round_tripped();
    let bad = counterpoint::out_of_range(&notes);
    for n in &bad {
        let covered = r
            .relaxations
            .iter()
            .any(|x| x.rule == counterpoint::Rule::Range && x.voice == n.voice);
        assert!(
            covered,
            "{} at MIDI {} in bar {:.0} is out of range and nothing recorded it",
            counterpoint::VOICE_NAMES[n.voice],
            n.midi,
            n.at_beat / 4.0 + 1.0
        );
    }
    // And the breach is small. A voice a whole tone over its ceiling is a
    // stretch; a voice a fifth over it is a different instrument.
    for n in &bad {
        let (lo, hi) = counterpoint::VOICE_RANGES[n.voice];
        let over = (lo - n.midi).max(n.midi - hi);
        assert!(over <= 2, "{} is {over} semitones outside its range", counterpoint::VOICE_NAMES[n.voice]);
    }
    eprintln!("{} notes outside a stated range, all recorded", bad.len());
}

#[test]
fn the_score_reaches_the_end_of_the_canonical_cut() {
    let notes = round_tripped();
    let last = notes.iter().map(|n| n.at_beat + n.beats).fold(0.0, f64::max);
    // 84 bars at 4/4 is 336 beats. The final accent is in the last half-bar.
    assert!(last > 330.0, "the score stops at beat {last}, well before the cut ends");
    assert!(last <= 336.5, "the score runs past the cut, ending at beat {last}");
}

#[test]
fn whatever_could_not_be_satisfied_is_recorded_rather_than_hidden() {
    let spec = spec();
    let r = spex_fugue::realise(&spec, 263_865);
    // Not "there are no relaxations" — that would be a claim about the music
    // this generator happens to produce today. The claim that matters is that
    // the record is *honest*: every relaxation names a rule, a bar and a
    // voice, so a reader can go and listen to it.
    for x in &r.relaxations {
        assert!(x.bar >= 0.0 && x.bar <= 84.0, "relaxation outside the piece: {x:?}");
        assert!(x.voice < 4);
        assert!(!x.detail.is_empty());
    }
    eprintln!(
        "relaxations: {} ({} parallel, {} range, {} crossing, {} dissonance)",
        r.relaxations.len(),
        r.relaxations.iter().filter(|x| x.rule == counterpoint::Rule::ParallelPerfect).count(),
        r.relaxations.iter().filter(|x| x.rule == counterpoint::Rule::Range).count(),
        r.relaxations.iter().filter(|x| x.rule == counterpoint::Rule::VoiceCrossing).count(),
        r.relaxations.iter().filter(|x| x.rule == counterpoint::Rule::WeakBeatDissonance).count(),
    );
}
