# Phase 6 - Rust in the browser via WebAssembly (M86-M90)

*One implementation of the resolver, the evaluator, the grid and the counterpoint. Back in scope since the premiere moved to the 2027 centenary — see [`plan.md`](plan.md) §1.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**
Read [`README.md`](README.md) first: it carries the working rules, the scope decision and the milestone index.
Review record and the reasoning behind every rev-2 change: [`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md).

---


**Why this is not a gimmick.** By the end of Phase 5 the project has, on
paper, two implementations of several load-bearing algorithms: the duration
resolver (Rust, `spex-show`) and the timeline evaluator (TypeScript,
`viewer/src/show/timeline.ts`); the counterpoint generator (Rust) and its
runtime realisation (TypeScript); the brick grid and easing curves in both.
Two implementations of one algorithm is two implementations that drift. The
first time a keyframe evaluates one way in `show-build` and another way in
the viewer, a whole day disappears into finding out why.

Compiling the Rust to WebAssembly and calling *it* from the viewer collapses
that: **one implementation, one set of unit tests, two targets.** That is
the architectural argument, and it is a good one on its own. But there is a
second, sharper one:

**Zero-copy instance transforms.** `InstancedMesh.instanceMatrix.array` is a
`Float32Array`. A `Float32Array` view over WebAssembly linear memory is also
a `Float32Array`. If the wasm timeline evaluator writes its 16 floats per
instance *directly into the buffer three.js uploads*, then evaluating
250 000 instance transforms costs one wasm call per frame and **zero**
JavaScript object allocations, zero marshalling, zero copies. That is the
difference between the 60-minute Atlas cut running and not running.

**Sequencing note — deliberate.** This phase comes *after* Phases 2–5, not
before. Porting a settled algorithm to wasm is a mechanical, verifiable
operation with a reference implementation to diff against. Designing the
algorithm in wasm first would mean debugging semantics and a toolchain at
the same time. The Rust-native pieces (`spex-show`'s resolver, `spex-fugue`'s
generator) are consumed as *JSON output* during Phases 2–5 precisely so that
this phase is a swap, not a rewrite.

### M86 — `spex-wasm`: the boundary and the toolchain

**Files.** `crates/spex-wasm/` (new, `crate-type = ["cdylib", "rlib"]`),
`viewer/src/wasm/` (generated bindings, gitignored), `viewer/vite.config.ts`,
`.github/workflows/` (build step), `docs/agents/wasm.md` (a new playbook
page — how to build, how to debug, what not to do).

**Dependencies.** `wasm-bindgen`, `serde-wasm-bindgen`, `js-sys`,
`console_error_panic_hook` (debug builds only). Toolchain: `wasm-pack build
--target web --release`, output into `viewer/src/wasm/`. Vite consumes it
with the standard `init()` pattern; **no new Vite plugin** if avoidable —
`wasm-pack --target web` emits an ES module that Vite bundles natively.

**The critical property to preserve:** `spex-server` embeds `viewer/dist`
via `rust-embed` at Rust compile time, so `spex serve` stays a single
self-contained binary. The `.wasm` blob is an ordinary asset inside
`viewer/dist`, so this property survives untouched — the binary simply now
contains a copy of some of its own logic compiled for a second target.
Note the pleasing consequence in the milestone commit message: the same
`resolve()` function ships twice in one executable, once as x86-64 and once
as wasm32.

**The build order gotcha, now three deep.** `docs/agents/verification.md`'s
warning gains a rung:

```sh
wasm-pack build crates/spex-wasm --target web --release --out-dir ../../viewer/src/wasm
cd viewer && npm run build          # bundles the wasm into viewer/dist
cd ..     && cargo build --release  # re-embeds viewer/dist into the binary
```

Skipping step 1 is the new "I changed it and nothing happened". Add it to
`docs/agents/verification.md` in this milestone, not later.

**Exports (M86 ships only the first two — prove the pipeline before widening it).**

```rust
#[wasm_bindgen]
pub fn version() -> String;

/// Resolves a show document at a target duration — the SAME
/// `spex_show::resolve` the CLI calls. Input/output are JSON strings;
/// this is a cold path (once per load), so serialisation cost is irrelevant
/// and legibility wins.
#[wasm_bindgen]
pub fn resolve_show(show_json: &str, target_sec: f64, seed: u64, endless: bool) -> Result<String, JsValue>;
```

**Acceptance criteria.**

1. `wasm-pack build` succeeds in CI on a clean checkout.
2. `resolve_show` called from the browser returns output byte-identical to
   `spex show-build`'s for the same inputs — asserted by fetching both in
   the headless session and comparing hashes.
3. Released `.wasm` is < 400 KB gzipped for this export set; record the
   real figure.
4. Every existing demo still loads with the wasm module **absent** — the
   point-cloud and graph pipelines must not gain a wasm dependency. Verified
   by loading a graph demo with the wasm fetch blocked.

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture). (6 runs at the end of the phase.)
---

### M87 — the timeline evaluator in wasm, zero-copy

**Why.** The hot path, and the whole performance argument.

**Files.** `crates/spex-wasm/src/timeline.rs`,
`crates/spex-show/src/eval.rs` (the evaluator itself moves *into* the Rust
crate — this is where the TS implementation is retired),
`viewer/src/show/timeline.ts` (becomes a thin wrapper).

**The interface.**

```rust
#[wasm_bindgen]
pub struct WasmTimeline { /* owns the ResolvedShow + all scratch buffers */ }

#[wasm_bindgen]
impl WasmTimeline {
    #[wasm_bindgen(constructor)]
    pub fn new(resolved_json: &str) -> Result<WasmTimeline, JsValue>;

    /// Total instances across all groups; the JS side allocates its
    /// InstancedMesh with exactly this capacity.
    pub fn instance_count(&self) -> usize;

    /// Byte offset, inside wasm linear memory, of the packed
    /// column-major 4x4 matrix array (16 f32 per instance). JS builds
    /// `new Float32Array(memory.buffer, ptr, count * 16)` ONCE and hands
    /// it to three.js as the InstancedMesh's instanceMatrix array.
    pub fn matrix_ptr(&self) -> *const f32;

    /// Same, for the per-instance dissolve/visibility/point-fade scalars.
    pub fn scalar_ptr(&self) -> *const f32;

    /// Evaluates every active track at show time `t` and writes straight
    /// into those buffers. Returns a bitmask of which instance groups
    /// changed, so JS only flags the changed `instanceMatrix.needsUpdate`.
    pub fn evaluate(&mut self, t: f64) -> u32;

    /// Camera + post state for this frame, as a small packed array
    /// (position 3, target 3, fov 1, bloom 3, exposure 1, ...). Small
    /// enough that a copy is free.
    pub fn frame_state(&self) -> Box<[f32]>;

    /// Cues crossed since the previous call, as JSON (cold path).
    pub fn take_cues(&mut self) -> String;
}
```

**The one hazard, and it is a real one.** WebAssembly linear memory can be
*reallocated* when it grows, which invalidates every existing
`Float32Array` view. Mitigations, all three required:

1. Allocate every per-instance buffer **once**, up front, in the
   constructor, sized from `instance_count()`. Never grow after
   initialisation.
2. Expose a `memory_generation(): u32` counter that increments if the
   module ever does grow; the JS side checks it each frame (one integer
   compare) and rebuilds its views if it changed.
3. Document this at the top of `timeline.ts` in full, with the reason. This
   is exactly the class of bug that costs a day and looks like a GPU
   problem.

**Acceptance criteria.**

1. The Rust evaluator reproduces the TS evaluator's output exactly for the
   full canonical cut: sample 500 times, compare every instance matrix,
   max component difference < 1e-5. **Then, and only then, delete the TS
   evaluator** — and say so in the commit message.
2. Per-frame evaluation of 250 000 instances costs < 2 ms (record the real
   number; the TS baseline from M55 is the comparison).
3. Zero JS allocations per frame, verified by a heap-allocation profile.
4. Growing-memory hazard covered by a test that deliberately triggers a
   grow and asserts the generation counter catches it.

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture). (6 runs at the end of the phase.)
---

### M88 — `spex-build`, `spex-ldraw` and `spex-flag` in the browser

**Why.** Once the Rust is already in the page, the tool can *build in the
browser*: type a recipe, see the bricks. The Atlas's provisional recipes
(M77) become reviewable live instead of via a CLI round-trip, and a future
web-based recipe editor becomes a small feature rather than a project.

**Exports.**

```rust
/// Runs a spex-build recipe and returns placements as a packed binary
/// buffer (part index u32, colour u32, translation 3xf32, matrix 9xf32),
/// ready to be turned into instances without JSON parsing.
#[wasm_bindgen]
pub fn build_recipe(recipe_json: &str) -> Result<Box<[u8]>, JsValue>;

/// Validates grid legality; returns the same Illegality list the CLI prints.
#[wasm_bindgen]
pub fn validate_recipe(recipe_json: &str) -> String;

/// Rasterises + quantises a flag spec against the real LDraw colour table
/// (passed in once as JSON, cached inside the module).
#[wasm_bindgen]
pub fn build_flag(spec_json: &str, width_studs: u32) -> Result<Box<[u8]>, JsValue>;

/// Parses a real .ldr model's placements — so a user can drop a local
/// .ldr onto the page and see it rendered, with no server involved.
#[wasm_bindgen]
pub fn parse_scene_text(ldr_text: &str) -> Result<String, JsValue>;
```

**Explicitly NOT exported:** anything that fetches. `spex-ldraw`'s network
layer (`ureq`, `zip`) does not compile to wasm and must not be dragged in —
gate it behind a `#[cfg(not(target_arch = "wasm32"))]` feature so
`spex-ldraw` builds for wasm32 with *parsing only*. Part geometry still
arrives as a pre-built mesh bundle from the server. Say this in the crate's
own doc comment; it is the boundary that keeps the wasm module small.

**Acceptance criteria.**

1. `spex-ldraw` compiles for `wasm32-unknown-unknown` with the network
   feature off, and its parsing unit tests pass under `wasm-pack test
   --headless --chrome`.
2. A browser page builds `recipes/heritage/stonehenge.json` live and
   renders it, producing geometry identical to the CLI's `.ldr` output
   (compare placement lists).
3. Drag-and-drop of a local `.ldr` renders it with no network request
   beyond the part bundle.
4. Total `.wasm` still < 900 KB gzipped with all of Phase 6's exports.

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture). (6 runs at the end of the phase.)
---

### M89 — the fugue in wasm, and DSP in an AudioWorklet

**Why.** The counterpoint generator is already Rust. Running it live means
the endless cut can generate genuinely new episodes per cycle instead of
replaying a pre-generated score — the piece composes as it plays.

**Two separate pieces, in this order.**

**M89a — score generation in wasm (safe, do this first).**

```rust
#[wasm_bindgen]
pub fn generate_fugue(spec_json: &str, seed: u64) -> Result<String, JsValue>;
/// Generates only the next `bars` bars from a running state — for endless
/// mode, so the module never has to hold an unbounded score in memory.
#[wasm_bindgen]
pub struct WasmFugue { /* … */ }
#[wasm_bindgen]
impl WasmFugue {
    #[wasm_bindgen(constructor)] pub fn new(spec_json: &str, seed: u64) -> Result<WasmFugue, JsValue>;
    pub fn generate_bars(&mut self, bars: u32) -> String;   // JSON note list
}
```

**M89b — synthesis in an `AudioWorkletProcessor` (harder, higher payoff).**
The voice synthesis of M69 is a `process()` callback filling 128-sample
blocks. That is exactly what wasm is for, and it runs on the audio thread
where a garbage-collection pause is audible. Port the oscillator bank, ADSR
and saturation into `crates/spex-wasm/src/dsp.rs`, instantiate the module
inside the worklet (`WebAssembly.instantiate` from a transferred
`ArrayBuffer` — worklets cannot `fetch`), and keep the *graph* (convolver,
compressor, routing) in native WebAudio nodes where it is already better.

**This is the milestone most likely to be cut for time, and cutting it is
fine.** M69's WebAudio-node synthesis is a complete, shippable answer.
M89b is an upgrade: lower jitter, no GC in the audio path, and one more
piece of the piece sharing one implementation. Decide at the phase gate
(§10) whether the schedule supports it, and record the decision.

**Acceptance criteria (M89a).** Generated score is byte-identical to the
CLI's for the same seed; endless mode generates 60 minutes of continuous
material without memory growth (measure).

**Acceptance criteria (M89b, if taken).** No audible glitches over a
10-minute run; audio-thread CPU below the M69 baseline; a deliberate main
-thread stall of 200 ms produces *no* audio dropout — the demonstration that
the port was worth doing.

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture), plus a human listen. (6 runs at the end of the phase.)
---

### M90 — the wasm phase gate: measure, document, decide

**Deliverables.**

1. A real before/after performance table in `TODOs.md`: TS vs. wasm, for
   timeline evaluation at 1k / 50k / 250k instances, and for score
   generation. Real measured numbers on named hardware, not estimates.
2. `docs/agents/wasm.md` finished: build order, debugging (source maps,
   `console_error_panic_hook`, why a wasm panic looks like `unreachable`),
   the memory-growth hazard, and the "no fetching in wasm" boundary.
3. `CLAUDE.md` and `AGENTS.md` updated with the new crate and the new build
   step.
4. **The honest verdict.** If wasm did not measurably help somewhere, say
   so in `TODOs.md` and keep only the parts that did. This project has a
   standing practice of writing down evaluated-and-not-adopted leads (the
   AA-lib entry is the model). Architecture is a means; the piece is the end.

---
