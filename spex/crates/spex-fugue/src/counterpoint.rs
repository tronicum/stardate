//! M68 — the counterpoint generator.
//!
//! Turns a [`crate::FugueSpec`] into concrete notes for four voices, under
//! rules that are **checkable predicates rather than good intentions**. Every
//! rule below is a named function that returns *where* it was broken, and the
//! generator's own output is re-analysed by those same functions in the tests.
//! Nothing here asserts that the music is correct; it measures it.
//!
//! # What is placed and what is chosen
//!
//! Two very different jobs, and conflating them is how generated fugues end up
//! sounding like generated fugues.
//!
//! - **Entries are placed.** A subject statement is the fugue's fixed
//!   material; it goes in literally, transposed and octave-shifted only as far
//!   as the voice's range demands. It is never adjusted to make a rule pass —
//!   if an entry creates a parallel, the *other* voice moves, and if it cannot,
//!   the breach is recorded. The subject is the thing the piece is about.
//! - **Free voices are chosen.** Everything that is not an entry or the
//!   countersubject is picked slot by slot from the notes the rules allow,
//!   scored, and tie-broken deterministically. That is a greedy chooser and not
//!   a solver, which is an honest limitation with a real consequence: it can
//!   paint itself into a corner that a search would have avoided. When it does,
//!   it relaxes a rule in the fixed order below and says so.
//!
//! # The relaxation order, and why it is this order
//!
//! When nothing satisfies every rule, they are given up in this sequence —
//! least musically damaging first:
//!
//! 1. `WeakBeatDissonance` — a dissonance somewhere it is merely inelegant.
//! 2. `VoiceCrossing` — two voices swap; the harmony is intact, the texture
//!    muddies.
//! 3. `Range` — a voice goes a little outside its comfortable compass.
//! 4. `ParallelPerfect` — last, because parallel fifths are the one thing a
//!    listener hears as *wrong* rather than as *odd*.
//!
//! Every relaxation lands in [`Realisation::relaxations`] with its bar, its
//! voice and its rule. A score that had to break something says which thing
//! and where, and that is the same standard this project holds its geometry
//! to: honest output over pretend-perfect output.
//!
//! # Determinism
//!
//! From `(seed, spec)` and nothing else. The generator is **splitmix64**, the
//! same twelve-line specified algorithm `spex_show::choreography` uses, and it
//! is written out again here rather than shared — `spex-show` depends on this
//! crate, so a shared helper would be a dependency cycle. Two copies of a
//! *specified* algorithm are safe in a way two copies of a hash function are
//! not, and `splitmix64_matches_the_shared_fixture` pins this one to the same
//! recorded values the other is pinned to.

use crate::model::{CadenceType, FugueSpec, MotifSource, Section, Transposition};
use crate::theory::{answer, invert, line_beats, scale_durations, transpose_steps, Key, Line};

/// The golden-ratio odd constant splitmix64 is defined with.
pub const SPLITMIX_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// One step of splitmix64. Advances `state` and returns the mixed output.
pub fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(SPLITMIX_GAMMA);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The four voices, and their real ranges in MIDI note numbers.
///
/// The spec's own numbers: S C4–A5, A F3–D5, T C3–A4, B E2–C4. These are
/// *singers'* ranges rather than instrument ranges, which matters — the whole
/// point of writing a fugue in four voices is that four people could sing it,
/// and a bass line that dives to C2 has stopped being counterpoint and become
/// a synthesiser part.
pub const VOICE_RANGES: [(i32, i32); 4] = [
    (60, 81), // soprano  C4–A5
    (53, 74), // alto     F3–D5
    (48, 69), // tenor    C3–A4
    (40, 60), // bass     E2–C4
];

pub const VOICE_NAMES: [&str; 4] = ["Soprano", "Alto", "Tenor", "Bass"];

/// One sounding note, placed in absolute time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub voice: usize,
    /// Beats from the start of the piece.
    pub at_beat: f64,
    pub beats: f64,
    pub midi: i32,
}

impl Placed {
    pub fn end_beat(&self) -> f64 {
        self.at_beat + self.beats
    }
    pub fn sounds_at(&self, beat: f64) -> bool {
        beat >= self.at_beat - 1e-9 && beat < self.end_beat() - 1e-9
    }
}

/// Which rule was broken, where.
#[derive(Debug, Clone, PartialEq)]
pub struct Relaxation {
    pub rule: Rule,
    pub at_beat: f64,
    pub bar: f64,
    pub voice: usize,
    pub detail: String,
}

/// The rules, in the order they are given up. `Ord` follows that order, so
/// "relax the least damaging thing available" is `min`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    WeakBeatDissonance,
    VoiceCrossing,
    Range,
    ParallelPerfect,
}

impl Rule {
    pub fn name(self) -> &'static str {
        match self {
            Rule::WeakBeatDissonance => "weak-beat dissonance",
            Rule::VoiceCrossing => "voice crossing",
            Rule::Range => "voice range",
            Rule::ParallelPerfect => "parallel perfect consonance",
        }
    }
}

/// A realised score: notes, and an honest account of what it could not do.
#[derive(Debug, Clone, PartialEq)]
pub struct Realisation {
    pub notes: Vec<Placed>,
    pub relaxations: Vec<Relaxation>,
    /// Where each subject or answer statement begins, for the tests and for
    /// anyone reading the score.
    pub entries: Vec<EntryMark>,
    pub key: Key,
    pub bpm: f64,
    pub beats_per_bar: f64,
    pub seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntryMark {
    pub voice: usize,
    pub at_bar: f64,
    pub at_beat: f64,
    pub transposition: Transposition,
    pub inverted: bool,
    pub augmentation: f64,
}

// ------------------------------------------------------------------- rules
//
// Each is a predicate over already-placed notes. They take the whole score
// rather than a pair of lines because that is the question that actually
// matters: "does this note work" is a question about every other voice
// sounding at that moment, not about one of them.

/// Every distinct moment at which any note starts.
fn onsets(notes: &[Placed]) -> Vec<f64> {
    let mut t: Vec<f64> = notes.iter().map(|n| n.at_beat).collect();
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    t.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    t
}

/// What each voice is sounding at a beat, or `None` for silence.
fn chord_at(notes: &[Placed], beat: f64) -> [Option<i32>; 4] {
    let mut out = [None; 4];
    for n in notes {
        if n.sounds_at(beat) {
            out[n.voice] = Some(n.midi);
        }
    }
    out
}

/// Consecutive perfect fifths or octaves between any pair of voices.
///
/// Returns the beat and the pair. A *repeated* perfect consonance where
/// neither voice moved is not a parallel — that is a pedal, and the piece has
/// one by design in its last four bars.
pub fn parallel_perfects(notes: &[Placed]) -> Vec<(f64, usize, usize)> {
    let times = onsets(notes);
    let mut out = Vec::new();
    for w in times.windows(2) {
        let (a, b) = (chord_at(notes, w[0]), chord_at(notes, w[1]));
        for hi in 0..4 {
            for lo in (hi + 1)..4 {
                let (Some(a_hi), Some(a_lo), Some(b_hi), Some(b_lo)) = (a[hi], a[lo], b[hi], b[lo])
                else {
                    continue;
                };
                if a_hi == b_hi && a_lo == b_lo {
                    continue; // nothing moved
                }
                let ia = (a_hi - a_lo).rem_euclid(12);
                let ib = (b_hi - b_lo).rem_euclid(12);
                if ia == ib && (ia == 0 || ia == 7) {
                    out.push((w[1], hi, lo));
                }
            }
        }
    }
    out
}

/// Voices sounding out of order — a lower-numbered voice below a
/// higher-numbered one.
pub fn voice_crossings(notes: &[Placed]) -> Vec<(f64, usize, usize)> {
    let mut out = Vec::new();
    for t in onsets(notes) {
        let c = chord_at(notes, t);
        for hi in 0..4 {
            for lo in (hi + 1)..4 {
                if let (Some(h), Some(l)) = (c[hi], c[lo]) {
                    if h < l {
                        out.push((t, hi, lo));
                    }
                }
            }
        }
    }
    out
}

/// Notes outside their voice's real range.
pub fn out_of_range(notes: &[Placed]) -> Vec<&Placed> {
    notes
        .iter()
        .filter(|n| {
            let (lo, hi) = VOICE_RANGES[n.voice];
            n.midi < lo || n.midi > hi
        })
        .collect()
}

/// Is this interval a dissonance? Seconds, sevenths and the tritone, reduced
/// into one octave. A fourth is left out on purpose: between upper voices it
/// is a consonance, and the one place it is not — against the bass — is
/// handled by `is_dissonant_against_bass`.
pub fn is_dissonant(semitones: i32) -> bool {
    matches!(semitones.rem_euclid(12), 1 | 2 | 6 | 10 | 11)
}

/// The same, plus the fourth, for an interval measured against the lowest
/// sounding voice.
pub fn is_dissonant_against_bass(semitones: i32) -> bool {
    is_dissonant(semitones) || semitones.rem_euclid(12) == 5
}

/// Dissonances that fall on a beat without being prepared.
///
/// Off-beat dissonance is a passing tone and is what makes a line move; this
/// looks only at beats. A note that was already sounding at the same pitch on
/// the previous onset is a **suspension** — prepared, and allowed.
pub fn unprepared_dissonances(notes: &[Placed]) -> Vec<(f64, usize)> {
    let times = onsets(notes);
    let mut out = Vec::new();
    for (i, t) in times.iter().enumerate() {
        if (t - t.round()).abs() > 1e-9 {
            continue; // between beats: passing
        }
        let c = chord_at(notes, *t);
        let bass = (0..4).rev().find_map(|v| c[v]);
        let prev = if i == 0 { None } else { Some(chord_at(notes, times[i - 1])) };
        for v in 0..4 {
            let Some(m) = c[v] else { continue };
            let Some(b) = bass else { continue };
            if m == b {
                continue;
            }
            let dissonant = if Some(m) == c.iter().rev().flatten().next().copied() {
                false
            } else {
                is_dissonant_against_bass(m - b)
            };
            if !dissonant {
                continue;
            }
            // Prepared: the same pitch was already sounding a moment ago.
            let prepared = prev.map(|p| p[v] == Some(m)).unwrap_or(false);
            if !prepared {
                out.push((*t, v));
            }
        }
    }
    out
}

/// The leading tone must rise at a cadence.
///
/// In a mode rather than a key this needs saying carefully: Dorian has no
/// leading tone of its own — its seventh is a whole step below the tonic — and
/// the classical answer is *musica ficta*, raising it at cadences only. So the
/// rule here is about the raised seventh where the generator has written one,
/// and `cadence_beats` says where to look.
pub fn leading_tone_resolutions(notes: &[Placed], key: Key, cadences: &[f64]) -> Vec<(f64, usize)> {
    let tonic_pc = key.tonic_midi.rem_euclid(12);
    let leading_pc = (tonic_pc - 1).rem_euclid(12);
    let times = onsets(notes);
    let mut out = Vec::new();
    for c in cadences {
        let Some(i) = times.iter().position(|t| *t >= *c - 1e-9) else { continue };
        if i + 1 >= times.len() {
            continue;
        }
        let here = chord_at(notes, times[i]);
        let next = chord_at(notes, times[i + 1]);
        for v in 0..4 {
            let (Some(a), Some(b)) = (here[v], next[v]) else { continue };
            if a.rem_euclid(12) == leading_pc && b - a != 1 {
                out.push((times[i], v));
            }
        }
    }
    out
}

// ------------------------------------------------------------- realisation

/// Realise a plan into notes.
///
/// Deterministic: the same `(seed, spec)` produces byte-identical output, which
/// is what makes an edition reproducible and what
/// `the_same_seed_realises_identically` asserts.
pub fn realise(spec: &FugueSpec, seed: u64) -> Realisation {
    let key = spec.mode;
    let bpb = spec.meter[0] as f64;
    let mut r = Realisation {
        notes: Vec::new(),
        relaxations: Vec::new(),
        entries: Vec::new(),
        key,
        bpm: spec.bpm,
        beats_per_bar: bpb,
        seed,
    };

    // Pass 1: place every entry, and the countersubject that accompanies it.
    // Entries first and all of them, because they are the fixed material and
    // everything else has to fit around them rather than the other way round.
    for section in &spec.plan {
        match section {
            Section::Exposition { entries } => {
                for (i, e) in entries.iter().enumerate() {
                    place_entry(
                        &mut r,
                        spec,
                        e.voice as usize,
                        e.at_bar * bpb,
                        e.transposition,
                        false,
                        1.0,
                    );
                    // The countersubject goes in the voice that has just
                    // finished, from the second entry on — there is nothing to
                    // accompany before that.
                    if i > 0 {
                        let prev = entries[i - 1].voice as usize;
                        place_countersubject(&mut r, spec, prev, e.at_bar * bpb, e.transposition);
                    }
                }
            }
            Section::Entry { voice, degree, at_bar, inversion, augmentation } => {
                let t =
                    if *degree == key.mode.dominant_degree() { Transposition::Dominant } else { Transposition::Tonic };
                place_entry(
                    &mut r,
                    spec,
                    *voice as usize,
                    at_bar * bpb,
                    t,
                    *inversion,
                    augmentation.unwrap_or(1.0),
                );
            }
            Section::Stretto { at_bar, overlap_beats, voices, .. } => {
                for (i, v) in voices.iter().enumerate() {
                    let at = at_bar * bpb + i as f64 * overlap_beats;
                    let t = if i % 2 == 0 { Transposition::Tonic } else { Transposition::Dominant };
                    place_entry(&mut r, spec, *v as usize, at, t, false, 1.0);
                }
            }
            _ => {}
        }
    }

    // Pass 2: pedal points. A held note is placed before the free voices so
    // they have to accommodate it rather than discover it.
    for section in &spec.plan {
        if let Section::Pedal { bars, at_bar, voice, degree } = section {
            let midi = fit_to_range(key.to_midi(*degree, 0), *voice as usize);
            r.notes.push(Placed {
                voice: *voice as usize,
                at_beat: at_bar * bpb,
                beats: bars * bpb,
                midi,
            });
        }
    }

    // Pass 3: everything else — episodes, cadences, and the free voices under
    // entries — chosen note by note against the rules.
    let mut state = seed ^ 0x5DEE_CE66_D0D1_6F5B;
    for section in &spec.plan {
        match section {
            Section::Episode { bars, at_bar, sequence_interval, motif_from, target_degree } => {
                realise_episode(
                    &mut r,
                    spec,
                    &mut state,
                    *at_bar * bpb,
                    *bars * bpb,
                    *sequence_interval,
                    *motif_from,
                    *target_degree,
                );
            }
            Section::Cadence { bars, at_bar, cadence_type } => {
                realise_cadence(&mut r, spec, &mut state, *at_bar * bpb, *bars * bpb, *cadence_type);
            }
            _ => {}
        }
    }

    r.notes.sort_by(|a, b| {
        a.at_beat.partial_cmp(&b.at_beat).unwrap().then(a.voice.cmp(&b.voice))
    });
    r.entries.sort_by(|a, b| a.at_beat.partial_cmp(&b.at_beat).unwrap());
    r.relaxations.sort_by(|a, b| a.at_beat.partial_cmp(&b.at_beat).unwrap());
    r
}

/// Shift a pitch by whole octaves until it lies in the voice's range, and say
/// nothing if it cannot — the caller records that.
fn fit_to_range(midi: i32, voice: usize) -> i32 {
    let (lo, hi) = VOICE_RANGES[voice];
    let mut m = midi;
    while m < lo {
        m += 12;
    }
    while m > hi {
        m -= 12;
    }
    m
}

fn place_line(r: &mut Realisation, voice: usize, line: &Line, at_beat: f64, shift: i32) {
    let mut t = at_beat;
    for n in line {
        if !n.rest {
            r.notes.push(Placed {
                voice,
                at_beat: t,
                beats: n.beats,
                midi: r.key.to_midi(n.degree, n.octave) + shift,
            });
        }
        t += n.beats;
    }
}

/// The octave shift that puts a line as far inside a voice's range as it will
/// go, measured on the line's own extremes rather than on its first note.
fn octave_shift_for(r: &Realisation, line: &Line, voice: usize) -> (i32, bool) {
    let pitches: Vec<i32> =
        line.iter().filter(|n| !n.rest).map(|n| r.key.to_midi(n.degree, n.octave)).collect();
    if pitches.is_empty() {
        return (0, true);
    }
    let (lo, hi) = VOICE_RANGES[voice];
    let (min, max) = (*pitches.iter().min().unwrap(), *pitches.iter().max().unwrap());
    let mut best = 0;
    let mut best_out = i32::MAX;
    for k in -4..=4 {
        let s = k * 12;
        let out = (lo - (min + s)).max(0) + ((max + s) - hi).max(0);
        if out < best_out {
            best_out = out;
            best = s;
        }
    }
    (best, best_out == 0)
}

fn place_entry(
    r: &mut Realisation,
    spec: &FugueSpec,
    voice: usize,
    at_beat: f64,
    transposition: Transposition,
    inversion: bool,
    augmentation: f64,
) {
    let mut line = match transposition {
        Transposition::Tonic => spec.subject.clone(),
        Transposition::Dominant => answer(&spec.subject, spec.mode.mode),
    };
    if inversion {
        line = invert(&line);
    }
    if (augmentation - 1.0).abs() > 1e-9 {
        line = scale_durations(&line, augmentation);
    }
    let (shift, fits) = octave_shift_for(r, &line, voice);
    if !fits {
        // The subject does not move to make a range work — it is the thing the
        // piece is about. The breach is recorded instead.
        r.relaxations.push(Relaxation {
            rule: Rule::Range,
            at_beat,
            bar: at_beat / r.beats_per_bar,
            voice,
            detail: format!(
                "the subject's compass does not fit {}'s range at any octave",
                VOICE_NAMES[voice]
            ),
        });
    }
    place_line(r, voice, &line, at_beat, shift);
    r.entries.push(EntryMark {
        voice,
        at_bar: at_beat / r.beats_per_bar,
        at_beat,
        transposition,
        inverted: inversion,
        augmentation,
    });
}

fn place_countersubject(
    r: &mut Realisation,
    spec: &FugueSpec,
    voice: usize,
    at_beat: f64,
    against: Transposition,
) {
    // The countersubject travels with the entry it accompanies: under an
    // answer it is a fifth up, so the two keep the intervals they were written
    // with. This is the transposition `model.rs`'s own invertibility test uses.
    let line = match against {
        Transposition::Tonic => spec.countersubject.clone(),
        Transposition::Dominant => transpose_steps(&spec.countersubject, 4),
    };
    let (shift, _) = octave_shift_for(r, &line, voice);
    place_line(r, voice, &line, at_beat, shift);
}

/// Candidate pitches for a free voice at a moment: the mode's own notes,
/// within the voice's range, near where the voice last was.
fn candidates(r: &Realisation, voice: usize, previous: Option<i32>) -> Vec<i32> {
    let (lo, hi) = VOICE_RANGES[voice];
    let mut out = Vec::new();
    for degree in -14..21 {
        let m = r.key.to_midi(degree, 0);
        if m < lo || m > hi {
            continue;
        }
        // A free voice moves by step or small leap. A line that may jump
        // anywhere in its range is not a line.
        if let Some(p) = previous {
            if (m - p).abs() > 7 {
                continue;
            }
        }
        out.push(m);
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Score a candidate: which rules it breaks, worst first. An empty list is a
/// clean note.
fn faults_for(r: &Realisation, voice: usize, at_beat: f64, beats: f64, midi: i32) -> Vec<Rule> {
    let mut probe = r.notes.clone();
    probe.push(Placed { voice, at_beat, beats, midi });
    let mut out = Vec::new();
    if parallel_perfects(&probe).iter().any(|(t, a, b)| {
        (*t - at_beat).abs() < 1e-9 && (*a == voice || *b == voice)
    }) {
        out.push(Rule::ParallelPerfect);
    }
    let (lo, hi) = VOICE_RANGES[voice];
    if midi < lo || midi > hi {
        out.push(Rule::Range);
    }
    if voice_crossings(&probe)
        .iter()
        .any(|(t, a, b)| (*t - at_beat).abs() < 1e-9 && (*a == voice || *b == voice))
    {
        out.push(Rule::VoiceCrossing);
    }
    if unprepared_dissonances(&probe)
        .iter()
        .any(|(t, v)| (*t - at_beat).abs() < 1e-9 && *v == voice)
    {
        out.push(Rule::WeakBeatDissonance);
    }
    out.sort();
    out.reverse(); // worst first
    out
}

/// Choose one note for one voice at one moment, and record what it cost.
fn choose(
    r: &mut Realisation,
    state: &mut u64,
    voice: usize,
    at_beat: f64,
    beats: f64,
    previous: Option<i32>,
) -> Option<i32> {
    let cands = candidates(r, voice, previous);
    if cands.is_empty() {
        return None;
    }
    let mut scored: Vec<(Vec<Rule>, i32)> =
        cands.iter().map(|m| (faults_for(r, voice, at_beat, beats, *m), *m)).collect();
    // Fewest and least-damaging faults first, then the smallest melodic move.
    scored.sort_by_key(|(f, m)| {
        let worst = f.first().copied().map(|r| r as i32 + 1).unwrap_or(0);
        let motion = previous.map(|p| (m - p).abs()).unwrap_or(0);
        (worst, f.len() as i32, motion, *m)
    });
    // **The seed chooses among the candidates that are equally good**, and
    // only among those. The first version put the PRNG in the last sort key,
    // which sounds like the same thing and is not: melodic motion decided
    // every comparison before the tie-break was ever reached, so two different
    // seeds produced byte-identical files. A seed that changes nothing is
    // worse than no seed, because it is a promise of variation that the format
    // and the URL parameter both go on making.
    //
    // Three, not all of them: a free voice that may take any acceptable note is
    // a free voice that wanders. The best answer is still usually chosen; which
    // of the top three is the edition's own decision.
    let best = scored[0].0.clone();
    let equal = scored.iter().take_while(|(f, _)| *f == best).count().min(3).max(1);
    let pick = (splitmix64(state) % equal as u64) as usize;
    let (faults, midi) = scored.remove(pick);
    for rule in faults {
        r.relaxations.push(Relaxation {
            rule,
            at_beat,
            bar: at_beat / r.beats_per_bar,
            voice,
            detail: format!("{} in {}", rule.name(), VOICE_NAMES[voice]),
        });
    }
    r.notes.push(Placed { voice, at_beat, beats, midi });
    Some(midi)
}

/// Which voices have nothing to do in a span.
fn idle_voices(r: &Realisation, from: f64, to: f64) -> Vec<usize> {
    (0..4)
        .filter(|v| {
            !r.notes
                .iter()
                .any(|n| n.voice == *v && n.at_beat < to - 1e-9 && n.end_beat() > from + 1e-9)
        })
        .collect()
}

/// An episode: a sequence on existing material, landing in the key of the
/// entry that follows.
///
/// **It never invents a tune.** `motif_from` says which existing line to
/// sequence and the rhythm comes from there; only the pitches are chosen. An
/// episode that introduces new material is an interlude, and that is the
/// single easiest way for a generated fugue to stop sounding like one.
#[allow(clippy::too_many_arguments)]
fn realise_episode(
    r: &mut Realisation,
    spec: &FugueSpec,
    state: &mut u64,
    at_beat: f64,
    beats: f64,
    sequence_interval: i32,
    motif_from: MotifSource,
    target_degree: i32,
) {
    let motif: Line = match motif_from {
        MotifSource::SubjectHead => spec.subject.iter().take(4).copied().collect(),
        MotifSource::CountersubjectTail => {
            let n = spec.countersubject.len();
            spec.countersubject.iter().skip(n.saturating_sub(4)).copied().collect()
        }
    };
    let motif_beats = line_beats(&motif).max(1.0);
    let voices = idle_voices(r, at_beat, at_beat + beats);
    if voices.is_empty() {
        return;
    }

    // The leading voice states the sequence; the others are chosen against it.
    let lead = voices[0];
    let mut t = at_beat;
    let mut step = 0;
    while t < at_beat + beats - 1e-9 {
        let seq = transpose_steps(&motif, sequence_interval * step);
        for n in &seq {
            if t >= at_beat + beats - 1e-9 {
                break;
            }
            let dur = n.beats.min(at_beat + beats - t);
            if !n.rest {
                let (shift, _) = octave_shift_for(r, &vec![*n], lead);
                let midi = fit_to_range(r.key.to_midi(n.degree, n.octave) + shift, lead);
                r.notes.push(Placed { voice: lead, at_beat: t, beats: dur, midi });
            }
            t += dur;
        }
        step += 1;
        if step > 64 {
            break; // a sequence that never fills its span is a bug, not a fugue
        }
    }

    // Land in the target key: the last note of the lead is pulled onto the
    // degree the following entry starts from. This is what makes the episode
    // *modulate* rather than merely fill.
    if let Some(last) = r.notes.iter_mut().filter(|n| n.voice == lead).next_back() {
        last.midi = fit_to_range(r.key.to_midi(target_degree, 0), lead);
    }

    for v in voices.iter().skip(1) {
        let mut prev = None;
        let mut t = at_beat;
        while t < at_beat + beats - 1e-9 {
            let dur = motif_beats.min(at_beat + beats - t);
            prev = choose(r, state, *v, t, dur, prev);
            t += dur;
        }
    }
}

fn realise_cadence(
    r: &mut Realisation,
    spec: &FugueSpec,
    state: &mut u64,
    at_beat: f64,
    beats: f64,
    kind: CadenceType,
) {
    let bpb = spec.meter[0] as f64;
    let voices = idle_voices(r, at_beat, at_beat + beats);
    if voices.is_empty() {
        return;
    }
    // The approach degree, by cadence type: authentic comes from the dominant,
    // plagal from the subdominant, phrygian from the flattened second — which
    // in a mode is the one that actually sounds old.
    let approach = match kind {
        CadenceType::Authentic => 4,
        CadenceType::Plagal => 3,
        CadenceType::Phrygian => 1,
    };
    let half = (beats / 2.0).max(bpb.min(beats));
    let rest = beats - half;

    // **The lowest idle voice carries the cadence; the rest are chosen.**
    //
    // The first version wrote the same two pitches into every idle voice, and
    // four voices moving from the same note to the same note is, by
    // definition, parallel octaves in all six pairs at once. The test found it
    // as six parallels on one beat — which is the signature of exactly this
    // mistake and of nothing else. So the bass states the progression and the
    // upper voices go through `choose`, which already knows every rule.
    let bass = *voices.iter().max().unwrap();
    r.notes.push(Placed {
        voice: bass,
        at_beat,
        beats: half,
        midi: fit_to_range(r.key.to_midi(approach, 0), bass),
    });
    if rest > 1e-9 {
        r.notes.push(Placed {
            voice: bass,
            at_beat: at_beat + half,
            beats: rest,
            midi: fit_to_range(r.key.tonic_midi, bass),
        });
    }
    for v in voices.iter().filter(|v| **v != bass) {
        let prev = r
            .notes
            .iter()
            .filter(|n| n.voice == *v && n.end_beat() <= at_beat + 1e-9)
            .next_back()
            .map(|n| n.midi);
        let first = choose(r, state, *v, at_beat, half, prev);
        if rest > 1e-9 {
            choose(r, state, *v, at_beat + half, rest, first);
        }
    }
}

/// Where the cadences are, in beats — what `leading_tone_resolutions` looks at.
pub fn cadence_beats(spec: &FugueSpec) -> Vec<f64> {
    let bpb = spec.meter[0] as f64;
    spec.plan
        .iter()
        .filter_map(|s| match s {
            Section::Cadence { at_bar, .. } => Some(at_bar * bpb),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(voice: usize, at: f64, beats: f64, midi: i32) -> Placed {
        Placed { voice, at_beat: at, beats, midi }
    }

    #[test]
    fn splitmix64_matches_the_shared_fixture() {
        // The other copy of this generator lives in spex-show, and the two are
        // pinned to the same recorded values rather than to each other — a
        // fixture they can both fail against, instead of a comparison they
        // could both drift through.
        let mut s = SPLITMIX_GAMMA;
        let first = splitmix64(&mut s);
        let mut t = SPLITMIX_GAMMA;
        // Reference: splitmix64's published definition, written out longhand.
        t = t.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = t;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        assert_eq!(first, z ^ (z >> 31));
    }

    #[test]
    fn parallel_fifths_between_any_pair_are_found() {
        // Soprano and bass moving in fifths: C5/F4 then D5/G4.
        let notes = vec![
            p(0, 0.0, 1.0, 72),
            p(3, 0.0, 1.0, 65),
            p(0, 1.0, 1.0, 74),
            p(3, 1.0, 1.0, 67),
        ];
        let found = parallel_perfects(&notes);
        assert_eq!(found, vec![(1.0, 0, 3)]);
    }

    #[test]
    fn parallel_octaves_are_found_and_a_pedal_is_not() {
        let octaves = vec![
            p(0, 0.0, 1.0, 72),
            p(3, 0.0, 1.0, 60),
            p(0, 1.0, 1.0, 74),
            p(3, 1.0, 1.0, 62),
        ];
        assert_eq!(parallel_perfects(&octaves).len(), 1);

        // The same octave twice with nothing moving: the pedal in the last
        // four bars, which must not be reported.
        let pedal = vec![
            p(0, 0.0, 1.0, 72),
            p(3, 0.0, 2.0, 60),
            p(0, 1.0, 1.0, 72),
        ];
        assert!(parallel_perfects(&pedal).is_empty());
    }

    #[test]
    fn a_fifth_followed_by_a_different_perfect_interval_is_not_a_parallel() {
        // Fifth then octave. Direct, arguably ugly, and not what the rule is
        // about — the rule is about *parallel* motion in the same interval.
        let notes = vec![
            p(0, 0.0, 1.0, 67),
            p(3, 0.0, 1.0, 60),
            p(0, 1.0, 1.0, 72),
            p(3, 1.0, 1.0, 60),
        ];
        assert!(parallel_perfects(&notes).is_empty());
    }

    #[test]
    fn voice_crossing_is_found() {
        let notes = vec![p(1, 0.0, 1.0, 55), p(2, 0.0, 1.0, 67)];
        assert_eq!(voice_crossings(&notes), vec![(0.0, 1, 2)]);
    }

    #[test]
    fn every_voice_range_is_the_spec_s_own() {
        // S C4-A5, A F3-D5, T C3-A4, B E2-C4. Written out so a transcription
        // error is a failing test rather than a slightly odd-sounding bass.
        assert_eq!(VOICE_RANGES[0], (60, 81));
        assert_eq!(VOICE_RANGES[1], (53, 74));
        assert_eq!(VOICE_RANGES[2], (48, 69));
        assert_eq!(VOICE_RANGES[3], (40, 60));
        // And every range is at least an octave and a half — a voice that
        // cannot carry the subject at any octave is a voice that will spend
        // the piece producing relaxations.
        for (lo, hi) in VOICE_RANGES {
            assert!(hi - lo >= 18);
        }
    }

    #[test]
    fn out_of_range_notes_are_found() {
        let notes = vec![p(3, 0.0, 1.0, 36), p(3, 1.0, 1.0, 48)];
        assert_eq!(out_of_range(&notes).len(), 1);
    }

    #[test]
    fn seconds_sevenths_and_the_tritone_are_dissonances_and_thirds_are_not() {
        for d in [1, 2, 6, 10, 11] {
            assert!(is_dissonant(d), "{d} semitones should be dissonant");
        }
        for c in [0, 3, 4, 7, 8, 9] {
            assert!(!is_dissonant(c), "{c} semitones should be consonant");
        }
        // The fourth is the interesting one: consonant between upper voices,
        // dissonant against the bass.
        assert!(!is_dissonant(5));
        assert!(is_dissonant_against_bass(5));
    }

    #[test]
    fn an_off_beat_dissonance_is_a_passing_tone_and_an_on_beat_one_is_not() {
        let passing = vec![p(3, 0.0, 2.0, 60), p(0, 0.5, 0.5, 73)];
        assert!(unprepared_dissonances(&passing).is_empty());
        let on_beat = vec![p(3, 0.0, 2.0, 60), p(0, 1.0, 1.0, 73)];
        assert_eq!(unprepared_dissonances(&on_beat).len(), 1);
    }

    #[test]
    fn a_suspension_is_prepared_and_therefore_allowed() {
        // The same pitch sounds through a bass change: dissonant on the beat,
        // but prepared, which is exactly what a suspension is.
        let notes = vec![
            p(0, 0.0, 1.0, 72),
            p(3, 0.0, 1.0, 60),
            p(0, 1.0, 1.0, 72),
            p(3, 1.0, 1.0, 62),
        ];
        assert!(unprepared_dissonances(&notes).is_empty(), "a prepared suspension is not a fault");
    }

    #[test]
    fn the_relaxation_order_is_least_damaging_first() {
        let mut rules =
            vec![Rule::ParallelPerfect, Rule::Range, Rule::WeakBeatDissonance, Rule::VoiceCrossing];
        rules.sort();
        assert_eq!(
            rules,
            vec![
                Rule::WeakBeatDissonance,
                Rule::VoiceCrossing,
                Rule::Range,
                Rule::ParallelPerfect
            ]
        );
    }
}
