# unibrick

Working directory for the planned Klemmbaustein/interlocking-brick
renderer milestone — see `/BRICKs.md` (real-terminology glossary, real
geometry-source confirmation, licensing) and `TODOs.md`'s "Klemmbaustein
brick voxel renderer" backlog entry (M40) for the full design and status.
This directory is the code; those root-level docs are the plan.

- **`gen_brick_demo.py`** — fetches a real part's real geometry from
  [LDraw](https://ldraw.org)'s official library (recursively resolving the
  real part → subpart → shared-primitive reference tree), samples its real
  surface into a colored point cloud, and writes it as a plain `.xyz` file
  that spex's existing point-cloud pipeline (`spex convert`/`spex serve`/
  `spex ascii`) renders unchanged — no new spex code needed. Usage:
  `python3 unibrick/gen_brick_demo.py <part.dat> <ldraw-color-code>
  <point-count> <out.xyz>` (all arguments optional, defaults to a real 1×1
  brick in red).
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
