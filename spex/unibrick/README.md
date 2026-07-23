# unibrick

Working directory for the planned Klemmbaustein/interlocking-brick
renderer milestone — see `/BRICKs.md` (real-terminology glossary, real
geometry-source confirmation, licensing) and `TODOs.md`'s M40/M41 entries
for the full design and status. This directory is the code; those
root-level docs are the plan.

- **`gen_brick_demo.py`** — fetches a real part's real geometry from
  [LDraw](https://ldraw.org)'s official library (recursively resolving the
  real part → subpart → shared-primitive reference tree), bakes real
  Lambertian shading + a specular highlight into each sampled point from
  its real source-triangle normal (a real, once-per-generation lighting
  calculation — neither spex's WebGL viewer nor its ASCII renderer compute
  lighting themselves, so this is the only way to get a "shiny" look out
  of either), and writes the result as a plain `.xyz` file that spex's
  existing point-cloud pipeline (`spex convert`/`spex serve`/`spex ascii`)
  renders unchanged — no new spex code needed. Usage: `python3
  unibrick/gen_brick_demo.py <part.dat> <ldraw-color-code> <point-count>
  <out.xyz>` (all arguments optional, defaults to a real 1×1 brick in
  red). Note: LDraw's own coordinate convention has +Y pointing *down* —
  this script flips Y only at final output so parts display stud-up under
  spex's standard +Y-up camera.
- **`gen_monolith_demo.py`** — reuses every real-geometry function above
  (not a separate implementation) to assemble *multiple* real parts into
  one real vertical stack — currently "the 2001 moment": 7 real
  "Brick 1×4" + 2 real "Plate 1×4", all real Black, approximating the
  film's canonical 1:4:9 monolith proportions as closely as real LEGO
  height units allow (exact 1:4:9 isn't achievable — see `TODOs.md`'s M41
  for the honest math). Usage: `python3 unibrick/gen_monolith_demo.py
  <point-count> <out.xyz>`.
- **`.ldraw-cache/`** — gitignored local cache of real files fetched from
  ldraw.org on demand, so repeated runs (and the recursive resolution of a
  single part's sub-file tree) don't re-fetch the same files. Not a
  library mirror — only whatever's actually been requested so far. A
  deliberate full-library mirror, if ever wanted, would be a separate
  git-lfs step (see `BRICKs.md`), not something this cache grows into
  automatically.

Output convention: this is the *point-cloud* pipeline (a brick is a solid
volume, not a tree), so generated files follow spex's plain `in/`/`out/`
convention (both gitignored), not the graph pipeline's `demos/` — e.g.
`in/2027-a-brick-odyssey.xyz` → `spex convert` → `out/2027-a-brick-odyssey/`.

Not yet built, next real design step (see `BRICKs.md`): splitting the
live-fetch-and-flatten LDraw resolution out of this script into its own
reusable intermediate format — resolve a part's real geometry once, then
reuse that flat, resolved result to generate different colors/point
densities of the same part without re-walking the real LDraw reference
tree every time.
