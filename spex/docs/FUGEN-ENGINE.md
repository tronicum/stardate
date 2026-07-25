# FUGEN-ENGINE.md — implementation spec, M51–M97

**Working title of the work:** *Die Geschichtliche Matrix* (The Historical Matrix)
**Engine:** the Fugen Engine — a vector-accurate Klemmbaustein renderer, a
runtime show engine, and a procedural fugue synthesizer, all built *inside*
`spex`, reusing what M1–M50 already shipped.
**Archive signature:** IA-2026-002
**Spec written:** 2026-07-25 · **Target completion:** 2027-01-24 (26 weeks)
**Audience:** the agent session implementing this. Read `AGENTS.md`,
`CLAUDE.md`, `TODOs.md`, `BRICKs.md` and `docs/agents/` first — this file
assumes all four.

---

## 0. How to work through this document

This spec is deliberately over-specified: exact file paths, exact function
signatures, exact acceptance criteria. That is not a straitjacket — where a
signature turns out wrong once real code exists, change it and say so in the
commit message. It exists so that no milestone ever starts with "what
exactly am I building?"

**The rules of engagement, unchanged from this repo's existing practice:**

1. **One milestone = one or more small commits, never one giant commit.**
   `docs/agents/working-mode.md`'s commit discipline applies verbatim.
2. **Verify before committing**, using `docs/agents/verification.md`'s
   ladder. Every milestone below names which rungs of that ladder are
   mandatory for it. Rung 5 (real headless-Chromium) is mandatory for
   *every* viewer-visible milestone in this document — this is a
   *cinematic* project; "it compiles" proves nothing about it.
3. **The viewer-rebuild-order gotcha is now a per-milestone hazard.** Half
   of this document is TypeScript. `cd viewer && npm run build && cd .. &&
   cargo build --release` after *every* viewer change, before *every*
   verification round.
4. **Real data only.** Real LDraw geometry, the real LDConfig colour table,
   the real UNESCO World Heritage List, real published flag construction
   specifications, real patent numbers. No fabricated dimensions, no
   invented colour codes, no "roughly like" flag proportions. Where a real
   value cannot be obtained, the milestone says so explicitly and the code
   records the gap rather than papering over it.
5. **Additive, not destructive.** The point-cloud pipeline
   (`spex-io`/`spex-tiler`/octree/LOD/`sequence.json`) stays exactly as it
   is and keeps working for every existing demo. The mesh renderer is a
   *second* render mode selected by the presence of a manifest, not a
   replacement. `spex brick-part`/`brick-model`/`brick-assembly`/
   `brick-cinematic` keep working unchanged for the entire duration of this
   plan. If a change to shared code would alter an existing demo's output,
   that is a bug in the change, not an acceptable cost.
6. **Update `TODOs.md` as you go.** Each finished milestone gets its entry
   in the Milestones list, in the same style as M1–M50: what was built,
   what was *verified*, and with which real numbers. `TODOs.md` remains the
   single source of truth for status; this file remains the plan.
7. **Do not batch milestones.** Ship M51 before starting M52. Twelve
   finished milestones beat forty started ones. Every milestone below is
   written so that the repository is in a working, demoable state at the
   end of it.

---

## 0.1 BINDING AMENDMENTS (rev 2) — read before implementing anything below

Rev 1 was reviewed by seven specialists (architecture, technical art,
screenplay, production, creative direction, cultural history, browser
performance) plus an agentic-coding budget review. The full record, with an
adopt/reject decision and a reason for every finding, is
**`docs/FUGEN-ENGINE-REVIEW-01.md`**. Where that document and the text below
disagree, **the review wins**. The items here are the ones that would
otherwise cause days of wasted work.

**Scope decision, taken in week 1 and overriding §10:** the deliverable is
the **4:00 cut with 3 Atlas sites**. October ships an **Act I preview**, not
the piece — this resolves a collision with
`claude/masterplan-iunctura-site.md`, which promises a running loop by end of
October. **Phase 6 (wasm), M77 (Atlas autopilot), Atlas tier C and the 60:00
cut are explicitly post-date.** An independent estimate put rev 1's full
scope at 41 weeks, not 26.

**Correctness (fix before writing M51):**

- **B1 — the conditional-edge test in §3 M51/M57 is inverted.** A type-5 line
  is a silhouette when both control points project to the **same** side of
  the edge. Draw when signs agree; collapse when they differ.
- **B2 — geometry must stay colour-neutral.** `resolve_part_full` preserves
  LDraw code 16 as `color_code: Option<u32>` (`None` = inherit); parts key on
  `part_file` only; `submeshes[].material: number | null`.
- **B3 — do not change `load_colors`'s signature.** Two real call sites
  destructure the tuple. Add `load_colors_full()` for `spex-mesh` only.
- **B4 —** `spex serve` bails unless `tileset.json`/`sequence.json` exists;
  M53 must add `mesh.json` and `show-resolved.json` to that guard.
- **B5 —** `PartGeometry` needs `sources: Vec<String>` and a per-triangle
  `source: u16` **in M51**, or M59's LOD1 has nothing to gate on.
- **B11 — `mesh.json` must not carry an `instances[]` array.** 250 k
  instances is 37 MB of JSON → ~120 MB heap → 1.5 s of main-thread parse.
  Binary: `(i16 x,y,z; u8 orientation; u8 material; u16 part)` = 10 B each.
- **B12/B13/B14 — colour and post.** Store **linear** colour in `mesh.json`;
  `NoToneMapping` on the renderer with ACES last in `OutputPass` (bloom must
  run in linear HDR); add triangular dither + ~1.5% grain, or the black field
  bands.

**Performance (binding budgets, per §2 of the review):**

- **Evaluate only dirty instances**, never all of them. M87's "<2 ms for
  250 k" is not achievable; touching the ~20 k instances an active track
  actually moves is <1 ms. *Largest single correction in the document.*
- **Geometric edges only above ~40 px projected height** (hero shots, ≤3 k
  bricks). Everything else gets a screen-space depth+normal outline pass,
  whose cost is independent of instance count.
- **`setPixelRatio(Math.min(devicePixelRatio, 1.5))`**, partial buffer
  uploads via `addUpdateRange`, visible-instance compaction into
  `mesh.count`.
- Long runs: no `innerHTML` per frame, no `setInterval` for audio scheduling
  (use an `AudioWorklet` pump — hidden tabs throttle to 1/min), **f64 for
  absolute time** (f32 resolves to 16 ms after three days), a
  `webglcontextlost` handler, a fixed WebAudio voice pool, and wasm linear
  memory with `initial == maximum` so a grow traps instead of silently
  detaching every `Float32Array` view.
- **CI asserts counters, never fps.** `--disable-gpu` is SwiftShader and is
  rejected as a "Low tier proxy".
- **WebGPU: not this year.** Deferred with reasons in the review.

**The work (§8 is superseded by the review's §3):**

- The screenplay is **re-authored in bars**: 84 bpm, 4/4 ⇒ 1 bar = 2.857 s,
  the canonical cut is **84 bars**. Rev 1's second-based timings are not
  bar-aligned and made rule 2 unsatisfiable.
- Act budget **17 / 20 / 20 / 6 / 21 bars**; **the Kick becomes 2 beats**
  (still `fixed` in every cut); **A4-S01b "Der letzte Stein" is added**;
  the HUD numeral in A2-S03 is cut in favour of an overrunning count; the
  Atlas **accumulates** instead of resetting; the monolith becomes a visible
  continuity object; **A1-S03 is the centre of the piece** and its edges must
  land in one frame with the music entering *after*.
- **Flags: no poles, no wind by default**, and per-site flag suppression is
  mandatory — never for transboundary or contested-sovereignty sites.

**Facts (all of §8's dates were checked and several were wrong):** Uruk is
not "the first standardised brick"; the token thesis is contested and
"the invention of number" must go; Lydia is *not* the first fungible unit
(Kroisos's mid-6th-century reform is); the Pont du Gard has no brick and is
c. 40–60 CE; Stonehenge's mortice-and-tenon sarsens are c. 2500 BCE, not
3000; `GB 529580` was filed in **1940**; **`BE 311029` is unconfirmed — use
FR 588985**; **Interlego v Tyco was lost by Lego**; and the AI "token" is a
pun on the clay token, not a descent — the work must say so. The Louis Cousin
genealogy in the Bewegung masterplan fuses two different people (the Cour des
Monnaies president died in 1707) and must be cut. Full corrections, with
sources and suggested wording, in the review's §4.

---

**A note on the name.** `BRICKs.md`'s naming convention holds throughout:
"Klemmbaustein" in prose, `brick` in code, never the trademarked brand name
in identifiers or commands. The project's own backronym — L.E.G.O., "Local
Evolved Great Objects" / "Lokale Erzeugte Gute Objekte" — is the wink where
one is wanted.

---

## 1. Where we are, and what is actually missing

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

## 2. Architecture

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

## 3. Phase 1 — the vector-accurate renderer (M51–M59)

### M51 — BFC-correct geometry and real edge extraction

**Why.** Two real defects block catalogue-quality rendering.
(a) `geometry.rs::triangle_normal`'s own doc comment admits it ignores
`BFC INVERTNEXT`, so some faces on composite parts carry inward normals —
tolerable for baked point shading, visibly wrong under real lighting.
(b) LDraw type 2 (edge) and type 5 (conditional edge) lines are currently
skipped outright. They are not decoration: the black outline they produce
*is* the visual signature of a rendered brick, and conditional edges are how
a cylinder's silhouette stays crisp without wireframing its whole tessellation.

**Files.** `crates/spex-ldraw/src/bfc.rs` (new),
`crates/spex-ldraw/src/edges.rs` (new), `geometry.rs` (extended),
`lib.rs` (re-exports).

**Signatures.**

```rust
// bfc.rs
/// Winding accumulated down a reference chain. LDraw's real BFC spec:
/// a file declares `0 BFC CERTIFY CCW` (the near-universal case) or `CW`;
/// `0 BFC INVERTNEXT` flips the *next* type-1 reference only; and a
/// reference whose own 3x3 matrix has a negative determinant is itself a
/// mirroring transform, which flips winding again. All three compose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Winding { Ccw, Cw }

impl Winding {
    pub fn flipped(self) -> Winding;
    /// `true` when the composed state means a face's stored vertex order
    /// must be reversed before its normal is taken.
    pub fn is_reversed(self) -> bool;
}

/// Per-file BFC state machine, fed one `0 BFC ...` meta line at a time.
#[derive(Clone, Debug, Default)]
pub struct BfcState {
    pub certified: bool,
    pub winding: Winding,
    pub invert_next: bool,
}

impl BfcState {
    pub fn apply_meta(&mut self, tokens: &[&str]);
    /// Consumes a pending INVERTNEXT and folds in the determinant sign of
    /// the reference's own matrix.
    pub fn winding_for_reference(&mut self, matrix: &[f64; 9]) -> Winding;
}

pub fn determinant3(m: &[f64; 9]) -> f64;

// edges.rs
/// A real LDraw line primitive. `Hard` is a type-2 edge (always drawn);
/// `Conditional` is a type-5 optional line, drawn only when its two
/// control points project to the SAME side of the edge in screen space
/// (i.e. the two adjacent facets face the same way, so the edge is a
/// silhouette) — the real mechanism that keeps a curved surface's
/// silhouette crisp without drawing its whole tessellation. Rev 1 stated
/// this backwards; see §0.1 B1.
#[derive(Clone, Debug, PartialEq)]
pub enum EdgeKind {
    Hard,
    Conditional { control: [[f64; 3]; 2] },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub vertices: [[f64; 3]; 2],
    pub color_code: u32,
    pub kind: EdgeKind,
}

// geometry.rs — the new full-resolution entry point. `resolve_part` stays
// exactly as it is (triangles only, no BFC correction) so that every
// existing caller — brick.rs, the point pipeline, every existing demo —
// produces byte-identical output to today. This is additive.
#[derive(Clone, Debug, Default)]
pub struct PartGeometry {
    pub triangles: Vec<Triangle>,
    pub edges: Vec<Edge>,
    /// Real part title from the file's first `0 <description>` line.
    pub description: Option<String>,
    /// Real `!LICENSE` / `0 Author:` provenance carried through, so a
    /// bundle can state where its geometry came from (see M84).
    pub license: Option<String>,
    pub author: Option<String>,
}

pub fn resolve_part_full(
    cache: &LdrawCache,
    part_file: &str,
    color_code: u32,
) -> Result<PartGeometry>;
```

**Behavioural requirements.**

- `resolve_part_full` composes BFC state exactly as LDraw's own spec
  describes: certification is per-file; `INVERTNEXT` applies to exactly one
  following type-1 line; a negative-determinant reference matrix flips
  winding for its whole subtree. When the composed winding is reversed, the
  emitted `Triangle`'s vertices are stored in reversed order, so that
  `triangle_normal` — unchanged — yields the true outward normal.
- Type 5 control points are transformed by the same composed matrix as the
  edge's own endpoints.
- An uncertified file (no `BFC CERTIFY`) is treated as CCW but flagged;
  `PartGeometry` gains no field for this, but a `tracing`/`eprintln!` warning
  once per distinct uncertified file is required so it is visible.

**Acceptance criteria.**

1. `resolve_part_full(cache, "3005.dat", 4)` returns > 0 edges, and the
   bounding box of `triangles` equals today's `resolve_part` bounding box
   exactly (8mm × 8mm × 11.2mm converted through `LDU_TO_MM`).
2. Every triangle of `3005.dat` has an outward normal: for each triangle,
   `dot(normal, centroid - part_centroid) > 0` holds for at least 95% of
   faces, and no face on the six flat outer walls fails. (The stud's inner
   tube legitimately faces inward; hence 95%, not 100% — record the real
   measured figure in the milestone's `TODOs.md` entry.)
3. `resolve_part` output is unchanged: a test resolves `3005.dat` both ways
   and asserts the triangle *set* is identical up to per-triangle vertex
   rotation.
4. Unit tests with synthetic fixtures cover: `INVERTNEXT` applying to
   exactly one reference; a mirroring matrix (`det < 0`) flipping winding; a
   type-2 line parsed with its colour; a type-5 line parsed with both
   control points transformed.

**Verification ladder.** Rungs 1, 2, 3 (`spex brick-part 1x1-brick` must
still produce a byte-identical tileset — diff it), 7.

---

### M52 — the mesh bundle format (`spex-mesh`)

**Why.** The viewer needs geometry it can upload once. JSON with a million
floats is not that. A small manifest plus tightly packed little-endian
binary buffers is — the same shape as `tileset.json` + `octree/*.bin`, which
this repo already knows how to write, serve, and parse.

**Files.** `crates/spex-mesh/` (new crate), `spec/mesh.schema.json` (new).

**Directory layout produced.**

```
<bundle-dir>/
  mesh.json                 the manifest
  buffers/
    p<partIndex>.pos.bin    f32 LE, 3 per vertex, LDraw LDU, Y already flipped to spex's Y-up mm
    p<partIndex>.nrm.bin    f32 LE, 3 per vertex, unit length
    p<partIndex>.idx.bin    u32 LE triangle indices
    p<partIndex>.edge.bin   f32 LE, 6 per hard edge (two endpoints)
    p<partIndex>.cond.bin   f32 LE, 12 per conditional edge (2 endpoints + 2 control points)
```

**Manifest shape (`mesh.json`).**

```jsonc
{
  "version": 1,
  "generator": "spex-mesh 0.1.0",
  "unit": "mm",
  "upAxis": "+Y",
  "bounds": { "min": [0,0,0], "max": [0,0,0] },
  "attribution": {
    "geometrySource": "LDraw Parts Library (ldraw.org), CCAL 2.0",
    "colorTable": "LDConfig.ldr",
    "note": "see docs/FUGEN-ENGINE.md §11"
  },
  "parts": [
    {
      "index": 0,
      "partFile": "3005.dat",
      "description": "Brick  1 x  1",
      "vertexCount": 336,
      "triangleCount": 112,
      "hardEdgeCount": 96,
      "conditionalEdgeCount": 48,
      "bounds": { "min": [0,0,0], "max": [0,0,0] },
      "buffers": {
        "position": "buffers/p0.pos.bin",
        "normal":   "buffers/p0.nrm.bin",
        "index":    "buffers/p0.idx.bin",
        "hardEdge": "buffers/p0.edge.bin",
        "condEdge": "buffers/p0.cond.bin"
      },
      "submeshes": [
        { "colorCode": 16, "indexOffset": 0, "indexCount": 336 }
      ]
    }
  ],
  "materials": [
    {
      "colorCode": 0,
      "name": "Black",
      "baseColor": [0.106, 0.165, 0.204],
      "edgeColor": [0.349, 0.349, 0.349],
      "alpha": 1.0,
      "finish": "solid",
      "metalness": 0.0,
      "roughness": 0.28,
      "luminance": 0.0
    }
  ],
  "instances": [
    { "part": 0, "material": 0, "translation": [0,0,0], "matrix": [1,0,0,0,1,0,0,0,1], "buildStep": 0, "id": "monolith/brick-00" }
  ]
}
```

**Signatures.**

```rust
// bundle.rs
pub struct MeshBundleBuilder { /* … */ }

impl MeshBundleBuilder {
    pub fn new() -> Self;
    /// Adds a distinct real part's geometry exactly once. Returns its
    /// part index — the caller reuses it for every instance of that part,
    /// the same resolve-once discipline brick.rs already applies.
    pub fn add_part(&mut self, part_file: &str, geometry: &PartGeometry) -> usize;
    pub fn add_material(&mut self, colors: &ColorTable, color_code: u32) -> usize;
    pub fn add_instance(&mut self, part: usize, material: usize, placement: &Placement, id: String);
    pub fn write(self, out_dir: &Path) -> Result<MeshBundleStats>;
}

pub struct MeshBundleStats {
    pub part_count: usize,
    pub instance_count: usize,
    pub total_vertices: usize,
    pub total_triangles: usize,
    pub bytes_written: u64,
}

// weld.rs
/// Welds coincident vertices and averages normals across faces whose
/// dihedral angle is below `crease_degrees`. A real brick's flat walls
/// must stay flat-shaded at their corners (90° >> crease), while a stud's
/// cylinder must smooth (its facet angle is ~22.5° for LDraw's 16-segment
/// primitives) — so the default is 33.0, comfortably between the two.
pub fn weld_and_smooth(triangles: &[Triangle], crease_degrees: f64) -> WeldedMesh;

pub struct WeldedMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub color_codes: Vec<u32>, // per triangle
}
```

**Acceptance criteria.**

1. `spex-mesh` writes a bundle for `3005.dat` whose `mesh.json` validates
   against `spec/mesh.schema.json`.
2. Sum of buffer file sizes matches the counts in the manifest exactly
   (`vertexCount * 12` for positions, etc.) — asserted in a test.
3. Welding reduces `3005.dat`'s vertex count by ≥ 40% versus the naive
   3-vertices-per-triangle expansion, with no visible seam (verified in M54,
   not here).
4. A stud's cylinder is smooth-shaded and the brick's box corners are not:
   test that the maximum angle between any two normals sharing a welded
   vertex is < 33° after welding.
5. `MeshBundleStats` printed by the CLI in M53 reports real numbers.

**Verification ladder.** 1, 2, 3, 7.

---

### M53 — `spex mesh-part` and `spex mesh-model`

**Why.** A CLI entry point per new pipeline stage is this repo's pattern
(`brick-part`, `brick-model`), and it makes the bundle inspectable long
before the viewer can draw it.

**Files.** `crates/spex-cli/src/mesh.rs` (new), `main.rs` (clap wiring).

**CLI.**

```
spex mesh-part <alias-or-part.dat> [--color <n>] [--crease <deg>] -o <bundle-dir> [--cache-dir <dir>]
spex mesh-model <name-or-path.ldr>  [--crease <deg>] -o <bundle-dir> [--cache-dir <dir>]
```

`mesh-model` resolves each distinct `(part, colour)` exactly once — the
same discipline as `brick::render_scene_to_points` — and emits one
`instances[]` entry per real placement, carrying the placement's own
`buildStep`.

**Acceptance criteria.**

1. `spex mesh-model ldraw-scenes/monolith.ldr -o demos/monolith-mesh/bundle`
   prints: 2 distinct parts, 9 instances, and a bounds whose Y extent is
   73.6 mm ± 0.01 — the already-established real monolith height.
2. `spex mesh-model car -o …` reports 26 distinct parts and 61 instances —
   the real numbers M44 established for `car.ldr`.
3. Running either command twice produces byte-identical output (determinism).
4. The bundle directory is servable as static files with no server change
   (`ServeDir` already handles it).

**Verification ladder.** 1, 2, 3, 7.

---

### M54 — the viewer's mesh render mode

**Why.** This is the milestone where the picture stops being soft.

**Files.** `viewer/src/mesh/bundle.ts`, `viewer/src/mesh/render.ts`,
`viewer/src/mesh/materials.ts`, `viewer/src/main.ts` (mode selection).

**Mode selection.** Exactly the pattern `fetchSequence` already established:

```ts
// bundle.ts
export interface MeshBundle { /* mirrors mesh.json, typed */ }

/** Returns null when mesh.json is absent — i.e. every existing point-cloud
 *  and graph tileset, which must keep working byte-for-byte as before. */
export async function fetchMeshBundle(baseUrl: string): Promise<MeshBundle | null>;

export async function fetchMeshBuffers(
  baseUrl: string,
  bundle: MeshBundle,
): Promise<Map<number, PartBuffers>>;

export interface PartBuffers {
  position: Float32Array;
  normal: Float32Array;
  index: Uint32Array;
  hardEdge: Float32Array;
  condEdge: Float32Array;
}
```

`main.ts` gains one branch at the top: if `fetchMeshBundle()` returns a
bundle, run the mesh path; otherwise run the existing point path completely
unchanged. No existing code path may be edited beyond adding that branch.

**Rendering requirements.**

- `THREE.WebGLRenderer` with `antialias: true`, `outputColorSpace =
  THREE.SRGBColorSpace`, `toneMapping = THREE.ACESFilmicToneMapping`,
  `toneMappingExposure` exposed on the controls panel.
- Lighting rig: one directional key light (shadow-casting, 2048² map), one
  hemisphere fill, one low rim light. Positions are relative to the scene's
  bounds diagonal, not absolute, so any scene scale works.
- Materials from `mesh.json`'s `materials[]`, via `materials.ts` (M56 refines
  the finishes; M54 may ship `MeshStandardMaterial` with baseColor/roughness
  only).
- Backface culling **on** — which is only correct because M51 fixed winding.
  A visible interior surface here means M51 is wrong; that is the point of
  doing them in this order.

**Acceptance criteria.**

1. `spex serve demos/monolith-mesh/bundle` renders the monolith as solid
   geometry with visibly crisp silhouettes and no z-fighting.
2. A real headless-Chromium screenshot at 1600×1000 shows: no interior
   faces, no black holes, shadow visible on the ground plane, ≥ 55 fps
   reported by the HUD.
3. Every existing demo (`spex gallery` over a full `walkthrough.sh`
   regeneration) renders exactly as before — the mesh branch never triggers.
4. Console has zero errors and zero WebGL warnings.

**Verification ladder.** 1, 2, 3, 5 (**mandatory**, with screenshots
attached to the milestone note), 6, 7.

---

### M55 — instanced rendering

**Why.** The Atlas movement will place tens of thousands of bricks. One
draw call per brick is not survivable; one per distinct `(part, material)`
pair is trivial.

**Files.** `viewer/src/mesh/instanced.ts`.

**Signatures.**

```ts
export interface InstanceGroup {
  part: number;
  material: number;
  mesh: THREE.InstancedMesh;
  hardEdges: THREE.LineSegments;      // instanced via InstancedBufferGeometry
  conditionalEdges: THREE.LineSegments;
  /** instanceId -> the bundle's own stable instance id, for choreography */
  ids: string[];
}

export function buildInstanceGroups(
  bundle: MeshBundle,
  buffers: Map<number, PartBuffers>,
): InstanceGroup[];

/** Per-frame write of one instance's transform. Callers batch these and
 *  call `flush()` once, so the instanceMatrix buffer is uploaded at most
 *  once per group per frame. */
export class InstanceWriter {
  constructor(groups: InstanceGroup[]);
  setTransform(id: string, position: THREE.Vector3, quaternion: THREE.Quaternion, scale: number): void;
  setVisible(id: string, visible: boolean): void;
  /** Per-instance scalar consumed by the dissolve shader (M65), 0..1. */
  setDissolve(id: string, amount: number): void;
  flush(): void;
}
```

**Acceptance criteria.**

1. A synthetic 50 000-instance scene (generated by `spex-build` in M72, or
   a temporary generator here) renders at ≥ 60 fps at 1080p on the
   development machine, with draw calls ≤ 3 × distinct part count.
2. `renderer.info.render.calls` is asserted in the headless check.
3. Updating every instance's transform each frame costs < 4 ms for 50 000
   instances (measure and record the real number).

**Verification ladder.** 1, 2, 5 (**mandatory**), 7.

---

### M56 — the real LDraw material system

**Why.** `LDConfig.ldr` carries far more than RGB: `ALPHA`, `LUMINANCE`,
and the finish keywords `CHROME`, `PEARLESCENT`, `RUBBER`, `MATTE_METALLIC`,
`METAL`, and `MATERIAL SPECKLE|GLITTER`. The electrum coin of Act II, the
chrome of Act IV's token grid, and the transparent phase of the dissolve all
depend on these being real rather than approximated.

**Files.** `crates/spex-ldraw/src/colors.rs` (extended),
`crates/spex-mesh/src/material.rs`, `viewer/src/mesh/materials.ts`.

**Signatures.**

```rust
// colors.rs — replaces the tuple ColorTable value. Keep a
// `ColorTable = HashMap<u32, LdrawColor>` alias and a
// `LdrawColor::rgb()` accessor so existing call sites change minimally.
#[derive(Clone, Debug, PartialEq)]
pub struct LdrawColor {
    pub code: u32,
    pub name: String,
    pub value: [u8; 3],
    pub edge: [u8; 3],
    pub alpha: u8,            // real ALPHA, default 255
    pub luminance: u8,        // real LUMINANCE, default 0
    pub finish: Finish,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Finish {
    Solid,
    Chrome,
    Pearlescent,
    Rubber,
    MatteMetallic,
    Metal,
    Speckle { value: [u8; 3], alpha: u8, luminance: u8, min_size: f64, max_size: f64 },
    Glitter { value: [u8; 3], alpha: u8, luminance: u8, fraction: f64, vfraction: f64, size: f64 },
}
```

**Mapping to PBR** (documented in `material.rs`, these are deliberate
artistic choices calibrated against real reference renders, and must be
recorded as such — not presented as physical measurements):

| Finish | metalness | roughness | notes |
|---|---|---|---|
| Solid (opaque ABS) | 0.00 | 0.28 | the baseline brick look |
| Solid (transparent) | 0.00 | 0.05 | `transmission` 0.9, `thickness` from part bounds |
| Rubber | 0.00 | 0.85 | |
| Pearlescent | 0.35 | 0.35 | + subtle iridescence via `sheenColor` |
| MatteMetallic | 0.80 | 0.55 | |
| Metal | 1.00 | 0.25 | |
| Chrome | 1.00 | 0.03 | needs an environment map — see below |
| Speckle / Glitter | 0.00 | 0.30 | procedural noise in a custom `onBeforeCompile` chunk |

Chrome and any metal need reflections. **No external HDRI asset** (real-data
rule plus offline-capability rule): generate the environment with
`THREE.PMREMGenerator` from a procedurally rendered gradient scene defined
in `materials.ts`, seeded from the show seed. Document that it is synthetic.

**Acceptance criteria.**

1. `load_colors` parses every real field from the current real
   `LDConfig.ldr`; a test asserts ≥ 5 distinct `Finish` variants are found
   in the real file, and that code 0 (Black), 4 (Red), 47 (Trans-Clear) and
   383 (Chrome Silver, if present in the current file) resolve to the
   expected finishes.
2. A test bundle rendering one brick per finish variant is screenshotted;
   transparent reads as transparent, chrome reflects, rubber is matte.
3. Existing point-pipeline callers still compile and produce identical
   output (they only ever read `.value`).

**Verification ladder.** 1, 2, 3, 5 (**mandatory**), 7.

---

### M57 — crisp edges and conditional edges *(the "vektorgenau" milestone)*

**Why.** This is the single most important visual milestone in the document.
A brick without its edge outline reads as a soft plastic blob; with it, it
reads as the catalogue image everybody has in their head.

**Files.** `viewer/src/mesh/edges.ts`.

**Requirements.**

- **Hard edges** (type 2) drawn as screen-space-constant-width lines. Not
  `LineBasicMaterial` (`linewidth` is ignored on every real platform):
  implement instanced quad expansion in a vertex shader (the standard
  "fat line" technique — `THREE.Line2`/`LineMaterial` from three's addons is
  acceptable if it instances correctly; if it does not, write the shader).
  Width in device pixels is a controls-panel parameter, default 1.4.
- **Conditional edges** (type 5) drawn only when the two control points
  project to the **same** side of the line (§0.1 B1 — rev 1 had this
  inverted). This is a per-frame, per-edge test and must run on the GPU:
  pass both control points as vertex attributes **to all four quad
  corners** (so the collapse decision is identical per corner and cannot
  produce flickering half-quads), do the test after the perspective divide
  with a `w > 0` guard on all four points, and collapse the quad to zero
  area when the signs **differ**.
- **Depth bias.** Edges must not z-fight with the faces they bound. Use a
  small view-space depth offset (`gl_Position.z -= bias * gl_Position.w`),
  parameterised, default tuned against the monolith at both extreme camera
  distances the screenplay uses.
- Edge colour comes from the material's real `edge` value from
  `LDConfig.ldr`, not a hardcoded black.

**Acceptance criteria.**

1. Screenshot of a single `3005.dat` at 2000×2000 shows a continuous,
   uniform-width outline with no gaps at the stud's silhouette and no
   wireframe lines across the stud's cylinder.
2. Orbiting the camera 360° in the headless session, sampling 12 angles:
   the conditional-edge count actually drawn changes between angles (proof
   the test is running), and no frame shows tessellation lines on the
   cylinder.
3. At 0.5× and 50× the default camera distance, no z-fighting artefacts.
4. Frame cost of the edge pass ≤ 25% of total frame time on the 50 000-
   instance scene.

**Verification ladder.** 1, 2, 5 (**mandatory — this milestone is defined by
its screenshots**), 7.

---

### M58 — the post-processing and lighting pipeline

**Why.** Act IV is neon; the Kick is a flash; the whole piece needs a
consistent filmic look. Doing this once, properly, is cheaper than fighting
it per-shot.

**Files.** `viewer/src/mesh/render.ts` (extended).

**Requirements.**

- `EffectComposer` chain: render pass → SSAO (optional, quality-gated) →
  UnrealBloom (threshold/strength/radius exposed and animatable from the
  timeline) → SMAA → output pass. All from `three/addons`; no new npm
  dependency.
- A `QualityTier` enum (`Low` / `Medium` / `High`) chosen automatically from
  a 2-second startup benchmark, overridable by `?quality=` and by a controls
  dropdown. Low disables SSAO and halves shadow map resolution; the piece
  must remain *watchable*, not merely runnable, at Low.
- Bloom, exposure, vignette and a global colour-grade LUT strength are
  exposed as timeline-animatable parameters (M62 consumes them).

**Acceptance criteria.**

1. On the development machine at 1080p: High ≥ 60 fps, Low ≥ 60 fps on a
   deliberately throttled headless session (Chromium `--disable-gpu` is an
   acceptable proxy; record what was actually used).
2. Bloom threshold animated from 1.0 → 0.2 over 1 s produces a visible,
   smooth ramp in a 30-frame headless capture.
3. No `NaN`/black-frame regressions when the scene is empty (an important
   edge case: Act I's first six seconds are almost empty).

**Verification ladder.** 1, 2, 5 (**mandatory**), 7.

---

### M59 — mesh LOD and culling

**Why.** The 60-minute Atlas cut will hold dozens of sites in memory. A
site 400 m "away" does not need its studs.

**Files.** `crates/spex-mesh/src/lib.rs` (LOD generation),
`viewer/src/mesh/instanced.ts` (LOD selection).

**Requirements.**

- The bundle writer emits up to three LODs per part:
  - **LOD0** — full geometry.
  - **LOD1** — studs and tubes removed (identify them by the real primitive
    names in the reference chain: `stud*.dat`, `4-4cyli.dat` inside a stud
    subpart — record the real detection rule in code comments, and gate it
    on the *reference path*, never on a heuristic about geometry).
  - **LOD2** — the part's oriented bounding box, 12 triangles, plus its 12
    hard edges.
- Selection by projected screen-space size of the instance's bounds, with
  hysteresis to avoid popping.
- Frustum culling per instance group via a per-group BVH of instance bounds
  (a simple uniform grid is sufficient and cheaper to get right).

**Acceptance criteria.**

1. A 40-site Atlas scene (from M74) with ≥ 200 000 instances renders at
   ≥ 45 fps at 1080p on High.
2. LOD transitions are invisible in a 60-frame dolly-back capture (compare
   consecutive frames; no single-frame luminance jump > 3%).
3. LOD1 reduces triangle count for `3001.dat` (Brick 2×4) by ≥ 55%; record
   the real measured figure.

**Verification ladder.** 1, 2, 5 (**mandatory**), 7.

---

## 4. Phase 2 — the runtime show engine (M60–M66)

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

**Verification ladder.** 1, 2, 7.

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

**Verification ladder.** 1, 2, 3, 7.

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

**Verification ladder.** 1, 2, 5 (**mandatory**), 7.

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

**Verification ladder.** 1, 2, 5 (**mandatory**), 7.

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
2. 9-part monolith and a 5 000-part Atlas site both animate at ≥ 60 fps.
3. `buildStep`-ordered stagger visibly differs from index-ordered stagger
   on a scene that has real `0 STEP` lines (use a real official model).

**Verification ladder.** 1, 2, 3, 5 (**mandatory**), 6, 7.

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

**Verification ladder.** 1, 2, 5 (**mandatory**), 7.

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

**Verification ladder.** 1, 2, 3, 5 (**mandatory**), 6, 7.

---

## 5. Phase 3 — the fugue, at runtime (M67–M71)

The piece is named for a double meaning — *Fuge* as the joint between two
stones, *Fuge* as the contrapuntal form. Playing a recording would be a
missed opportunity and a licensing question; generating the counterpoint
from the same seed that generates the visuals makes the double meaning
literal. That is the choice made here, and §11 records the alternative.

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

## 6. Phase 4 — the construction kit, the Atlas, and the flags (M72–M76)

### M72 — `spex-build`: parametric, grid-legal brick construction

**Why.** Forty World Heritage sites cannot be hand-authored placement line
by placement line, and should not be: the *point* of the work's thesis is
that the module composes. A kit of parametric primitives that emit real
LDraw placements of real parts on the real grid is both the honest
implementation of the thesis and the only tractable one.

**Files.** `crates/spex-build/` (new crate), `spec/recipe.schema.json`.

**The grid.** Everything is expressed in real LDraw units and validated
against them: stud pitch 20 LDU (8.0 mm), plate height 8 LDU (3.2 mm),
brick height 24 LDU (9.6 mm). A "grid-legal" placement has translation
components that are integer multiples of 10 LDU in X/Z (half-stud, to
permit real jumper-plate offsets) and integer multiples of 8 LDU in Y, and
a rotation matrix that is one of the 24 real axis-aligned orientations —
unless the primitive explicitly declares itself off-grid, which is then
recorded in the recipe output as an intentional illegal connection.

**Signatures.**

```rust
// grid.rs
pub const STUD_LDU: f64 = 20.0;
pub const HALF_STUD_LDU: f64 = 10.0;
pub const PLATE_LDU: f64 = 8.0;
pub const BRICK_LDU: f64 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridPos { pub x: i32, pub y: i32, pub z: i32 } // x/z in half-studs, y in plates

impl GridPos { pub fn to_ldu(self) -> [f64; 3]; }

/// One of the real 24 axis-aligned orientations, as an index into a table
/// of real rotation matrices — never a free-form matrix, so legality is
/// decidable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Orientation(pub u8);
impl Orientation { pub fn matrix(self) -> [f64; 9]; pub const IDENTITY: Orientation; }

#[derive(Debug)]
pub enum Illegality {
    OffGridTranslation { placement_index: usize, axis: char, ldu: f64 },
    NonAxisRotation { placement_index: usize },
    Overlap { a: usize, b: usize, overlap_ldu3: f64 },
    Floating { placement_index: usize },   // no supporting part below and not the base course
}

/// Validates a whole emitted scene. Returns every real problem found, not
/// just the first — a build report, not an assertion.
pub fn validate(placements: &[Placement], footprints: &FootprintTable) -> Vec<Illegality>;

/// Real part footprints (studs W x D, height in plates) for the kit's
/// working part set, read from a committed table that cites its source.
pub struct FootprintTable(HashMap<String, Footprint>);
```

```rust
// primitives.rs — every primitive returns real Placements, nothing else.
pub struct Wall  { pub width_studs: u32, pub height_plates: u32, pub depth_studs: u32,
                   pub bond: Bond, pub color: u32, pub part_set: PartSet }
pub enum Bond { Running, Stack, EnglishCross }

pub struct Column { pub height_plates: u32, pub diameter_studs: u32, pub color: u32 }
pub struct Arch   { pub span_studs: u32, pub rise_plates: u32, pub thickness_studs: u32, pub color: u32 }
pub struct Stair  { pub run_studs: u32, pub rise_plates: u32, pub width_studs: u32, pub color: u32 }
pub struct Ziggurat { pub base_studs: u32, pub tiers: u32, pub tier_height_plates: u32, pub setback_studs: u32, pub color: u32 }
pub struct Pyramid  { pub base_studs: u32, pub color: u32, pub stepped: bool }
pub struct Dome     { pub radius_studs: u32, pub color: u32 }   // corbelled approximation, documented as such
pub struct Trilithon{ pub post_height_plates: u32, pub gap_studs: u32, pub color: u32 }
pub struct Colonnade{ pub columns: u32, pub spacing_studs: u32, pub column: Column, pub architrave: bool }
pub struct Mosaic   { pub cells: Vec<Vec<u32>>, pub tile_part: String }  // color codes per cell

pub trait Primitive {
    /// Emits real placements at the given grid origin/orientation.
    fn emit(&self, origin: GridPos, orientation: Orientation) -> Vec<Placement>;
    /// Bounding footprint in studs/plates, for composition without emitting.
    fn extent(&self) -> (u32, u32, u32);
}
```

**Recipes.** A site is a small JSON document composing primitives:

```jsonc
{
  "version": 1,
  "id": "stonehenge",
  "title": "Stonehenge",
  "scale": { "studsPerMetre": 0.5, "note": "stated, not implied — every recipe declares its own scale" },
  "palette": { "stone": 72, "grass": 288 },
  "steps": [
    { "primitive": "Trilithon", "count": 5, "arrangeOn": { "kind": "arc", "radiusStuds": 18, "startDeg": 200, "endDeg": 340 },
      "params": { "postHeightPlates": 21, "gapStuds": 3, "color": "stone" } },
    { "primitive": "Column", "count": 30, "arrangeOn": { "kind": "circle", "radiusStuds": 30 },
      "params": { "heightPlates": 13, "diameterStuds": 1, "color": "stone" } }
  ]
}
```

**Acceptance criteria.**

1. `spex build <recipe.json> -o <out.ldr>` writes a real `.ldr` that
   `spex-ldraw`'s existing `parse_scene` reads back without error, and that
   `spex mesh-model` renders.
2. `validate` reports zero `Illegality` for every shipped recipe, or the
   recipe explicitly declares and justifies each exception in a
   `"knownIllegal"` array — and the CLI prints them.
3. `Wall{width: 20, height: 9, bond: Running}` produces the real running
   bond: alternating courses offset by exactly one half-brick, verified in a
   unit test against explicit expected translations.
4. Every primitive has a unit test asserting real part counts and real
   overall extents.
5. Emitted `.ldr` files carry a `0 Author:` line stating they are
   machine-generated by `spex-build` from a named recipe, and a `0 !SPEX`
   comment with the recipe's own hash — provenance travels with the file,
   the same discipline `spex-brick-mesh` used.

**Verification ladder.** 1, 2, 3, 4 (render it), 7.

---

### M73 — `spex-heritage`: the real World Heritage index

**Why.** The Atlas movement needs real sites with real metadata: name,
state party, year of inscription, criteria, coordinates.

**A real licensing finding that shapes this milestone.** The World Heritage
Centre's own syndication terms state that *"any republication, online or in
any other form, of any UNESCO/WHC data requires prior written
authorization"*, that content may not be modified, and that a specific
copyright notice must accompany any use. That is incompatible with
publishing a generative artwork that displays site names and descriptions —
**unless** authorisation is obtained. Separately, the **World Heritage
Emblem is a protected symbol** with its own use guidelines; it must not
appear in the work without authorisation.

**Therefore, the implementation rule for this milestone:**

- **Displayed metadata comes from Wikidata** (CC0), queried via its public
  SPARQL endpoint for items with heritage designation "World Heritage Site":
  label, state party, inscription year, coordinates, criteria, and the
  Commons category. CC0 is redistributable without conditions, which is what
  a published artwork needs.
- **The WHC list may be used as a cross-check** during development (does
  Wikidata's set match the official list's count for a given year?), and
  the result of that check is recorded as a number in `TODOs.md` — but no
  WHC text is redistributed.
- **No UNESCO emblem, no World Heritage Emblem, no UNESCO wordmark** in any
  rendered frame or exported asset. The work may state, in plain prose, that
  a site *is* a World Heritage Site — a fact, not a mark.
- A `docs/LICENSING.md` (M84) records all of this with links, and Stefan is
  asked once, explicitly, whether he wants to pursue WHC authorisation for
  the installation cut. **Do not decide this unilaterally.**

**Files.** `crates/spex-heritage/src/list.rs`, `curation.rs`,
`scripts/heritage-data/wikidata-whs-<date>.json` (a committed real snapshot,
per `docs/agents/working-mode.md`'s committed-snapshot pattern),
`spec/heritage.schema.json`.

**Signatures.**

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeritageSite {
    pub id: String,               // wikidata QID
    pub name: String,
    pub state_parties: Vec<String>,   // ISO 3166-1 alpha-2
    pub inscribed_year: u32,
    pub criteria: Vec<String>,        // "i".."x"
    pub category: Category,           // Cultural | Natural | Mixed
    pub lat: f64,
    pub lon: f64,
    pub source: String,               // "wikidata:Q..." — provenance in the record
}

/// The buildability filter. Documented, deterministic, and reviewable —
/// not a vibe. A site qualifies for the Atlas when ALL hold:
///   1. category is Cultural or Mixed (a natural landscape is not a
///      Klemmbaustein subject and pretending otherwise would be dishonest);
///   2. its dominant structure is architectural and composed of discrete,
///      repeated modules — encoded as a hand-curated `buildable: bool` per
///      site with a one-line written justification, NOT inferred;
///   3. it is not on the exclusion list.
pub fn is_buildable(site: &HeritageSite, curation: &Curation) -> bool;

/// The exclusion list is an ethical constraint, not a technical one.
/// Sites of atrocity, genocide, slavery, and mass death are NOT rendered
/// as toy bricks — among them Auschwitz Birkenau, the Hiroshima Peace
/// Memorial, the Island of Gorée, Robben Island, and the Bikini Atoll
/// nuclear test site. Active places of worship are excluded by default and
/// may only be included with an explicit, recorded decision. This list is
/// data, it is reviewed by a human before every release, and the code
/// fails closed: an unclassified site is excluded.
pub struct Curation { /* … */ }
pub fn load_curation(path: &Path) -> Result<Curation>;
```

**CLI.** `spex heritage-index -o scripts/heritage-data/…json` (the live
fetch tool, run rarely) and `spex heritage-list [--buildable]` (reads the
committed snapshot, prints the working set) — exactly the
`gen_wikipedia_crawl.py` / `gen_wikipedia_demo.py` split this repo already
established.

**Acceptance criteria.**

1. The committed snapshot contains ≥ 900 real sites with complete required
   fields; the count is recorded in `TODOs.md` alongside the WHC's own
   published total for the same date, with the delta explained.
2. The curated buildable set contains ≥ 40 sites, each with a written
   justification, and the exclusion list is non-empty and reviewed.
3. `spex heritage-list --buildable` prints a table a human can read.
4. No WHC-sourced text appears in any committed file.

**Verification ladder.** 1, 2, 3, plus an explicit human review of the
curation and exclusion lists before the milestone closes.

---

### M74 — the Atlas site models

**Why.** Twelve sites for the 10-minute cut, forty for the installation cut.

**Files.** `recipes/heritage/<site>.json`, `ldraw-scenes/heritage/<site>.ldr`
(generated, committed — they are small text files and they are the work).

**Tier structure** (each tier is additive; ship tier by tier, never all at
once):

- **Tier A — 3 sites, needed for the 4:00 cut.** Chosen because they are
  the three Postilla addressee states: **Stonehenge** (GB), **Grand-Place,
  Brussels** (BE), **Jelling Mounds, Runic Stones and Church** (DK).
- **Tier B — 9 more, for the 10:00 cut.** Great Wall (CN), Pyramids of Giza
  (EG), Colosseum / Historic Centre of Rome (IT), Acropolis of Athens (GR),
  Taj Mahal (IN), Machu Picchu (PE), Borobudur (ID), Cologne Cathedral (DE),
  Pont du Gard (FR).
- **Tier C — 28 more, for the 60:00 cut.** Curated in M73; candidates
  include Petra, Angkor, Chichén Itzá, Persepolis, Mesa Verde, Bauhaus
  Dessau, Zollverein, Speicherstadt Hamburg, Kronborg Castle, Bruges
  Belfry, Wieliczka Salt Mine, Alhambra, Mont-Saint-Michel, Sydney Opera
  House, Brasília, Rapa Nui, Timbuktu, Bagan, Himeji Castle, Vatican City's
  colonnade, Segovia Aqueduct, Ironbridge Gorge, Völklingen Ironworks,
  Sigiriya, Meroë, Great Zimbabwe, Fujian Tulou, Derbent.

Each recipe declares its own `studsPerMetre` scale and its own part palette
in real LDraw colour codes. **No site is rendered "photorealistically"** —
each is a *brick abstraction* of its dominant module, and the recipe's
`title`/`note` says which module it is abstracting. That is honest, it is
cheaper, and it is the thesis.

**Acceptance criteria.**

1. Each tier's recipes build, validate grid-legal, and render.
2. Per-site brick counts recorded; no site exceeds 8 000 placements
   (a budget, so the 40-site Atlas stays under ~250 000 instances).
3. A contact-sheet render of every site (one headless screenshot each,
   assembled into a single image) is produced and *looked at by a human*
   at the end of each tier.

**Verification ladder.** 1, 2, 3, 4, 5 (**mandatory**), 6, 7.

---

### M75 — `spex-flag`: flags as real brick mosaics

**Why.** The Atlas needs each site's state party's flag, and the piece's
diplomatic dimension (the Postillen to Belgium, the United Kingdom, Denmark
and the LEGO Group) makes flags load-bearing rather than decorative. A flag
built from 1×1 tiles in real LDraw colours is the same argument as the rest
of the work: the standardised module renders the world.

**Files.** `crates/spex-flag/`, `flags/<iso2>.json`, `spec/flag.schema.json`.

**The flag specification format.** Each flag is a *declarative construction
sheet* transcribed from its own state's published specification — not a
traced bitmap. This keeps the real-data rule intact and sidesteps image
licensing entirely.

```jsonc
{
  "version": 1,
  "iso2": "DK",
  "name": "Dannebrog",
  "ratio": [37, 28],
  "source": "Danish flag proportions as published in the Danish state's own specification — cite the exact document at implementation time",
  "colors": { "red": { "srgb": [198, 12, 48] }, "white": { "srgb": [255, 255, 255] } },
  "field": "red",
  "elements": [
    { "kind": "cross", "color": "white",
      "armWidthFraction": 0.2143,
      "verticalArmOffsetFraction": 0.4286 }
  ]
}
```

Supported element kinds: `stripesHorizontal`, `stripesVertical`, `cross`,
`saltire`, `canton`, `disc`, `rect`, `triangle`, `star` (n-pointed, by real
construction geometry), `crescent`, and `overlay` (a nested element list, for
the Union Flag's saltire-over-cross-over-field construction). Anything a
supported element cannot express is **not approximated silently** — the
flag is marked `"unsupported": true`, excluded from the Atlas, and listed in
the milestone note. Honest gaps over fake flags.

**Signatures.**

```rust
pub struct FlagSpec { /* mirrors the JSON */ }

/// Rasterises the declarative spec at mosaic resolution. Deterministic,
/// analytic (no image library, no anti-aliasing — a stud is a stud):
/// a cell takes the colour of whichever element covers its centre.
pub fn rasterize(spec: &FlagSpec, width_studs: u32) -> Vec<Vec<[u8; 3]>>;

/// Maps each sRGB cell to the nearest REAL LDraw colour, by CIEDE2000
/// distance in CIELAB, restricted to a permitted palette (opaque, solid
/// finish, currently-produced colours). Returns the mapping plus the real
/// worst-case ΔE, so the milestone can report how faithful it actually is.
pub fn quantize(cells: &[Vec<[u8; 3]>], colors: &ColorTable, palette: &[u32])
    -> (Vec<Vec<u32>>, QuantizeReport);

pub struct QuantizeReport { pub max_delta_e: f64, pub mean_delta_e: f64, pub used_colors: Vec<u32> }

/// Emits a real .ldr: a plate substrate plus one 1x1 tile per cell, on the
/// real grid, plus an optional pole and halyard built from real parts.
pub fn emit_flag_ldr(spec: &FlagSpec, cells: &[Vec<u32>], opts: &FlagBuildOptions) -> String;
```

**CLI.** `spex flag <iso2> [--width-studs 48] [--pole] -o <out.ldr>`

**Acceptance criteria.**

1. `spex flag DK --width-studs 48` produces a cross whose arm width and
   offset, measured in studs, match the published fractions to within one
   stud — asserted numerically in a test, not eyeballed.
2. `spex flag GB` reproduces the Union Flag's real asymmetric saltire
   (the broad white diagonal above the red on the hoist side, reversed on
   the fly) — the single hardest real construction detail, and the one that
   proves the element system is real rather than approximate. If it cannot
   be expressed, mark it unsupported and say so.
3. `QuantizeReport.max_delta_e` is recorded for every shipped flag. Any
   flag whose max ΔE exceeds 12 is flagged for review rather than shipped
   silently.
4. Every flag used in the Atlas has a cited source for its construction
   specification, in `flags/<iso2>.json`'s `source` field.

**Verification ladder.** 1, 2, 3, 4, 5 (**mandatory** — a contact sheet of
every flag, reviewed by a human), 7.

---

### M76 — the wave: flags that actually fly

**Why.** A flat flag mosaic lying in the scene is a chart. A flag *flying*
is cinema, and it costs one instanced vertex shader.

**Files.** `viewer/src/flag/wave.ts`, `viewer/src/mesh/instanced.ts`
(per-instance wave attributes).

**The model.** Each 1×1 tile is a rigid body; the mosaic is a grid of them.
Per instance, given its grid coordinate `(u, v)` normalised to `[0,1]` with
`u = 0` at the hoist:

```
amplitude(u)   = A * u^1.5                      // pinned at the hoist, free at the fly
phase(u, v, t) = k1*u + k2*v - omega*t
offset.z       = amplitude(u) * sin(phase)
offset.y       = amplitude(u) * 0.35 * sin(phase * 0.5 + 1.7)
yaw            = amplitude(u) * k1 * cos(phase)   // tiles turn with the surface, so lighting reads
```

Two summed waves at incommensurate frequencies (so the motion never visibly
repeats), all parameters timeline-animatable (`windStrength`, `windSpeed`),
computed **in the vertex shader** from a per-instance `(u, v)` attribute —
no CPU-side per-tile transform, so a 48×36 flag is 1 728 instances at zero
per-frame CPU cost.

**Acceptance criteria.**

1. A 48×36 Dannebrog flies at ≥ 60 fps with 12 flags on screen
   simultaneously.
2. Lighting responds to the wave (the surface visibly catches the key light
   as it turns) — verified in a 60-frame capture, not asserted.
3. `windStrength = 0` produces a perfectly flat flag, bit-identical to the
   static mosaic.
4. No tile ever separates from its neighbours by more than one stud width
   (the mosaic must read as cloth, not as confetti) — measured.

**Verification ladder.** 1, 2, 5 (**mandatory**), 7.

---

### M77 — Atlas autopilot: the XML-driven site pipeline *(the "perfect case")*

**Why.** M73/M74 curate by hand, which is right for the first forty sites
and wrong forever. The intended end state is that the tool **fetches the
built structures itself from the real World Heritage List XML feed** and
builds them — so that the Atlas is not a fixed list but a living index that
grows when the List grows. This milestone is deliberately placed *after*
the hand-curated tiers exist, because the automatic path needs the manual
one as its ground truth to be measured against.

**The real feed.** `https://whc.unesco.org/en/list/xml/` (also offered as
RSS, XLS/XLSX, KML and GeoRSS from `https://whc.unesco.org/en/syndication/`)
— per-site `id_number`, `site` name, `states`, `category`, `criteria_txt`,
`date_inscribed`, `latitude`/`longitude`, `short_description`.

**The licensing gate, restated because it governs this milestone.** WHC's
own syndication terms require prior written authorisation for republication
of their data and forbid modification. So:

- the XML may drive **selection, classification, and geometry generation**
  inside the pipeline;
- **no WHC-authored text is displayed or committed** — on-screen labels
  keep coming from CC0 Wikidata (M73);
- the fetched feed lives in the gitignored cache, never in the repo;
- if Stefan obtains authorisation, a single flag flips and WHC text becomes
  usable. Build for that switch; do not assume it.

**Files.** `crates/spex-heritage/src/xml.rs`, `archetype.rs`, `massing.rs`;
`crates/spex-cli/src/heritage.rs`.

**Signatures.**

```rust
// xml.rs — real feed parsing, no interpretation.
pub struct WhcRecord {
    pub id_number: u32,
    pub name: String,
    pub states: Vec<String>,
    pub category: Category,
    pub criteria: Vec<String>,
    pub date_inscribed: u32,
    pub lat: f64,
    pub lon: f64,
}
pub fn parse_list_xml(text: &str) -> Result<Vec<WhcRecord>>;
pub fn fetch_list_xml(cache: &LdrawCache /* or a sibling HttpCache */) -> Result<String>;

// archetype.rs — deterministic, reviewable classification into a buildable
// structural archetype. Rule table + real Wikidata P31 ("instance of")
// values; never a language model, never a heuristic on the name alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Archetype {
    MegalithCircle, SteppedPyramid, SmoothPyramid, ColonnadeTemple,
    Cathedral, WalledTown, Fortress, Aqueduct, Ziggurat, StupaTerrace,
    IndustrialHall, ModernistSlab, PalaceCourtyard, Amphitheatre,
    Unclassified,
}
pub fn classify(record: &WhcRecord, wikidata: Option<&WikidataFacts>) -> (Archetype, Confidence);

// massing.rs — the geometry half. Real footprints where obtainable.
/// Queries the real OpenStreetMap Overpass API for building footprints
/// within `radius_m` of the site's real coordinates, simplifies each to a
/// grid polygon on the real stud grid, and extrudes it in real bricks at
/// the archetype's own course pattern. ODbL: attribution required, and
/// any redistributed derived database must carry the licence — recorded in
/// docs/LICENSING.md and in the emitted .ldr's own header comment.
pub fn massing_from_osm(lat: f64, lon: f64, radius_m: f64, opts: &MassingOptions)
    -> Result<Option<Vec<GridPolygon>>>;

/// Falls back to the archetype's own parametric template when no usable
/// footprint exists (the common case for ruins and landscapes).
pub fn recipe_for(record: &WhcRecord, archetype: Archetype, massing: Option<&[GridPolygon]>)
    -> Result<Recipe>;
```

**CLI.**

```
spex heritage-sync            [--xml-url <url>] [--out scripts/heritage-data/whc-index-<date>.json]
spex heritage-autobuild <id>  [--osm] -o recipes/heritage/<slug>.json
spex heritage-autobuild --all --buildable --provisional-out recipes/heritage/provisional/
```

**Non-negotiable safety properties.**

1. **Fails closed.** An `Unclassified` archetype, a low-confidence
   classification, or an unclassified exclusion status ⇒ the site is *not*
   added to the Atlas. Silence is exclusion, never inclusion.
2. **Everything auto-generated is `"provisional": true`** and lands in
   `recipes/heritage/provisional/`. A human moves it to
   `recipes/heritage/` after review. The Atlas only ever loads reviewed
   recipes. Automation proposes; a person admits.
3. **The exclusion list (M73) is applied before classification**, not after.

**Acceptance criteria.**

1. `spex heritage-sync` parses the real live feed and reports a site count;
   the count is compared against the WHC's own published total and the
   delta explained in `TODOs.md`.
2. Re-running against the cached feed is deterministic and offline.
3. Classification is measured against the hand-curated tiers A+B (12 real
   sites) as ground truth: report the real accuracy figure. Target ≥ 9/12;
   below that, the rule table is wrong and gets fixed before the milestone
   closes — do not ship a classifier you have not scored.
4. `heritage-autobuild --osm` produces a grid-legal, renderable recipe for
   at least three real sites with usable OSM footprints (Cologne Cathedral,
   Grand-Place, and one industrial site are good candidates), and each
   emitted `.ldr` carries its ODbL attribution in its own header.
5. Every provisional recipe renders without error; a contact sheet is
   produced and reviewed by a human before any of them is promoted.

**Verification ladder.** 1, 2, 3, 4, 5 (**mandatory**), plus human review of
every promotion.

---

## 7. Phase 5 — the work itself (M78–M85)

By this point the engine exists. Phase 5 is authorship: `show.json` grows
movement by movement, and each milestone is "this act now plays end to end,
in time, with sound".

| M | Deliverable | Gate |
|---|---|---|
| **M78** | **Act I — Archäologie der Fuge** plays end to end | §8's A1 shot list realised; monolith height 73.6 mm on screen; the fugue's first two entries land on their scored bars |
| **M79** | **Act II — Der Core Standard** | the bulla→coin metamorphosis reads as one continuous transformation, not a cut; 4-voice exposition completes exactly at the act boundary |
| **M80** | **Act III — Die Fuge** | the three patent bricks (1923 / 1939 / 1949) render from real geometry, the clutch-power macro shot is legible, stretto begins on cue |
| **M81** | **The Atlas movement** | tier-A sites + flags + chronicle cards, with the site→flag→card choreography working generically for any site, so tiers B and C are data, not code |
| **M82** | **Act IV — Der Token, and Der Kick** | mesh→point dissolve, the neon grid, and an 800 ms Kick that is frame-exact against its audio accent |
| **M83** | **The four cuts** | 4:00, 10:00, 60:00 and endless all resolve, run, and loop seamlessly (§8.4) |
| **M84** | **Titles, credits, attribution, `docs/LICENSING.md`** | LDraw CCAL attribution, Wikidata CC0, OSM ODbL, the no-UNESCO-emblem rule, the no-brand-name rule, all stated in the work and in the repo |
| **M85** | **Seeded editions** | `?seed=` visibly and audibly varies the piece within authored bounds, deterministically — the on-chain-edition prerequisite |

Each of M78–M82 follows the same internal shape and the same verification
ladder (1, 2, 3, 5 **mandatory**, 6, 7):

1. Author the act's shots in `show.json` from §8's shot list.
2. Build or generate every scene it needs.
3. Realise the camera track.
4. Bind the act's audio cues.
5. Headless-capture the act at 2 fps for a contact sheet and at 60 fps for
   the two or three moments that must be exact.
6. **Watch it.** A human watches the act, in a browser, with sound, before
   the milestone closes. Record what they said in `TODOs.md`. This is not
   ceremony: every visual decision in this document is a guess until
   somebody looks at it.

### M85 in detail — seeded editions

What the seed is allowed to vary (authored bounds, never structure):

- the Atlas's site *selection and order* (from the reviewed buildable set);
- the fugue's episode material and the exact voicing of entries — never the
  subject, never the section plan;
- brick colour palette within a per-act permitted set of real LDraw codes;
- camera orbit start angles and the scatter seed of every assembly;
- the wind seed for the flags.

What the seed must never vary: act order, act durations, the Kick, the loop
seam, the subject, or any factual content.

**Acceptance criteria.** Two different seeds produce visibly and audibly
different runs; the same seed twice produces frame-identical runs (verified
by hashing 20 sampled frames); every seed in [0, 999] resolves without error
(run all thousand headlessly at 1 fps, assert no exceptions).

---

## 8. THE SCREENPLAY — *Die Geschichtliche Matrix*

> *"Wir bauen, um zu verstehen. Wir archivieren, um zu bewahren. Wir fügen,
> um zu sein."*

The canonical cut is **4:00.000** — 240.000 seconds, 84 bpm, 4/4, Dorian on
D, 336 bars. All timecodes below are for the canonical cut. §8.4 defines how
they scale.

**The palette** (real LDraw colour codes where bricks are involved, sRGB
where light is):

| Name | Value | Where |
|---|---|---|
| Terrakotta | LDraw 70 *Reddish Brown* family / `#B5704E` | Mesopotamian brick, the clay bulla, Act II |
| Elektrum | LDraw 297 *Pearl Gold* / `#D4AF37` | the Lydian coin, Act II |
| Terminalgrün | `#00E633` | the token grid, Act IV, and the final pixel |
| Schwarz | LDraw 0 *Black* `#1B2A34` | the monolith, Act I |
| Patentweiß | `#F2EFE6` | the studio of Act III, patent-drawing overlays |
| Steingrau | LDraw 72 *Dark Bluish Gray* | Stonehenge and the Atlas's stone sites |

**The three recurring rules of the piece:**

1. **Every object is made of real modules.** Nothing in frame is a mesh that
   is not a real LDraw part, except light, text, and the final pixel.
2. **Every cut lands on a bar line.** The resolver enforces this (M61 step 6).
3. **The piece begins and ends on the same single green pixel.** The loop is
   the argument.

---

### 8.1 ACT I — ARCHÄOLOGIE DER FUGE (0:00.000 – 1:00.000)

*The individual natural object. Counting has not been invented yet.*

| Shot | In–Out | Dur | Scaling | Tier | Content |
|---|---|---|---|---|---|
| **A1-S01** | 0:00.000–0:06.000 | 6.0 s | fixed | 1 | **Black.** One point, Terminalgrün, exactly at frame centre, 1 device pixel, on a black field. It pulses once, slowly. Camera static. Audio: silence, then a 55 Hz sine fades up from −∞ over 4 s. *This frame is byte-identical to the last frame of the piece.* |
| **A1-S02** | 0:06.000–0:14.000 | 8.0 s | stretch (4–20 s) | 1 | The point **swells into a point swarm** — the existing point-cloud renderer, ~3 000 points, sampled from `3005.dat`'s real surface. It has no shape yet; it is a cloud. Camera pushes in slowly. Audio: the sine gains partials 2 and 3. |
| **A1-S03** | 0:14.000–0:24.000 | 10.0 s | stretch (6–30 s) | 1 | **The crossfade** (M65): the swarm collapses onto the real mesh of a single 1×1 brick, Black. Edge lines fade in last, and the moment they land is the moment the object becomes *legible*. The brick makes one full revolution, constant angular velocity (port `build_spin_frames`'s exclusive framing so the loop has no duplicate frame). **Bar 1, beat 1 of the fugue: voice 1 (alto) enters with the subject, solo, on the frame the edges appear.** |
| **A1-S04** | 0:24.000–0:38.000 | 14.0 s | stretch (8–40 s) | 1 | **The assembly.** Nine real parts (7× `3010.dat` Brick 1×4 + 2× `3710.dat` Plate 1×4, all Black) fly in from the scattered start (`FLOAT_HEIGHT_LDU` 420, `SCATTER_RADIUS_LDU` 260, per-placement splitmix seed — the exact existing constants), eased `cubicInOut`, staggered by real build step, and settle into `ldraw-scenes/monolith.ldr`. Each landing is a tile-click accent. **Voice 2 (soprano) enters with the tonal answer on the dominant as the first part lands.** |
| **A1-S05** | 0:38.000–0:50.000 | 12.0 s | stretch (6–90 s) | 1 | **The monolith stands.** Camera orbits 180° from low, the object filling 80% of frame height. A thin HUD line, hairline type, lower right: `1 : 4 : 9.20 — 73.6 mm — 9 real parts`. Key light rakes across the studs so the module count is *countable*. **Voice 3 (tenor) enters.** |
| **A1-S06** | 0:50.000–1:00.000 | 10.0 s | stretch (6–120 s) | 1 | **Stonehenge rises** out of the dark behind the monolith — `heritage/stonehenge.ldr`, Steingrau, materialising (M65) from the ground up, at a scale that reveals the monolith was small all along. Camera dollies back. The last 2 s: both objects lit only by a low rim. **Voice 4 (bass) enters; the exposition is now complete in four voices.** Cut on the downbeat of 1:00.000. |

**Director's note for Act I.** Nothing here is fast. The act's job is to
establish that a module is an object before it is a system. If a viewer is
bored at 0:45, the act is working; if they are bored at 0:20, S02 is too long
and should be re-weighted, not re-cut.

---

### 8.2 ACT II — DER CORE STANDARD (1:00.000 – 2:00.000)

*The birth of the module. Counting is invented. Value becomes portable.*

| Shot | In–Out | Dur | Scaling | Tier | Content |
|---|---|---|---|---|---|
| **A2-S01** | 1:00–1:10 | 10.0 s | stretch (5–60 s) | 1 | **Mesopotamia, ~3500 BCE.** A wall builds itself: `spex-build`'s `Wall{bond: Running}` in Terrakotta, course by course, each course landing on a beat, 12 courses in 12 bars' worth of accents. The camera holds absolutely still — the first static shot of the piece, so the *construction* moves and nothing else does. |
| **A2-S02** | 1:10–1:22 | 12.0 s | stretch (6–70 s) | 1 | **One brick leaves the wall.** A single brick lifts out, leaving a legible gap, and floats to camera. It rotates; as it rotates it *becomes* a clay bulla — a corbelled `Dome` of Terrakotta bricks growing around it while the source brick dissolves (M65). Not a morph: a rebuild. The module becomes a container. |
| **A2-S03** | 1:22–1:36 | 14.0 s | stretch (7–90 s) | 1 | **The bulla breaks.** It splits along a real seam and small tokens — 1×1 round plates, Terrakotta — spill out and arrange themselves into a counting row. A HUD numeral counts them, 1…24, one per beat. This is the invention of number, and it is deliberately the most literal image in the piece. |
| **A2-S04** | 1:36–1:50 | 14.0 s | stretch (7–90 s) | 1 | **Lydia, ~600 BCE.** The tokens compress into a single cylinder — 2×2 round bricks + a tile, in Elektrum (real Pearl Gold, real metallic finish from M56). A die descends and **strikes on the beat**: one hard accent, one bloom flash, one frame of white. The coin is struck. |
| **A2-S05** | 1:50–2:00 | 10.0 s | stretch (5–60 s) | 1 | **The first fungible unit.** Macro on the coin's face, rotating. Chrome reflections from the procedural environment. In the surface, at the edge of legibility, the piece's first hidden inscription (§8.6). The fugue reaches its first full cadence exactly at 2:00.000. |

---

### 8.3 ACT III — DIE FUGE (2:00.000 – 3:00.000)

*Mass production, and the line of ownership. The joint becomes an industry.*

| Shot | In–Out | Dur | Scaling | Tier | Content |
|---|---|---|---|---|---|
| **A3-S01** | 2:00–2:12 | 12.0 s | stretch (6–70 s) | 1 | **Rome, ~100 CE.** An `Arch` assembles over a `Colonnade` — the Pont du Gard's module, Steingrau. The keystone drops last, on a bar line, and the whole structure settles by one plate as it takes the load. (Authored, not simulated — but authored to look like load.) |
| **A3-S02** | 2:12–2:26 | 14.0 s | stretch (7–80 s) | 1 | **Hard cut to Patentweiß.** A white studio, no horizon. One brick rotates at centre: the **Batima** system, 1923 — the earliest documented stud-and-socket building block. A patent-drawing overlay (line art derived from the real published patent drawing, credited) fades in beside it, then out. Lower-third: `BE 311029 · 1923`. |
| **A3-S03** | 2:26–2:38 | 12.0 s | stretch (6–80 s) | 1 | **Kiddicraft, 1939.** A second brick materialises beside the first, same lighting, same framing. The two align. A cutaway reveals the hollow underside — the anti-stud. Lower-third: `GB 529580 · 1939`. |
| **A3-S04** | 2:38–2:50 | 12.0 s | stretch (6–80 s) | 1 | **1949, and the clutch.** A third brick joins; the three snap into one column. **Macro shot:** a stud entering a tube, real LDraw geometry, real tolerance, filling the frame. This is the title image of the whole work — *die Fuge*, the joint, at the scale where it is an engineering fact. Lower-third: `Automatic Binding Bricks · 1949`. |
| **A3-S05** | 2:50–3:00 | 10.0 s | stretch (5–60 s) | 1 | **Multiplication.** The column becomes a grid becomes a field — instanced bricks filling the volume, thousands of them, marching outward. Lower-third, held 3 s: `Interlego AG v Tyco Industries Inc · [1988] UKPC 3`. **The stretto begins:** subject entries overlap at half their own length, and the visual multiplication and the contrapuntal multiplication are the same event. |

---

### 8.4 ATLAS — DER ATLAS DER FUGE (from 3:00.000)

*The module, applied to the whole world. The longest and most scalable movement — this is where the 10:00 and 60:00 cuts live.*

The Atlas is a **repeating unit**, authored once and instantiated N times:

| Sub-shot | Duration (unit) | Content |
|---|---|---|
| **ATL-a** | 0.9 s | The world plate: a low, dark field of 1×1 plates. The camera arrives at a site's real coordinates (mapped to the plate by an equirectangular projection, stated as such). |
| **ATL-b** | 1.1 s | **The site builds itself** from its recipe, bottom-up, staggered by build step, in its own real palette. |
| **ATL-c** | 0.4 s | **The flag unfurls** beside it: the state party's brick mosaic (M75) rises on its pole and catches the wind (M76). |
| **ATL-d** | 0.2 s | **The chronicle card**: site name, state party, year of inscription, criteria — from CC0 Wikidata, in the archive's own typography. |
| **ATL-e** | *hold* | The camera arcs once around the pair. Duration is the unit's stretchable remainder. |

Unit total in the canonical cut: **2.6 s**. `scaling: 'repeat'`,
`unitSec: 2.6`, `minCount: 3`, `maxCount: 40`.

| Cut | Atlas length | Sites | Unit length |
|---|---|---|---|
| 4:00 | 7.8 s | 3 (GB, BE, DK — the Postilla states) | 2.6 s |
| 10:00 | 2:24 | 12 (tiers A+B) | 12.0 s |
| 60:00 | 30:00 | 40 (tiers A+B+C) | 45.0 s |
| endless | 7.8 s per cycle | 3, **rotating with the cycle seed** | 2.6 s |

In the 60:00 cut the unit expands rather than merely slowing: at
`unitSec ≥ 20` the shot enables its tier-3 sub-beats — a real construction
*sequence* (foundation, walls, roof, detail), a slow flag close-up, and a
second chronicle card carrying the site's own historical module (which
brick, which bond, which unit of measure). Those sub-beats are authored once
and enabled by duration, not written per site.

---

### 8.5 ACT IV — DER TOKEN, AND DER KICK (3:07.800 – 4:00.000)

*Digital immortality. The module stops being physical and does not stop being a module.*

| Shot | In–Out | Dur | Scaling | Tier | Content |
|---|---|---|---|---|---|
| **A4-S01** | 3:07.800–3:22 | 14.2 s | stretch (8–120 s) | 1 | **Der Inkpour.** Everything on screen — sites, flags, the monolith held in the background all along — dissolves into points (M65, mesh→point crossfade), each brick becoming its own swarm, drifting outward along its own normals. The Terrakotta and Steingrau drain out of the palette. Audio: the pulse enters underneath, half-time. |
| **A4-S02** | 3:22–3:38 | 16.0 s | stretch (8–150 s) | 1 | **The grid forms.** The points reorganise into a regular lattice: Terminalgrün nodes, connected by the *existing* line-edge renderer, activations propagating along the edges. It is a neural network and it is also a stud grid, and the shot's entire job is to make those the same picture. |
| **A4-S03** | 3:38–3:52 | 14.0 s | stretch (8–150 s) | 1 | **Tokens.** Each node emits a small glyph that travels an edge and is absorbed. The chronicle line runs beneath: `2012 · Colored Coins` → `2017 · Attention Is All You Need` → `2026 · Fugen Engine`. The pulse doubles. The fugue's subject is now in the bass, quantised to sixteenths — the same intervals, a different century. |
| **A4-S04** | 3:52–3:59.200 | 7.2 s | stretch (4–60 s) | 1 | **Saturation.** The lattice fills the frame and keeps growing past it; bloom rises; the camera stops moving. Everything is Terminalgrün on black. The last four bars are a pedal point in the bass under the full four voices. |
| **DER KICK** | **3:59.200–4:00.000** | **0.800 s** | **fixed** | 1 | On the final accent — **one event, both meanings of the word** — the camera zooms out exponentially by 10⁴. The entire network collapses toward the centre. Bloom collapses with it. At 4:00.000 exactly, what remains is **one Terminalgrün pixel at frame centre** on black: the same pixel A1-S01 opened on. Audio cuts to the same 55 Hz sine, and the loop closes. |

**The Kick is `scaling: 'fixed'` in every cut, at every duration, forever.**
It is 800 ms in the 4-minute cut and 800 ms in the 60-minute cut. A
resolver that stretches it is broken.

---

### 8.6 The hidden inscriptions

Three, all real, none announced:

1. **`GB 587,206`** — Hilary Fisher Page's real 1947 patent — written into
   the `mesh.json` instance metadata of every brick-shaped object in the
   work, and legible in the coin's struck face in A2-S05 at full zoom.
   The stated reason, per the Postilla to the LEGO Group: it is a quiet
   monument to the man without whom none of this exists.
2. **`IA-2026-002`** — the archive signature, in the world plate's own
   tile pattern in the Atlas, readable only from directly above.
3. **The seed** — rendered in the credits (M84), so any frame anyone ever
   screenshots can be traced back to the exact run that produced it.

---

### 8.7 The four cuts, and how they are the same document

| Cut | `--duration` | Tiers | Atlas | Purpose |
|---|---|---|---|---|
| **Der Schnitt** | `240` | 1 | 3 sites | The canonical work. Festival submission, on-chain edition, the thing that is *the piece*. |
| **Die Fassung** | `600` | 1 + 2 | 12 sites | Gallery loop, lecture, presentation. Every act breathes; the Atlas becomes a movement rather than a montage. |
| **Die Installation** | `3600` | 1 + 2 + 3 | 40 sites | Museum-hour installation. Each site gets a real construction sequence. A visitor entering at any minute finds a complete image. |
| **Die Schleife** | `endless` | 1 | 3 sites, **rotating** | Screensaver / permanent installation / a browser tab left open. Cycle *n* uses seed `hash(seed, n)`; the sites, palette variation, episodes and wind all change; the structure never does. |

**The loop seam** (this is a hard requirement, not a nicety): the final
frame of any cut and the first frame of any cut must be **pixel-identical** —
black field, one Terminalgrün pixel at centre — and the audio must arrive at
and depart from the same 55 Hz sine at the same phase. The endless cut
crossfades nothing; it simply continues, because there is nothing to
crossfade. Verify by hashing frame 0 and frame N of a headless capture and
asserting equality (M83).

**Endless-mode seed advance.** `ShowClock.onLoop` fires at the seam;
`cycle` increments; every seeded generator re-derives from
`splitmix64(seed ^ cycle)`. Nothing else changes. A viewer who watches four
cycles sees four different Atlases, one piece.

---

## 9. Phase 6 — one implementation, two targets: Rust in the browser via WebAssembly (M86–M90)

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

**Verification ladder.** 1, 2, 3, 5 (**mandatory**), 6, 7.

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

**Verification ladder.** 1, 2, 5 (**mandatory**), 6, 7.

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

**Verification ladder.** 1, 2, 3, 5 (**mandatory**), 7.

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

**Verification ladder.** 1, 2, 5 (**mandatory**), plus a human listen.

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

## 10. The six-month plan (2026-07-25 → 2027-01-24)

Twenty-six weeks. Five phase gates. The gates are the schedule's load-bearing
elements: at each one, if the phase is not done, **scope comes out of Phase 6
(wasm) and Atlas tier C first** — never out of Phase 1 (the renderer) or the
Kick.

| Weeks | Dates | Phase | Milestones | Gate criterion |
|---|---|---|---|---|
| 1–4 | Jul 27 – Aug 23 | **P1 Renderer** | M51–M57 | A single 1×1 brick renders with catalogue-quality edges, in a browser, at 60 fps. If this is not true at the end of week 4, everything after it is theoretical. |
| 5–6 | Aug 24 – Sep 6 | **P1 Renderer** | M58–M59 | The 50 000-instance scene holds 60 fps with bloom and shadows. |
| 7–10 | Sep 7 – Oct 4 | **P2 Show engine** | M60–M66 | Act I plays end to end from `show.json`, silent, at all four durations. |
| 11–13 | Oct 5 – Oct 25 | **P3 Audio** | M67–M71 | Four voices, in tune, in time, generated, in the browser, in sync with the visuals. |
| 14–17 | Oct 26 – Nov 22 | **P4 Kit / Atlas / Flags** | M72–M77 | Three sites and three flags build from recipes and fly. |
| 18–21 | Nov 23 – Dec 20 | **P5 The work** | M78–M85 | The canonical 4:00 cut exists, end to end, with sound, and loops seamlessly. **This is the real deadline; everything after it is improvement.** |
| 22–24 | Dec 21 – Jan 10 | **P6 wasm** | M86–M90 | One implementation of the evaluator and the resolver, measurably faster. Cuttable. |
| 25–26 | Jan 11 – Jan 24 | **P7 Ship** | M91–M97 | Deployed, documented, licensed, reproducible. |

**Deliberate slack.** There is none in weeks 18–21 and that is a problem to
manage, not to ignore: if Phase 4 slips, cut Atlas tier C (28 sites) before
cutting anything in Phase 5. The Atlas is *designed* to be data-scalable
precisely so it can absorb schedule pressure without a rewrite.

**Parallel track (Stefan, not the agent).** Per
`claude/masterplan-iunctura-site.md`: DNS, Pinata/IPFS, the Ethereum wallet,
the physical Postillen, the LEGO Group contact, and — decided once, early —
whether to pursue WHC data authorisation (§6 M73). None of these block the
engine; all of them block the *launch*.

---

## 11. Phase 7 — shipping (M91–M97)

| M | Deliverable |
|---|---|
| **M91** | **Determinism harness.** Headless capture of 20 sampled frames + an audio render hash per (seed, duration); committed as a regression fixture. Any commit that changes a frame hash must say why in its message. |
| **M92** | **Performance and compatibility matrix.** Chrome/Firefox/Safari × macOS/Windows/Linux × High/Medium/Low. Record real fps and real load times. Safari is the one that will hurt; find out in week 25, not on launch day. |
| **M93** | **Accessibility and fallbacks.** `prefers-reduced-motion` (the Kick becomes a 2 s fade, the wave stills, the orbit slows), keyboard control (space, arrows, `m`, `f`), captions for every chronicle card, a WebGL-unavailable message that is a *statement* rather than an error, and a genuinely watchable Low tier. |
| **M94** | **Static export + deployment** to `research.iunctura.org/matrix/` (or `fugen.iunctura.de`), via the existing `export-static` discipline: relative paths throughout, works from a domain root or a subpath, no backend. |
| **M95** | **The single-file edition.** One self-contained HTML file with the wasm, the bundles and the score inlined (base64), for the on-chain edition and for the USB stick that goes to Billund with the Postilla. Size budget: ≤ 12 MB. Seed injected from the token hash at mint time — one documented entry point, `window.__SPEX_SEED__`. |
| **M96** | **Documentation sync.** `CLAUDE.md`, `AGENTS.md`, `BRICKs.md`, `README.md`, `spec/README.md`, `TODOs.md` (M51–M97 entries in the established style), `docs/LICENSING.md`, `docs/agents/wasm.md`. A reader arriving cold must be able to build the piece from the repo alone. |
| **M97** | **The archive record.** `quellenregister.json` extended with every source this phase used; IPFS pin; the Ethereum anchor; the run that produced the canonical edition recorded with its exact seed, commit hash, and frame hashes. The work is not finished until it is *provable*. |

---

## 12. Licensing, attribution, and the things we must not do

Every item here is a real constraint that was checked, not an assumption.
`docs/LICENSING.md` (M84) is the canonical version; this is the summary the
implementing session needs before it writes code.

| Source | Terms | What this means here |
|---|---|---|
| **LDraw Parts Library** | Individual part files are CCAL 2.0 (Creative Commons Attribution). | Usable and redistributable **with attribution**. Every mesh bundle carries it in `mesh.json`; the credits carry it on screen. Re-check ldraw.org's Legal Info before any redistribution. |
| **ldraw.org official `models/`** | No CCAL header; the Legal Info page does not address them the same way. `brickscene.py`'s original caveat still stands. | **Unconfirmed — treat as not-licensed.** `car.ldr`/`pyramid.ldr` stay development fixtures. Nothing in the shipped work depends on them. |
| **UNESCO / WHC List data (XML, XLS, RSS)** | *"Any republication, online or in any other form, of any UNESCO/WHC data requires prior written authorization"*; no modification; mandated copyright notice. | Usable **internally** for selection/classification/geometry (M77). **No WHC text is displayed or committed.** Ask Stefan once whether to seek authorisation; do not decide it. |
| **World Heritage Emblem / UNESCO name and logo** | Protected symbol with its own use guidelines. | **Never rendered.** The work may state that a site is a World Heritage Site — a fact, not a mark. |
| **Wikidata** | CC0. | The source for every on-screen site label, year, criteria and state party. |
| **OpenStreetMap** | ODbL. | Attribution required; a redistributed derived database carries the licence. Every OSM-derived `.ldr` carries it in its header (M77). |
| **National flags** | Designs are generally not copyrightable; *specific renderings* can be. | Flags are built from **transcribed construction specifications**, never traced from an image. Each `flags/<iso2>.json` cites its own source. |
| **Patent drawings (BE 311029, GB 529580, GB 587206, FR 588985)** | Pre-1930 drawings are PD by age; later ones need checking individually. | Only PD drawings appear on screen; each is credited with its patent number and date. The `todo-johnny-world-proof.md` licensing work already done for Commons applies. |
| **The trademarked brand name** | Not ours. | Never in code, commands, filenames, on-screen text, or metadata. `BRICKs.md`'s rule, without exception. The work's whole argument is that the *system* is older and larger than any one company; using the mark would undercut it. |
| **Audio** | — | 100% synthesised. No samples, no recordings, no scores under copyright. The fugue is generated by our own code from our own subject. |

**And one ethical constraint that is not a licence.** The exclusion list in
M73 — sites of genocide, atrocity, slavery, and mass death are not rendered
as toy bricks. It fails closed. It is reviewed by a human before every
release. Nobody gets to skip that review because the build is green.

---

## 13. Risks, honestly

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| The mesh renderer looks worse than the point renderer | low | high | M57 is gated on screenshots reviewed by a human, in week 4, before anything depends on it. If it looks worse, we find out early and the point pipeline is still there. |
| Conditional edges are fiddly and eat a week | **medium** | medium | Time-boxed: if the GPU implementation is not working after 3 days, ship hard edges only and open a follow-up. Hard edges alone already achieve 80% of the look. |
| The generated fugue is technically correct and musically dead | **medium** | **high** | Human listen is a gate on M68, M69 and M71. If it is dead, the fallback is a hand-composed subject *and* a hand-composed exposition with only the episodes generated. Decide by week 13; do not carry a bad score into Phase 5. |
| 40 Atlas sites is too much authoring | **high** | medium | Tier C is explicitly the first thing cut. The 10:00 cut needs 12; the canonical cut needs 3. |
| M77's classifier is unreliable | medium | low | Fails closed; provisional recipes require human promotion. A bad classifier costs review time, never a bad frame. |
| wasm toolchain friction | medium | low | Phase 6 is cuttable by design, and sequenced last for exactly this reason. |
| Safari | **high** | medium | Test in week 25 at the latest, ideally in week 6. Budget a week. |
| Scope creep from the piece being interesting | **high** | **high** | The four cuts, the shot list, and this milestone table are the scope. New ideas go in `TODOs.md`'s backlog, not into the current milestone. |

---

## 14. Definition of done

The work is finished when all of the following are true, and not before:

1. `spex show-build show/matrix.show.json --duration 240 -o out/240` and
   `spex show out/240` play the canonical cut end to end, with sound, at
   ≥ 55 fps at 1080p, with zero console errors.
2. The 10:00, 60:00 and endless cuts each resolve to their exact duration
   and play end to end.
3. Frame 0 and the final frame of every cut are pixel-identical, and the
   audio meets itself at the seam.
4. The Kick is 800 ms in every cut, and its audio accent and its first
   frame are within one frame of each other.
5. The same seed produces frame-identical runs; different seeds produce
   visibly different ones.
6. Every rendered brick is real LDraw geometry; every colour is a real
   LDraw colour code; every site's metadata is real and CC0-sourced; every
   flag matches a cited construction specification; every attribution
   required by §12 appears both in the repo and on screen.
7. The exclusion list has been reviewed by a human for this release.
8. `cargo test --workspace` green; `npx tsc --noEmit` clean;
   `./scripts/walkthrough.sh` regenerates every pre-existing demo unchanged.
9. `TODOs.md` carries an entry for every milestone M51–M97 in the
   established style, stating what was *verified* and with which real
   numbers.
10. A human has watched the whole thing, all four cuts, with sound, and
    said what they thought.

---

*Iunctura Archiv · Signatur IA-2026-002 · spec written 2026-07-25*
*"Die Römer fügten Ziegel. Wir fügen Kunststoff. Die Sprachmodelle fügen Token."*
*Zählen. Bauen. Berechnen. Derselbe Instinkt. Für immer.*




