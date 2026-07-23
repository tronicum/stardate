# BRICKs.md — interlocking-brick domain glossary

Shared vocabulary for a planned future milestone: rendering real
Klemmbaustein-compatible bricks (starting from the simplest possible case, a
1×1 plate) as voxel-filled point clouds through spex's existing point-cloud
pipeline, sourced from [Rebrickable](https://rebrickable.com)'s real, open
parts data and [LDraw](https://ldraw.org)'s real vector geometry — later,
maybe, full build instructions. Not implemented yet; this is the
terminology the design/planning conversation and any future code should
agree on, so "brick", "plate", "stud", "element", etc. all mean one specific
real thing, not whatever seemed reasonable at the time.

**A note on naming.** This project deliberately avoids the trademarked
brand name in code, commands, and prose — "Klemmbaustein" (the real,
long-established generic German term for interlocking building bricks) is
the default. When a fun, more acronym-friendly English name is wanted, this
project's own callback is the backronym **L.E.G.O. — "Local Evolved Great
Objects"** (or, in German, "**L**okale **E**rzeugte **G**ute **O**bjekte"):
it spells out the familiar brand's letters as a wink without using the mark
itself. Not a strict rule for every identifier (CLI commands should stay
plain and short, e.g. a future `spex brick`, not `spex lego`) — just the
project's shared in-joke and prose convention whenever a name is needed.

Real terms only, sourced from the actual LEGO/AFOL/BrickLink/LDraw/
Rebrickable community's own long-established usage — nothing here is
invented for this project. Where an exact number matters for rendering
(measurements, real part/color IDs), treat this doc as a *starting
reference*, not the source of truth — pull the authoritative current value
from Rebrickable's real API at generation time, the same "real data, not a
fabricated number" rule the rest of `TODOs.md` already holds every demo to.

## Community & culture terms

- **AFOL** — Adult Fan Of LEGO. The general term for adult hobbyists/
  collectors, as distinct from:
- **TFOL** / **KFOL** — Teen / Kid Fan Of LEGO.
- **MOC** — My Own Creation: an original build, not an official LEGO set.
- **Kitbashing** — building a MOC using parts scavenged from multiple
  official sets rather than parts bought individually.
- **Dark Ages** — the period (often years) an AFOL stops actively
  building/collecting after childhood; "coming out of the Dark Ages" is a
  real, commonly used phrase for resuming as an adult.
- **Sig fig** — an AFOL's "signature figure": a customized minifigure used
  to represent themselves in photos/MOCs.
- **Clutch power** — the actual physical grip strength between a stud and
  an anti-stud tube; the entire system's defining engineering property
  (real, molded to extremely tight tolerances — this is *why* the parts
  interlock at all, not just a figure of speech).
- **Legal** vs. **illegal** build/connection — a "legal" connection loads
  studs/tubes/clips the way LEGO's own engineering intends (won't stress or
  warp parts over time); an "illegal" connection (e.g. forcing a stud into
  a non-matching gap, off-grid angles) works today but risks part damage or
  loosening over years — a real, meaningful distinction in the community,
  not just informal slang.
- **SNOT** — Studs Not On Top: a building technique/aesthetic where a
  surface is built sideways or with tiles so no studs show, versus the
  default "studs visible" look.
- **Greebling** / **greeble** — adding small, often nonfunctional detail
  parts (technic pieces, small bricks) purely for visual texture/detail.
- **BURP** — Big Ugly Rock Piece: informal, widely-used term for a large
  single molded rock/boulder part, since it's a big "cheat" piece rather
  than built from smaller bricks.
- **Polybag** — a small set sold in a plastic bag rather than a box, often
  a promotional/exclusive item.
- **GWP** — Gift With Purchase: a promotional set/part given free above a
  spend threshold, not sold on its own.
- **PAB** — Pick A Brick: LEGO's real per-part (not per-set) retail
  program/wall, sold by individual real part+color combination.

## Real geometry & measurement terms

- **Stud** — the cylindrical nub on a part's top surface.
- **Anti-stud** / **tube** — the hollow cylindrical socket on a part's
  underside that a stud clutches into.
- **Stud pitch** — the real, fixed center-to-center spacing between studs:
  **8.0 mm**. This is the single number the entire system's grid is built
  from — every part's footprint is a whole or half multiple of it.
- **Brick** vs. **plate** vs. **tile** — three real, standardized heights,
  not just casual size categories:
  - **Plate** height: **3.2 mm** (1 plate = 1/3 of a brick).
  - **Brick** height: **9.6 mm** (= 3 plates stacked).
  - **Tile**: a plate-height part with **no studs on top** (smooth finish),
    used for SNOT/finished-surface builds.
- **LDU** (LDraw Unit) — the internal unit LDraw-format files use:
  **1 LDU = 0.4 mm**. So a stud pitch is 20 LDU, a plate is 8 LDU tall, a
  brick is 24 LDU tall. Any voxel-grid generator working from real LDraw
  `.dat` geometry needs to convert through this, not invent its own grid
  unit.
- **Nominal size** (e.g. "2×4", "1×1") — width × length in studs; height is
  implied by whether it's a brick/plate/tile, not part of the nominal name.
- **Jumper plate** — a plate whose studs are offset by a half-unit from its
  own footprint grid, used to break the default half-stud alignment.
- **Slope**, **wedge**, **arch**, **cheese slope** — real named part-shape
  families (not bricks/plates at all) used as actual category names in
  Rebrickable/BrickLink's own part taxonomy.
- **Technic** — the pin-and-axle-based sub-system (round cross-sections,
  functional connections), distinct from the stud-based **System** line and
  from **Duplo** (System's larger-scale, younger-child-oriented sibling
  line) — three real, separate LEGO product-line families, not marketing
  fluff.

## Rebrickable / LDraw / BrickLink data terms

These matter specifically because a future generator would pull real data
through them — get the vocabulary right here and the code that consumes it
follows naturally:

- **Part number** (a.k.a. **design ID**) — identifies a part's *mold/shape*
  only, independent of color (e.g. the mold for a 1×1 brick).
- **Color ID** — Rebrickable/BrickLink/LDraw each keep their own real,
  numbered color palettes (and a mapping between them) — "color" is a
  first-class, separately-identified real attribute, not a free-text field.
- **Element ID** — identifies one specific real *part-number + color-ID*
  combination as actually molded/sold — the thing that has a real physical
  SKU, not just a shape.
- **Set number** — identifies an official released product (a real box of
  parts); a set's real "inventory" is its list of (element ID, quantity)
  pairs.
- **LDraw** — the open, real, community-maintained CAD format/standard for
  LEGO-compatible digital models (`.ldr` model files referencing `.dat`
  part-primitive files, one real `.dat` per real part mold). This is the
  actual real geometry source a voxel-fill generator would read from, not
  something to reverse-engineer from images.
- **Rebrickable's REST API** (`/api/v3/lego/parts/`, `/colors/`, `/sets/`,
  `/minifigs/`, ...) — the real, open, documented data source this
  project's future milestone is meant to build against, in the same
  "real, public, no ToS ambiguity" category as the Big Mac Index or Alpha
  Vantage (see `TODOs.md`'s existing LEGO-price-history entry, which
  already flagged Rebrickable as the right real source over scraping a
  commercial hobbyist site).

## How this maps onto spex (for later — nothing here is built yet)

- **Confirmed: the real vector geometry for the shape already exists — no
  need to model or guess it.** LDraw (ldraw.org) is a real, open,
  actively-maintained library with a precise triangle-mesh `.dat` file for
  essentially every officially-released part (new parts added within weeks
  of release), used as the actual backbone of every major unofficial LEGO
  CAD tool (BrickLink's own Studio, LDCad, LeoCAD, MLCad). Rebrickable's own
  part numbering is closely aligned with LDraw's IDs, so "look up a part in
  Rebrickable's real API" → "load the matching real LDraw `.dat` file" is
  the intended, well-trodden path, not something to reverse-engineer.
  License terms should be re-checked directly at ldraw.org before any real
  redistribution, rather than assumed.
- **The plan: real vector geometry in, point cloud out.** Rather than
  choosing point-cloud *or* mesh permanently, use the real LDraw mesh as
  the accurate source of truth for a part's shape, then *sample its surface*
  into points (spex's existing octree/LOD pipeline only knows how to render
  points, by design — this reuses it unchanged rather than teaching the
  viewer a second, mesh-based rendering mode). A single real part (e.g. the
  smallest case, a 1×1 plate) becomes one small real-surface-sampled point
  cloud, fed through the *existing* point-cloud pipeline (`spex convert`/
  `spex serve`), not the graph pipeline (a brick's shape is a solid volume,
  not a tree).
- **Done: a first real spike** — `unibrick/gen_brick_demo.py` (see M40 in
  `TODOs.md`) proves this end to end for the smallest real case (a real
  1×1 brick, LDraw part `3005.dat`): live-fetches and recursively resolves
  the real part → subpart → shared-primitive reference tree, samples the
  real surface into colored points, writes a plain point cloud spex's
  existing pipeline renders unchanged. Verified against real brick
  dimensions and a real headless-Chromium session — see M40 for the
  concrete numbers.
- **Done: the `spex-brick-mesh` intermediate format** (see `TODOs.md`'s
  M43). The spike originally did two jobs in one pass — resolving a part's
  real LDraw geometry, and sampling it into points — every single run,
  even to just try a different color or point density. `unibrick/ldraw.py`
  (real LDraw fetch/parse), `unibrick/brickmesh.py` (resolve-once cache +
  placement/recolor), and `unibrick/sampling.py` (point sampling + baked
  lighting) now split those concerns: a part's real geometry is resolved
  *once*, cached under `unibrick/.ldraw-cache/meshes/*.json` (real
  provenance — source part number, part description, LDraw attribution —
  travels with it), then reused for different colors, point densities, or
  (see `gen_monolith_demo.py`) multiple placements in an assembly without
  re-walking LDraw's real reference tree. `spec/brickmesh.schema.json` is
  the formal spec, `unibrick/brickmesh.py`'s `validate_mesh()` the real
  structural check run against actual generated output (see
  `spec/README.md`'s note on why this one isn't in the Rust test suite —
  it's a Python-only cache format, no Rust reader exists for it yet).
- **Done: the `spex-brick-scene` format — real assemblies sourced from a
  real LDraw model file, not hand-written** (see `TODOs.md`'s M44).
  `unibrick/brickscene.py` parses a real, official LDraw *model* file's own
  type-1 placement lines directly (`https://library.ldraw.org/library/
  official/models/car.ldr`/`pyramid.ldr` — real official sample models
  authored by James Jessiman, LDraw's original creator) into a flat list of
  real `(part, color, translation, rotation matrix, build-step)`
  placements — the actual "build instructions" half of this idea, as
  opposed to `gen_monolith_demo.py`'s hand-written stack. `gen_model_demo.py`
  resolves each *distinct* referenced part exactly once via
  `brickmesh.get_or_resolve_mesh` (car.ldr's 61 real placements are only 26
  distinct real parts) and places each real occurrence at its own real
  position *and* rotation (`brickmesh.place_mesh` gained real 3x3-matrix
  support for this — car.ldr genuinely rotates placed parts, e.g. its
  wheels, not just translates them). `spec/brickscene.schema.json` is the
  formal spec. **A real, checked licensing caveat**: unlike individual part
  files (each explicitly `CCAL 2.0`-licensed), ldraw.org's official
  `models/` sample files carry no such header and the Legal Info page
  doesn't address them the same way — treated as unconfirmed, not assumed,
  same as this project's standing rule; see `brickscene.py`'s docstring.
  **A real forcing case for a genuine rate limit**: resolving car.ldr's 26
  distinct parts (each needing its own subpart/primitive fetches) via
  live per-file HTTP requests hit ldraw.org's real HTTP 429 rate limit —
  fixed two ways: `ldraw.fetch()` now retries with real exponential
  backoff (same pattern as the Wikipedia-crawl adapter's own 429 fix), and
  a real, once-off local mirror of the *entire* official library
  (`complete.zip`, ~136MB, downloaded via `ldraw.download_library_zip()`
  to the gitignored `.ldraw-cache/`, never committed/uploaded/git-lfs'd)
  lets `fetch()` read any real file straight out of the local archive with
  zero network requests at all when it's present.
- **Done: the real monolith assembly reveal animation** (see `TODOs.md`'s
  M45) — the "floats a whole series of Klemmbausteine into the perfect
  monolith" idea floated earlier in this project's own discussion, now
  actually built: `unibrick/gen_monolith_assembly.py` shows the same 9 real
  parts from M41's static monolith starting scattered/floating apart and
  converging into the finished stack. Deliberately built as a standalone,
  self-contained HTML artifact (embedded JSON frame data + a hand-rolled
  canvas 2D point renderer, no three.js/WebGL dependency) rather than a
  live-interpolated feature inside spex's actual WebGL viewer/tileset
  pipeline — animating real per-point positions there would need the
  octree/tileset format itself to track point identity across frames, a
  real, much bigger and riskier change to a shared, heavily-tested core
  format used by every existing demo. Each output point's local sampled
  position and baked shading are computed *once* (translation is the only
  thing that changes per frame — a triangle's normal doesn't move when only
  its parent part translates), so points move smoothly frame to frame
  instead of shimmering from independent resampling. Real mouse-drag
  orbit + scroll zoom + a slow auto-rotate (paused while dragging) built by
  hand for this same reason (no shared-viewer-code dependency to keep this
  fully standalone).
- A true mesh/vector renderer (crisp catalog-quality edges, rendering
  LDraw's real triangle faces directly instead of sampling them into
  points) is a real, deliberately bigger alternative for later, if the
- A true mesh/vector renderer (crisp catalog-quality edges, rendering
  LDraw's real triangle faces directly instead of sampling them into
  points) is a real, deliberately bigger alternative for later, if the
  point-cloud look ever feels too soft for what's wanted — a genuine second
  rendering mode alongside the point pipeline, not a small addition.
- Real geometry only: no fabricated brick shapes or invented color IDs —
  the same standing rule as every other spex demo.
