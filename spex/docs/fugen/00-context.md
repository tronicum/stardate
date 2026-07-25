# Context and architecture

*Where the project stands after M1-M50, what is missing, and the shape of everything Phases 1-7 add.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**
Read [`README.md`](README.md) first: it carries the working rules, the scope decision and the milestone index.
Review record and the reasoning behind every rev-2 change: [`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md).

---

## Where we are, and what is actually missing

### 1.1 What M1–M50 plus `brick-cinematic` already give us

| Capability | Where | State |
|---|---|---|
| Real LDraw fetch/cache/zip-mirror | `crates/spex-ldraw/src/cache.rs` | done |
| Real recursive part resolution (type 1/3/4) | `geometry.rs::resolve_part` | done |
| Real LDConfig colour table | `colors.rs::load_colors` | done |
| Real `.ldr` scene/placement parsing + `0 STEP` | `scene.rs::parse_scene` | done |
| Area-weighted surface sampling + baked shading | `sampling.rs` | done |
| Octree tileset + LOD streaming viewer | `spex-tiler`, `viewer/src/lod.ts` | done |
| Multi-frame animation via baked frames | `frame_sequence.rs`, `sequence.json` | done |
| Hero-spin + assembly cinematic | `brick.rs::build_spin_frames` / `build_assembly_frames` | done |
| One hand-authored real scene | `ldraw-scenes/monolith.ldr` | done |

### 1.2 What is missing for the work we actually want

Six gaps, and every milestone in this document closes part of one of them.

**Gap A — the picture is soft.** Everything renders as sampled points.
A 1×1 brick's edge is a statistical cloud, not an edge. `BRICKs.md` already
names the fix and files it as "a real, deliberately bigger alternative for
later": render LDraw's real triangles directly, with LDraw's real *edge*
lines (type 2) and *conditional* edge lines (type 5) — which are precisely
what makes a catalogue-quality brick render read as a brick. That is what
"vektorgenau" means here, and it is Phase 1.

**Gap B — animation is baked.** `spex frame-sequence` pre-tiles N frames.
A 4-minute piece at 30fps is 7 200 tilesets. That is not a demoscene
engine, it is a video file with extra steps. The work needs a *runtime*
timeline: geometry uploaded once, transforms evaluated per frame from a
declarative score. Phase 2.

**Gap C — there is no sound.** The work is called *Fuge* in both senses and
has none of the second one. Phase 3.

**Gap D — there is one scene.** `monolith.ldr`, nine parts, hand-authored.
The work needs Mesopotamian walls, a Roman arch, three patent bricks, a
token grid, and — per the new requirement — a substantial library of UNESCO
World Heritage sites. Hand-authoring forty sites is not viable; a parametric
brick-construction kit that emits *real, grid-legal* LDraw placements is.
Phase 4.

**Gap E — there are no flags.** The Atlas movement needs each site's state
party's flag, built from real bricks, to real published construction
specifications, waving. Phase 4.

**Gap F — there is no work, only an engine.** Phase 5 is the piece itself:
the full screenplay realised, in four durations.

### 1.3 The one-sentence architecture

> Real LDraw vector geometry → a compiled, versioned mesh bundle → uploaded
> once to WebGL as instanced meshes and line segments → transformed every
> frame by a declarative timeline evaluated against one deterministic clock
> → which also drives a procedurally generated fugue rendered by WebAudio.

Everything downstream of "a list of triangles" is new. Everything upstream
of it already exists and is not touched except to *add* edges and correct
BFC winding.

---

## Architecture

### 2.1 New crates and modules

```
crates/
  spex-ldraw/          (existing — extended in M51)
    src/edges.rs       NEW: type 2 / type 5 line extraction
    src/bfc.rs         NEW: BFC CERTIFY / INVERTNEXT winding resolution
  spex-mesh/           NEW crate — the mesh bundle writer
    src/lib.rs
    src/bundle.rs      manifest + binary buffer writer
    src/material.rs    LDraw colour → PBR material mapping
    src/weld.rs        vertex welding / normal smoothing by crease angle
  spex-build/          NEW crate — the parametric brick construction kit
    src/lib.rs
    src/grid.rs        the real 20/8/24 LDU grid + legality validation
    src/primitives.rs  wall, column, arch, stair, ziggurat, dome, tower
    src/mosaic.rs      2D image/spec → 1x1 tile mosaic
    src/emit.rs        Placement list -> .ldr text
  spex-flag/           NEW crate — flags as brick mosaics
    src/lib.rs
    src/spec.rs        declarative flag construction specs
    src/quantize.rs    sRGB -> CIELAB -> nearest real LDraw colour
  spex-heritage/       NEW crate — UNESCO World Heritage ingestion
    src/lib.rs
    src/list.rs        real whc.unesco.org List parsing
    src/curation.rs    buildability filter + exclusion list
  spex-show/           NEW crate — the show compiler
    src/lib.rs
    src/model.rs       Show / Movement / Shot / Track / Cue
    src/resolve.rs     duration resolver (4:00 / 10:00 / 60:00 / endless)
    src/compile.rs     .show.json + scenes -> a servable show bundle
  spex-fugue/          NEW crate — the score generator (Rust side)
    src/lib.rs
    src/theory.rs      pitch/interval/mode primitives
    src/counterpoint.rs subject, answer, countersubject, episode, stretto
    src/emit.rs        FugueScore JSON (+ optional .mid export)
  spex-wasm/           NEW crate (Phase 6, M86-M90) — cdylib + rlib
    src/lib.rs         wasm-bindgen exports: resolve_show, build_recipe, …
    src/timeline.rs    the zero-copy per-frame evaluator
    src/dsp.rs         AudioWorklet synthesis (M89b, optional)
```

Phase 6 (§9) compiles `spex-show`, `spex-build`, `spex-flag` and
`spex-fugue` to `wasm32-unknown-unknown` and calls them from the viewer, so
the resolver, the evaluator, the grid and the counterpoint exist exactly
once. Until then the viewer consumes their *output* as JSON — deliberately,
so that phase is a swap and not a rewrite.

```
viewer/src/
  mesh/
    bundle.ts          fetch + parse a mesh bundle
    instanced.ts       InstancedMesh construction and per-instance updates
    materials.ts       LDraw material -> three.js material
    edges.ts           hard edges + conditional-edge shader
    render.ts          the mesh render pass, lighting, tone mapping, bloom
  show/
    clock.ts           the deterministic master clock
    timeline.ts        Show model + per-frame track evaluation
    easing.ts          the easing function library
    camera.ts          camera director
    choreography.ts    per-instance transform application
    effects.ts         dissolve, materialize, point<->mesh crossfade
    hud.ts             chronicle cards, titles, credits
  audio/
    engine.ts          WebAudio graph + master bus + limiter
    scheduler.ts       lookahead note scheduler bound to the show clock
    synth.ts           voice synthesis (organ, bass, percussion)
    reverb.ts          procedurally generated impulse response
    fugue.ts           runtime realisation of a FugueScore
    midi.ts            MIDI event model + optional SMF import/export
  flag/
    wave.ts            per-instance wave displacement
```

### 2.2 New on-disk formats (all get a JSON Schema in `spec/`)

| File | Schema | Written by | Read by |
|---|---|---|---|
| `mesh.json` + `buffers/*.bin` | `mesh.schema.json` | `spex mesh-part` / `mesh-model` / `show-build` | `viewer/src/mesh/bundle.ts` |
| `show.json` (source) | `show.schema.json` | hand-authored + `spex-show` | `spex show-build` |
| `show-resolved.json` | `show-resolved.schema.json` | `spex show-build` | `viewer/src/show/timeline.ts` |
| `fugue.json` | `fugue.schema.json` | `spex-fugue` | `viewer/src/audio/fugue.ts` |
| `flag.json` | `flag.schema.json` | hand-authored per flag | `spex flag` |
| `heritage.json` | `heritage.schema.json` | `spex heritage-index` | `spex-build`, the Atlas |

Every one of these gets an explicit `"version": 1` from day one — the
`sequence.json` precedent, not the unversioned `graph.json` one. Every one
of them gets a case in `crates/spex-cli/tests/schema_validation.rs` that
validates *real generated output*, not a hand-written fixture.

### 2.3 What is explicitly out of scope

- Physics simulation. Every "settle", "drop", "collapse" in the screenplay
  is authored easing, not a solver. If a rigid-body moment is ever wanted,
  it gets its own milestone and its own dependency discussion first.
- Ray tracing / path tracing / global illumination. Real-time raster with a
  good material and edge model, plus one shadow-casting key light.
- Video export. The work is a live WebGL piece. If a video file is needed
  for a submission, capture it externally from the running piece.
- Any use of the trademarked brand name, any official set's design, any
  scraped commercial catalogue. See §11.

---
