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

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture). (6 runs at the end of the phase.)
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
