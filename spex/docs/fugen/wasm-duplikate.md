# Phase 6's inventory — what is actually implemented twice

*Prepared 11 August 2026, ahead of M86. Not a plan; a count.*

Read [`phase6-wasm.md`](phase6-wasm.md) for the phase itself. Its opening
argument is that by the end of Phase 5 the project has two implementations of
several load-bearing algorithms and that "two implementations of one algorithm
is two implementations that drift". That is now checkable rather than
predicted, so here is the check. **The point of this page is that M86 should
start from a list and not from a survey.**

## Implemented twice today, function for function

`crates/spex-show/src/choreography.rs` (224 lines) against
`viewer/src/show/choreography.ts` (304 lines):

| Rust | TypeScript |
|---|---|
| `splitmix64` | `splitmix64` over a hand-rolled `U64` |
| `next_f64` | `nextFloat` |
| `placement_seed` | `placementSeed` |
| `start_offset_ldu` | `startOffsetLdu` (+ `startOffsetMm`) |
| `cubic_in_out` | `cubicInOut` (in `easing.ts`, ported not re-derived) |
| `staggered_progress` | `staggeredProgress` |
| `FLOAT_HEIGHT_LDU` 420, `SCATTER_RADIUS_LDU` 260 | the same two constants |

`crates/spex-show/src/tokens.rs` against `viewer/src/show/tokens.ts` joined
them on 11 August — `position_at` / `positionAt`, the reflected walk and the
half-sine arc — deliberately, so that the list is complete rather than short.

Both pairs are pinned to a fixture (`assembly-scatter.json`, `token-flow.json`)
rather than to each other, which is the right arrangement for two
implementations and is not a substitute for having one.

**The sharpest line in the whole case for wasm is in the TypeScript file's own
header:**

> `splitmix64` over a pair of 32-bit halves, because JavaScript has no u64.
> `BigInt` would be the obvious way and is roughly an order of magnitude
> slower.

`add64`, `mul64` and `xorShiftRight` — about fifty lines — exist for no reason
except the target. In wasm they are `u64` and they are free.

## Implemented twice by construction

- **The keyframe evaluator.** `viewer/src/show/timeline.ts` has ten sampling
  entry points; `crates/spex-show/` has **no** evaluator at all
  (`phase6-wasm.md`'s M87 introduces `eval.rs`). So this is not duplication
  yet — it is a single implementation on the wrong side of the boundary, and
  every new track kind is written into it by hand.
- **Every track kind, twice.** Adding `color` on 11 August meant
  `Track::Color` + `ResolvedTrack::Color` + validation in Rust, and
  `ResolvedTrack` + `sampleColor` + a sink in TypeScript — including writing
  the same linear-RGB lerp in both languages an hour apart. That is the drift
  surface, measured on a real change.
- **The easing library.** `easing.ts` (196 lines) is named in its own header as
  a *port* of `brick.rs::ease_in_out_cubic`, pinned to 0.01 mm.

## What the phase claims, against what is measurable now

| Claim in `phase6-wasm.md` | Evidence today |
|---|---|
| "two implementations that drift" | **True and specific**: six functions and two constants, plus a hand-rolled u64. |
| "zero-copy instance transforms … the difference between the 60-minute Atlas running and not" | **Unmeasured.** The largest scene built is 3 600 instances (`feld`), then 1 921 (`gitter`). 250 000 is the Atlas, and the Atlas depends on M73/M74/M75. There is no frame-time measurement showing the evaluator is the bottleneck — on this project's software rasteriser the frame is 160–240 ms and the rasteriser owns essentially all of it. |
| "porting a settled algorithm is mechanical" | **The evaluator is not settled.** A track kind was added to it today. |

## Apple Silicon against x86-64 — and what wasm actually abstracts

Asked while this page was being written, and worth answering precisely because
the intuition ("wasm sandboxes the CPU away") is the wrong shape.

**For the audience: irrelevant, and already is.** A screening is a URL. The
viewer is TypeScript in a browser, and `spex show-export` writes static files.
Nobody watching the piece is running an architecture.

**For building: relevant in exactly one place, and it is not the one people
expect.** IEEE-754 f64 add/mul/div are bit-identical on both architectures, and
Rust does not contract to FMA the way C compilers do, so ordinary arithmetic is
safe. The exception is the transcendentals: `start_offset_ldu` calls
`angle.cos()` and `angle.sin()`, and Rust's `f64::sin` is the *system* libm —
glibc on x86-64 Linux, Apple's on arm64 macOS. Those agree to within an ulp and
are not guaranteed to agree exactly.

Two consequences, both measured rather than assumed:

- The shared fixture's tolerance is `1e-9` (`choreography.rs`, the
  `the_shared_fixture_still_describes_this_generator` test). A last-ulp libm
  difference on values of order 100 is about `1e-14`, so the fixture **absorbs**
  cross-platform variation — it will not false-alarm on a Mac, and it also
  would not catch a real divergence smaller than `1e-9`.
- The **TypeScript** side is, ironically, the more portable one: V8 ships its
  own fdlibm port, so `Math.sin` is the same code on every machine V8 runs on.
  The native Rust is the half that can vary by architecture.

**What wasm does about it is not emulation.** WebAssembly *specifies* its
arithmetic: IEEE-754, no FMA contraction, no x87 extended precision, defined
rounding. And `sin`/`cos` are not wasm instructions at all — they are compiled
*into* the module from Rust's own libm, so one `.wasm` file computes the same
bits on an M-series Mac, on x86-64 and on whatever the gallery's machine turns
out to be.

For a work whose thesis is that **an edition is reproducible from its
signature**, that is arguably a better argument for Phase 6 than the
performance one, and it is the only one of the three that is stronger in wasm
than in native Rust rather than merely equal.

## "An alternative runtime, and keep the Rust" — yes, and the Rust was never at risk

Worth saying plainly, because the phrasing of Phase 6 invites the opposite
reading: **wasm is not a second implementation of the Rust. It is the same
Rust, compiled for a second target.** `phase6-wasm.md` says so in its own
milestone note — the same `resolve()` ships twice in one executable, once as
x86-64 and once as wasm32. Nothing about the CLI, `spex show-build`,
`show-export` or the test suite changes; they keep calling the native build.

The only thing Phase 6 proposes to delete is the **TypeScript** evaluator
(M87's AC1: reproduce the TS output for the full canonical cut, 500 samples,
max component difference < 1e-5, "then, and only then, delete the TS
evaluator"). So the real question is not whether to keep the Rust — it is what
happens to the TS half, and there are two honest answers:

1. **Delete it**, as the spec plans. One implementation, one set of tests, no
   drift. The cost is that a failed `.wasm` fetch has no fallback for the show
   path (the point-cloud and graph pipelines never gain the dependency at all —
   M86's AC4).
2. **Keep it as a fallback**, but only if the equivalence check runs *on every
   commit* rather than once at M87. A fallback that is never exercised is a
   fallback that does not work, and this project's own doctrine already says
   so: a probe that cannot see cannot testify. Keeping two paths without a
   standing comparison is the drift this phase exists to end, with extra steps.

Recommended, if the fallback is wanted: option 2 **with the comparison in CI**,
and with the fallback path exercised by a probe that blocks the `.wasm` fetch
the way M86's AC4 already blocks it for the demos. Then "alternative runtime"
is a true statement about the system rather than a hope about a code path.

## The recommendation, in one line

**Not yet as work; already as a constraint.** The two things worth doing before
M86 both cost nothing:

1. **Do not let the duplicate list grow silently.** Anything added to
   `choreography.ts` gets a twin in `choreography.rs` and a fixture entry, or
   it is written on the Rust side from the start. **Done for `tokens.ts` the
   same day this page was written**: `crates/spex-show/src/tokens.rs`, pinned
   to `fixtures/token-flow.json`, which was generated from the TypeScript
   itself rather than re-derived. The two agree to 1e-9 across a fourteen-hop
   reflected walk at five moments and five instances. A generator with no Rust
   side is one M87 would have to *port* under deadline rather than delete, and
   porting a walk is exactly where an off-by-one in the hop index is
   invisible.
2. **Get one frame-time number before believing the performance argument.**
   The zero-copy case deserves a measurement on real hardware with a real
   scene, not a software rasteriser. Until then the architectural argument —
   one implementation, one set of tests — is the only one that is carrying its
   own weight, and it is enough on its own.

The trigger for actually starting M86 is whichever comes first: **the Atlas
lands** (and the instance count leaves five figures), or **a defect is traced
to the two implementations disagreeing**. The first is scheduled; the second is
the one to watch for, and the reason this page exists is so that it is
recognised as that rather than debugged as a renderer problem.

---

*Iunctura Archiv · IA-2026-002*
