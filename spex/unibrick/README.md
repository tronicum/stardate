# unibrick

Working directory for the Klemmbaustein/interlocking-brick renderer
milestone — see `/BRICKs.md` (real-terminology glossary, real
geometry-source confirmation, licensing) and `TODOs.md`'s M40-M43 entries
for the full design and status. This directory is the code; those
root-level docs are the plan.

## Modules

- **`ldraw.py`** — the real network + LDraw-file-parsing layer: fetches a
  real part's files from [LDraw](https://ldraw.org)'s official library
  (recursively resolving the real part → subpart → shared-primitive
  reference tree), parses the real official `LDConfig.ldr` color table, and
  exposes `resolve_geometry()` (flattens one real part into world-space
  triangles) plus the small matrix/vector helpers it needs. No
  brick-mesh-format or point-sampling code lives here.
- **`brickmesh.py`** — the `spex-brick-mesh` intermediate format (see
  `spec/brickmesh.schema.json`): resolves a real part's geometry via
  `ldraw.py` *once*, caches the flat result as plain JSON under
  `.ldraw-cache/meshes/`, and exposes `get_or_resolve_mesh()` (the "resolve
  once, reuse forever" entry point), `place_mesh()` (cheap translation of
  already-resolved triangles into an assembly — no re-fetch), and
  `mesh_triangles()` (back to the flat triangle list `sampling.py` needs).
  This is what lets both scripts below skip re-walking LDraw's real
  reference tree just to try a different color, point density, or stack
  placement of a part already resolved on a previous run.
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
- **`.ldraw-cache/`** — gitignored local cache of real files fetched from
  ldraw.org on demand (`.ldraw-cache/parts/`, `.ldraw-cache/p/`,
  `.ldraw-cache/LDConfig.ldr`), so repeated runs (and the recursive
  resolution of a single part's sub-file tree) don't re-fetch the same
  files. `.ldraw-cache/meshes/` (also gitignored) holds the resolved
  `spex-brick-mesh` JSON files themselves, one per (part, requested color)
  pair actually generated so far. Neither is a library mirror — only
  whatever's actually been requested. A deliberate full-library mirror, if
  ever wanted, would be a separate git-lfs step (see `BRICKs.md`), not
  something either cache grows into automatically.

Real LDraw color codes/RGB values always come from the real, official
`LDConfig.ldr`, fetched and parsed by `ldraw.load_ldraw_colors()` — never
invented.

Output convention: this is the *point-cloud* pipeline (a brick is a solid
volume, not a tree), so generated files follow spex's plain `in/`/`out/`
convention (both gitignored), not the graph pipeline's `demos/` — e.g.
`in/2027-a-brick-odyssey.xyz` → `spex convert` → `out/2027-a-brick-odyssey/`.

Not yet built, next real design step (see `BRICKs.md`): a full real set's
inventory (many elements, each with a real position from a real LDraw
`.ldr` model) — the "build instructions" half of the idea, and a much
bigger step than reusing already-resolved single-part meshes in a hand-
written stack. A true mesh/vector renderer consuming `spex-brick-mesh`
files directly (instead of point-sampling them) is the other real,
deliberately-bigger alternative for later.
