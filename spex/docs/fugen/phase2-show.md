# Phase 2 - the runtime show engine (M60-M66)

*The screenplay as data, one deterministic clock, transforms evaluated per frame instead of baked.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**
Read [`README.md`](README.md) first: it carries the working rules, the scope decision and the milestone index.
Review record and the reasoning behind every rev-2 change: [`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md).

---


### M60 — the `show.json` format

**Why.** The screenplay must be *data*, not code. Everything that follows —
the four duration cuts, seeded editions, seeking, live re-authoring —
depends on the piece being a declarative document that a resolver can
transform.

**Files.** `spec/show.schema.json`, `spec/show-resolved.schema.json`,
`crates/spex-show/src/model.rs`.

**The model.**

```ts
// The authored source document (show.json)
export interface Show {
  version: 1;
  id: string;                    // "die-geschichtliche-matrix"
  title: string;
  subtitle?: string;
  archiveSignature: string;      // "IA-2026-002"
  baseDurationSec: number;       // 240 — the canonical cut
  seed: number;                  // default edition seed
  palette: Record<string, [number, number, number]>;  // sRGB 0..1
  scenes: SceneRef[];
  movements: Movement[];
  audio: FugueSpec;              // see §5
  credits: CreditsSpec;
}

export interface SceneRef {
  id: string;                    // "monolith"
  /** A real .ldr path, or a generator invocation resolved at build time. */
  source:
    | { kind: 'ldr'; path: string }
    | { kind: 'build'; recipe: string }        // spex-build descriptor, M72
    | { kind: 'flag'; flag: string }           // spex-flag spec id, M75
    | { kind: 'heritage'; siteId: string };    // spex-heritage id, M73
  /** Instance-id prefix, so choreography can address subsets by glob. */
  prefix: string;
}

export interface Movement {
  id: string;                    // "act-1"
  title: string;                 // "Archäologie der Fuge"
  romanNumeral: string;          // "I"
  shots: Shot[];
}

export interface Shot {
  id: string;                    // "A1-S04"
  title: string;
  /** Proportional share of the movement's stretchable time. */
  weight: number;
  /** fixed  — duration never changes, in any cut (the Kick).
   *  stretch — duration scales with the cut's length, clamped to min/max.
   *  repeat  — the shot's body loops N times; N scales, one pass does not. */
  scaling: 'fixed' | 'stretch' | 'repeat';
  durationSec: number;           // the canonical-cut duration
  minSec?: number;
  maxSec?: number;
  /** 1 = present in every cut. 2 = only in cuts >= 600s. 3 = only >= 3600s. */
  tier: 1 | 2 | 3;
  /** For scaling: 'repeat' — how the repeat count is derived. */
  repeat?: { unitSec: number; minCount: number; maxCount: number };
  scenes: string[];              // SceneRef ids active in this shot
  camera: CameraTrack;
  tracks: Track[];
  cues: Cue[];
  /** Free-form direction, carried into the resolved output so the piece
   *  documents itself — displayed by the debug HUD with ?director=1. */
  note?: string;
}

export type Track =
  | { kind: 'transform'; target: string; keys: Keyframe<TransformValue>[] }
  | { kind: 'dissolve';  target: string; keys: Keyframe<number>[] }
  | { kind: 'material';  target: string; property: MaterialProperty; keys: Keyframe<number | [number,number,number]>[] }
  | { kind: 'post';      property: PostProperty; keys: Keyframe<number>[] }
  | { kind: 'hud';       element: string; keys: Keyframe<number>[] }
  | { kind: 'pointCloud'; target: string; keys: Keyframe<number>[] };  // mesh<->point crossfade

export interface Keyframe<T> {
  /** Normalised shot-local time, 0..1 — so a keyframe survives retiming. */
  t: number;
  value: T;
  easing: EasingName;
  /** Optional: snap this keyframe to the nearest musical beat instead of
   *  to normalised time. The resolver rewrites `t` accordingly. */
  snapToBeat?: boolean;
}

export interface TransformValue {
  position?: [number, number, number];
  /** Euler XYZ in degrees, or a quaternion — quaternion wins if both. */
  rotation?: [number, number, number];
  quaternion?: [number, number, number, number];
  scale?: number | [number, number, number];
}

export interface CameraTrack {
  /** 'keyed' — explicit keyframes. 'orbit' — parameterised turntable.
   *  'dolly' — straight-line move. 'exponentialZoom' — the Kick. */
  mode: 'keyed' | 'orbit' | 'dolly' | 'exponentialZoom';
  fovDeg?: number;
  keys?: Keyframe<CameraValue>[];
  orbit?: { center: [number,number,number]; radius: number; height: number; startDeg: number; endDeg: number };
  dolly?: { from: [number,number,number]; to: [number,number,number]; lookAt: [number,number,number] };
  exponentialZoom?: { from: number; to: number; lookAt: [number,number,number] };
  /** Shutter-style motion blur strength, 0..1. */
  motionBlur?: number;
}

export interface Cue {
  /** Normalised shot-local time. */
  t: number;
  kind: 'audio' | 'hud' | 'seed' | 'marker';
  payload: Record<string, unknown>;
}
```

`target` is a glob over instance ids (`"monolith/*"`, `"atlas/site-07/**"`,
`"flag/dk/tile-*"`), resolved once at load into an instance-index list —
never re-globbed per frame.

**Acceptance criteria.**

1. Both schemas exist, are self-contained JSON Schema 2020-12, and are
   listed in `spec/README.md`'s table.
2. A minimal hand-written `show.json` (one movement, two shots) validates.
3. `crates/spex-cli/tests/schema_validation.rs` validates a real
   `spex show-build` output against `show-resolved.schema.json`.

**Verification ladder.** 1, 2, 3. (6 runs at the end of the phase.)

**Status: ✅ done.** `crates/spex-show/{Cargo.toml,src/lib.rs,src/model.rs}`,
`spec/show.schema.json`, `spec/show-resolved.schema.json`,
`shows/die-geschichtliche-matrix.show.json` (Act I, real), and
`ldraw-scenes/brick-1x1.ldr` (A1-S02 and A1-S03 must sample and render *one*
piece of geometry, or the crossfade cannot converge). 16 tests pass.

**Four decisions this milestone made against the draft model above.**

- **Time is authored in bars, not seconds.** The draft used `durationSec`.
  The screenplay is written in bars at 84 bpm 4/4 and sets itself the rule
  that every cut lands on a bar line — which is unenforceable in seconds,
  since one bar is 20/7 s and `2.857142857` is a bar only if you type enough
  sevens. `Shot` carries `durationBars`/`minBars`/`maxBars`, `Show` carries
  `tempo` and `baseDurationBars`, and seconds are derived. The canonical cut
  is 84 bars = 240.000 s *exactly*, and there is a test that says so.
  M61's resolver spec still reads in seconds; it works in seconds internally
  and converts at the boundary, which is correct — the *authoring* unit and
  the *arithmetic* unit do not have to match.
- **`MaterialProperty` gained `edgeOpacity`.** Authoring A1-S03 found the
  gap: the crossfade lands the mesh and M57's outlines arrive **one frame
  later**, and with no channel of its own that beat could only be written as
  a comment. A closed property set is only useful if adding to it is the
  normal way to make a shot authorable.
- **`scale` is a single number, not a vector.** Non-uniform scale on a
  Klemmbaustein is a lie about the module — the whole thesis is that the part
  has one true size.
- **No `audio: FugueSpec` field yet.** The draft has one; `FugueSpec` is
  defined by M67–M70 and does not exist. Adding the field now would mean
  inventing a shape with no producer and no consumer, and every guess would
  be frozen into version 1. It lands with M67.

**AC3 moved to M61 — and is now met there.** It validates a real
`spex show-build` output, and `show-build` is M61's binary — the criterion
could not be met here by anything except a hand-written file pretending to be
output. `crates/spex-cli/tests/schema_validation.rs::show_build_output_matches_its_schema_and_is_deterministic`
is the real thing. What *was* done
instead: `crates/spex-show/tests/documents.rs` hand-writes a minimal
`show-resolved.json` and validates it, because a schema that has never had a
valid instance is a schema nobody has checked. That fixture is now the
contract M61's resolver has to hit.

**One real defect, found by rung 2 and not by any test.** The first draft of
the Act I document addressed geometry as `monolith/*`. Running the real
`spex mesh-model` on the real scenes and *reading* the ids showed why that is
wrong: a bundle numbers its own instances `<part>/<n>` — `3010/0`, `3710/7` —
so a scene-prefixed id is `monolith/3010/0`, with **two** separators. A `*`
does not cross a separator, which is precisely what makes the spec's own
`flag/dk/tile-*` example meaningful, so `monolith/*` matches nothing at all,
silently, and the shot simply never animates. Corrected to `**` throughout,
written into the schema's own description, and asserted by a test — this is
the failure mode a glob has, and it produces no error anywhere.

**Measured and recorded.** The Act I document is 17 bars — 0:48.571 — over
six shots at 2/2/3/4/3/3, which is what `screenplay.md` §4 says, asserted
per-shot rather than in total so a compensating pair of errors cannot pass.
Three scenes, one of them (`stonehenge`) a `heritage` source with no
generator until M73 — which is the point of the four source kinds: the
document does not have to know which milestone produces its geometry.

---

### M61 — `spex-show`: the compiler and the duration resolver

**Why.** The four cuts are not four edits; they are four resolutions of one
document. This is the milestone that makes that true.

**Files.** `crates/spex-show/src/resolve.rs`, `compile.rs`,
`crates/spex-cli/src/show.rs`, `main.rs`.

**The resolver algorithm** (specify precisely; it is the heart of §8):

```rust
pub struct ResolveOptions {
    pub target_sec: f64,     // 240.0 | 600.0 | 3600.0
    pub seed: u64,
    /// Endless mode resolves to `base_duration_sec` and marks the output
    /// `"endless": true`; the viewer then loops it with a per-cycle seed
    /// advance (see M82).
    pub endless: bool,
}

pub fn resolve(show: &Show, opts: &ResolveOptions) -> Result<ResolvedShow>;
```

1. **Tier filter.** Keep shots with `tier == 1`; add `tier == 2` when
   `target_sec >= 600`; add `tier == 3` when `target_sec >= 3600`.
2. **Fixed budget.** `F = Σ durationSec` over `scaling == 'fixed'` shots.
   If `F > target_sec`, error out — a cut shorter than its own fixed
   material is not resolvable, and silently dropping a fixed shot would be
   worse than failing.
3. **Repeat expansion.** For each `scaling == 'repeat'` shot, its duration
   is `count * unitSec`; `count` is solved in step 4 as a continuous
   variable and then rounded to an integer, with the rounding residual
   pushed back into the stretch pool.
4. **Water-filling over the stretch pool.** Let `R = target_sec - F`.
   Distribute `R` across the stretchable shots proportionally to `weight`.
   Any shot whose share exceeds `maxSec` is clamped and removed from the
   pool; any shot below `minSec` is raised and removed. Repeat until no
   clamping occurs (a standard water-filling fixpoint; terminates because
   each pass removes at least one shot).
5. **Assert.** `|Σ resolved durations − target_sec| < 1e-3`. If the clamps
   make the target unreachable, error with the exact shortfall — never
   silently deliver a 9:47 "10-minute" cut.
6. **Beat snapping.** With the tempo map from the resolved `FugueSpec`,
   rewrite every `snapToBeat` keyframe's absolute time to the nearest beat,
   and every shot boundary flagged `snapToBeat` to the nearest bar line.
   Then re-run step 5's assertion; absorb the drift into the *last*
   stretchable shot of the movement.
7. **Absolutise.** Emit `show-resolved.json`: every shot with absolute
   `startSec`/`endSec`, every keyframe with absolute `timeSec`, every glob
   pre-resolved to instance indices, every scene referenced by bundle path.

**CLI.**

```
spex show-build <show.json> -o <show-dir> [--duration 240|600|3600] [--endless] [--seed <n>] [--cache-dir <dir>]
spex show        <show-dir> [--port 8080] [--no-open]
spex show-export <show.json> -o <static-dir> [--durations 240,600,3600,endless]
```

`show-build` compiles every referenced scene into a mesh bundle (M52/M53),
generates the fugue score (M67–M70, Rust side), resolves the timeline, and
writes:

```
<show-dir>/
  show-resolved.json
  fugue.json
  bundles/<sceneId>/mesh.json + buffers/
  assets/                      (HUD text, SVG chronicle cards)
```

**Acceptance criteria.**

1. A property test: for 200 random `(weight, min, max)` configurations and
   targets in [60, 7200], `resolve` either errors explicitly or produces a
   timeline summing to the target within 1 ms.
2. Resolving the real `show.json` at 240/600/3600 produces exactly
   240.000/600.000/3600.000 s.
3. Tier filtering is verified: the 240 s cut contains no tier-2 shot; the
   3600 s cut contains every tier-3 shot.
4. `show-build` is deterministic: same seed → byte-identical output.

**Verification ladder.** 1, 2, 3. (6 runs at the end of the phase.)

**Status: ✅ done.** `crates/spex-show/src/resolve.rs` and `compile.rs`,
`crates/spex-cli/src/show.rs` and `main.rs`. 36 tests pass.

**The split is not the one the file list implies, and the reason is Phase 6.**
`compile.rs` binds globs to instance indices and nothing else; the scene
*building* lives in `crates/spex-cli/src/show.rs`. `spex-show` therefore still
depends on nothing but serde — which matters because
[`phase6-wasm.md`](phase6-wasm.md) exposes `resolve_show(show_json, …)` to the
browser, and a resolver that dragged the whole LDraw part resolver behind it
would be the reason that wasm bundle is measured in megabytes. Resolving a
timeline is arithmetic over text; it should link like it.

**Two additions to the algorithm as specified.**

- **Step 6 quantises to beats, not just `snapToBeat` keyframes.** Water-filling
  produces arbitrary reals, and the screenplay's own rule is that cuts land on
  the grid — so when the target is a whole number of beats *and* every fixed
  shot is (336 / 840 / 5040 beats for the three cuts), every duration is
  rounded to a whole beat by **largest remainder**, which sums to the target
  *by construction* rather than approximately. Beats and not bars because the
  screenplay itself contains a half-bar shot: DER KICK is two beats, and a bar
  grid could not represent it. When a caller asks for a duration that is not
  beat-aligned the continuous solution is kept and `beatAligned: false` says
  so — the total is exact either way.
- **The unreachable-target message had to stop inventing tiers.** At 3600 s
  every tier is already in, so "add tier-4 material" is advice about something
  that does not exist. It now says the document is too short.

**AC1 passes.** 200 pseudo-random `(weight, min, max, scaling, tier)`
configurations against targets in [60, 7200]: every one either resolves to
within 1 ms of its target or refuses with a reason, and the test asserts that
**both** outcomes occur — a property test where nothing is ever refused is
only testing the easy half. splitmix64 rather than a crate, so a failure is
reproducible from the seed in the panic.

**AC2 passes for 240; 600 and 3600 correctly refuse, and that is the honest
result.** The document is Act I only — 17 bars, six shots, whose `maxBars`
add up to 108 bars = 308.571 s. There is no arrangement of six shots that
fills ten minutes, and the resolver says exactly that: *"the clamps make
600.000 s unreachable: the timeline resolves to 308.571 s, -291.429 s off."*
Making that AC pass by widening a maximum would be authoring the piece to fit
its test. The resolver's ability to hit 240 / 600 / 3600 exactly is proven
against a document that can reach them
(`the_three_canonical_cuts_are_exact_to_the_millisecond`); this AC becomes
meetable on the real document when Phase 5 authors Acts II–IV.

**AC3 passes** on synthetic documents, for the same reason: Act I has no
tier-2 or tier-3 material yet. 240 s keeps tier 1 only, 599 s still does,
600 s admits tier 2, 3600 s admits tier 3.

**AC4 passes**, with its scope stated: two runs of the real `spex show-build`
at the same seed are byte-identical. Nothing in the resolver *consults* the
seed yet — the seeded choices are M74's site selection and M82's cycle
advance — so today this asserts the weaker true thing, and keeps asserting the
right one once those land. What it does prove now is that the largest-remainder
tie-break is order-stable, which is the only place floating point could have
made two runs differ.

**Measured on the real document at 240 s.** Boundaries `5.714 | 20.000 |
31.429 | 40.000 | 71.429 | 71.429`, every one on a beat, summing to 240.000.
A1-S02, S03 and S04 all resolve **at their ceilings** (7 / 11 / 14 bars) and
S05 and S06 absorb the rest at equal weight — which is the water-filling doing
visibly what it is for, and also a note for Phase 5: three of Act I's six
shots have no headroom left at four minutes.

**One real defect, and only reading the output found it.** A1-S03 keys three
tracks at the same shot-local `t`: the point-cloud crossfade, the edge arrival
and the start of the brick's revolution. Two were marked `snapToBeat` and the
third was not, so the resolver put two on beat 51 and left the third 0.343 s
earlier — the brick began turning before the outlines it exists to reveal had
arrived. Nothing failed. No test was red, no console printed anything; the
numbers were simply different. There is now a test that every key authored at
the same `t` within a shot resolves to the same second, which is the failure
mode beat snapping actually has.

**`--no-bundles` and `--skip-unbuildable` are both consequences of honesty.**
The Act I document references `heritage/stonehenge`, whose generator is M73.
Building it silently without that scene would produce a show that is missing
its last image and says nothing about it, so `show-build` fails and names the
milestone; `--skip-unbuildable` is the explicit opt-in, and it then prints
every dropped scene and every target left matching nothing. `--no-bundles`
resolves the timeline alone, which is what the duration arithmetic and the
schema tests want, and turns a live LDraw fetch into a few milliseconds.

**`spex show` and `show-export` are deferred to M66**, where the runtime that
would play a show directory exists. A verb that serves a directory no viewer
can read yet is a verb that only looks finished.

---

### M62 — the clock and the timeline evaluator

**Why.** Everything — visuals, camera, music, HUD — must read the *same*
time value, or the piece drifts apart. One clock, one source of truth.

**Files.** `viewer/src/show/clock.ts`, `timeline.ts`, `easing.ts`.

**Signatures.**

```ts
// clock.ts
export class ShowClock {
  constructor(durationSec: number, opts: { endless: boolean; audioContext?: AudioContext });
  /** Current show time in seconds. When an AudioContext is present this is
   *  derived from `audioContext.currentTime` — the only clock in a browser
   *  that does not drift against what you hear. `performance.now()` is the
   *  fallback for muted/no-audio sessions. */
  get time(): number;
  get cycle(): number;             // completed loops, for endless seed advance
  get playing(): boolean;
  play(): void;
  pause(): void;
  seek(sec: number): void;
  /** Called once per rAF; returns the delta actually applied. */
  tick(): number;
  onLoop(cb: (cycle: number) => void): void;
}

// timeline.ts
export class Timeline {
  constructor(resolved: ResolvedShow);
  /** Which shot(s) are active at time t — normally one, two during a
   *  cross-dissolve. */
  activeShots(t: number): ActiveShot[];
  /** Evaluates every track of every active shot and pushes the results
   *  into the supplied sinks. Allocation-free after warm-up: all scratch
   *  vectors/quaternions are preallocated members. */
  evaluate(t: number, sinks: TrackSinks): void;
  /** Fires cues whose time was crossed since the previous call. Monotonic
   *  in normal playback; a `seek` resets the cursor rather than firing
   *  every cue in between. */
  fireCues(prevT: number, t: number, handler: (cue: ResolvedCue) => void): void;
}

export interface TrackSinks {
  instances: InstanceWriter;       // M55
  camera: CameraSink;              // M63
  post: PostSink;                  // M58
  hud: HudSink;                    // M65
}
```

**Easing library** (`easing.ts`) — named, pure, unit-tested, all
`(t: number) => number` on [0,1]: `linear`, `quadIn/Out/InOut`,
`cubicIn/Out/InOut` (the existing `ease_in_out_cubic` in `brick.rs` is the
reference implementation — port it, do not re-derive it), `quartInOut`,
`expoIn/Out/InOut`, `circInOut`, `backOut`, `elasticOut`, `bounceOut`,
`step` (hold), and `smootherstep`. Plus `cubicBezier(x1,y1,x2,y2)` for
authored curves.

**Acceptance criteria.**

1. Seeking to t and playing forward 1 s produces the same state as playing
   from 0 to t+1 s (state hash comparison over all instance transforms).
2. With an `AudioContext` present, visual time and `audioContext.currentTime`
   stay within 5 ms over a 60-second run.
3. `evaluate` allocates zero objects per frame after the first 60 frames
   (verified with a Chromium heap-allocation sampling profile).
4. Every easing function is unit-tested for `f(0)===0`, `f(1)===1`, and
   monotonicity where it applies.

**Verification ladder.** 1, 2, 3. Rung 5 is **not** applicable here, and
the correction is the point of rev 2's rule: this milestone adds no render
pass, no material and no geometry, and nothing it produces reaches a frame
until M66 wires it up. There is no picture for a screenshot to be of. Rung 5
belongs to M63, where a camera first moves. (6 runs at the end of the phase.)

**Status: ✅ done.** `viewer/src/show/{easing.ts, clock.ts, timeline.ts,
resolved.ts}` plus `scripts/viewer-shot/showprobe.mjs`, which measures all four
criteria in real Chromium against the real resolved Act I document.

**`resolved.ts` was not in the file list and had to exist.** The evaluator
reads `show-resolved.json`, so something has to describe its shape in
TypeScript. It mirrors the schema by hand, the same way `mesh/bundle.ts`
mirrors `mesh.schema.json` — generating it would be one more build step to
keep alive, and the schema is already validated against real `show-build`
output on the Rust side, so the schema is the authority and this is a reader.

**Time is derived, never accumulated.** Not `time += delta`. Show time is
*(source now − source reading when playback last started) + the offset it
started from*. Accumulating deltas accumulates their rounding — sixty
additions a second for an hour is 216 000 of them — and makes a dropped frame
permanent, where derived time is already correct on the very next tick after
a stall of any length.

**AC1 passes, at four points in the show.** Seeking to *t* and evaluating one
second produces a byte-identical FNV hash of every value the evaluator emits,
compared against the same window reached by playing from zero. `7e08f98d`,
`8ed0bc5c`, `2772e07c`, `4468f859` — seeked and played-through, identical at
all four. And the one piece of state the evaluator *does* keep gets its own
check: reaching 75 % by seeking fires **0** cues, reaching it by playing fires
**9**. A seek that replayed three minutes of accents would be audible.

**AC2 passes by four orders of magnitude, and the interesting number is the
other one.** Show time against `audioContext.currentTime`: worst **0.0000 ms**
over 722 frames — which is not luck, it is what "derived, never accumulated"
means. The number worth recording is the comparison the design exists for:
`performance.now()` against the audio clock drifted **18.3 ms in 12 s**, about
90 ms a minute. **That rate is a headless-Chromium figure, not a hardware
one** — there is no audio device here and the context runs on a synthesised
clock, so the magnitude should not be quoted as what a real machine does. What
it does show is that the two clocks are genuinely independent, which is the
whole reason the show reads the one the sound is on.

**AC3 passes, and the positive control is the part that makes it mean
anything.** Chromium heap sampling at a 512 B interval over 6 000 frames:
`evaluate` 1.90 B/frame against an empty loop's 1.19 B/frame — the difference
is sampling noise. The measurement is only worth stating because the same
instrument, in the same run, sees **28.25 B/frame** from a loop that allocates
one small object per frame. So the instrument can see what is being claimed
absent.

**The first version of that control measured nothing, and said so
confidently.** It built 6 000 objects into a local array and reported only
`array.length`, so V8's escape analysis proved they never leave the loop and
allocated none of them — 6 000 "allocations" came out as 1.4 kB. A positive
control the optimiser is free to delete is not a control. It now parks the
array on `globalThis`.

**AC4 passes for all sixteen curves**, with one correction to the criterion:
`bounceOut` is **not** monotonic and must not be asserted to be. It reaches
1.0 at t≈0.364 and falls away again, three times — that is what a bounce is.
`backOut` and `elasticOut` overshoot to 1.100 and 1.373 by design. All
sixteen have exact endpoints and all sixteen clamp outside [0,1] rather than
extrapolating, which matters because a keyframe segment gets sampled a hair
past its end by floating point. `cubicBezier(0,0,1,1)` reproduces the identity
to 0.00e+0.

**`cubicInOut` is a port of `brick.rs::ease_in_out_cubic`, not a
re-derivation** — same branch, same expressions. M64 has to match the baked
`brick-assembly` demo to within 0.01 mm, and two independently written cubics
agree to about three decimals.

**One design note that will look like an omission later.** Euler rotations are
interpolated componentwise and deliberately *not* wrapped to a shortest path.
A1-S03 turns the brick 0° → 360°, and any "take the short way round" step —
which is exactly what a quaternion conversion does — turns a full revolution
into no revolution at all. The quaternion path is still there and still
slerps; it is correct for every swing up to a half-turn, which is every swing
except that one.

---

### M63 — the camera director

**Files.** `viewer/src/show/camera.ts`.

**Requirements.**

- All four `CameraTrack.mode`s implemented.
- `exponentialZoom` is the Kick: distance `d(t) = from * (to/from)^ease(t)`
  with `ease = expoIn`. At `to/from = 1e4` over 800 ms this must not
  produce depth-buffer artefacts — dynamically adjust `camera.near`/`far`
  to `[d/1e4, d*1e4]` each frame and call `updateProjectionMatrix()`.
- Motion blur: a velocity-buffer pass is out of scope; use a
  camera-velocity-driven radial blur in the composer, strength from
  `CameraTrack.motionBlur`. Document it as a stylistic approximation.
- OrbitControls remain available but are **disabled during playback** and
  re-enabled on pause (`?free=1` forces free camera for inspection).

**Acceptance criteria.**

1. The Kick captured at 120 fps headless shows a monotonic, artefact-free
   collapse to a single bright pixel cluster ≤ 3×3 px in the final frame.
2. No near/far clipping visible at any point in the zoom.
3. `?free=1` gives full orbit control without breaking the timeline.

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture). (6 runs at the end of the phase.)

**Status: ✅ done.** `viewer/src/show/camera.ts`, a radial-blur pass in
`viewer/src/mesh/post.ts`, and `scripts/viewer-shot/kick.mjs`.
Screenshots: [`m63-kick-start.png`](screenshots/m63-kick-start.png),
[`m63-kick-mid.png`](screenshots/m63-kick-mid.png),
[`m63-kick-end.png`](screenshots/m63-kick-end.png).

**The last frame is the piece's own last frame, and it fell out of the
geometry.** At 3 000 000 mm the monolith is a single bright point at frame
centre on black — 2×2 px, 4 lit pixels — which is the image A1-S01 opens on.
Nothing was faked to get it: it is a nine-brick stack rendered at 10⁴ its
framing distance.

**`exponentialZoom` needed a direction, and the format did not have one.** A
distance and a look-at point do not determine a position. `ZoomSpec` gained an
optional `direction`, defaulting to the piece's own framing axis
(`[0, 0.15, 1]`), in the model and both schemas — a director that silently
picks an axis is a director that picks a *different* one after a refactor.

**The spec's near/far is a mistake and is not implemented.** It asked for
`[d/1e4, d·1e4]`: a far:near ratio of **10⁸**, which is worse than the static
1:20 000 the viewer already had and would guarantee the artefacts it was
written to prevent. What matters is bracketing the scene at the current
distance, and the scene is small compared with `d`: `near = d/100`,
`far = d·10` is a ratio of **10³**, five orders tighter, and still clears an
object of radius up to 0.99·d.

**Honest result on that correction: the two are pixel-equivalent on this
scene.** Driving the zoom twice, once under each policy, showed **zero** sudden
losses of the object under either and a mean difference of 7.3 lit pixels
across 36 frames — which is the variance between two page loads, not an
artefact. Nine bricks with no near-coplanar surfaces do not z-fight at 10⁸
either. So the correction rests on the arithmetic and not on a picture, and
that is recorded rather than dressed up: the run was done to find a visible
difference and did not find one.

**AC1 passes: 9 883 → 4 lit pixels, 182×189 → 2×2 px, two frames rising by
more than 2 (worst +27).** Measured with **motion blur off**, and that
separation is the milestone's real methodological point — see below.

**AC2 passes:** no sudden loss of the object at any point of the pull-back
under either depth policy.

**AC3 passes:** with `?free=1` the camera does not move by 10⁻⁹ mm over 30
timeline frames while `controls.enabled` is true — *and the show keeps
running underneath*, which is the part worth testing. Near and far still
track the timeline's distance (30 000 / 30 000 000 at the end) and the blur
strength is still being computed. Only the transform is withheld.

**Two measurement defects, both the same defect the project has now hit
three times.** The first version counted *bright* pixels, and reported the
object **growing by 1 181 px** while it was in fact shrinking — it was
measuring the ground plane, whose lit area changes as the camera pulls back.
That is M59's dolly confound exactly, and it takes M59's fix: render each
frame twice with the same camera, once with the bricks and once without, and
count the pixels that differ. The second was subtler. With that fixed the
collapse still showed nine rises, worst +1 942 — and the cause was the motion
blur, which *legitimately* grows an object's footprint while the object
shrinks. Measuring the collapse through the blur is measuring the blur. So
the harness now drives the zoom three times: blurred for the pictures,
unblurred for the collapse, and spec-range for the depth comparison. The
blurred run is reported alongside (9 883 → 12 px, worst rise +1 977, final
cluster 4×4) as evidence that the effect is doing something, rather than
being quietly excluded.

**Motion blur is a stylistic approximation and is named as one in the
code.** A real velocity buffer needs a previous-frame matrix per object and a
second render target, which at Atlas scale costs more than the effect is
worth. The pass streaks radially from the shot's focus point, driven by the
camera's own speed relative to its viewing distance — very nearly the truth
for a dolly or the Kick, merely plausible for an orbit. It sits **before** the
grade pass so it smears linear radiance: blurring already-tone-mapped pixels
darkens the streak as it spreads, which is backwards for a bright object in
motion. It is disabled at zero strength rather than run with a no-op uniform,
because most shots hold their camera and a full-screen pass that provably
changes nothing is still a full-screen pass.

**And the three-deep build order caught me again**, in exactly the way
`docs/agents/verification.md` says it does: the harness died on
`post.setMotionBlur is not a function` because the served binary still
embedded the previous `viewer/dist`.

---

### M64 — runtime choreography (retiring baked frames for the show)

**Why.** This is Gap B closed. `spex frame-sequence` and
`spex brick-assembly` stay exactly as they are — they remain the right tool
for a quick demo. The *show* stops using them.

**Files.** `viewer/src/show/choreography.ts`.

**Requirements.**

- The `transform` track type applied per instance through `InstanceWriter`.
- The scattered-start choreography that `brick.rs::start_translations`
  bakes today is re-expressed as a *generated* transform track at
  `show-build` time: same deterministic splitmix seeding, same
  `FLOAT_HEIGHT_LDU`/`SCATTER_RADIUS_LDU` constants, so the assembly reads
  identically to the existing `brick-assembly` demo — but evaluated at
  runtime. Port the constants; do not invent new ones.
- Per-instance stagger: an instance's own eased progress is offset by
  `stagger * index / count`, so parts land in sequence rather than
  simultaneously. Where the scene has real `buildStep` data (from
  `0 STEP` lines), the stagger follows the *real build order* instead of the
  index — this is the point of having parsed build steps at all.

**Acceptance criteria.**

1. A side-by-side headless capture of `spex brick-assembly
   ldraw-scenes/monolith.ldr` (baked) and the show's Act I S04 (runtime) at
   matching normalised times: per-part positions agree within 0.01 mm.
2. Measure and record the per-frame transform cost for the 9-part monolith
   and a 5 000-part Atlas site, via `__spexMesh.benchTransforms()` — a CPU
   number that means the same thing on every machine. **Frame rate is not
   asserted here**: this pipeline has no GPU outside M92, and M55 already
   measured 6.3 ms for 50 000 instances through the matrix path against a
   "< 4 ms" that was written for hardware this is not.
3. `buildStep`-ordered stagger visibly differs from index-ordered stagger
   on a scene that has real `0 STEP` lines (use a real official model).

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture). (6 runs at the end of the phase.)

**Status: ✅ done.** `viewer/src/show/choreography.ts`,
`crates/spex-show/src/choreography.rs`,
`docs/fugen/fixtures/assembly-scatter.json`, and
`scripts/viewer-shot/assembly.mjs`. Screenshots:
[`m64-assembly-start.png`](screenshots/m64-assembly-start.png),
[`m64-assembly-mid.png`](screenshots/m64-assembly-mid.png),
[`m64-assembly-end.png`](screenshots/m64-assembly-end.png).

**`rand::StdRng` had to go, and that is the milestone's real content.** The
baked demo seeded `StdRng::seed_from_u64` per placement. `StdRng` is ChaCha12
— reimplementable in TypeScript only at real cost, and, decisively, **`rand`
makes no promise its output is stable across versions.** A piece whose
choreography changes when a transitive dependency bumps a minor version is
not reproducible, and "the 2027 edition looked different after `cargo update`"
is not a sentence anyone wants to write. The generator is now **splitmix64**:
twelve lines, a fixed specification, bit-identical in any language with a u64.
The old code's comment already said "a splitmix-style constant" — it used
that constant to *seed ChaCha*. **This changes the existing demo's scatter**,
which is cosmetic (the layout was always arbitrary-but-deterministic) and buys
the property everything else here rests on.

**The two languages are pinned to a fixture, not to each other.**
`docs/fugen/fixtures/assembly-scatter.json` is generated by the Rust module
and checked by *both* — the Rust crate's own test and the browser harness.
Comparing the two implementations directly would let them drift together;
comparing each to a committed artefact means whichever one moves is the one
that fails. **Result: worst |TypeScript − Rust| = 0.000e+0 across 96
components and two editions.** Not "within tolerance" — the same bits.

**AC1 passes with four orders of margin.** The baked demo has **no stagger**
(one eased lerp for everything), so that is the configuration the runtime has
to reproduce, and the comparison runs in millimetres through the LDU→mm
**mirror** — the one conversion in this project that has silently inverted a
whole library before. Five frames × nine parts: worst difference
**6.1 × 10⁻⁶ mm** against a 0.01 mm allowance, which is float32 quantisation
and nothing else. At `t01 = 1` every instance is 3.1 × 10⁻⁶ mm from where the
bundle itself placed it, and the scatter starts **+168.0 mm** — positive, so
the mirror is being applied rather than merely believed in.

**AC2, measured on three real scenes** (median of 9 full passes, no frame rate
asserted — there is no GPU here and M92 owns that number):

| scene | instances | compose | matrix |
|---|---|---|---|
| monolith | 9 | 0.00 ms | 0.00 ms |
| car | 61 | 0.10 ms | 0.10 ms |
| synthetic Atlas site | 5 000 | **1.90 ms** | **0.90 ms** |

**AC3 passes on real `0 STEP` data.** The official `car.ldr` has **8 distinct
build steps** across 61 placements. Staggering by real build order against
staggering by array index puts **49 of 61 instances in a different place** at
the midpoint, up to **109.3 mm** apart. Build steps are also not dense — a
scene may go 0, 0, 1, 3 — so the stagger ranks them rather than using the raw
number, or a scene with sparse steps would hand over across a shorter span
than a dense one for no reason a viewer could perceive.

**`mesh.json` gained an optional `instanceBuildSteps`**, written only when a
scene actually has step markers *and* they are not all identical. Additive, so
no version bump — the opposite of M56, which added required fields and needed
one.

**The generator is declared, not expanded, and that is a deviation.** The spec
asked for the scatter to become a transform track at `show-build` time. That
works for nine bricks and not for an Atlas site: a transform track carries one
value per keyframe shared across its whole target, so per-instance scatter
needs one track *per instance* — five thousand tracks to express two constants
and a seed. A1-S04's `seed` cue already declares the generator and its
constants (authored in M60); this evaluates it. The resolved document stays
the size of a screenplay.

**And a real latent bug, which the pictures found and the numbers did not.**
Every measurement above passed on the first run — and all three screenshots
showed a car that had never moved. Since M59, `InstanceWriter.flush()` marks
groups dirty and uploads *nothing*: `group.matrices` is authoritative and the
**LOD selector** is the only thing that copies it into the meshes the GPU
reads. So on a bundle with no LOD levels, every transform the choreography
writes lands in the authoritative array and never reaches the screen — a
perfectly correct still frame, with no error anywhere. It had not bitten
because every bundle the CLI writes today carries levels, and "it works
because of a property of the writer at the other end of the pipeline" is not a
thing to leave standing. `flush()` now uploads level 0 itself unless a
`LodSelector` has claimed it (`InstanceWriter.lodManaged`). The harness also
has to call `lod.update()` after the choreography, which is what M66's loop
will do.

---

### M65 — effects: dissolve, materialise, and the point↔mesh crossfade

**Why.** Act I begins with a single point becoming a solid brick; Act IV
ends with solid geometry becoming a point swarm. The engine must be able to
cross the boundary between its own two render modes *on screen*. This is
also the moment the existing point pipeline earns its keep inside the mesh
show rather than being superseded by it.

**Files.** `viewer/src/show/effects.ts`, shader chunks in
`viewer/src/mesh/materials.ts`.

**Requirements.**

- **Dissolve** — a per-instance scalar `0..1` fed to a shader that discards
  fragments below a 3D-noise threshold, with a thin emissive rim at the
  threshold edge (colour from the palette). Instanced attribute, no
  per-instance material.
- **Materialise** — dissolve run backwards, plus an emissive flash on
  completion.
- **Point↔mesh crossfade** — for a target set of instances, sample their
  own mesh surface *on the GPU side is unnecessary*: reuse `spex-ldraw`'s
  real `sample_surface` at build time to emit a companion point buffer per
  part (`buffers/p<N>.pts.bin`, positions + baked colour, the exact same
  15-bytes-per-point layout as `octree/*.bin` so the format is not a new
  one). At runtime, a `pointCloud` track's value `0..1` fades mesh opacity
  down while fading a `THREE.Points` cloud up, with the points lerping
  outward along their own normals so the object visibly "comes apart" —
  the *inkpour* of Act IV.
- The point cloud rendering path here is the *existing* `PointsMaterial`
  with `vertexColors`, not a new renderer.

**Acceptance criteria.**

1. A 3-second dissolve of the monolith captured at 30 fps shows a smooth,
   noise-driven disappearance with a visible rim, no popping.
2. The crossfade at value 0.5 shows both representations superimposed and
   spatially coincident (their bounding boxes agree within 1%).
3. Cost of the dissolve shader ≤ 15% frame-time increase on the
   50 000-instance scene.

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture). (6 runs at the end of the phase.)

**Status: ✅ done.**
`viewer/src/show/dissolve.ts`, the injected chunks in
`viewer/src/mesh/materials.ts`, the eroding outline in `edges.ts`, the
eroding shadow in `instanced.ts`/`lod.ts`, and
`scripts/viewer-shot/dissolve.mjs`. Screenshots:
[`m65-dissolve-000.png`](screenshots/m65-dissolve-000.png),
[`m65-dissolve-034.png`](screenshots/m65-dissolve-034.png),
[`m65-dissolve-062.png`](screenshots/m65-dissolve-062.png).

**The milestone is split at its own seam.** Its three requirements are three
things, and AC1/AC3 belong to the dissolve while AC2 belongs to the crossfade.
The crossfade needs a companion point buffer per part, a `THREE.Points` path
that scales past one brick, and a Rust-side sampler — comfortably another
milestone's work. Splitting it is better than half-landing both.

**A design decision for that half, recorded now so it is not re-litigated:**
the spec says to bake colour into the point buffer and reuse the octree's
15-bytes-per-point layout. **That cannot be right.** A part's geometry is
deliberately colour-neutral (M51 leaves LDraw code 16 unresolved) precisely so
one mesh serves every colour it is placed in; baking colour into its point
cloud would mean one point cloud per (part, colour), which is the instancing
argument thrown away at the transition. The companion buffer must carry
**positions and normals** — normals because the spec's own "points lerping
outward along their own normals" needs them, and the octree layout has no room
for them — with colour coming from the instance's material at runtime.

**AC1 passes: 17 309 → 0 lit pixels**, largest single-frame change 6.67 % of
the object, and the rim measured rather than asserted — 5 231
brighter-than-background pixels when solid, peaking at **9 181** at dissolve
0.43. The object gets *brighter* as it goes, which is the rim doing its job.

**AC3 measured: −18 % render time at dissolve 0.5** versus solid, on the
monolith. The sign is meaningful and the magnitude is not: this is a software
rasteriser, where `discard` is a branch that saves shading work, and on a GPU
it is close to free but can defeat early-Z. The criterion asked for "≤ 15 %
increase" and what happened was a decrease, which says the shader is not
adding measurable cost here and says nothing at all about a GPU.

**Three defects, each found by a picture that the numbers had already passed.**
Every one is the same shape: something that renders the object *other than the
lit surface* did not know about dissolving.

- **The outline survived its own object.** At dissolve 1.0 the first run still
  had 13 637 lit pixels — a perfect wireframe of the monolith hanging in
  space, which is a striking image and entirely the wrong one. M57's edge
  material is its own `ShaderMaterial`; it now reads the same per-instance
  value through a `DataTexture` (a texture and not an attribute, because a
  quad is per *edge* and a 1×4 brick has 360 of them).
- **The shadow survived too.** With the edges fixed, 5 591 pixels remained and
  `meanDelta` went **negative** — the object was making the frame darker than
  the empty scene, which is a shadow of nothing and cannot be anything else.
  three.js renders shadows with its own depth material, so
  `customDepthMaterial` now carries the same erosion.
- **And the outline eroded at the wrong rate.** With both fixed, the halfway
  frame showed bricks turned into wire cages: the edge pass used a uniform
  per-edge hash, and smoothed two-octave value noise is concentrated around
  0.5, so at threshold 0.56 most of the surface was gone and only about half
  the edges were. Same field now, sampled at the edge midpoint — the noise
  GLSL is exported from one module and used by all three shaders, because two
  copies of a hash function is the most reliable way for "the same fragments"
  to stop being true.

**Part 2 — the crossfade.** `crates/spex-mesh/src/points.rs` samples every
part's welded output surface into `buffers/p<N>.pts.bin`;
`viewer/src/show/points.ts` draws it as **one instanced `POINTS` call per
group**, sharing M57's own instance-matrix texture rather than uploading a
second copy. Screenshots:
[`m65-crossfade-mid.png`](screenshots/m65-crossfade-mid.png),
[`m65-crossfade-end.png`](screenshots/m65-crossfade-end.png).

**Two decisions against the spec.** The buffer is **colour-neutral and carries
normals** — 24 bytes a point, not the octree's 15-byte layout. A part's
geometry is deliberately colourless (M51 leaves LDraw code 16 unresolved) so
one mesh serves every colour it is placed in; baking colour into its point
cloud would mean one cloud per (part, colour), which is the instancing
argument thrown away at the one moment the scene is most expensive. And the
octree layout has nowhere to put a normal, which the spec's own "points
lerping outward along their own normals" needs. Sampling is **deterministic
without a PRNG**: golden-ratio stratification over the cumulative-area table,
2-D Halton inside the triangle — no seed, none of the cross-version stability
question M64 had to solve, and a visibly more even cloud than uniform random.
For a swarm standing in for a solid object, clumping reads as holes.

**The crossfade's two halves do different things**, and this is the part worth
protecting. 0 → 0.5 the *representation* changes: the mesh erodes through the
dissolve while the cloud fades up, and the points sit **exactly on the surface
they were sampled from**. 0.5 → 1 the *object* comes apart, and only then do
the points drift outward. Spreading from 0 would look identical in a still and
be wrong in motion — nothing would ever be *both* representations of one
shape, and the moment the piece is about (a statistical cloud and a countable
thing being the same object) would never happen.

**AC2, measured, and the criterion restated because it cannot be met as
written.** "Bounding boxes agree within 1 %" is not a property a filled
silhouette and a *finite sample of a surface* can have. The outermost of ~1 200
samples lands a few pixels inside the true silhouette, and how far inside
depends on sampling density rather than on alignment; drawn at their real size
the points instead stick out by their own radius. So the two bracket the truth
from either side and neither is the answer. What the run reports:

| | vs the mesh |
|---|---|
| cloud box, real point size | −0.52 % wide, −3.03 % tall |
| cloud box, 1 px points | −1.04 % wide, −3.90 % tall |
| **centroid of lit pixels** | **0.37 % , 2.18 %** of the object |
| RMS spread | 9.18 % , 1.92 % |

The centroid is the unbiased statistic and it agrees to within half a percent
horizontally. What the harness *asserts* is only what a real misalignment
would look like — a centroid or an extent out by more than a tenth of the
object. Everything finer is the finite sample, and is reported rather than
judged. Tuning the point size or the density until a number came out under 1 %
would have been fitting the instrument to the answer.

**Four measurement defects, and the first one was worth all the rest.** The
initial run had the cloud **36 % narrower and 30 % shorter** than the mesh and
offset by a third of the object's width — a systematic mismatch that looked
like a coordinate bug. It was **bloom**: a lit brick's specular blooms several
pixels past its own silhouette, and a point cloud at the same opacity is far
dimmer per pixel and blooms far less, so the measurement was comparing two
glows and not two shapes. With bloom off for the box pass the error fell to
4 %. The other three: the mesh's box included **the shadow it casts** and the
cloud casts none; `renderer.shadowMap.enabled = false` **changed the numbers
by exactly zero** without a material recompile — the same silent no-op M58's
`--no-shadows` already produced, so the *ground* is hidden instead, because
there is no shadow if there is nothing for it to fall on; and `gl_PointSize`
assumed **metres** in a millimetre scene, so every point clamped at the 14 px
ceiling and a 1×1 brick rendered as one solid red blob.

**Two constants came out of looking at the pictures rather than the numbers.**
`POINT_RADIUS_MM` started at 0.35 and is 0.08: at 0.35 a brick's 1 261 points
overlap into a solid mass at close range, which is a picture of a red brick
and not of a swarm. And the spread is now **relative** — 1.6 × the part's own
radius — because a flat 26 mm is a gentle loosening on a 200 mm monolith and
throws an 8 mm brick clean off the frame.

**Materialise is the same ramp backwards**, with one addition: it ends on an
emissive flash decaying over 0.45 s — just under a beat at 84 bpm. Without it
a materialise finishes on a completely ordinary frame, and the *arrival* is
the event.

---

### M66 — `spex show` / `spex show-export`, URL parameters, and the HUD

**Files.** `crates/spex-cli/src/show.rs`, `crates/spex-server/src/lib.rs`
(a `show` mode alongside single-tileset and gallery modes),
`viewer/src/show/hud.ts`, `viewer/index.html`.

**URL parameters** (all optional, all documented in `docs/`):

| Param | Meaning |
|---|---|
| `?t=<sec>` | seek to time |
| `?duration=240\|600\|3600\|endless` | pick a resolved cut (if the bundle contains several) |
| `?seed=<n>` | edition seed |
| `?quality=low\|medium\|high` | override the auto tier |
| `?mute=1` | start muted (see M71's autoplay policy) |
| `?free=1` | free camera, timeline still runs |
| `?director=1` | show the director HUD: shot id, note, t, fps, draw calls, instance count, current fugue voice entries |
| `?loop=0` | play once and hold the final frame |

**HUD requirements.** A title card system (movement numeral + title, fades
in/out per movement), the chronicle cards used by the Atlas (M80), and the
credits crawl (M84). Text rendered as DOM overlay, not WebGL — the existing
`#labels` pattern, which already handles projection and is far cheaper to
get typographically right.

**Acceptance criteria.**

1. `spex show demos/matrix/240` runs the full canonical cut end to end with
   no console errors, correct total duration (measure it), and a clean loop.
2. `spex show-export` output runs identically from `file://` and from a
   subpath-hosted static server (the same relative-path discipline
   `export_static.rs` already enforces).
3. Every URL parameter above verified individually in the headless session.

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture). (6 runs at the end of the phase.)
---
