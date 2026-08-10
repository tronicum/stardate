# Phase 4 - the construction kit, the Atlas and the flags (M72-M77)

*Grid-legal parametric brick building, the World Heritage Atlas, and flags as brick mosaics.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**
Read [`README.md`](README.md) first: it carries the working rules, the scope decision and the milestone index.
Review record and the reasoning behind every rev-2 change: [`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md).

---


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
pub struct Mosaic   { pub cells: Vec<Vec<Option<u32>>>, pub tile_part: String }  // color per cell; None = a real hole (lattice)

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

**Verification ladder.** 1, 2, 3, 4. (6 runs at the end of the phase.)
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

**Verification ladder.** 1, 2, 3, 4, 5 (**mandatory** — this milestone changes the picture). (6 runs at the end of the phase.)
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

1. A 48×36 Dannebrog with 12 flags on screen simultaneously: record the
   instance count, the draw calls and the per-frame wave-evaluation cost.
   **Frame rate is measured in M92, not here** — there is no GPU in this
   pipeline, and a number from a software rasteriser would say nothing about
   the real thing.
2. Lighting responds to the wave (the surface visibly catches the key light
   as it turns) — verified in a 60-frame capture, not asserted.
3. `windStrength = 0` produces a perfectly flat flag, bit-identical to the
   static mosaic.
4. No tile ever separates from its neighbours by more than one stud width
   (the mosaic must read as cloth, not as confetti) — measured.

**Verification ladder.** 1, 2, 3, 5 (**mandatory** — this milestone changes the picture). (6 runs at the end of the phase.)
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
