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

### MIDI is the format. One file, no second one.

Rev 3 had two score formats: `fugue.json` as the runtime path and a `.mid`
export as a review tool, plus a `.mid` reader as an "escape hatch". That is
one format too many. **Dropped: the generator emits a standard SMF, the
browser loads it, and that is the score.**

| Was (rev 3) | Is (rev 4) |
|---|---|
| `fugue.json` — runtime score | **gone** |
| `.mid` export — review only, never loaded | **`fugue.mid` — the score. Loaded at runtime, and openable in any DAW** |
| `.mid` reader — optional escape hatch | **the runtime path**, so it is exercised every single run instead of rotting |
| `fugue.schema.json` | **stays**, but only for the *plan* — subject, sections, tempo map. The input, not the output |

What this buys, beyond one less format: the review tool and the runtime are
the same file, so anything Stefan hears in Logic is exactly what the piece
plays. A bar can be hand-edited and it just works. And the SMF reader cannot
silently rot, because nothing plays without it.

**The one thing MIDI cannot do for us**, and the only reason `synth.ts`
exists: a browser has no MIDI sound generator. `AudioContext` is not a
General MIDI device — a `.mid` is a note list, and something still has to
make a sound. Shipping a soundfont would be several megabytes with its own
licence, against the loading budget and the no-audio-assets rule. So the
synth stays. It is ~200 lines and it was always going to exist.

Sending MIDI *out* to real hardware is [`backlog.md`](backlog.md) B4.

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

**Verification ladder.** 1, 2, 3. (6 runs at the end of the phase.)

**Status: ✅ done.** `crates/spex-fugue/src/{theory.rs, model.rs}`,
`spec/fugue.schema.json`, the `audio` block in
`shows/die-geschichtliche-matrix.show.json`, and `audio: Option<FugueSpec>` on
`spex_show::Show`. **42 tests** — 27 theory, 11 on the authored music, 4 on the
document.

**One correction to the milestone's own title.** It is called
"`fugue.json`: the score format", and rev 4 deleted that format two sections
above: the score is a standard MIDI file emitted by M68, and `fugue.schema.json`
describes the **plan**, which is the input. So AC1 reads "the schema validates
the real `audio` block", and that is what is tested. Nothing here emits a score.

**AC1 passes**, and against the real document rather than a fixture: the Act I
`audio` block validates, and `show.schema.json` now `$ref`s the fugue schema —
which needed a `Retrieve` implementation in the test, because a validator with
no retriever tries to fetch a sibling file over HTTP. Loosening `audio` to
"any object" would have dodged that and validated nothing where the music is.

**AC2 passes with 27 assertions where 20 were asked for**, and three of them
were wrong the first time — which is the point of writing them as checkable
claims rather than as confidence. Dorian's semitones, every mode's fifth,
`degree_semitones(7)` an octave above `degree_semitones(0)` in all seven modes,
diatonic transposition changing interval *quality* while the step count stays
put, inversion being its own inverse, and the tonal/real answer distinction
tested against the classical example rather than against the implementation.

**AC3 passes: the subject is in the spec file, in `TODOs.md`, and in the
document**, in scale degrees and in letter names, and a test asserts the three
agree. It is:

> **A3 – D4 – G4 – F4 – E4 – B4 – A4 – D4**, two bars, D Dorian.
> Degrees `4/-1, 0/0, 3/0, 2/0, 1/0, 5/0, 4/0, 0/0`;
> durations `1, 1, ½, ½, 1, 2, 1, 1`.

Three decisions, each of them asserted rather than described:

- **It opens on the dominant**, so the answer *must* be tonal — the head's A is
  answered by D, up a fourth, not up a fifth. A subject that never touches the
  dominant in its head would never exercise the one rule that separates a fugue
  from a canon.
- **It carries 1 : 4 : 9.** The monolith's proportions, three times over: it
  rests on the **tonic**, it opens with a leap of a **fourth**, and its compass
  is a **ninth**. `the_subject_carries_the_monoliths_proportions` asserts all
  three, so the subject cannot be edited into disagreeing with the object it is
  about.
- **It holds the sixth.** B natural is the one note that separates D Dorian from
  D minor, and it is the longest note in the subject, on the downbeat of bar 2,
  at its highest point. A mode nobody hears is a mode that was not chosen.

**The countersubject is: (two beats rest) – E4 – C4 – D4 – G4 – F4 – B3**, and
the rests are the interesting part.

The first draft covered all eight beats and passed every check against the
subject. It failed against the **answer** — an unprepared second on beat 1, a
perfect fifth on beat 3 — and the reason is structural, not a slip:

> **A countersubject cannot cover the subject's head, because the head is
> precisely the part that is not the same in the answer.**

That is what a tonal answer *is*. Any countersubject written to fit the
subject's first two notes is guaranteed to fight the answer's. The classical
solution is the obvious one once seen — the voice that has just finished the
subject takes a breath — and it needs a rest, which is why `Note` gained one.
It also thins the texture to a single voice exactly where the second entry
arrives, which is what makes an entry audible as an entry.

Invertibility is **checked, not claimed**: `theory::invertibility_faults`
returns the beat of every violation, the countersubject was written against it
rather than verified after the fact, and there are three tests — against the
subject, against the answer, and with the two parts actually swapped an octave.

**Two design decisions the spec left open, settled here.**

- **`mode.tonic` is a MIDI note number, not a pitch class.** A pitch class
  would need a second octave reference from somewhere else, and somewhere else
  is where octave errors live.
- **A key needs a letter, and a pitch does not supply one.** E-flat major and
  D-sharp major are the same keys on a piano and different keys in a score. The
  first version derived the tonic's letter from its pitch class by nearest
  distance, which for a black key is a tie broken by array order; it spelled a
  key that had asked for E-flat as D-sharp, and the test caught it. `Key` now
  carries an optional `tonicLetter`, and absent means the flat reading — which
  is unambiguous for every white-key tonic, including this piece's D.

**Not done here, deliberately:** no notes are generated, nothing is emitted,
nothing makes a sound. M68 realises the plan and writes the SMF; M69 gives the
browser something that can play it.
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
- **The output is a type-1 SMF, and it is the only score artefact.** A
  hand-rolled writer (~150 lines, no new dependency). It is what the browser
  loads and what a human opens in a DAW — the same file, so review and
  runtime can never disagree.

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

**Verification ladder.** 1, 2, 3, plus a human listen. (6 runs at the end of the phase.)

**Status: ⏳ AC1–AC3 done, AC4 waiting on a human.**
`crates/spex-fugue/src/{counterpoint.rs, emit.rs}` and `spex fugue-build`.
The canonical cut realises to **475 notes in four voices over 84 bars**, a
4 473-byte type-1 SMF.

**Every check runs against the score that comes out, not the one in memory.**
`emit::read_smf` exists for exactly that: analysing the `Realisation` would
test the generator against itself and would pass unchanged if the writer
dropped every second note. So the tests write the file, read it back, and run
the same predicates on what they find.

**AC1 passes: zero parallel fifths or octaves** in the emitted score. It did
not at first — there were **six on one beat**, every voice pair at once, which
is the signature of one specific mistake and of nothing else: `realise_cadence`
wrote the same two pitches into every idle voice, and four voices moving from
the same note to the same note *is* parallel octaves in all six pairs. The
cadence now gives the progression to the lowest idle voice and puts the others
through the same constrained chooser as everything else — which already knows
every rule, so the fix was to stop hand-writing music the generator could
choose better.

**AC2 passes:** four entries, alto → soprano → tenor → bass, alternating tonic
and dominant, at bars 5, 7, 11 and 14, each of them actually sounding notes.
**AC3 passes:** the stretto's entries are 4 beats apart against an 8-beat
subject, so they genuinely overlap.

**Six relaxations in 84 bars, and the breakdown is the interesting part:**
0 parallel, **4 range**, 1 voice crossing, 1 weak-beat dissonance.

The four range breaches are one fact repeated: **this subject does not fit
every voice at every transposition, and the generator will not move it.** The
subject spans a ninth — A3 to B4 at the tonic, fourteen semitones — and the
tenor's stated range is C3–A4, twenty-one semitones, which is wider and still
does not hold it: D4 sits near the tenor's top, so the subject goes two
semitones over the ceiling at pitch and three under the floor an octave down.
There is no octave that works. The same happens to the bass on the answer and
to the soprano at bar 21.

That is a consequence of the authored subject, and the rule that produces it is
deliberate: **entries are placed, free voices are chosen.** A subject statement
is the fugue's fixed material and is never adjusted to make a rule pass — if it
does not fit, the least-bad octave is used and the breach is recorded with its
bar and its voice. Every one of the four is over by exactly two semitones,
which a singer would call a stretch rather than an impossibility, and the test
asserts that bound rather than asserting there are none.

**A seed that changed nothing.** The chooser put the PRNG in its last sort key,
which sounds like a tie-break and was not one: melodic motion decided every
comparison before the tie-break was ever reached, and two different seeds
produced byte-identical files. It now chooses among the candidates that are
*equally good* — at most the top three — so `?seed=` is a real edition and not
a promise the format goes on making about nothing.

**AC4 is the one that cannot be automated, and it is open.** The file has been
delivered to Stefan as `die-geschichtliche-matrix.mid`, together with a
soundfont render for anyone without a DAW to hand. **The milestone stays open
until a person has listened and this block records who and what they said** —
which is the criterion as written, and the only one here that a test could
never have replaced.
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

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture), plus a human listen. (6 runs at the end of the phase.)

**Status: ✅ AC1–AC3 measured; the human listen is open.**
`viewer/src/audio/{engine.ts, synth.ts, reverb.ts}` and
`scripts/viewer-shot/audioprobe.mjs`.

**Rung 5 does not apply and this is the correction the rev-2 rule exists for.**
The ladder marks it mandatory "because this milestone changes the picture" —
it does not. M69 adds no render pass, no material, no geometry; nothing here
reaches a frame. What it changes is the *sound*, and the equivalent of a
screenshot for sound is a render, which is what the probe does. The picture
arrives in M71, where the binding is.

**Everything is measured on an `OfflineAudioContext`**, which is why
`AudioEngine` takes a `BaseAudioContext` and not an `AudioContext`. That one
type turns "does this clip" from a thing someone listens for into arithmetic
over samples.

**AC1 passes: zero console warnings or errors**, three procedural spaces
(cathedral 4.2 s, plate 1.4 s, gated 0.85 s), 3 EQ bands, 6 partials a voice.
The impulse responses are **seeded and verified as such**: same seed gives
bit-identical buffers, a different seed differs, and the two channels are
decorrelated — an IR whose channels match is a mono reverb wearing two
speakers.

**AC2 passes, after the criterion caught two real bugs and then had to be
split in two.**

*The clipping was real and my own comment was wrong about why it could not
happen.* The first version ended in a `DynamicsCompressor` and claimed that
made "no cut ever clips" a property of the graph. **It measured 1.376 on the
first render.** A compressor is not a limiter: it has an attack, so transients
pass, and a ratio above a threshold still permits overshoot. The fix is a
**bounded transfer function** — a `WaveShaper` whose `tanh` curve is clamped to
±0.985, which cannot be exceeded for any input at all.

*And then the ceiling itself was wrong, twice, in ways only a measurement
finds.* Adding it took the peak **up**, to 1.44. Two mistakes:

- **`WaveShaper`'s curve is indexed by input over [-1, 1], and inputs outside
  that are clamped to its ends.** I built the table over ±8 on the theory that
  a loud input needed somewhere to land, which is exactly backwards: an input
  of 0.125 read the entry for 1.0 and came out at 0.75. An eightfold gain,
  dressed as headroom. The node's own clamping *is* the headroom.
- **`oversample` must be `'none'` on a ceiling.** Oversampling runs the signal
  through an upsample filter, the curve, and a downsample filter, and that last
  filter rings — on the near-square signal the first bug produced, the
  overshoot measured **46% over the ceiling**, which is the classic Gibbs
  figure and could not be anything else. Oversampling is right for a saturator,
  where the goal is to avoid aliasing in something meant to be heard, and wrong
  for a bound, because a filter after the bound can exceed it.

Isolated, the ceiling now caps an 8× overdrive at **0.98496**. In the full
60-second worst case — 24 subject entries in stretto, 79 pulses under them —
**peak 0.7564, nothing at full scale**.

*The 6 dB band needed a control, and the control is what makes the number mean
anything.* Across the whole passage the level moves **11.67 dB**, which reads
like a failure and is not: the passage deliberately grows from one voice to
four plus percussion, and four voices is **+6 dB of arithmetic before any
music**. So the probe renders a second passage with the texture held constant —
four voices, no pulse, no entries piling in — and measures **1.00 dB across 20
windows**. The engine's level is stable to a decibel; the 11.67 is the
exposition's own crescendo, which is the thing the exposition *is*. Compressing
it away to satisfy a number would be fitting the instrument to the answer.

**AC3 is bounded rather than met, and says so.** The criterion is "audio thread
under 10% on the development machine", which needs a real-time context on real
hardware. What this container can say is that the whole passage *schedules* in
**56.6 ms** and renders 60 seconds of four voices plus pulse in 20.9 s —
**2.9× faster than real time** in headless software Chromium with no audio
device. That bounds the cost loosely and honestly; the criterion as written is
for the M92 hardware.

**One design decision worth stating, because it is about the music and not
about taste.** The four contrapuntal voices are a soft additive organ —
partials 1, 2, 3, 4, 6, 8, with a chiff at the onset — because **it sustains**.
Counterpoint is the perception of several lines at once, and a line audible
only at its attack is not a line: a plucked voice decays through the bar and
leaves the listener tracking a sequence of events instead of following four
simultaneous melodies. Every instrument fugues were written for sustains. The
chiff is what keeps that from turning to mud, since an entry has to be audible
*as* an entry.

The pulse bus is deliberately **not** sent to the reverb. A kick in a cathedral
is mud; the percussion belongs at the front of the room while the voices are at
the back of it, and that contrast is most of what will make Act IV feel like a
different place.
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
  with tick and second stamps) **and the SMF reader, which is the runtime
  path** — the piece loads `fugue.mid` and plays it. Substituting a
  different, clearly-licensed `.mid` therefore needs no code at all: drop in
  another file.
- Cue emission: every subject entry, every section boundary, and every
  pulse accent emits a `Cue` the visual side can bind to (M71).

**Acceptance criteria.**

1. Note onsets measured from a rendered `OfflineAudioContext` capture are
   within 3 ms of their scored times over a 4-minute run.
2. Seeking to 5 arbitrary times and playing 3 s from each produces musically
   correct material for that position (verified by comparing rendered
   spectra against a full-run render at the same offsets).
3. No stuck notes after 100 randomised seek/pause/play operations.

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture). (6 runs at the end of the phase.)

**Status: ✅ done.** `viewer/src/audio/{midi.ts, scheduler.ts, fugue.ts}` and
`scripts/viewer-shot/scheduleprobe.mjs`. All three criteria measured, and each
of them was wrong first in a way worth keeping.

**The reader is the runtime path, and that is what makes it un-rottable.**
Rev 4 deleted a format so that one file is the score: M68 writes a standard
MIDI file, `midi.ts` reads it, and the browser plays what a person opens in a
DAW. So the reader is not an escape hatch someone remembers to exercise —
nothing sounds without it, and every run is a test of it. Substituting a
different, clearly-licensed `.mid` needs no code at all, which for a work
whose licensing questions are live ([`licensing.md`](licensing.md)) is not a
small property.

**`a.pos += f()` when `f` advances `pos`.** The reader parsed, produced 475
notes, and put them at times that were *nearly* right. JavaScript evaluates
the left-hand reference before calling the right-hand function, so
`tr.pos += tr.vlq()` computes `oldPos + length` and throws away the bytes the
length field itself occupied — one byte behind for the rest of the track,
every subsequent delta read from the middle of some other event, and a parse
that desynchronised into plausible-looking garbage rather than an error. It
was found by printing a tick of 111 and recognising it as the letter `o` of
"Soprano". The first pass over the same bytes did not have the bug, because it
reads meta payloads with `bytes(len)` instead of skipping them: **two readers
of one format, one of them subtly wrong.**

**AC1 passes: 191 scored soprano onsets, 0 missed, median 0.22 ms, p95
1.04 ms, worst 1.71 ms** over the full 240 s — measured from rendered audio,
not from the scheduler's own account of itself. Three substitutions are made
for the measurement and all three are stated in the harness: a fast attack
(the organ's 35 ms ramp is not an edge, and an instrument that cannot resolve
3 ms cannot measure 3 ms), one voice (four lines attacking within a few
milliseconds cannot be separated by any detector, and the question is
scheduling rather than polyphony), and a tap at the voice bus rather than the
master output — for which see the next paragraph.

**The mastering chain delays everything by a measured 5.986 ms, and nothing
in the Web Audio API says so.** Onsets taken from the master output were
consistently ~6 ms late with a spread under a millisecond, which is never a
scheduling error and always a delay line. An impulse through a bare
`DynamicsCompressorNode` with this project's settings comes out **264 samples
later at 44.1 kHz**; the waveshaper and the EQ add none. A compressor has to
look ahead — a limiter that reacted only to samples it had already passed on
could not attenuate the transient that triggered it — and the lookahead is a
pre-delay on the signal path. A constant delay on everything is inaudible.
**It is M71's problem**: "the Kick's audio onset and the first frame of the
camera Kick within one frame" is a 16.7 ms budget, and six of those
milliseconds are spent before any binding code exists. `measureOutputLatency()`
therefore lives in `engine.ts` and measures it at runtime rather than trusting
a constant, because the number depends on the browser and the sample rate.

**Two onset detectors were wrong before this one, and the arithmetic says why
each had to be.** The first used a level threshold with hysteresis and found
**2 onsets in four minutes**: with four sustaining voices the envelope never
falls back below the disarm level, so it arms once and never again. Sustain is
the entire point of the instrument (M69), so the detector has to look at the
*rise*. The second smoothed |x| with a one-pole filter and took its flux, and
found **1107 onsets in a part that has 191 notes**: |sin| ripples at twice the
fundamental, so at 220 Hz the envelope bumps every 2.3 ms, and any smoother
slow enough not to follow that is too slow to resolve 3 ms. **There is no
setting of that filter that works.** What works is a **running maximum** over
more than one period: ripple-free by construction, and — unlike any low-pass —
it rises on the very sample a louder signal arrives, because a maximum has no
time constant.

**And then the setting that mattered came from the music rather than from
tuning.** Even with the right envelope, a note's 50 ms release beats against
the next note's attack and the beat is a rise. Raising the level threshold
suppressed those *and* the quietest real entries — 50 spurious detections and
**two real onsets missed**. The shortest interval between two soprano onsets
in this score is a quaver at 84 bpm, **0.357 s**, so any second detection
within 0.25 s of the first cannot be a note: a 250 ms refractory period leaves
**8 spurious and none missed**. The threshold was a guess about levels; the
refractory is a fact about the piece.

**AC2 passes: 0.9956, 0.9898, 0.9936, 0.9959, and a silence.** Two things had
to be fixed before the numbers meant anything, one in the scheduler and one in
the criterion.

*The scheduler was silent at bar 81.* The first `seek()` placed the cursor at
the next note and stopped, on the reasoning that a note has an attack and
starting one in the middle is a click with a pitch. That is true, and it
produced **nothing at all** when the harness landed at t = 221.7 s — the piece
has a three-and-a-half-bar pedal point there, one held note under everything
else, started long before. Playing nothing is not "musically correct material
for that position", it is the absence of the material. A spanning note is now
resumed for its *remaining* duration and the click is solved where it belongs,
in the envelope.

*And the criterion compared against the wrong thing, twice.* The reference was
a full-run render — which the harness had rendered **soprano-only** for AC1, so
four of the five seeks were being compared against a different piece of music.
The reference is now a render that **played into** the moment from six seconds
earlier, so it arrives with the releases and the reverb tail a listener would
have. Then t = 221.7 s: it is inside the ten-second caesura before the final
chord, the seek correctly produced silence, and a cosine similarity between a
silence and a reverb tail is 0. **The right answer was being reported as a
failure by a metric that could not represent it.** A position the score leaves
empty is now compared against the score: silent where silent is correct, and
the reference's residual 0.030 rms of tail is printed rather than pretended
away — a fresh seek cannot have it and should not.

**AC3 passes: 0 pending, 0 sounding after 100 randomised operations** (29
seeks, 39 pauses, 32 plays). It reported four pending, and they were not a
leak: the run ended on a seek **while paused**, and `seek()` was resuming the
spanning notes. Scrubbing a stopped show must not start a note — nothing will
ever come along to stop it, because nothing is playing — so the resume is now
gated on the clock. The harness also runs time forward past every scheduled
release before counting, since a note whose release is still in the future is
just a note.

**Rung 5, and what it was worth.** The milestone is marked *mandatory* for the
screenshot, which was written before rev 2's rule; M70 changes no pixel. The
frames were shot anyway because M70 touches `viewer/src`, and they are
identical to M66's. Two console 404s, both `/tileset/cuts.json` — that
directory has no cuts index and `fetchCutsIndex` returning `null` is the
designed behaviour. `cargo test --workspace --no-fail-fast`: **273 pass, 1
fails for want of `sqlite3` on this container's PATH**, unrelated and
pre-existing.
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

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture), plus a human listen. (6 runs at the end of the phase.)

**Status: ✅ done, AC1–AC3 measured; the human listen is still M68's AC4.**
`viewer/src/show/{binding.ts, hud.ts, player.ts}`, `viewer/src/audio/engine.ts`,
`viewer/index.html`, `crates/spex-fugue/src/emit.rs`, and
`scripts/viewer-shot/{bindprobe.mjs, bindframes.mjs}`.

**The pulse went into the score, and so did the form.** `spex fugue-build` now
writes a General MIDI **channel-10** drum track and **22 marker events**
alongside the four voices: 928 notes in six tracks, 9,025 bytes. A runtime
generator reading `PulseSpec` would have worked and would have meant that the
file a person opens in a DAW is missing an entire layer of the piece — against
rev 4's one-artefact rule, which is the rule this whole phase turns on. The
markers are not documentation either: they *are* the section list M71 binds the
HUD card to, so the caption cannot name a stretto the music is not playing.
**The 475 contrapuntal notes are byte-identical to M68's file**, checked rather
than assumed.

**And channel 10 is not a voice.** Two of M68's tests failed the moment the
drums existed, both correctly: note 36 against note 39 is a minor third that
never moves, so a rule checker handed the kit alongside the fugue reports
parallel thirds in a kick drum. The filter is in the *tests*, not in the
reader — a reader's job is to report what is in the file.

**A cue is bound to when its note sounds, not to when its callback arrives.**
M70's scheduler hands a cue over up to **150 ms** before it is audible, which
is the entire point of a lookahead scheduler; a visual bound to the arrival
would fire a tenth of a second before its own note, every time, for ever.
`Scheduler.onCue` therefore carries the absolute `AudioContext` time the cue
will sound at, `CueBinder` holds it until that time has come, and the frame
loop asks "what is due?" against the same clock the sound is on. The binding is
accurate to one frame **by construction** rather than by tuning.

**AC1 passes: 60 minutes, sampled every 10, worst binding latency 0.999
frames** — and the criterion had to be restated to mean anything.

*What "in sync" cannot be.* Show time already **is** audio time: `ShowClock`
reads `currentTime` when there is an `AudioContext` (M62), so a drift between
them is a drift between a number and itself. It measures **0.000 ms at every
one of the six samples**, and reporting that as an achievement would be the
emptiest possible pass. What can move is the *binding's* latency, and it is
bounded below by the frame interval and by nothing else.

*So the assertion is ≤ 1 frame, and the 20 ms is a 60 Hz number.* One frame at
60 Hz is 16.7 ms. This container renders the show at **4.6 fps**, so the same
code that will be within 16.7 ms on the premiere hardware measures within
**772 ms** here — the milliseconds are the rasteriser's and the frame count is
the code's. Same doctrine as M69's AC3, and the same reason.

*And the run produced a number worth more than the criterion.* Over the hour,
**`performance.now()` and `AudioContext.currentTime` drifted 1.014 seconds
apart** — 793 ms of it in the first ten minutes. M62 chose the audio clock on
the argument that "a browser's audio clock and its high-resolution timer
routinely differ by tens of milliseconds"; on this container they differ by a
**second an hour**, and the piece stayed in sync with its own music throughout
because it reads the one the music is on. That argument is now a measurement.
(Headless Chromium has no audio device, so its context runs on a null sink —
the figure bounds the effect rather than characterising real hardware, and says
which.)

**AC2 passes: the Kick's binding applied 147.8 ms — 0.684 of a frame — after
its audio onset.** One honest limitation, recorded rather than worked around:
the camera half of the Kick is an `exponentialZoom` shot **authored in Act IV**,
which is not built. What M71 adds is the binding, and the binding's own latency
is what is measured; the camera is already on show time and cannot be late
against it.

**AC3 passes: 0.000 ms of drift while muted and 0.000 ms after unmuting**, the
clock still on `audio`, still playing. That is not a foregone conclusion —
**`?mute=1` and the mixer's mute are different things** and the difference is
exactly the sort that gets collapsed: the parameter decides *which clock the
show reads* (no `AudioContext`, so `performance.now()`), while the mixer ramps
a gain on a context that is still ticking. M66 already watched a suspended
context hold the opening frame for two minutes; a mute that took the clock with
it would do the same thing on demand.

**Two seams, both found by building this and neither in a component.**

*The per-instance scalars were indexed against the wrong thing, and had been
since M59.* `aDissolve` lived as an attribute on the level-0 mesh, indexed by
**instance** — while since M59 the LOD selector re-packs each level's matrix
buffer, so mesh row *j* holds instance *i* only as long as every brick is at
level 0. One demotion and the dissolve erodes a different brick from the one
the timeline named. It had never bitten because M65's probe shoots the brick
close up, where nothing demotes. `dissolve` and the new `lift` are now
authoritative arrays with a per-level packed copy — **the rule `matrices` has
followed since M59**, now followed by the scalars too, and `LodSelector.repack`
packs all three in one pass.

*The endless edition would have played the fugue once and then looped in
silence.* The scheduler's cursor is monotonic by design (M70), so a show time
that jumps back to zero leaves it past every note in the file, for ever, with
no error anywhere. `clock.onLoop` re-seeks the score. The 60-minute run went
round **15 times** and applied cues in every one of them, which is how this is
known and not asserted.

**Rung 5 was mandatory here and earned it.** The first version of the entry
lift used a scale of **1.6**, chosen by analogy with the dissolve rim's 2.5 —
and that analogy is wrong by a whole object: the rim multiplies a term that is
non-zero on a thin band of fragments which only just survived the erosion,
while the lift multiplies **every fragment of the brick**. The screenshot pair
measured the difference at **89 luma out of 255** on a white monolith. Not a
voice announcing itself: an object replaced by a light source. At **0.18** it
measures **5.05 luma** and the monolith is still a monolith.

The frames are shot in pairs — identical camera, identical show time, lift held
at 1 and then at 0 — because an emissive addition is easy to claim and easy to
not actually make. The first version of *that* shot the two frames a second
apart with the piece still playing, and reported the lift as **−1.2 luma**: it
was measuring a dolly. A pair of frames that differ in one thing has to differ
in one thing.

**Also measured.** 142 cues derived from the score (36 entries, 22 sections, 84
accents); the gate shown before `begin()` and gone after; the mixer's three
rows reachable and the monitor switch moving `both → pulse → counterpoint →
both`; **zero console warnings or errors** across the hour and across the
frames. `cargo test --workspace --no-fail-fast`: **273 pass**, the one failure
being `sqlite3` missing from this container's PATH, unrelated and pre-existing.

**One small thing the probe found about the DOM.** The mixer's three rows were
built with the HUD's `el()` helper, which sets an **id** — three elements
sharing an id is a document no selector and no stylesheet can talk about, and
the probe counted zero rows on a mixer that was plainly there. They are a class
now.

---
