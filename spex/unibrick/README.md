# unibrick

Working directory for the Klemmbaustein/interlocking-brick renderer
milestone — see `/BRICKs.md` (real-terminology glossary, real
geometry-source confirmation, licensing) and `TODOs.md`'s M40-M45 entries
for the full design and status. This directory is the code; those
root-level docs are the plan.

## Modules

- **`ldraw.py`** — the real network + LDraw-file-parsing layer: fetches a
  real part's files from [LDraw](https://ldraw.org)'s official library
  (recursively resolving the real part → subpart → shared-primitive
  reference tree), parses the real official `LDConfig.ldr` color table, and
  exposes `resolve_geometry()` (flattens one real part into world-space
  triangles) plus the small matrix/vector helpers it needs. No
  brick-mesh-format or point-sampling code lives here. `fetch()` prefers
  reading straight out of a local full-library mirror (see
  `download_library_zip()`/`.ldraw-cache/complete.zip` below) when one's
  present, falling back to a real per-file HTTP fetch (retried with real
  exponential backoff on a real HTTP 429) otherwise.
- **`brickmesh.py`** — the `spex-brick-mesh` intermediate format (see
  `spec/brickmesh.schema.json`): resolves a real part's geometry via
  `ldraw.py` *once*, caches the flat result as plain JSON under
  `.ldraw-cache/meshes/`, and exposes `get_or_resolve_mesh()` (the "resolve
  once, reuse forever" entry point), `place_mesh()` (cheap translation
  *and rotation* of already-resolved triangles into an assembly — no
  re-fetch), and `mesh_triangles()` (back to the flat triangle list
  `sampling.py` needs). This is what lets every script below skip
  re-walking LDraw's real reference tree just to try a different color,
  point density, or placement of a part already resolved on a previous run.
- **`brickscene.py`** — the `spex-brick-scene` format (see
  `spec/brickscene.schema.json`): parses a real, official LDraw *model*
  file's own type-1 placement lines directly (e.g.
  [`car.ldr`](https://library.ldraw.org/library/official/models/car.ldr),
  a real official LDraw sample model by James Jessiman) into a flat list
  of real `(part, color, translation, rotation matrix, build-step)`
  placements — the "build instructions" half of the plan, sourced from a
  real model file rather than hand-written. Caches the parsed result under
  `.ldraw-cache/scenes/`. **Licensing note, checked not assumed**: official
  model files (unlike individual part files) carry no explicit CCAL
  2.0/CC BY license header, and ldraw.org's Legal Info page doesn't
  address `models/` the way it does `parts/` — see this module's own
  docstring for the honest caveat.
- **`sampling.py`** — real face-area-weighted surface sampling + baked-in
  Lambertian/specular shading, computed once per point from its source
  triangle's real normal (neither spex's WebGL viewer nor its ASCII
  renderer compute lighting themselves, so this is the only way to get a
  "shiny" look out of either). Takes any flat triangle list — doesn't care
  whether it came from a single resolved mesh or a placed multi-part
  assembly — and is also the natural seam where a future true mesh/vector
  renderer (see `BRICKs.md`) would branch off instead of sampling to points.
- **`gen_brick_demo.py`** — CLI: resolves one real part (via
  `brickmesh.get_or_resolve_mesh`), samples it, writes a `.xyz` point cloud
  that spex's existing point-cloud pipeline (`spex convert`/`spex serve`/
  `spex ascii`) renders unchanged — no new spex code needed. Usage:
  `python3 unibrick/gen_brick_demo.py <part.dat> <ldraw-color-code>
  <point-count> <out.xyz>` (all arguments optional, defaults to a real 1×1
  brick in red).
- **`gen_monolith_demo.py`** — CLI: assembles *multiple* real parts into one
  real vertical stack — currently "the 2001 moment": 7 real "Brick 1×4" + 2
  real "Plate 1×4", all real Black, approximating the film's canonical
  1:4:9 monolith proportions as closely as real LEGO height units allow
  (exact 1:4:9 isn't achievable — see `TODOs.md`'s M41 for the honest
  math). Resolves each *distinct* real part exactly once regardless of how
  many times it's placed (9 placements, only 2 real network fetches).
  Usage: `python3 unibrick/gen_monolith_demo.py <point-count> <out.xyz>`.
- **`gen_model_demo.py`** — CLI: renders a real, official LDraw *model*
  file (via `brickscene.py`), reusing each distinct referenced part's
  resolved mesh across every real placement (car.ldr's 61 placements are
  only 26 distinct real parts). Usage: `python3 unibrick/gen_model_demo.py
  <model.ldr> <point-count> <out.xyz>` (defaults to `car.ldr`; `pyramid.ldr`
  is the other real official sample model available the same way).
- **`gen_monolith_assembly.py`** — CLI: the animated version of
  `gen_monolith_demo.py` — the same 9 real parts starting
  scattered/floating (a real, honestly-labeled *stylized* reveal, not a
  physics simulation) and converging into the finished stacked monolith.
  Writes a single self-contained animated HTML file (embedded JSON frame
  data + a hand-rolled canvas 2D point renderer with drag-orbit/scroll-zoom
  — no three.js/WebGL/server dependency, opens directly from disk) rather
  than going through spex's tileset/octree pipeline at all, since animating
  real per-point positions there would require the tileset format itself to
  track point identity across frames. Usage: `python3
  unibrick/gen_monolith_assembly.py <point-count> <frame-count> <out.html>`.
- **`.ldraw-cache/`** — gitignored local cache of real files fetched from
  ldraw.org (`.ldraw-cache/parts/`, `.ldraw-cache/p/`,
  `.ldraw-cache/models/`, `.ldraw-cache/LDConfig.ldr`), so repeated runs
  don't re-fetch the same files. `.ldraw-cache/meshes/` and
  `.ldraw-cache/scenes/` hold resolved `spex-brick-mesh`/`spex-brick-scene`
  JSON files. `.ldraw-cache/complete.zip` (~136MB, gitignored, never
  committed or git-lfs'd) is an optional real full-library mirror —
  download it once with `python3 -c "import ldraw;
  ldraw.download_library_zip()"` (run from inside `unibrick/`) and every
  subsequent `fetch()` call reads straight out of it with zero network
  requests, which is what resolving a real multi-part model (dozens of
  distinct file fetches) actually needs to avoid tripping ldraw.org's real
  rate limit.

Real LDraw color codes/RGB values always come from the real, official
`LDConfig.ldr`, fetched and parsed by `ldraw.load_ldraw_colors()` — never
invented.

Output convention: this is the *point-cloud* pipeline (a brick is a solid
volume, not a tree), so generated files follow spex's plain `in/`/`out/`
convention (both gitignored), not the graph pipeline's `demos/` — e.g.
`in/2027-a-brick-odyssey.xyz` → `spex convert` → `out/2027-a-brick-odyssey/`.

Not yet built, next real design step (see `BRICKs.md`): a true mesh/vector
renderer consuming a `spex-brick-mesh`/`spex-brick-scene` pair directly
(instead of point-sampling them) — a real, deliberately bigger alternative
for later, if the point-cloud look ever feels too soft for what's wanted.
