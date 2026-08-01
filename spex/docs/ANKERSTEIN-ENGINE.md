# ANKERSTEIN-ENGINE.md — implementation spec, M98–M108

Working title: **the Ankerstein spike** — a real, vector-accurate renderer for Richter's Anchor Stone Building Sets, built inside `spex`, reusing the point-cloud pipeline and the `spex-ldraw` crate's own architecture as its template rather than inventing a new one.

Archive signature: IA-2026-003 (follows `docs/FUGEN-ENGINE.md`'s IA-2026-002).
Spec written: 2026-08-01.
Audience: the agent session implementing this. Read `AGENTS.md`, `CLAUDE.md`, `TODOs.md`, `BRICKs.md`, `docs/agents/` and `docs/FUGEN-ENGINE.md` first — this spec assumes all of them, and deliberately copies FUGEN-ENGINE.md's phase/milestone shape rather than reinventing a format.

**Milestone numbering is provisional.** `docs/FUGEN-ENGINE.md` runs through M97 across a 26-week plan that may still be in flight when this work starts. M98–M108 here is a placeholder sequence — renumber against whatever `TODOs.md` actually shows as the next free milestone at the time this is picked up, don't just paste M98 in blind.

## 0. Why this exists

Concept note (see `ankerstein-concept.md` in the companion planning session): the Stein Saga's own arc runs raw stone → Ankerstein (the first systematized, gridded stone-building-toy, 1880) → Lego/Klemmbaustein (`spex-ldraw`, M40–M50, plastic, injection-molded). Ankerstein is the missing middle chapter, both narratively and technically — same point-cloud pipeline, different material and shape vocabulary, and a real historical toy (Rudolstadt, Gustav Lilienthal, Friedrich Adolf Richter) rather than a fictionalized one.

This spec covers only the rendering pipeline (catalog → geometry → assembly → cinematic). Story/beat decisions live in the concept note, not here.

## 0a. A second, explicit goal: an open interchange format worth sharing back

Beyond rendering, the user's own framing is worth stating as a real goal,
not just an incidental side effect: since AnkerPlan is closed/CVA-restricted
and AnkerCAD's data license is unconfirmed (§1), a genuinely open,
versioned, JSON-Schema'd stone + set format (`spec/ankerstein-shapes.schema.json`,
`spec/ankerstein-sets.schema.json`) is something the wider Ankerstein fan
community (CVA, AnkerWiki, fredhartjes-home.nl) could plausibly reuse
directly, independent of `spex` itself — the "LDraw for Ankerstein" that
doesn't currently exist. Keep both schemas self-contained and readable
without needing the rest of this repo, the same discipline `spec/README.md`
already holds every other schema here to, so that stays realistic rather
than aspirational.

## 1. The one real asymmetry versus `spex-ldraw`, and the licensing decision it forces

`spex-ldraw` works because LDraw is a mature, open, CC-BY-2.0-licensed, community-maintained parts library — `resolve_part()` just fetches and recursively resolves an existing `.dat` file. **No equivalent exists for Ankerstein**, and the two CAD tools that do exist are not simply usable:

- **AnkerPlan 2** (Michael Erhard + Andreas Rhodin) — proprietary, restricted to Club van Ankervrienden (CVA) members. Not a usable data source without CVA membership and explicit permission; the user has flagged this directly (see `ankerstein-technical-plan.md`'s revision). **Do not scrape, decompile, or otherwise extract AnkerPlan's bundled stone definitions.**
- **AnkerCAD** (Anders Isaksson's BlockCAD fork, stone definitions by Burkhard Schulz) — genuinely freeware, publicly downloadable (`blockcad.net/ankercad.htm`), with a linked stone-definitions page. More promising than AnkerPlan, but its exact license terms for the *stone definition data itself* (as opposed to the program binary) were not confirmed as part of this research pass — treat as **unconfirmed, not assumed covered**, exactly the same posture `BRICKs.md`/`spex-brick-scene`'s docstring already takes toward LDraw's unlicensed `models/` folder. Read the actual stone-definition file format before deciding whether to parse it; if its license is silent or restrictive, don't ingest it wholesale — extract only individual real dimensions that can be independently corroborated against a publicly citable source (see below), the same "real number, real citation" bar `BRICKs.md` holds LDraw/Rebrickable data to.
- **Public, citable dimension sources that are safe to build the catalog from directly**: the CVA's own historical reference book (`ankerstein.ch/downloads/CVA/Book-PC.pdf`), George Hardy's *Richter's Anker (Anchor) Stone Building Sets* (English/German), and Fred Hartjes' summary page (`fredhartjes-home.nl/Anker.html`) — all public, all citable, all already used for the two companion planning docs. **The shape catalog (§3) should be hand-authored from these, not parsed from AnkerCAD**, until AnkerCAD's data license is explicitly confirmed with its maintainers.

**Action before M98 starts**: send Burkhard Schulz / the AnkerCAD maintainers a real licensing question (mirroring the "check ldraw.org's Legal Info page directly, don't assume" discipline `BRICKs.md` already models) — do this once, document the answer in this file's changelog, and only then decide whether AnkerCAD's stone-definition format becomes a real second data source alongside the hand-authored catalog.

## 2. Real dimensions confirmed so far (seed data, not the full catalog)

Sourced and cross-checked across the research pass (ankerstein.ch, Fred Hartjes, Grokipedia's summary of the CVA book):

| Shape | Dimensions (mm) | Caliber | Source note |
|---|---|---|---|
| Full cube | 25 × 25 × 25 | GK (Großes Kaliber) | base unit of the GK grid |
| Half-height block | 25 × 25 × 12.5 | GK | explicitly pictured/measured on Fred Hartjes' page |
| Full cube (small caliber) | 20 × 20 × 20 | KK (Kleines Kaliber) | base unit of the KK grid, a separate, non-interchangeable scale from GK |
| Brick-shaped block | 1 × 2 × 4 units | GK | introduced in the historical 4th set |

Everything else (prisms, arches, columns, the fractional 0.5/0.25/0.125/1.25/2.5/4 multiples Fred Hartjes' page mentions, 1000+ total shapes by the later sets) is real but not yet reduced to exact millimeter tables in this research pass — **M98 must pull these from the CVA Book PDF page by page, not guess proportions**, the same standard `TODOs.md`'s own milestones hold every adapter to ("real data only").

## 3. Crate: `spex-ankerstein`

Module-for-module mirror of `spex-ldraw`'s shape (see `crates/spex-ldraw/src/{lib,cache,colors,geometry,sampling,scene}.rs` for the exact pattern being followed):

- **`catalog.rs`** (replaces `spex-ldraw`'s `cache.rs` — no network fetch needed; a hand-authored, versioned, in-repo data file instead). Defines `AnkersteinShape { id, shape_type, dimensions_mm: [f64;3], caliber: Caliber, source_citation: String }` and `load_catalog() -> Vec<AnkersteinShape>`, reading `spex/data/ankerstein-shapes.json` (or `include_str!`'d at compile time — decide based on whether the catalog needs runtime hot-editing; `spex-ldraw` has no equivalent decision to copy since its data is always fetched). **Every entry's `source_citation` field is mandatory, not optional** — this is the concrete mechanism that makes "real data, not fabricated" enforceable in code review, not just a doc convention.
- **`colors.rs`** — static three-entry table (brick red / cement yellow / slate blue-grey), real RGB values to be picked from an actual photographed Ankerstein (a real hex sample, not an invented "brick red"-sounding value) — same rigor `spex-ldraw::colors` applies to `LDConfig.ldr`, just without a file to parse.
- **`geometry.rs`** — parametric solid generation (box, wedge/prism, arch voussoir as a cylindrical wedge segment) instead of `spex-ldraw::geometry`'s LDraw-triangle resolution, since there's no mesh file to resolve. Emits the same `Triangle { vertices: [[f64;3];3], color_code }` shape `spex-ldraw` uses, so **`sampling.rs` can be reused with zero changes** by depending on `spex-ldraw`'s `sample_surface`/`shade_color`/`to_point_cloud` directly rather than reimplementing them — check whether those functions are already crate-public enough to import, or need a small visibility change in `spex-ldraw` (a one-line, additive change, not a rewrite).
- **`sets.rs`** — the real "set/inventory" layer: which stones, and how many of each, make up a given historical or modern set ("Nr. 5A", "GK 2", "2A", ...) — distinct from `catalog.rs` (the set-independent stone/part library) and from `scene.rs` (one specific build). Mirrors LDraw/Rebrickable's own part-vs-set-vs-model split (see `BRICKs.md`'s "Set number" entry). Formal schema: `spec/ankerstein-sets.schema.json`. `data/ankerstein-sets.json` starts as an intentionally empty `[]` — scaffolded ahead of any real set data at the user's own request (real physical Nr. 5A / "2½" / a third set, purchased 2026-08-01, plus a large existing CVA/AnkerWiki fan-community literature), rather than seeded with a fabricated-looking placeholder entry. `validate_against_catalog()` checks every set's referenced shape ids actually exist in the shape catalog — the cross-file consistency check the two schemas deliberately don't enforce via JSON Schema `$ref` (this project's self-contained-schema convention).
- **`scene.rs`** — placement format for an assembled structure: `{ shape_id, translation, rotation_y_degrees }` list, deliberately simpler than `spex-ldraw::scene`'s full 3×3-matrix LDraw parsing (Ankerstein assemblies only need translation + Y-axis rotation for the milestones below; add full-matrix support later only if a real design needs a non-Y rotation, e.g. a sloped roof stone laid at an angle — don't build it speculatively).

`Cargo.toml`: depends on `spex-core` and `spex-ldraw` (for the reused sampling functions) plus `serde`/`serde_json` for the catalog file — no `ureq`/`zip` (no network fetch, unlike `spex-ldraw`).

## 4. Milestones

| # | Name | What it proves |
|---|---|---|
| **M98** | Real shape catalog, seed set | `data/ankerstein-shapes.json` with the 4 shapes in §2 above (real, cited dimensions) plus the real 3-color table (real RGB sampled from an actual photo, cited). `spex ankerstein-part <id> -o <dir>` renders one shape through `spex convert`+`spex serve`. Verification: bounds match the cited mm dimensions exactly (same "real bounds match real known dimensions" check M40 used for the 1×1 LDraw brick). |
| **M99** | Prism/wedge geometry | Add sloped-block generation (the 5th historical set's prism shapes) — real angle values from the CVA Book PDF, not guessed. Verification: rendered silhouette via `spex ascii` visibly reads as a wedge, not a box. |
| **M100** | Scene/assembly format | `scene.rs` + `spex ankerstein-model <scene.json> -o <dir>`, a real small stacked structure (e.g. a plinth or short wall) using only M98/M99's shapes. Reuses `spex-tiler`/`spex-server` unchanged, same as every `spex-ldraw` milestone. |
| **M101** | Arch geometry | The hardest real shape: a Roman-arch voussoir set, real span/rise proportions sourced from the CVA book (Fred Hartjes' page confirms "various sizes of roman arcs and gothic arcs" existed — pull the actual numbers, don't invent a generic arch). This is the milestone most likely to need the AnkerCAD-licensing decision from §1 resolved first, since arch geometry is the least likely to be fully spelled out in prose-form historical sources. |
| **M102** | A real assembled arch or short colonnade | `ankerstein-model` on a scene combining M98–M101's shapes into the concept note's "actual architectural form" beat. Verification: a real headless-Chromium session (per `docs/agents/verification.md`'s ladder, rung 5) confirming the arch visually closes (no gap at the keystone) from multiple camera angles. |
| **M103** | `spex ankerstein-cinematic` | The hero shot: single-block spin (reusing `spex-ldraw::geometry::rotation_y`, already crate-public) → pull back into the M102 assembly, staged as this chapter's counterpart to `spex brick-cinematic`'s spin-then-cut-to-monolith beat. Exact timing/staging is a story decision, not an engineering one — coordinate with whoever owns the concept note before locking parameters. |
| **M104** | Gallery + docs wiring | `spex demos`/`spex gallery` list the new demos alongside the existing brick ones; `ANKERSTEINE.md` (mirrors `BRICKs.md`'s structure: community/history terms, real geometry terms, licensing status, "how this maps onto spex") added at repo root. `TODOs.md` gets real M98–M104 entries in the established style (see the M40–M50 entries for the level of concrete verification detail expected — "real bounds matched X" not "looks about right"). |
| **M105** | AnkerCAD licensing follow-up (conditional) | Only if §1's outreach got a clear answer by this point: either wire `AnkerCAD`'s real stone-definition file as a second, richer catalog source (with a `spex-brickscene`-style unconfirmed-license caveat if the answer was ambiguous, exactly like the LDraw `models/` precedent), or explicitly close this as "asked, declined/no answer, sticking with the hand-authored catalog" — don't leave it silently open. |
| **M106** | Caliber correctness pass | Confirm every catalog entry is internally consistent within its own caliber (GK entries don't accidentally mix in a KK dimension) — a real correctness check, not a new feature; write it as a unit test over `load_catalog()`'s output, not a one-off manual read-through. |
| **M107** | Full `cargo test --workspace` + `walkthrough.sh` gate | Same final-gate discipline `TODOs.md`'s M46–M50 entry describes for the brick pipeline's Rust port. |
| **M108** | Retrospective doc pass | Update `CLAUDE.md`'s crate-layout section with a `spex-ankerstein` paragraph (mirroring the existing `spex-ldraw` one), same standard `docs/FUGEN-ENGINE.md`'s M96 sets for its own phase. |

## 5. Verification requirements (per `docs/agents/verification.md`'s ladder)

Every milestone above needs, at minimum: clean `cargo build --release`, full `cargo test --workspace`, and a real functional CLI check (rung 3). M102/M103 (viewer-visible) additionally require rung 5 (real headless-Chromium, screenshot inspected, not assumed). Do not skip rung 5 for the cinematic milestone specifically — it's the one shot most likely to look wrong (wrong pivot point, wrong pacing) in a way no unit test catches, exactly the class of bug M45's monolith-assembly work found and fixed only via a real screenshot.

## 6. Explicit non-goals for this spec

- **Not** attempting to reproduce AnkerPlan's or AnkerCAD's UI/editing experience — this is a renderer for pre-authored scenes, not an interactive Ankerstein CAD tool.
- **Not** attempting all ~1000+ historical shapes — the catalog grows shape-by-shape, each with a real citation, as specific milestones need them. A "someday, complete catalog" item belongs in `TODOs.md`'s backlog, not this spec's milestone list.
- **Not** modeling both KK and GK calibers to scale simultaneously in one scene (per the concept note's own recommendation) — pick GK (the better-documented, larger caliber) as the default unless a later story beat specifically needs KK.

## Changelog

- 2026-08-01 — initial spec written, following the concept-note/technical-plan planning pass and a licensing correction from the user (AnkerPlan 2's stone definitions are CVA-members-only, not open) that reshaped §1.
