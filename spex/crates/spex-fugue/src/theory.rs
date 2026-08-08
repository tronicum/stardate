//! M67 — the music theory the rest of Phase 3 stands on.
//!
//! Everything here is pure arithmetic over integers: no audio, no time, no
//! I/O, no randomness. That is deliberate and it is the same argument
//! `spex-show` makes about depending on nothing but serde — M69's WebAudio
//! engine and M68's constraint solver both need these answers, and neither
//! should have to bring the other along to get them.
//!
//! # Everything is a scale degree until the last possible moment
//!
//! A fugue subject is not a list of pitches. It is a shape *within a mode*,
//! and the whole point of a fugue is that the same shape appears at different
//! pitches, inverted, augmented, in another voice. So the score carries
//! `(degree, octave, beats)` and `to_midi` is the one boundary where that
//! becomes a note number — exactly the way `spex-mesh` keeps geometry in LDU
//! until `to_output_position`.
//!
//! Writing it the other way round — MIDI numbers with a mode implied — makes
//! transposition a table of accidentals and inversion nearly impossible to
//! get right, because both operations are *diatonic*: a third inverted is a
//! sixth, and whether it is major or minor is decided by the mode, not by the
//! interval.
//!
//! # The tonal answer is the reason this module exists at all
//!
//! A fugue's second entry is not a transposition. It is *nearly* one, and
//! where it is not is the single thing that makes a generated fugue sound
//! like a fugue instead of like a canon:
//!
//! > Where the subject's head touches the **dominant**, the answer replies
//! > with the **tonic**; where the subject touches the tonic, the answer
//! > replies with the dominant. Everything else moves by the interval.
//!
//! A real answer transposes the whole subject up a fifth. A tonal answer
//! adjusts the head so the music stays in the home key long enough for the
//! key to still be there when the third voice arrives. `answer()` implements
//! both, `AnswerKind` chooses, and the tests below check the classical
//! example rather than the implementation's own opinion.

use serde::{Deserialize, Serialize};

/// The seven diatonic modes, as rotations of one scale.
///
/// Rotations rather than seven independent tables: they *are* rotations, and
/// two spellings of the same fact drift. `dorian` is the mode the piece uses
/// — the one with a minor third and a **major sixth**, which is the note that
/// distinguishes it from the minor scale and therefore the note a subject in
/// Dorian has to touch if the mode is to be audible at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Ionian,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Aeolian,
    Locrian,
}

/// The major scale, in semitones from its own tonic. Every mode is a rotation
/// of this and nothing else is hard-coded.
const MAJOR_STEPS: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

impl Mode {
    /// How many scale steps this mode is rotated from Ionian.
    pub const fn rotation(self) -> usize {
        match self {
            Mode::Ionian => 0,
            Mode::Dorian => 1,
            Mode::Phrygian => 2,
            Mode::Lydian => 3,
            Mode::Mixolydian => 4,
            Mode::Aeolian => 5,
            Mode::Locrian => 6,
        }
    }

    /// Semitones above this mode's own tonic for each of its seven degrees.
    ///
    /// Derived by rotating the major scale and re-basing on the new tonic, so
    /// e.g. Dorian comes out `[0, 2, 3, 5, 7, 9, 10]` — minor third, major
    /// sixth — without that ever being written down anywhere.
    pub fn semitones(self) -> [i32; 7] {
        let r = self.rotation();
        let base = MAJOR_STEPS[r];
        let mut out = [0; 7];
        for (i, slot) in out.iter_mut().enumerate() {
            let idx = (r + i) % 7;
            let octaves = ((r + i) / 7) as i32;
            *slot = MAJOR_STEPS[idx] + 12 * octaves - base;
        }
        out
    }

    /// Semitones above the tonic for a degree, which may be negative or past
    /// the octave. `degree_semitones(7)` is an octave above
    /// `degree_semitones(0)` in every mode, by construction.
    pub fn degree_semitones(self, degree: i32) -> i32 {
        let table = self.semitones();
        let octave = degree.div_euclid(7);
        let within = degree.rem_euclid(7) as usize;
        table[within] + 12 * octave
    }

    /// Which scale degree the dominant is. Five above the tonic, counted in
    /// scale steps — degree 4, zero-based, in every mode.
    ///
    /// A named constant rather than a literal 4 scattered through the answer
    /// logic: Locrian's fifth is diminished, and the day someone has to
    /// special-case that, this is the function they will look for.
    pub const fn dominant_degree(self) -> i32 {
        4
    }

    /// Is this mode's fifth a perfect one? False for Locrian alone.
    ///
    /// The tonal answer's whole mechanism is the tonic/dominant exchange, and
    /// in Locrian there is no dominant to exchange with. The piece is in
    /// Dorian, so this never fires — it exists so that a future document in
    /// Locrian fails loudly rather than producing a subtly wrong answer.
    pub fn has_perfect_fifth(self) -> bool {
        self.degree_semitones(4) == 7
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Ionian => "ionian",
            Mode::Dorian => "dorian",
            Mode::Phrygian => "phrygian",
            Mode::Lydian => "lydian",
            Mode::Mixolydian => "mixolydian",
            Mode::Aeolian => "aeolian",
            Mode::Locrian => "locrian",
        }
    }
}

/// A key: a mode, the MIDI note its tonic sits on, and which letter that
/// tonic is called.
///
/// `tonic_midi` is a full MIDI note number, not a pitch class, and that is a
/// decision worth stating because the spec left it ambiguous. A pitch class
/// would make `to_midi` need a second octave reference from somewhere else,
/// and "somewhere else" is where octave errors live. D4 is 62.
///
/// # Why the letter is stored separately
///
/// **A pitch does not determine a key.** E-flat major and D-sharp major are
/// the same keys on a piano and different keys in a score: one spells its
/// third G, the other F-double-sharp. Deriving the letter from the pitch
/// class means picking one of them arbitrarily, and the first version of this
/// file did exactly that — `min_by_key` over the distance to each letter,
/// which for a black key is a tie broken by whichever letter happened to come
/// first in the array. It produced D-sharp for a key that had asked for
/// E-flat, and the test caught it.
///
/// `tonic_letter` is 0..=6 for C..B. `None` means "use the flat spelling",
/// which is the convention for modal and minor keys and is unambiguous for
/// every white-key tonic — including this piece's D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    #[serde(rename = "tonic")]
    pub tonic_midi: i32,
    #[serde(rename = "name")]
    pub mode: Mode,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "tonicLetter")]
    pub tonic_letter: Option<u8>,
}

impl Key {
    pub fn new(tonic_midi: i32, mode: Mode) -> Self {
        Self { tonic_midi, mode, tonic_letter: None }
    }

    /// The same key, spelled from an explicit tonic letter (0 = C .. 6 = B).
    pub fn spelled(tonic_midi: i32, mode: Mode, tonic_letter: u8) -> Self {
        Self { tonic_midi, mode, tonic_letter: Some(tonic_letter) }
    }

    /// `degree` is a scale step, zero-based within the mode; `octave` shifts
    /// by whole octaves from the tonic reference. Both may be negative.
    pub fn to_midi(&self, degree: i32, octave: i32) -> i32 {
        self.tonic_midi + self.mode.degree_semitones(degree) + 12 * octave
    }

    /// The letter name of a degree, spelled the way this mode spells it.
    ///
    /// Spelling, not pitch: D Dorian's sixth is **B**, and calling it A♯
    /// would be the same key on a piano and the wrong note in a score. The
    /// rule is the one every notation program uses — one letter per scale
    /// step, accidentals applied to whichever letter the step lands on.
    pub fn spell(&self, degree: i32, octave: i32) -> String {
        const LETTERS: [char; 7] = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];
        const NATURAL: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
        // Which letter the tonic is. Given explicitly where it matters; for a
        // black key with no letter stated, the flat spelling — see the type's
        // own documentation for why this cannot be derived.
        let tonic_pc = self.tonic_midi.rem_euclid(12);
        let tonic_letter = match self.tonic_letter {
            Some(l) => l as usize % 7,
            None => LETTERS
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let d = tonic_pc - NATURAL[i];
                    let d = if d > 6 { d - 12 } else if d < -6 { d + 12 } else { d };
                    (i, d)
                })
                // Flats first: among letters equally far away, prefer the one
                // the pitch sits *below*, i.e. the more negative offset —
                // that is the flat reading. E-flat, not D-sharp.
                .min_by_key(|(_, d)| (d.abs(), *d))
                .map(|(i, _)| i)
                .unwrap(),
        };

        let step = degree.rem_euclid(7) as usize;
        let letter_index = (tonic_letter + step) % 7;
        let letter_octaves = ((tonic_letter + step) / 7) as i32;

        let midi = self.to_midi(degree, octave);
        // The natural pitch of that letter in the octave the note lands in.
        let scientific_octave = (midi - NATURAL[letter_index]).div_euclid(12) - 1;
        let natural_midi = NATURAL[letter_index] + 12 * (scientific_octave + 1);
        let mut alter = midi - natural_midi;
        let mut sci = scientific_octave;
        // A letter can be a semitone either side of where the pitch landed;
        // pick the reading that needs the smallest accidental.
        if alter > 6 {
            alter -= 12;
            sci += 1;
        } else if alter < -6 {
            alter += 12;
            sci -= 1;
        }
        let _ = letter_octaves;
        let accidental = match alter {
            0 => "",
            1 => "#",
            2 => "##",
            -1 => "b",
            -2 => "bb",
            _ => "?",
        };
        format!("{}{}{}", LETTERS[letter_index], accidental, sci)
    }
}

/// One note of a melodic line: a scale step, an octave offset, a length.
///
/// No pitch, no MIDI number, no absolute time — see the module header. The
/// length is in beats and is a float because a fugue subject legitimately
/// contains dotted values and triplets; it is never used as a time, only
/// summed and compared.
///
/// # Rests
///
/// `rest` was not in the first version of this type and had to be added,
/// which is worth recording because the reason is musical rather than
/// technical. A countersubject **cannot cover the subject's head**: the head
/// is precisely the part that is *not* the same in the answer — that is what
/// a tonal answer is — so a countersubject written against the subject's
/// first two notes is guaranteed to collide with the answer's. The classical
/// solution is the obvious one once seen: the countersubject starts after the
/// head. That needs a rest.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// Defaulted, because a rest has no degree and writing `"degree": 0` on
    /// one would be a pitch that is not sounding — a number a later reader
    /// could take seriously.
    #[serde(default)]
    pub degree: i32,
    #[serde(default)]
    pub octave: i32,
    pub beats: f64,
    /// A silence of `beats`. `degree` and `octave` are then meaningless and
    /// are held at zero.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub rest: bool,
}

impl Note {
    pub fn new(degree: i32, octave: i32, beats: f64) -> Self {
        Self { degree, octave, beats, rest: false }
    }
    pub fn rest(beats: f64) -> Self {
        Self { degree: 0, octave: 0, beats, rest: true }
    }
    /// Absolute scale-step index from the tonic: the quantity every diatonic
    /// operation actually works on.
    pub fn step(&self) -> i32 {
        self.degree + 7 * self.octave
    }
}

/// A melodic line, as scale degrees. The subject, the countersubject, an
/// episode's sequence — all the same type.
pub type Line = Vec<Note>;

/// Total length in beats.
pub fn line_beats(line: &[Note]) -> f64 {
    line.iter().map(|n| n.beats).sum()
}

/// Highest minus lowest, in scale steps. A compass of 8 steps is a ninth.
pub fn compass_steps(line: &[Note]) -> i32 {
    let sounding = || line.iter().filter(|n| !n.rest).map(Note::step);
    match (sounding().max(), sounding().min()) {
        (Some(hi), Some(lo)) => hi - lo,
        _ => 0,
    }
}

/// Transpose diatomically by whole scale steps — the operation a fugue does
/// constantly, and the reason nothing here is stored as a pitch. Moving "up a
/// fifth" is `+4` steps, and which of those fifths comes out perfect and
/// which diminished is the mode's business, not the caller's.
pub fn transpose_steps(line: &[Note], steps: i32) -> Line {
    line.iter()
        .map(|n| {
            if n.rest {
                return *n;
            }
            let s = n.step() + steps;
            Note::new(s.rem_euclid(7), s.div_euclid(7), n.beats)
        })
        .collect()
}

/// Melodic inversion about the line's own first note: every rising interval
/// becomes the same falling one, in scale steps.
///
/// About the first note rather than about the tonic, because that is what
/// makes an inverted entry recognisably the same subject — the shape is
/// mirrored, the starting pitch is not moved.
pub fn invert(line: &[Note]) -> Line {
    let Some(first) = line.iter().find(|n| !n.rest) else { return Vec::new() };
    let axis = first.step();
    line.iter()
        .map(|n| {
            if n.rest {
                return *n;
            }
            let s = axis - (n.step() - axis);
            Note::new(s.rem_euclid(7), s.div_euclid(7), n.beats)
        })
        .collect()
}

/// Multiply every duration. `2.0` is augmentation, `0.5` diminution.
pub fn scale_durations(line: &[Note], factor: f64) -> Line {
    line.iter().map(|n| Note { beats: n.beats * factor, ..*n }).collect()
}

/// Retrograde: the same notes backwards, durations included.
pub fn retrograde(line: &[Note]) -> Line {
    line.iter().rev().copied().collect()
}

/// Which kind of answer a subject asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerKind {
    /// Transpose everything by the same interval.
    Real,
    /// Exchange tonic and dominant in the head, then transpose the rest.
    Tonal,
}

/// How many notes of the head participate in the tonal adjustment.
///
/// Two, and this is a judgement rather than a law: the classical rule is
/// "until the subject leaves the tonic-dominant orbit", which is a musical
/// reading no integer can make. Two notes covers the overwhelmingly common
/// case — a subject whose opening leap is the tonic-dominant pair — and it is
/// the case the piece's own subject is. `answer_with_head` takes the number
/// explicitly for anything else.
pub const DEFAULT_TONAL_HEAD: usize = 2;

/// Does this subject ask for a tonal answer?
///
/// The test is the classical one: a subject whose **head touches the
/// dominant** does. Touching it later does not count — by then the music has
/// moved, and adjusting there would deform the subject rather than preserve
/// the key.
pub fn answer_kind(subject: &[Note], mode: Mode) -> AnswerKind {
    let dom = mode.dominant_degree();
    let head = &subject[..subject.len().min(DEFAULT_TONAL_HEAD)];
    if head.iter().any(|n| n.degree == dom) {
        AnswerKind::Tonal
    } else {
        AnswerKind::Real
    }
}

/// The answer to a subject: a fifth above, adjusted if the subject asks for
/// it. See the module header for why this is not a transposition.
pub fn answer(subject: &[Note], mode: Mode) -> Line {
    answer_with_head(subject, mode, answer_kind(subject, mode), DEFAULT_TONAL_HEAD)
}

/// The answer, with both decisions made explicitly by the caller.
///
/// The transposition is **up four scale steps** — a fifth counted the way
/// musicians count, inclusively. Within the tonal head, a dominant is
/// answered by a tonic (up three steps, a fourth) and a tonic by a dominant
/// (up four steps, a fifth); the octave is recomputed from the resulting step
/// so an adjusted note stays in the register the transposition put it in.
pub fn answer_with_head(subject: &[Note], mode: Mode, kind: AnswerKind, head: usize) -> Line {
    let dom = mode.dominant_degree();
    let fifth = 4;
    let fourth = 3;
    subject
        .iter()
        .enumerate()
        .map(|(i, n)| {
            if n.rest {
                return *n;
            }
            let interval = if kind == AnswerKind::Tonal && i < head {
                if n.degree == dom {
                    // Dominant answered by tonic: a fourth, not a fifth.
                    fourth
                } else if n.degree == 0 {
                    // Tonic answered by dominant: a fifth.
                    fifth
                } else {
                    fifth
                }
            } else {
                fifth
            };
            let s = n.step() + interval;
            Note::new(s.rem_euclid(7), s.div_euclid(7), n.beats)
        })
        .collect()
}

// ---------------------------------------------------------------- intervals

/// A vertical interval between two sounding lines, in **scale steps** — 0 a
/// unison, 2 a third, 4 a fifth, 7 an octave.
///
/// Steps and not semitones, because every rule that consumes this is diatonic:
/// "no parallel fifths" is about fifths, not about seven semitones, and in
/// Locrian those are different things.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertical {
    /// Beat at which this interval begins.
    pub at_beat: f64,
    /// Upper line's step index minus lower line's.
    pub steps: i32,
    /// The same interval in semitones — needed to tell a perfect fifth from a
    /// diminished one, which no number of scale steps can.
    pub semitones: i32,
}

impl Vertical {
    /// Reduced into one octave: 0..=6, where 0 is a unison/octave, 2 a third,
    /// 4 a fifth. This is the quantity contrapuntal rules are stated in.
    pub fn simple_steps(&self) -> i32 {
        self.steps.rem_euclid(7)
    }
    /// Is this a perfect consonance — unison, octave, or perfect fifth? These
    /// are the two that may not move in parallel.
    pub fn is_perfect_consonance(&self) -> bool {
        let s = self.simple_steps();
        (s == 0) || (s == 4 && self.semitones.rem_euclid(12) == 7)
    }
    /// Third, sixth, or their compounds. The intervals counterpoint is mostly
    /// built out of.
    pub fn is_imperfect_consonance(&self) -> bool {
        matches!(self.simple_steps(), 2 | 5)
    }
    pub fn is_dissonance(&self) -> bool {
        !self.is_perfect_consonance() && !self.is_imperfect_consonance() && self.simple_steps() != 3
            || self.simple_steps() == 1
            || self.simple_steps() == 6
    }
}

/// Every vertical interval between two lines, sampled at each point where
/// either line changes note.
///
/// Sampling at every attack in **either** line is the whole trick: a rule
/// about parallel fifths is about consecutive *simultaneities*, and a line
/// that holds while the other moves creates a new simultaneity without
/// creating a new note. Walking one line's notes would miss exactly those.
pub fn verticals(upper: &[Note], lower: &[Note], key: Key) -> Vec<Vertical> {
    let mut points: Vec<f64> = Vec::new();
    for line in [upper, lower] {
        let mut t = 0.0;
        for n in line {
            points.push(t);
            t += n.beats;
        }
    }
    points.sort_by(|a, b| a.partial_cmp(b).unwrap());
    points.dedup_by(|a, b| (*a - *b).abs() < 1e-9);

    let end = line_beats(upper).min(line_beats(lower));
    points
        .into_iter()
        .filter(|t| *t < end - 1e-9)
        .filter_map(|t| {
            let u = note_at(upper, t)?;
            let l = note_at(lower, t)?;
            // Silence has no interval with anything. A rest is not a fault,
            // it is the absence of a simultaneity to have a fault about.
            if u.rest || l.rest {
                return None;
            }
            Some(Vertical {
                at_beat: t,
                steps: u.step() - l.step(),
                semitones: key.to_midi(u.degree, u.octave) - key.to_midi(l.degree, l.octave),
            })
        })
        .collect()
}

/// Which note of a line is sounding at a beat.
pub fn note_at(line: &[Note], beat: f64) -> Option<Note> {
    let mut t = 0.0;
    for n in line {
        if beat < t + n.beats - 1e-9 {
            return Some(*n);
        }
        t += n.beats;
    }
    None
}

/// Consecutive perfect fifths or octaves between two lines, as the beats at
/// which the second of the pair falls.
///
/// The check is "two perfect consonances of the *same size* in a row, with at
/// least one line actually moving" — a repeated note under a held note is not
/// a parallel anything, and counting it would flag every pedal point in the
/// piece.
pub fn parallel_perfects(upper: &[Note], lower: &[Note], key: Key) -> Vec<f64> {
    let v = verticals(upper, lower, key);
    let mut out = Vec::new();
    for w in v.windows(2) {
        let (a, b) = (w[0], w[1]);
        if !a.is_perfect_consonance() || !b.is_perfect_consonance() {
            continue;
        }
        if a.simple_steps() != b.simple_steps() {
            continue;
        }
        // Something has to have moved.
        let moved = note_at(upper, a.at_beat).map(|n| n.step())
            != note_at(upper, b.at_beat).map(|n| n.step())
            || note_at(lower, a.at_beat).map(|n| n.step())
                != note_at(lower, b.at_beat).map(|n| n.step());
        if moved {
            out.push(b.at_beat);
        }
    }
    out
}

/// Why a pair of lines is not invertible at the octave.
#[derive(Debug, Clone, PartialEq)]
pub enum InversionFault {
    /// A perfect fifth: it becomes a fourth when the parts swap, and a fourth
    /// against the bass is a dissonance. This is *the* rule of double
    /// counterpoint at the octave and it is the one that catches people.
    PerfectFifth { at_beat: f64 },
    /// The lines cross or exceed an octave apart, so swapping them does not
    /// produce the intended intervals at all.
    ExceedsOctave { at_beat: f64, steps: i32 },
    /// A dissonance that is not passing — checked only on beats, since a
    /// dissonance between beats is what passing tones are.
    UnpreparedDissonance { at_beat: f64, steps: i32 },
}

/// Is this countersubject invertible against this subject at the octave?
///
/// Not an opinion: the three faults above, each with the beat it happens on.
/// An empty list is the pass. Written this way round because "it is
/// invertible" is a claim nobody can check and "there is a perfect fifth on
/// beat 3" is a claim anybody can.
pub fn invertibility_faults(upper: &[Note], lower: &[Note], key: Key) -> Vec<InversionFault> {
    let mut out = Vec::new();
    for v in verticals(upper, lower, key) {
        if v.steps < 0 || v.steps > 7 {
            out.push(InversionFault::ExceedsOctave { at_beat: v.at_beat, steps: v.steps });
            continue;
        }
        if v.is_perfect_consonance() && v.simple_steps() == 4 {
            out.push(InversionFault::PerfectFifth { at_beat: v.at_beat });
            continue;
        }
        // On a beat, only consonances (and the octave/unison) are allowed
        // without preparation. Off-beat dissonance is a passing tone and is
        // exactly what makes a countersubject move.
        let on_beat = (v.at_beat - v.at_beat.round()).abs() < 1e-9;
        let s = v.simple_steps();
        let consonant = s == 0 || s == 2 || s == 5 || (s == 3 && v.semitones.rem_euclid(12) == 5);
        if on_beat && !consonant {
            out.push(InversionFault::UnpreparedDissonance { at_beat: v.at_beat, steps: v.steps });
        }
    }
    out
}

/// Convenience over `invertibility_faults`.
pub fn is_invertible_at_octave(upper: &[Note], lower: &[Note], key: Key) -> bool {
    invertibility_faults(upper, lower, key).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D4. Every test below that names a letter is checking against this.
    const D4: i32 = 62;
    fn d_dorian() -> Key {
        Key::new(D4, Mode::Dorian)
    }

    // ------------------------------------------------- 1-7: mode spellings

    #[test]
    fn ionian_is_the_major_scale() {
        assert_eq!(Mode::Ionian.semitones(), [0, 2, 4, 5, 7, 9, 11]);
    }

    #[test]
    fn dorian_has_a_minor_third_and_a_major_sixth() {
        // The whole reason the piece is in Dorian: the sixth is major (9),
        // which an Aeolian scale spells 8.
        assert_eq!(Mode::Dorian.semitones(), [0, 2, 3, 5, 7, 9, 10]);
        assert_eq!(Mode::Aeolian.semitones(), [0, 2, 3, 5, 7, 8, 10]);
        assert_ne!(Mode::Dorian.semitones()[5], Mode::Aeolian.semitones()[5]);
    }

    #[test]
    fn phrygian_is_the_one_with_a_flat_second() {
        assert_eq!(Mode::Phrygian.semitones()[1], 1);
    }

    #[test]
    fn lydian_is_the_one_with_a_sharp_fourth() {
        assert_eq!(Mode::Lydian.semitones()[3], 6);
    }

    #[test]
    fn mixolydian_is_major_with_a_flat_seventh() {
        assert_eq!(Mode::Mixolydian.semitones(), [0, 2, 4, 5, 7, 9, 10]);
    }

    #[test]
    fn locrian_is_the_only_mode_without_a_perfect_fifth() {
        for m in [
            Mode::Ionian, Mode::Dorian, Mode::Phrygian, Mode::Lydian,
            Mode::Mixolydian, Mode::Aeolian,
        ] {
            assert!(m.has_perfect_fifth(), "{} should have a perfect fifth", m.name());
        }
        assert!(!Mode::Locrian.has_perfect_fifth());
        assert_eq!(Mode::Locrian.semitones()[4], 6);
    }

    #[test]
    fn d_dorian_is_the_white_notes_from_d() {
        let k = d_dorian();
        // D E F G A B C — no accidentals anywhere, which is the definition.
        let midi: Vec<i32> = (0..7).map(|d| k.to_midi(d, 0)).collect();
        assert_eq!(midi, vec![62, 64, 65, 67, 69, 71, 72]);
        let letters: Vec<String> = (0..7).map(|d| k.spell(d, 0)).collect();
        assert_eq!(letters, vec!["D4", "E4", "F4", "G4", "A4", "B4", "C5"]);
    }

    // -------------------------------------------- 8-11: degrees and octaves

    #[test]
    fn degree_seven_is_an_octave_above_degree_zero_in_every_mode() {
        for m in [
            Mode::Ionian, Mode::Dorian, Mode::Phrygian, Mode::Lydian,
            Mode::Mixolydian, Mode::Aeolian, Mode::Locrian,
        ] {
            assert_eq!(m.degree_semitones(7) - m.degree_semitones(0), 12, "{}", m.name());
            assert_eq!(m.degree_semitones(-7) - m.degree_semitones(0), -12, "{}", m.name());
        }
    }

    #[test]
    fn a_negative_octave_is_below_the_tonic() {
        let k = d_dorian();
        // The subject's own first note: the dominant a fourth *below* D4.
        assert_eq!(k.to_midi(4, -1), 57);
        assert_eq!(k.spell(4, -1), "A3");
    }

    #[test]
    fn the_dominant_is_degree_four_and_seven_semitones_up() {
        let k = d_dorian();
        assert_eq!(k.mode.dominant_degree(), 4);
        assert_eq!(k.to_midi(4, 0) - k.to_midi(0, 0), 7);
    }

    #[test]
    fn spelling_follows_the_letter_and_not_the_piano_key() {
        // E-flat major's third is G, and its sixth is C. Written as pitch
        // classes these are the same keys as F## and B##; written as music
        // they are not, and a score that says F## is unreadable.
        let e_flat = Key::spelled(63, Mode::Ionian, 2); // Eb4, letter E
        assert_eq!(e_flat.spell(0, 0), "Eb4");
        assert_eq!(e_flat.spell(2, 0), "G4");
        assert_eq!(e_flat.spell(3, 0), "Ab4");
        assert_eq!(e_flat.spell(6, 0), "D5");

        // The same pitches, called by their other name. This is the pair that
        // proves a key is not a pitch: identical MIDI throughout, and not one
        // letter in common.
        let d_sharp = Key::spelled(63, Mode::Ionian, 1); // D#4, letter D
        assert_eq!(d_sharp.spell(0, 0), "D#4");
        assert_eq!(d_sharp.spell(2, 0), "F##4");
        for d in 0..7 {
            assert_eq!(e_flat.to_midi(d, 0), d_sharp.to_midi(d, 0));
        }

        // And with no letter stated, a black key takes the flat reading.
        assert_eq!(Key::new(63, Mode::Ionian).spell(0, 0), "Eb4");
    }

    // ------------------------------------------- 12-16: diatonic operations

    #[test]
    fn transposing_up_four_steps_is_a_fifth() {
        let k = d_dorian();
        let line = vec![Note::new(0, 0, 1.0)];
        let up = transpose_steps(&line, 4);
        assert_eq!(k.to_midi(up[0].degree, up[0].octave) - k.to_midi(0, 0), 7);
    }

    #[test]
    fn a_diatonic_transposition_changes_interval_quality() {
        // This is the point of doing it in steps. D up a diatonic third is F
        // (minor third, 3 semitones); E up a diatonic third is G (also minor);
        // F up a diatonic third is A (major, 4). Same operation, different
        // interval — which a semitone transposition cannot express.
        let k = d_dorian();
        let thirds: Vec<i32> = (0..3)
            .map(|d| {
                let up = transpose_steps(&[Note::new(d, 0, 1.0)], 2);
                k.to_midi(up[0].degree, up[0].octave) - k.to_midi(d, 0)
            })
            .collect();
        assert_eq!(thirds, vec![3, 3, 4]);
    }

    #[test]
    fn inversion_mirrors_every_interval_about_the_first_note() {
        // Up a fourth then down a step becomes down a fourth then up a step.
        let line = vec![Note::new(0, 0, 1.0), Note::new(3, 0, 1.0), Note::new(2, 0, 1.0)];
        let inv = invert(&line);
        assert_eq!(inv[0].step(), 0);
        assert_eq!(inv[1].step(), -3);
        assert_eq!(inv[2].step(), -2);
        // And inverting twice is the identity.
        let back = invert(&inv);
        assert_eq!(back.iter().map(Note::step).collect::<Vec<_>>(), vec![0, 3, 2]);
    }

    #[test]
    fn augmentation_doubles_the_durations_and_nothing_else() {
        let line = vec![Note::new(0, 0, 1.0), Note::new(3, 0, 0.5)];
        let aug = scale_durations(&line, 2.0);
        assert_eq!(line_beats(&aug), 3.0);
        assert_eq!(aug.iter().map(Note::step).collect::<Vec<_>>(), vec![0, 3]);
    }

    #[test]
    fn retrograde_reverses_notes_and_durations_together() {
        let line = vec![Note::new(0, 0, 2.0), Note::new(3, 0, 0.5)];
        let r = retrograde(&line);
        assert_eq!(r[0].beats, 0.5);
        assert_eq!(r[1].beats, 2.0);
        assert_eq!(line_beats(&r), line_beats(&line));
    }

    // ------------------------------- 17-21: the tonal answer, the real point

    #[test]
    fn a_subject_whose_head_touches_the_dominant_asks_for_a_tonal_answer() {
        let head_on_dominant = vec![Note::new(4, -1, 1.0), Note::new(0, 0, 1.0)];
        assert_eq!(answer_kind(&head_on_dominant, Mode::Dorian), AnswerKind::Tonal);
    }

    #[test]
    fn a_subject_that_stays_off_the_dominant_gets_a_real_answer() {
        let head_on_tonic_and_third = vec![Note::new(0, 0, 1.0), Note::new(2, 0, 1.0)];
        assert_eq!(answer_kind(&head_on_tonic_and_third, Mode::Dorian), AnswerKind::Real);
    }

    #[test]
    fn a_dominant_head_is_answered_by_the_tonic_a_fourth_up_not_a_fifth() {
        // The classical rule, and the one thing that makes this a fugue.
        // Subject opens A3 -> D4 (dominant, tonic).
        // Answer must open D4 -> A4 (tonic, dominant): a fourth, then a fifth.
        let k = d_dorian();
        let subject = vec![Note::new(4, -1, 1.0), Note::new(0, 0, 1.0), Note::new(2, 0, 1.0)];
        let a = answer(&subject, Mode::Dorian);
        assert_eq!(k.to_midi(a[0].degree, a[0].octave), 62, "A3 answered by D4");
        assert_eq!(k.to_midi(a[1].degree, a[1].octave), 69, "D4 answered by A4");
        // And past the head, it is a plain fifth: F4 -> C5.
        assert_eq!(k.to_midi(a[2].degree, a[2].octave), 72);
    }

    #[test]
    fn a_real_answer_transposes_every_note_by_the_same_interval() {
        let k = d_dorian();
        // Degree 5 is B, and the diatonic fifth above B in D Dorian is F —
        // a *diminished* fifth, six semitones. Every other degree gives seven.
        // Same operation, different interval; that is the whole reason this
        // module counts in scale steps and not in semitones.
        let subject = vec![Note::new(0, 0, 1.0), Note::new(2, 0, 1.0), Note::new(5, 0, 1.0)];
        let a = answer_with_head(&subject, Mode::Dorian, AnswerKind::Real, 0);
        for (s, x) in subject.iter().zip(a.iter()) {
            assert_eq!(x.step() - s.step(), 4, "every note up a diatonic fifth");
        }
        let sizes: Vec<i32> = subject
            .iter()
            .zip(a.iter())
            .map(|(s, x)| k.to_midi(x.degree, x.octave) - k.to_midi(s.degree, s.octave))
            .collect();
        assert_eq!(sizes, vec![7, 7, 6]);
    }

    #[test]
    fn the_answer_keeps_the_subjects_rhythm_exactly() {
        let subject = vec![Note::new(4, -1, 1.0), Note::new(0, 0, 0.5), Note::new(1, 0, 2.5)];
        let a = answer(&subject, Mode::Dorian);
        assert_eq!(
            a.iter().map(|n| n.beats).collect::<Vec<_>>(),
            subject.iter().map(|n| n.beats).collect::<Vec<_>>()
        );
    }

    // --------------------------------------------- 22-26: vertical analysis

    #[test]
    fn verticals_are_sampled_where_either_line_moves() {
        let k = d_dorian();
        // Upper holds for two beats while lower moves twice: two intervals,
        // not one. Walking only the upper line would find one and miss the
        // parallel that the second creates.
        let upper = vec![Note::new(4, 0, 2.0)];
        let lower = vec![Note::new(0, 0, 1.0), Note::new(1, 0, 1.0)];
        let v = verticals(&upper, &lower, k);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].steps, 4);
        assert_eq!(v[1].steps, 3);
    }

    #[test]
    fn a_perfect_fifth_is_perfect_and_a_diminished_one_is_not() {
        let k = d_dorian();
        // A over D: 4 steps, 7 semitones — perfect.
        let v = verticals(&[Note::new(4, 0, 1.0)], &[Note::new(0, 0, 1.0)], k);
        assert!(v[0].is_perfect_consonance());
        // B over E is *also* a perfect fifth (E4 64 -> B4 71). The tritone in
        // this mode is not there, and the first version of this test asserted
        // it was: the diminished fifth in D Dorian is F over B — B3 59 to F4
        // 65, four scale steps and six semitones. Same step count, different
        // interval, which is exactly why `Vertical` carries both.
        let b_over_e = verticals(&[Note::new(5, 0, 1.0)], &[Note::new(1, 0, 1.0)], k);
        assert_eq!((b_over_e[0].steps, b_over_e[0].semitones), (4, 7));
        assert!(b_over_e[0].is_perfect_consonance());

        let f_over_b = verticals(&[Note::new(2, 0, 1.0)], &[Note::new(5, -1, 1.0)], k);
        assert_eq!((f_over_b[0].steps, f_over_b[0].semitones), (4, 6));
        assert!(!f_over_b[0].is_perfect_consonance());
    }

    #[test]
    fn parallel_fifths_are_found_and_a_held_note_is_not_one() {
        let k = d_dorian();
        let upper = vec![Note::new(4, 0, 1.0), Note::new(5, 0, 1.0)];
        let lower = vec![Note::new(0, 0, 1.0), Note::new(1, 0, 1.0)];
        assert_eq!(parallel_perfects(&upper, &lower, k), vec![1.0]);

        // Same fifth twice, nothing moving: not a parallel.
        let held_u = vec![Note::new(4, 0, 1.0), Note::new(4, 0, 1.0)];
        let held_l = vec![Note::new(0, 0, 1.0), Note::new(0, 0, 1.0)];
        assert!(parallel_perfects(&held_u, &held_l, k).is_empty());
    }

    #[test]
    fn parallel_octaves_are_found_too() {
        let k = d_dorian();
        let upper = vec![Note::new(0, 1, 1.0), Note::new(1, 1, 1.0)];
        let lower = vec![Note::new(0, 0, 1.0), Note::new(1, 0, 1.0)];
        assert_eq!(parallel_perfects(&upper, &lower, k), vec![1.0]);
    }

    #[test]
    fn a_perfect_fifth_makes_a_pair_of_lines_uninvertible() {
        // The defining rule of double counterpoint at the octave: invert a
        // fifth and you get a fourth, and a fourth above the bass is a
        // dissonance. Thirds and sixths invert into each other and are fine.
        let k = d_dorian();
        let fifth = verticals(&[Note::new(4, 0, 1.0)], &[Note::new(0, 0, 1.0)], k);
        assert!(fifth[0].is_perfect_consonance());
        assert!(!is_invertible_at_octave(&[Note::new(4, 0, 1.0)], &[Note::new(0, 0, 1.0)], k));
        assert!(is_invertible_at_octave(&[Note::new(2, 0, 1.0)], &[Note::new(0, 0, 1.0)], k));
        assert!(is_invertible_at_octave(&[Note::new(5, 0, 1.0)], &[Note::new(0, 0, 1.0)], k));
    }

    #[test]
    fn lines_more_than_an_octave_apart_are_not_invertible_at_the_octave() {
        let k = d_dorian();
        let faults = invertibility_faults(&[Note::new(2, 1, 1.0)], &[Note::new(0, 0, 1.0)], k);
        assert!(matches!(faults[0], InversionFault::ExceedsOctave { .. }));
    }
}
