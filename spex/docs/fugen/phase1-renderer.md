# Phase 1 - the vector-accurate renderer (M51-M59)

*Real LDraw triangles and edges, rendered as meshes. This is what 'vektorgenau' means.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**
Read [`README.md`](README.md) first: it carries the working rules, the scope decision and the milestone index.
Review record and the reasoning behind every rev-2 change: [`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md).

---


## Rev 3 corrections — these override the milestone text below

Seven specialist reviews (`../FUGEN-ENGINE-REVIEW-01.md`) found nine defects
in this phase. Six of them would have invalidated an acceptance criterion
*after* the code was written. Read these before M51.

| # | What rev 1 said | What is actually true |
|---|---|---|
| **B1** | A type-5 conditional edge is drawn when its control points fall on **opposite** sides. | **Inverted.** It is a silhouette when both project to the **same** side. As written it would have drawn the cylinder tessellation and hidden the silhouette. *Already corrected inline in M51/M57 below.* |
| **B2** | `MeshBundleBuilder::add_part` keys on `(part, colour)`; `resolve_part` bakes colour during recursion. | **Geometry must stay colour-neutral**, or per-instance colour is impossible and instancing is defeated. `resolve_part_full` preserves LDraw code 16 as `color_code: Option<u32>` (`None` = inherit). Parts key on `part_file` **only**. `submeshes[].material: number \| null`, where `null` means "take the instance's material". |
| **B3** | M56 replaces `ColorTable`'s tuple value with a struct and adds an `rgb()` accessor. | **That breaks the point pipeline** — `sampling.rs:95` and `brick.rs:216` both *destructure the tuple*, which an accessor cannot rescue, contradicting M56's own AC3. `load_colors` keeps its signature untouched; add **`load_colors_full() -> HashMap<u32, LdrawColor>`** used only by `spex-mesh`. The `LDConfig` parser must also split at `MATERIAL` **first** — `ALPHA`/`LUMINANCE`/`VALUE` appear on both sides of that token, so `find_after` alone mis-parses every speckle and glitter colour. |
| **B4** | M53 AC4: "servable as static files with no server change". | **False.** `crates/spex-cli/src/main.rs:684` bails unless `tileset.json` or `sequence.json` exists. M53 must add `mesh.json` (and later `show-resolved.json`) to that guard. |
| **B5** | M59 gates stud/tube removal "on the reference path, never on a heuristic". | **`resolve_into` discards the reference chain**, so there is no path to gate on. The fix belongs in **M51**, not M59: `PartGeometry` gains `sources: Vec<String>`, and every triangle/edge a `source: u16` index into it. |
| **B9** | M51 AC3 compares triangles "identical up to per-triangle vertex rotation"; AC1 quotes 8×8×11.2 mm. | Reversing winding is **not** in the rotation group, so the assertion fails on exactly the triangles M51 fixes — compare **unordered vertex-position sets**. And `resolve_part` returns **LDU, Y-down** (20×20×28 LDU); `LDU_TO_MM` is applied only in `to_point_cloud`. Pick mm/Y-up **at the bundle boundary** and say it once. |
| **B10** | M59 AC1 measures against "a 40-site Atlas scene (from M74)". | A week-6 milestone gated on a week-17 deliverable. M59 verifies against a **synthetic 200 k-instance scene**; the real-Atlas measurement moves to an AC on M81. |
| **B11** | `mesh.json` carries an `instances[]` JSON array. | At 250 k instances that is **37 MB of text → ~120 MB parsed heap → 0.8–1.5 s of main-thread parse.** Encode binary: `(i16 x, y, z; u8 orientation; u8 material; u16 part)` = **10 B/instance = 2.5 MB**. Grid legality (M72) is what makes this exact rather than lossy, and it is also what makes M95's single-file edition possible at all. |
| **B12/13/14** | `baseColor: [0.106, 0.165, 0.204]`; ACES tone mapping on the renderer in M54; bloom in M58. | `[0.106,…]` is sRGB ÷ 255, which three.js r152+ reads as **linear** — every material ships ~2.2× too dark. **Store linear and declare the colour space in the schema.** Tone mapping on the renderer happens *before* bloom, making the bloom threshold meaningless: use **`NoToneMapping`**, HalfFloat targets, bloom in linear HDR, **ACES last in `OutputPass`**. And a wide soft green gradient over `#000` on an 8-bit backbuffer **bands** — add ±0.5/255 triangular dither plus ~1.5 % fixed grain. That is the cheapest thing in this document that separates "cheap WebGL" from "print". |

**Two performance decisions that change what this phase builds** (full budgets
in [`budgets.md`](budgets.md)):

- **Geometric edges are for hero shots only.** WebGL2 has no
  instancing-of-instances; fat-line quads at 250 k instances is ~150 M
  vertices/frame, and at Atlas distance every outline merges into a black
  mass anyway. Real type-2/5 edges above ~40 px projected height (≤ 3 k
  bricks); everything else gets a **screen-space depth+normal-discontinuity
  outline pass**, whose cost is independent of instance count. M57's
  "≤ 25 % of frame time on the 50 k scene" is unreachable any other way.
- **`setPixelRatio(Math.min(devicePixelRatio, 1.5))`.** One line; on a 3× DPR
  tablet it is otherwise 9× the fragments. `main.ts:300` currently uses the
  raw ratio.

**Rejected:** M58's `--disable-gpu` as a Low-tier proxy. That is SwiftShader,
~100× slower; tuning Low against it would make Low far uglier than it needs
to be. CI asserts **counters**, never fps; fps is asserted only on the named
real hardware in M92.

**Better material numbers** than M56's table below (technical-art review —
these are calibrated artistic choices, not measurements, and must be recorded
as such): opaque ABS roughness **0.34** with clearcoat 0.15 /
clearcoatRoughness 0.25 (ABS has a distinct skin layer); black specifically
**0.22**, because black reads only by its specular; transparent roughness
**0.10**, transmission 0.85, **ior 1.53**; rubber **0.92**; pearlescent is
**not a metal** — metalness 0.0, roughness 0.42, `iridescence 0.4`,
`iridescenceIOR 1.8`; matte-metallic metalness **1.0** / roughness 0.62 (0.8
is the classic in-between that reads as neither); metal 1.0 / **0.35**;
chrome roughness **0.06** (0.03 gives one hard env dot and reads as plastic).
Weld crease **30°** (LDView's value, safe for both 16- and 48-segment
primitives). Edge width **1.25 px** at DPR 1, **1.6 px** at DPR ≥ 2, faded out
below 40 px. Background: a vertical gradient `0x05070a → 0x0d1219` rather than
flat `0x0b0e12`, so the black monolith has something to silhouette against.

**Missing from rev 1 entirely, and required for this look:** baked per-vertex
ambient occlusion computed in `spex-mesh` (SSAO at default radius misses the
stud annulus and instead darkens silhouettes — baked AO is what makes studs
read), stud-interior/tube-cavity darkening, contact shadows (a single 2048²
directional map over an Atlas gives metre-scale texels), transparent-group
back-to-front sorting with `depthWrite: false` and `side: DoubleSide` (a
transparent brick reads *because* you see its own tubes, which backface
culling deletes), and analytic coverage falloff across the edge quad — SMAA
does not anti-alias a 1.4 px quad.

---

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

1. `spex serve <mesh bundle>` renders the model as solid geometry with
   visibly crisp silhouettes and no z-fighting.
2. A real headless-Chromium screenshot at 1600×1000 shows: no interior
   faces, no black holes, shadow visible on the ground plane. **Counters,
   not fps** — see AC2's correction below.
3. Every existing demo renders exactly as before — the mesh branch never
   triggers.
4. Console has zero errors and zero three.js warnings.

**AC2 correction, adopted during implementation.** Rev 1 asked for "≥ 55 fps
reported by the HUD" in the headless screenshot. That contradicts a decision
this phase had *already* recorded: SwiftShader was rejected as a Low-tier
performance proxy because it is ~100× slower than real hardware, and "CI
asserts **counters**, never fps; fps is asserted only on the named real
hardware in M92." A headless container has no GPU, so the screenshot harness
runs on exactly the SwiftShader that was rejected — the monolith reports 4 fps
there and would report several hundred on real hardware. Same class of defect
as B10: an acceptance criterion that cannot be met by the thing asked to meet
it. **What AC2 asserts instead:** instance count, part count, triangles drawn
vs. unique, draw calls, zero console errors, zero failed requests — all
hardware-independent — plus the picture. Frame rate moves to M92.

**Verification ladder.** 1, 2, 3, 5 (**mandatory**, with screenshots
attached to the milestone note), 6, 7.

**Status: ✅ done.** Screenshots in [`screenshots/`](screenshots/):
`m54-car.png` (61 instances, 26 parts, 24 921 triangles drawn / 13 729
unique, 125 draw calls, 7 materials), `m54-monolith.png` (9 / 2 / 3 276 /
728, 19 draw calls), `m54-brick.png` (1 / 1 / 76 / 76, 3 draw calls),
`m54-regression-points.png` (the point path, untouched: 4 260 points, legend,
packet animation, hover tooltip, debug panel, and the Exposure control
correctly hidden). Zero console errors and zero failed requests in all four.

**Three findings recorded rather than fixed here**, each with the milestone
that owns it:

- *Seam lines on a stacked model are geometry, not z-fighting.* A nine-brick
  stack shows faint dashed lighter lines at every brick boundary — the lower
  brick's brightly-lit top face rasterising as a sub-pixel sliver where two
  flush outer walls abut. Proven by `scripts/viewer-shot/isolate.mjs`:
  shrinking the depth range from 20 000:1 to 4:1 left the crop
  pixel-identical, which no depth-precision artefact survives. AC1 holds.
  **M57's edge pass covers the seam**, because a 1.25 px outline is drawn on
  exactly that line.
- *A large flat top face blows its specular to pure white at close range.*
  Also proven by isolation: forcing roughness to 1.0 removes it completely,
  so normals and welding are correct and this is purely the finish. A direct
  light with no environment term is a delta with nothing to spread it.
  **M56** owns it — clearcoat, the real roughness table, and an IBL are the
  fix, not a lower key light.
- *A black model on a dark ground barely reads.* True, and by construction:
  LDraw Black is linear `[0.011, 0.023, 0.034]`. The rev 3 corrections
  already assign both remedies elsewhere — the vertical background gradient
  to **M56**, real silhouette edges to **M57**. Deliberately not pulled
  forward on a guess; the car demo, which has colour, satisfies AC1 on its
  own.

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
  /** instanceId -> the bundle's own stable instance id, for choreography */
  ids: string[];
  /** per-instance 0..1 scalar, uploaded as `aDissolve` — M65 reads it */
  dissolve: THREE.InstancedBufferAttribute;
}
// The `hardEdges` / `conditionalEdges` fields rev 1 put here are gone: the
// rev 3 corrections re-scoped edges after this signature was written (real
// type-2/5 edges only above ~40 px projected height, a screen-space outline
// pass otherwise), so building line geometry per group here would commit to
// a decision that belongs to M57. M57 adds whichever of the two it needs.

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

1. A synthetic 50 000-instance scene renders with a draw-call count that is
   **independent of instance count** — see AC1's correction below for the
   real bound.
2. `renderer.info.render.calls` is asserted in the headless check.
3. Updating every instance's transform each frame is measured and the real
   number recorded.

**AC1 correction, adopted during implementation.** Rev 1 asked for "draw
calls ≤ 3 × distinct part count". That bound cannot hold, and the reason is
the same fact B2 already established: **colour is a material binding, not
vertex data.** One part placed in seven LDraw colours is seven
`InstancedMesh`es no matter how few parts the scene has — the stress scene is
2 parts in 7 colours, so 14 groups, and 29 draw calls, or 14.5 × the distinct
part count. It is not a regression; the formula counted the wrong thing.
**The real invariant, and the one that matters:** draw calls scale with
distinct `(part, material)` pairs and their submeshes — roughly 2 × groups,
because the shadow pass re-submits every caster — and are **independent of
instance count**. The measurements prove it directly: 61 instances → 67 calls,
50 000 instances → 29. Fps is not asserted here for the reason recorded at
M54's AC2; it belongs to M92.

**AC3, measured but not settled.** Rewriting every one of 50 000 transforms
and uploading them, median of 5 full passes, on the container (no GPU,
SwiftShader competing for the same cores): **6.3 ms** through `setMatrix`
(the path an animation curve actually takes — it already has a `Matrix4`) and
**14.2 ms** through `setTransform` (position/quaternion/scale, the general
case). Rev 1's "< 4 ms" was written for "the development machine", which this
is not, so the criterion is recorded rather than passed and re-measured on
the named hardware in M92 alongside fps. The gap between the two paths is
itself the finding: choreography should hand over matrices it already has.

**Verification ladder.** 1, 2, 5 (**mandatory**), 7.

**Status: ✅ done.** `viewer/src/mesh/instanced.ts`; the stress fixture is
generated, not committed (`scripts/gen_stress_scene.py`, 50 000 real bricks
on the real stud grid, 0.000 mm quantisation error).

| scene | instances | parts | groups | draw calls | before (M54) |
|---|---|---|---|---|---|
| 1×1 brick | 1 | 1 | 1 | 3 | 3 |
| monolith | 9 | 2 | 2 | **5** | 19 |
| car | 61 | 26 | 32 | **67** | 125 |
| stress lattice | 50 000 | 2 | 14 | **29** | ~100 000 |

Screenshots: `screenshots/m55-car.png`, `m55-monolith.png` — visually
identical to M54's, which is the point. The stress scene has **no**
screenshot: 11 M triangles per frame plus a shadow pass exceeds any sane
timeout on a software rasteriser, so its numbers come from
`scripts/viewer-shot/probe.mjs`, which reads the same hooks without ever
asking for pixels.

**One real defect found by measuring.** `InstanceWriter.setTransform`
allocated a fresh `THREE.Vector3` per call — 50 000 allocations per frame in
the hot path. Removed. No speedup is claimed from it: the same code path
measured 14.5, 11.0 and 14.2 ms across runs, so this container's variance is
wider than the effect. It stays because it is the difference between a flat
per-frame cost and one that grows with GC pressure across a 60-minute show.

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
recorded as such — not presented as physical measurements). **The rev 3
corrections override the first-draft table; what shipped is:**

| Finish | metalness | roughness | other |
|---|---|---|---|
| Solid (opaque ABS) | 0.00 | **0.34** | clearcoat 0.15 / ccRoughness 0.25 — ABS has a real skin layer |
| Solid, code 0 (Black) | 0.00 | **0.22** | black reads only by its specular |
| Solid (transparent) | 0.00 | **0.10** | transmission 0.85, **ior 1.53** (the one real measurement here — polycarbonate's) |
| Rubber | 0.00 | **0.92** | no clearcoat |
| Pearlescent | **0.00** | 0.42 | iridescence 0.4, iridescenceIOR 1.8 — **not a metal** |
| MatteMetallic | 1.00 | 0.62 | 0.8 metalness reads as neither metal nor plastic |
| Metal | 1.00 | **0.35** | |
| Chrome | 1.00 | **0.06** | 0.03 gives one hard dot and reads as plastic |
| Speckle / Glitter | 0.00 | 0.30 | particle params carried in the manifest; the procedural chunk is not built yet |
| any `LUMINANCE` > 0 | | | emissive = base colour, intensity = LUMINANCE / 255 |

**Transparency composes with the finish rather than replacing it** — most
real glitter colours are transparent too. And it is **proportional**, not a
switch: `Trans_Clear` is `ALPHA 128` and really is glass, but
`Glow_In_Dark_Opaque` is `ALPHA 245` — it says *opaque* in its own name.
Treating both the same gave the glow bricks transmission 0.85 and roughness
0.10, i.e. a lump of resin. Glassiness is normalised at LDraw's own 128.

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

**Status: ✅ done.** `ldraw-scenes/finishes.ldr` is the test bundle AC2 asks
for: nine real LDraw colour codes, one per finish keyword the official file
actually uses, plus the two modifiers that are not finishes (`ALPHA`,
`LUMINANCE`). Screenshots: `screenshots/m56-finishes.png`,
`m56-car.png` — the car's cab is now really glass, with the seats and
steering wheel visible through it.

AC1 passes against the real `LDConfig.ldr`: **6 distinct finishes** present
(chrome 7 colours, pearlescent 24, rubber 59, metal 11, speckle 4, glitter
15), codes 0/4 solid, 47 `ALPHA 128`, 383 chrome. `MATTE_METALLIC` is in
LDraw's grammar and **no colour in the current official file uses it** — the
test asserts that absence, so it stays a recorded fact instead of a suspected
parser bug. AC3 passes by construction: `load_colors` keeps its exact
signature and the point pipeline's two destructuring call sites are untouched
(B3).

**Judged by measurement, not by eye.** `scripts/viewer-shot/swatch.mjs`
projects every instance to screen space and reads the rendered pixel back, so
"does chrome read as metal" is a number. Final values on the finish row:

| | rendered sRGB | |
|---|---|---|
| Red (solid) | `231, 62, 63` | real red, not clipped |
| Trans_Clear | `180,182,188` | transparent |
| Chrome_Silver | `200,202,207` | mirror |
| Metallic_Silver | `117,121,128` | metal, darker base and rougher |
| Pearl_Light_Gold | `231,213,171` | |
| Rubber_Black | ` 44, 60, 76` | matte |
| Speckle_Black_Silver | ` 39, 40, 44` | |
| Glitter_Trans_Clear | `166,169,175` | transparent |
| Glow_In_Dark | `229,236,217` | emissive |

**The environment took four measured attempts, and each failure was a real
thing to know.** (1) Chrome rendered grey — the environment's floor was
near-black and a mirrored 1×1 brick spends most of its reflection looking
down; a real product studio has a white sweep under the subject for exactly
this reason. (2) Raising the whole environment to fix that blew out every
dielectric: real LDraw Red clipped its own channel and rendered orange. A
dielectric responds to *irradiance* (broad and dim wins), a metal to
*radiance* in one direction (narrow and bright wins) — one brightness cannot
serve both. (3) Small bright light cards were supposed to solve that, and
switching the direct lights off proved they had not: red rendered
`255,120,88` **from the environment alone**, against `40,0,0` from the entire
direct rig. A 1.3×1.8 card at 4.7 units subtends ~0.85 % of the sphere, so at
intensity 260 it contributes ~2.2 of irradiance each — the cards *were* the
lighting, and nobody had said so. (4) At 40/20/14 they light the highlights
and the rig lights the scene.

**The M54 rig was rebalanced, not tweaked.** Its hemisphere fill is gone: the
environment is the fill now, and a better one, because it has structure. Two
fills double-counted the ambient and forced the environment to stay dim,
which is what kept chrome grey.

**Deferred, with the reason:** the procedural speckle/glitter particle chunk.
The parameters are parsed, resolved and carried through the manifest in full
(`pbr.speckle`), so it is a shader change and not a format change — it lands
with M65's dissolve shader, which is where custom `onBeforeCompile` chunks
already live.

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

**Status: ✅ done.** `viewer/src/mesh/edges.ts`. Screenshots:
`screenshots/m57-brick-2000.png` (AC1, and a crop of the stud),
`m57-car.png`, `m57-monolith.png`.

**AC2's phrasing was wrong, and the measurement is the more interesting
result.** Rev 1 asked that "the conditional-edge count actually drawn changes
between angles". It does not, and a correct renderer is why: a cylinder is a
body of revolution, so it presents exactly **two** silhouette edges from every
direction. Measured on a real `3005.dat` stud across twelve orbit angles: 2
drawn out of 16, at every single one. What changes is *which* two —

```
angle  0: 56, 64      angle  4: 62, 69      angle  8: 58, 67
angle  1: 58, 65      angle  5: 63, 70      angle  9: 60, 68
angle  2: 59, 66      angle  6: 56, 64      angle 10: 61, 70
angle  3: 60, 68      angle  7: 57, 66      angle 11: 62, 71
```

— ten distinct sets over twelve angles, rotating with the camera around the
16-segment stud (two repeat because 12 angles alias against 16 segments). The
criterion now asserts on the **set**, not its size; asserting on the count
would have failed a working renderer. `scripts/viewer-shot/orbit.mjs` runs it.

**Three defects found by rendering it.**

- *Nothing drew at all, with no error anywhere.* The program linked, the draw
  call was issued, `renderer.info.render.calls` went up, and the frame was
  unchanged. A screen-space quad's winding flips with the direction of the
  line it expands, so at any camera angle about half of them are back-facing —
  and the material was `FrontSide`. `DoubleSide` is not an optimisation to
  skip here, it is a correctness requirement.
- *Biasing the edges toward the viewer cannot work.* The spec's own
  prescription (`gl_Position.z -= bias * gl_Position.w`) was implemented and
  produced a brick whose **interior** edges showed through its front wall —
  and on a hollow LDraw brick that is most of them. Any offset large enough
  to beat a coplanar face is large enough to beat the wall in front of it.
  The fix is at the other end: `polygonOffset` on the **solid** material,
  which moves only the coincident surface and scales with its own depth
  slope. `uDepthBias` stays, defaulting to **0**, for tuning at extremes.
- *AC3's 50× camera distance rendered an empty frame.* Not a z-fighting pass
  — the object simply fell outside `far = diag * 20`, which is what the
  camera had been configured with since M54 while the screenplay pulls back
  fifty times further. Now `near = diag/200`, `far = diag*150`. Widening it
  is safe precisely *because* coincident surfaces are handled by
  `polygonOffset` rather than by depth precision.

**And one caught in the format rather than the renderer:** M56 added `finish`
and `pbr` as *required* material fields without bumping `FORMAT_VERSION`. A
bundle built before it passed the viewer's `version === 1` check and then died
on `entry.pbr.opacity`, with a stack trace pointing into minified three.js.
The format is now **version 2**, and the viewer's error names the fix
("rebuild it with `spex mesh-model`"). A format that changes what readers must
find has to change its number, or the check it offers them is worthless.

**AC4 is not met, deliberately, and the number says why.** Geometric edges are
gated by a build-time budget (`MAX_EDGE_QUADS`, 1.5 M ≈ 84 MB of attributes)
and a per-frame projected-height test (40 px, per the rev 3 corrections).
Measured: the 50 000-instance lattice wants **10 800 000** edge quads — about
600 MB — so the pass is skipped entirely with a console warning naming the
real number. WebGL2 has no instancing-of-instances, so there is no cheaper
encoding available; crowd-scale outlines need the **screen-space
depth+normal-discontinuity pass**, whose cost is independent of instance
count. That is recorded here as not-built rather than half-built. Real scenes
stay well under: the car is 22 284 quads, the monolith 3 240.

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

**Status: ✅ done.** `viewer/src/mesh/post.ts` (a module rather than more of
`render.ts` — the chain has its own vocabulary and its own order, and that
order is the milestone). Screenshots: `screenshots/m58-car.png`,
`m58-brick.png` (Low), `m58-brick-high.png` (High).

**The order, which the rev 3 corrections rewrote:** the scene renders into a
**HalfFloat** target with the renderer's tone mapping **off**, so linear HDR
above 1.0 survives → optional SSAO → **UnrealBloom, reading that HDR** → a
custom grade pass that does vignette (in linear, before the curve), ACES, the
sRGB encode, **±0.5/255 triangular dither and 1.5 % fixed grain** → SMAA last,
on the encoded signal, where it belongs. The grade pass replaces three's
`OutputPass` rather than following it: dither has to be applied to the encoded
signal immediately before the 8-bit write, and anything after an `OutputPass`
is already quantised.

**AC1 was already rejected by this document and is not attempted.** It
offered Chromium `--disable-gpu` as a Low-tier proxy; the rev 3 corrections
struck that out — it is SwiftShader, ~100× slower than the slowest real
hardware, and tuning Low against it would make Low far uglier than it needs
to be. What is recorded instead: the tier actually chosen, and the honest
per-frame draw-call total. Frame rates belong to M92, on named hardware.

**AC2 passes, measured rather than eyeballed.** Ramping the bloom threshold
1.0 → 0.2 over 30 captured frames and taking each frame's mean luminance:

```
1.00 → 62.6    0.53 → 63.5    0.31 → 68.1
0.79 → 62.7    0.45 → 64.4    0.26 → 86.4
0.64 → 62.8    0.37 → 64.6    0.20 → 91.5
```

Total rise **29.7 luma, zero non-monotonic steps**. The curve is nearly flat
until ~0.35 and then climbs steeply, which is the real distribution of
radiance in the shot: only the specular highlight lives above that. A flat
curve would have meant bloom was reading an already-clipped signal — the
exact defect the pipeline order exists to prevent.

**AC3 passes:** with every brick, every edge and the ground hidden, the frame
comes back at mean luminance **11.0**, not 0, with no `NaN` and no console
errors. Act I's first six seconds are almost empty and now stay lit.

**The quality benchmark runs against frames the viewer is already watching**,
starting at Medium and settling after two seconds, rather than behind a
two-second black screen. Same number, better first impression. `?quality=`
and the dropdown override it, and choosing from the dropdown stops the
automatic choice from second-guessing a human.

**A defect in the verification harness, not the renderer** — and the more
useful kind to have found. `shot.mjs --no-shadows` disabled the shadow map
without forcing a material recompile, so every draw call kept running against
disposed shadow textures: 243 × `GL_INVALID_OPERATION` per frame, and a
plausible-looking picture of a car whose opaque bricks were invisible and
whose outlines were not. A tool that renders a *wrong* picture quietly is
worse than one that crashes. Fixed by marking every material for recompile.

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

**Status: 🟡 half done — LOD *generation* ships; LOD *selection* does not.**
`crates/spex-mesh/src/lod.rs`, plus the change in `spex-ldraw` that makes it
possible at all. The bundle still carries LOD0 only: a format that advertises
levels no reader selects is a promise, not a feature. What remains is the
writer emitting them and `instanced.ts` choosing between them per instance,
with hysteresis, and the uniform-grid frustum cull.

**B5 is now actually fixed, not just relocated.** M51 gave every triangle and
edge a `source`, but `sources` held the *leaf file name* — and that is not
enough to gate on. `p/4-4cyli.dat` is a quarter-cylinder primitive, and the
same file is a stud, an underside tube, and a hole through a technic beam.
`sources` now holds the real reference **chain**:

```
parts/3001.dat > parts/s/3001s01.dat > p/stud.dat  > p/4-4cyli.dat   <- a stud
parts/3001.dat > parts/s/3001s01.dat > p/stud4.dat > p/4-4cyli.dat   <- a tube
parts/3001.dat > parts/s/3001s01.dat > p/box5.dat                    <- the wall
```

A cylinder *reached through* a stud is a stud. The match is anchored to
LDraw's `p/` primitive directory and to each segment's own file stem, so a
*part* named like a primitive cannot be deleted by its name — there is a test
for exactly that.

**AC3 passes by a wide margin, measured on the real parts, and reported by
the CLI on every build:**

| part / scene | LOD0 | LOD1 (no studs/tubes) | LOD2 (box) |
|---|---|---|---|
| `3001.dat` Brick 2×4 | 700 tris | **28 — 96.0 % fewer** | 12 — 98.3 % |
| the car (26 parts) | 13 729 | 6 747 — 50.9 % | 312 — 97.7 % |
| stress lattice (2 parts) | 440 | 56 — 87.3 % | 24 — 94.5 % |

AC3 asked for ≥ 55 % on `3001.dat`; the real figure is **96 %**, and the
reason is worth stating plainly: a 2×4 brick is *almost entirely studs and
tubes* by triangle count. Its walls are two box primitives and nothing else.
The car's 50.9 % is lower only because a quarter of its parts are wheels,
windscreens and a steering wheel, which have no studs to remove.

**AC1 was already corrected by review 01 (B10):** it measured against "a
40-site Atlas scene (from M74)", a week-6 milestone gated on a week-17
deliverable. It verifies against the synthetic 200 k-instance scene instead,
and the real-Atlas measurement is an AC on M81. Frame rate is not asserted
here for the reason recorded at M54's AC2.

---
