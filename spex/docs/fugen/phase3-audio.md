# Phase 3 - the fugue, at runtime (M67-M71)

*A generated four-voice fugue and a synthesised techno layer, in the browser, in sync.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**
Read [`README.md`](README.md) first: it carries the working rules, the scope decision and the milestone index.
Review record and the reasoning behind every rev-2 change: [`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md).

---


The piece is named for a double meaning — *Fuge* as the joint between two
stones, *Fuge* as the contrapuntal form. Playing a recording would be a
missed opportunity and a licensing question; generating the counterpoint
from the same seed that generates the visuals makes the double meaning
literal. That is the choice made here, and §11 records the alternative.

## Rev 3 — the form is binding, and it is already the form of the film

Stefan set out the classical shape and asked that it be *the* structure of the
music, not a flavour of it. It is now binding on `FugueSpec`, and the useful
discovery in writing it down is that **the film is already built on it.** The
double name stops being a pun at this point: the acts and the fugue are the
same object seen twice.

| Fugue | What it is | Where it already is in the film |
|---|---|---|
| **Dux / Subject** | The fugue opens with one voice alone | **A1-S03** — the alto enters alone, and by rev 3's own direction *after* the edges land, over one bar of silence |
| **Comes / Answer** | The next voice repeats the subject, a fifth away | **A1-S04** — the soprano's tonal answer as the first part of the monolith lands |
| **Countersubject** | While voice 2 carries the subject, voice 1 continues **against** it with its own independent line | Runs under A1-S04 onward. Rev 1 had it optional. **It is now mandatory** — see below |
| **Durchführung** (entries through the voices) | The subject travels through every voice by rule | **A1-S05** (tenor), **A1-S06** (bass); the exposition closes exactly at the end of Act I |
| **Zwischenspiel** (episode) | Between entries the voices move freely and **modulate**, before the subject returns | Act II's construction passages, and the whole Atlas movement |
| **Stretto** | Entries overlap before the previous one has finished | **A3-S05** — already scored there, against the visual multiplication |
| **Pedal point** | A held bass note under the final approach | **A4-S04**, the last bars before the Kick |

### Three tightenings this forces on M67/M68

1. **The countersubject is no longer optional.** `FugueSpec.countersubject`
   loses its `?`. And it must be **invertible counterpoint** — writable both
   above and below the subject (double counterpoint at the octave; at the
   tenth if the generator can manage it). That is not a decoration: a fugue
   swaps its voices, and a countersubject that only works above the subject
   collapses the moment the parts exchange. `counterpoint.rs` gets an explicit
   `is_invertible(subject, countersubject) -> bool` check, tested.
2. **An episode must modulate, and must not introduce new material.** Rev 1's
   `episode` had `sequenceInterval` and `motifFrom` but never said where it
   *lands*. It now carries `targetDegree`: the episode ends in the key of the
   entry that follows it, reached by real sequence on subject-head or
   countersubject-tail material. An episode that invents a new tune is not an
   episode, it is an interlude — and that is the single easiest way for a
   generated fugue to stop sounding like one.
3. **Tonal answer, not real transposition**, wherever the subject's head
   touches the dominant. Already in M67; restated because it is the first
   thing that sounds wrong if the generator takes the easy path.

### The 84-bar plan of the canonical cut

Bars are the screenplay's bars ([`screenplay.md`](screenplay.md) §1: 84 bpm,
4/4, one bar = 20/7 s). Subject length: **2 bars**. This is the plan
`FugueSpec.plan` encodes; the generator fills in the notes.

| Bars | Section | Detail |
|---|---|---|
| 0–5 | *silence, then the 55 Hz sine* | The piece has no music until the brick is legible |
| **5–7** | **Exposition, entry 1** | Alto, subject, tonic. Alone |
| **7–9** | **entry 2** | Soprano, **tonal answer**, dominant. Countersubject in the alto |
| 9–11 | Episode 1 | Sequence on the subject head, modulating to the tonic |
| **11–13** | **entry 3** | Tenor, subject, tonic |
| 13–14 | Episode 2 | Short, one sequence step |
| **14–16** | **entry 4** | Bass, answer. **Exposition complete at bar 17 = the end of Act I** |
| 17–30 | **First Durchführung** | Entries in related keys under Act II's construction. Episodes carry the modulations |
| **30** | Cadence gesture | The coin strike in A2-S04 lands on it |
| 34–37 | Cadence | The first full cadence, closing Act II exactly at bar 37 |
| 37–53 | **Second Durchführung** | Inversion and, if the generator manages it, one augmented entry under the patent studio |
| **53–57** | **Stretto** | Entries overlapping at half the subject's length, with the visual multiplication |
| 57–63 | **Atlas episode** | One voice per site — three sites, three voices — over a walking bass. The fourth voice returns as the Atlas ends |
| 63–76 | Development against the pulse | The percussive layer enters at bar 63, half-time |
| **76–80** | **Subject in the bass, sixteenths** | Same intervals, different century (A4-S03) |
| **80–83½** | **Pedal point** | Held bass under all four voices |
| **83½–84** | **The final accent** | One event, both meanings of *Kick* |
| 84 = 0 | The 55 Hz sine returns at the same phase | The seam ([`screenplay.md`](screenplay.md) §6) |

**A note for whoever implements M68.** The mapping above is not decoration
that can be quietly dropped when the constraint solver struggles. If a section
cannot be realised within the contrapuntal rules, the generator **relaxes a
rule and records the relaxation** (M68's existing requirement) — it does not
move an entry off its bar. The bars belong to the film.

---

### M67 — `fugue.json`: the score format and the subject

**Files.** `spec/fugue.schema.json`, `crates/spex-fugue/src/theory.rs`,
`crates/spex-fugue/src/lib.rs`.

**The specification carried in `show.json`:**

```ts
export interface FugueSpec {
  version: 1;
  /** Beats per minute of the canonical cut. Longer cuts do NOT slow the
   *  tempo — they add episodes and entries. */
  bpm: number;                      // 84 for the canonical cut
  meter: [number, number];          // [4, 4]
  /** Real diatonic mode, by name. The work uses Dorian on D — the mode of
   *  the oldest continuously notated European repertoire, chosen for the
   *  archaeological register of Act I. */
  mode: { tonic: number; name: 'ionian'|'dorian'|'phrygian'|'lydian'|'mixolydian'|'aeolian'|'locrian' };
  voices: 4;
  /** The subject, as scale degrees (0-based within the mode) and durations
   *  in beats. Authored, not generated — this is the one musical decision
   *  the piece does not delegate to an algorithm. */
  subject: { degree: number; octave: number; beats: number }[];
  countersubject?: { degree: number; octave: number; beats: number }[];
  /** Structural plan; the generator fills in the notes. */
  plan: FugueSection[];
  /** The Act IV techno layer. */
  pulse: PulseSpec;
}

export type FugueSection =
  | { kind: 'exposition'; entries: { voice: number; transposition: 'tonic'|'dominant'; atBar: number }[] }
  | { kind: 'episode'; bars: number; sequenceInterval: number; motifFrom: 'subject-head'|'countersubject-tail' }
  | { kind: 'entry'; voice: number; degree: number; atBar: number; inversion?: boolean; augmentation?: number }
  | { kind: 'stretto'; bars: number; overlapBeats: number; voices: number[] }
  | { kind: 'pedal'; bars: number; voice: number; degree: number }
  | { kind: 'cadence'; bars: number; type: 'authentic'|'plagal'|'phrygian' };

export interface PulseSpec {
  /** From which show-time the percussive layer enters (Act IV). */
  enterAtSec: number;
  bpmMultiplier: number;            // 2 — half-time feel doubling into 168
  /** The Kick: one final accent, sample-aligned to the camera Kick. */
  finalAccentAtSec: number;
}
```

**Rust side** (`theory.rs`): pitch class, interval, mode, scale-degree →
MIDI note number, transposition, inversion, augmentation/diminution. All
pure, all unit-tested against known real values (a real Dorian scale on D is
D E F G A B C; the real tonal answer to a subject beginning on the dominant
transposes the head by a fourth, not a fifth — implement the real
tonal/real-answer distinction and test it).

**Acceptance criteria.**

1. `fugue.schema.json` exists and validates a real generated `fugue.json`.
2. Theory unit tests: 20 real, checkable assertions (mode spellings,
   interval inversion, tonal vs. real answer).
3. The authored subject is recorded in the spec file *and* in `TODOs.md`,
   in both scale-degree notation and letter names, so it is human-checkable.

**Verification ladder.** 1, 2, 7.

---

### M68 — the counterpoint generator

**Files.** `crates/spex-fugue/src/counterpoint.rs`, `emit.rs`.

**Requirements.**

- Realises each `FugueSection` into concrete notes for 4 voices.
- Enforces real, checkable contrapuntal constraints, each implemented as a
  named predicate with its own unit test:
  - no parallel fifths or octaves between any voice pair;
  - no voice crossing (except where explicitly permitted in stretto);
  - each voice within its real range (S: C4–A5, A: F3–D5, T: C3–A4,
    B: E2–C4);
  - dissonances (2nd, 7th, tritone) only as passing tones, suspensions
    prepared and resolved down by step, or on weak beats;
  - the leading tone resolves upward at cadences.
- Where the constraint solver cannot satisfy all rules, it relaxes them in
  a documented, fixed priority order and *records the relaxation* in the
  emitted score (`"relaxations": [...]`) rather than silently breaking a
  rule. Honest output over pretend-perfect output — the same standard the
  rest of this project holds data to.
- Deterministic from `(seed, spec)`.
- Optional `--midi <file.mid>` export via a minimal SMF type-1 writer
  (hand-rolled, ~150 lines; no new dependency) so the score can be opened in
  any notation program for human review. **This is a review tool, not the
  runtime path.**

**Acceptance criteria.**

1. A generated canonical-cut score has zero parallel fifths/octaves —
   asserted by a test that re-analyses the emitted notes.
2. The exposition has exactly four subject entries, alternating
   tonic/dominant, in the order specified.
3. The stretto section's entries genuinely overlap (assert entry onsets are
   closer together than the subject's own length).
4. The exported `.mid` opens in a real notation program and is
   *listened to by a human* before the milestone closes. This one cannot be
   automated; the milestone note records who listened and what they said.

**Verification ladder.** 1, 2, 3, plus the human listen.

---

### M69 — the WebAudio engine

**Files.** `viewer/src/audio/engine.ts`, `synth.ts`, `reverb.ts`.

**Signal chain.**

```
per-voice: oscillator bank (additive, 6 partials, per-voice partial weights)
           -> per-partial gain -> ADSR gain -> voice pan -> voice bus
voice bus -> [dry] ------------------------------> mix bus
          -> [send] -> ConvolverNode (procedural IR) -> mix bus
pulse bus -> kick/hat/clap synths -> saturation (WaveShaper) -> mix bus
mix bus   -> master EQ (3-band BiquadFilter) -> DynamicsCompressor (limiter)
          -> masterGain -> destination
```

**Requirements.**

- **No audio assets.** Every sound is synthesised. The reverb impulse
  response is generated at startup into an `AudioBuffer` (exponentially
  decaying filtered noise, seeded) — a cathedral-length tail for Acts I–II,
  a short plate for Act III, a gated tail for Act IV, cross-faded by the
  timeline.
- **Voice model:** the four contrapuntal voices are a soft additive organ
  (partials 1, 2, 3, 4, 6, 8 with a slow chiff transient), because it
  sustains — a plucked voice would make counterpoint inaudible.
- **Pulse model:** kick = pitch-swept sine + click; hat = filtered noise
  burst; clap = three short noise bursts. All standard, all synthesised.
- **The Kick (the drum one):** a single accent whose onset is scheduled to
  the exact `AudioContext` time that the camera Kick begins. The two Kicks
  are the same event; §7 is explicit about this.
- Master limiter so no cut ever clips, at any quality tier.

**Acceptance criteria.**

1. Audio graph builds with zero console warnings; `AudioContext.state` is
   `running` after the user gesture.
2. A 60-second capture (via `MediaStreamDestination` + `MediaRecorder` in
   the headless session, or an `OfflineAudioContext` render) shows: no
   samples at ±1.0 (no clipping), RMS within a 6 dB band across the run.
3. CPU: audio thread under 10% on the development machine at 4 voices +
   pulse.

**Verification ladder.** 1, 2, 5 (**mandatory**), plus a human listen.

---

### M70 — the scheduler and runtime realisation

**Files.** `viewer/src/audio/scheduler.ts`, `fugue.ts`, `midi.ts`.

**Requirements.**

- Standard lookahead scheduler: a 25 ms `setInterval` tick schedules every
  note whose onset falls within the next 150 ms, using absolute
  `AudioContext.currentTime` values. Never schedule from `requestAnimationFrame`.
- The scheduler reads the *same* `ShowClock`; on `seek`, it flushes pending
  voices with a 20 ms release ramp (never an abrupt cut) and re-primes.
- `midi.ts` defines the event model (`NoteOn`/`NoteOff`/`Tempo`/`Marker`
  with tick and second stamps) and an optional SMF *reader*, so a real,
  clearly-licensed `.mid` can be substituted for the generator later
  without touching the synth or the scheduler. The generator is the default
  and the only path used by the shipped piece.
- Cue emission: every subject entry, every section boundary, and every
  pulse accent emits a `Cue` the visual side can bind to (M71).

**Acceptance criteria.**

1. Note onsets measured from a rendered `OfflineAudioContext` capture are
   within 3 ms of their scored times over a 4-minute run.
2. Seeking to 5 arbitrary times and playing 3 s from each produces musically
   correct material for that position (verified by comparing rendered
   spectra against a full-run render at the same offsets).
3. No stuck notes after 100 randomised seek/pause/play operations.

**Verification ladder.** 1, 2, 5 (**mandatory**), 7.

---

### M71 — audio↔visual binding, autoplay policy, and the mixer UI

**Files.** `viewer/src/show/timeline.ts` (cue handling),
`viewer/src/audio/engine.ts`, `viewer/index.html`.

**Requirements.**

- **Autoplay policy.** Browsers will not start an `AudioContext` without a
  gesture. The piece opens on a title card — *Die Geschichtliche Matrix*,
  the archive signature, and a single "▶ begin" affordance. This is not a
  workaround dressed as a feature; it is the correct behaviour for an
  installation piece and it is also the gate that starts the clock. A
  `?mute=1` path starts the visuals immediately with a silent clock, for
  embedding.
- **Bindings** (each authored as `Cue`s in `show.json`, not hardcoded):
  - subject entry → a brief emissive lift on the instance group belonging
    to the entering voice's assigned scene element;
  - section boundary → HUD movement card;
  - pulse accent → bloom strength pulse in Act IV;
  - the final accent → the Kick.
- **Mixer UI**: master volume, mute, and a "counterpoint only / pulse only /
  both" monitor switch, in the controls panel. Useful for review, and
  harmless to ship.

**Acceptance criteria.**

1. Audio and visuals stay in sync over a full 60-minute run: sample the
   drift at 10-minute intervals; ≤ 20 ms at every sample.
2. The Kick's audio onset and the first frame of the camera Kick are within
   one frame (16.7 ms) — measured, recorded.
3. Muting mid-run and unmuting does not desync anything.

**Verification ladder.** 1, 2, 5 (**mandatory**), plus a human listen.

---
