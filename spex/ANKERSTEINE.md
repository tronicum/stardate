# ANKERSTEINE.md — Ankerstein domain glossary

Shared vocabulary for `spex-ankerstein` (see `docs/ANKERSTEIN-ENGINE.md` for
the full implementation spec) — the real, historical counterpart to
`BRICKs.md`'s Klemmbaustein/LDraw glossary, covering Richter's Anchor Stone
Building Sets (Richters Anker-Steinbaukasten), Rudolstadt, 1880.

## History terms

- **Ankerstein / Anchor Stone** — the mineral building-stone material
  itself: baked quartz sand, chalk, and raw linseed oil, pressed and cured
  at 100–150°C, colored with real quarry pigments to imitate red brick, tan
  limestone, and blue-grey slate.
- **Gustav Lilienthal** — inventor of the material (brother of aviator Otto
  Lilienthal); went bankrupt trying to commercialize it.
- **Friedrich Adolf Richter** — bought the method/equipment from Lilienthal,
  patented the stone-making process in 1880, and built the actual
  Rudolstadt (Thüringen) factory and product line.
- **Neue Folge (NF)** — the main historical set-numbering series (e.g. NF6,
  NF34); higher numbers mean more/larger stone sets, reachable by buying
  supplement sets rather than starting over.
- **VE/DS (Vernickeld Eisen / Dach Steine)** — a separate, smaller set
  series enhanced with nickel-plated (VE) or lacquered (DS) metal parts for
  roofs/bridges.
- **CVA (Club van Ankervrienden)** — the real international collectors'
  club (~250 members, meets in Hilversum, NL); runs `ankerstein.center` and
  the BiK design-review committee.
- **BiK (Baukunst im Kleinen)** — the CVA committee that reviews new
  building designs for a specific stone set before publishing them as
  official design booklets.

## Real geometry & measurement terms

- **GK (Großes Kaliber)** — the large caliber grid, 25mm base cube. The
  default caliber for this crate (`spex_ankerstein::GK_UNIT_MM`).
- **KK (Kleines Kaliber)** — the small caliber grid, 20mm base cube. A
  separate, non-interchangeable scale from GK — never mixed within one real
  historical set (`spex_ankerstein::KK_UNIT_MM`).
- **Voussoir** — one wedge-shaped stone in an arch, the real term this
  project uses for `ShapeType::ArchVoussoir` (see M101 in
  `docs/ANKERSTEIN-ENGINE.md` — not yet implemented).
- **The three canonical colors** — brick red, cement/ochre yellow, slate
  blue-grey, matching the real quarry pigments used in the baking process
  (see `spex_ankerstein::colors` — provisional RGB values, flagged for a
  real photo-sourced correction pass).

## Data/licensing terms — read before touching any external Ankerstein tool

- **AnkerPlan** — the historical/current 3D CAD tool for designing real
  Ankerstein buildings (versions 1.0–1.5 by a commercial firm; AnkerPlan 2
  rewritten by Michael Erhard + Andreas Rhodin). **Proprietary, restricted
  to CVA members.** Do not scrape, decompile, or otherwise extract its
  bundled stone definitions into this project.
- **AnkerCAD** — a real, freeware BlockCAD-family CAD program (Anders
  Isaksson, stone definitions by Burkhard Schulz), publicly downloadable.
  More promising than AnkerPlan as a future data source, but its stone
  *data* license was not confirmed as part of this project's research pass
  — treated as unconfirmed, not assumed covered, same posture `BRICKs.md`
  already takes toward LDraw's unlicensed `models/` folder. See
  `docs/ANKERSTEIN-ENGINE.md` §1 for the outreach step that should happen
  before this changes.
- **ankerstein.center** — Michael Erhard's real web platform: a stone
  catalog, personal-collection manager, and building-plan database.
  Access requires a CVA-issued login; not a source to scrape.

## How this maps onto spex

- **Confirmed real, citable dimensions exist for a small seed set** (see
  `docs/ANKERSTEIN-ENGINE.md` §2 and `data/ankerstein-shapes.json`) — the
  full historical catalog (1000+ shapes across all sets) is real but not
  yet reduced to a machine-readable table anywhere public; growing this
  catalog means reading the actual CVA Book PDF page by page, not guessing
  proportions.
- **The plan: parametric solid generation in, point cloud out** — since no
  open mesh library exists for Ankerstein (unlike LDraw for Lego), each
  catalog shape's geometry is generated directly from its real millimeter
  dimensions (`spex_ankerstein::geometry::generate_shape`), then sampled
  into a point cloud by *reusing* `spex-ldraw`'s existing
  `sample_surface`/`shade_color` functions unchanged (deliberately not
  `to_point_cloud`, which bakes in an LDraw-specific unit conversion — see
  `crates/spex-cli/src/ankerstein.rs`'s own doc comment) — the same "real
  geometry in, point cloud out" principle `BRICKs.md` describes for the
  Klemmbaustein pipeline, just with a different geometry source.
- **Gallery wiring (M104)**: `spex gallery`/`spex demos` need no
  Ankerstein-specific code — they're already directory-shape-agnostic (see
  README.md's "Point-cloud pipeline: bricks and Ankerstein" section).
  Rendered `ankerstein-part`/`ankerstein-model` output under `out/` shows up
  in `spex gallery out` alongside brick output automatically.
