# spex machine-readable formats

Formal specs for every JSON file spex reads or writes, so the output is
reusable by other tools without reverse-engineering it from source. Each
`.schema.json` is a self-contained JSON Schema (2020-12, no external `$ref`s)
— validate any real file with a standard validator, e.g.:

```sh
npx ajv-cli validate -s spec/graph.schema.json -d demos/decix-trace/graph.json
```

These are also validated in CI-equivalent fashion by
`crates/spex-cli/tests/schema_validation.rs`, which generates a real
graph/tileset/nodes/meta set and checks each against its schema — so this
isn't just aspirational prose, it's enforced against real output.

## Files

| File | Schema | Produced by | Consumed by |
|---|---|---|---|
| `graph.json` | [`graph.schema.json`](graph.schema.json) | any input adapter (`trace`, `ps-tree`, `brew-deps`, `cargo-deps`, `npm-deps`, `deb-deps`, `sql-schema`, `disk-usage`, `molecule`, `pstree-demo`) | `graph-print`, `graph-layout`, `graph-diff` |
| `tileset.json` | [`tileset.schema.json`](tileset.schema.json) | `spex-tiler` (both pipelines) | `spex-server`, viewer |
| `nodes.json` | [`nodes.schema.json`](nodes.schema.json) | `graph-layout` only | viewer (hover labels) |
| `meta.json` | [`meta.schema.json`](meta.schema.json) | `graph-layout` only | viewer (header/legend) |
| `sequence.json` | [`sequence.schema.json`](sequence.schema.json) | `spex frame-sequence` (`crates/spex-cli/src/frame_sequence.rs`) | viewer (`fetchSequence`, real frame-advance playback) |
| `mesh.json` | [`mesh.schema.json`](mesh.schema.json) | `spex-mesh` (`crates/spex-mesh/src/bundle.rs`) | the viewer's mesh render mode (M54) |
| `show.json` | [`show.schema.json`](show.schema.json) | hand-authored (`shows/*.show.json`) | `spex-show` (`crates/spex-show/src/model.rs`), `spex show-build` |
| `show-resolved.json` | [`show-resolved.schema.json`](show-resolved.schema.json) | `spex show-build` (`crates/spex-cli/src/show.rs`) | the runtime show engine (M62+) |

`octree/<node-id>.bin` (the point data itself) is a small binary format, not
JSON — see the "Tileset format" section of `CLAUDE.md`: `u32` LE point
count, then per point `3x f32` LE position + `3x u8` RGB (15 bytes/point).

A mesh bundle's payload is binary for the same reason, and more so. Alongside
`mesh.json` sit `buffers/p<N>.{pos,nrm,idx,edge,cond}.bin` (packed f32/u32
arrays, sizes derivable from the manifest's own counts) and `instances.bin`
(**10 bytes per instance**: `i16 x, y, z` in LDraw units, `u8` orientation
index, `u8` material index, `u16` part index). At Atlas scale a JSON
`instances[]` array would be ~37 MB of text and about a second of main-thread
parse before the first frame; this is 2.5 MB and needs no parsing at all.

Two properties of `mesh.json` are load-bearing enough that the manifest
states them rather than leaving readers to assume: positions are in
**millimetres, +Y up** (LDraw is LDU and Y-down, and because negating Y is a
*mirror*, the writer also reverses triangle winding — without which backface
culling is inverted for the entire library), and colours are **linear**, not
sRGB, because three.js r152+ reads them as linear.

## Versioning

`tileset.json` has an explicit `version` field (currently `1`) since
`spex-tiler` is the most likely of these to gain a breaking format change
(compression, out-of-core support — see `TODOs.md`). The graph-pipeline
files (`graph.json`, `nodes.json`, `meta.json`) don't version yet; treat
them as v0/unstable until this note is removed. `sequence.json` also has
an explicit `version` (currently `1`) for the same reason, since it's the
youngest format here.

## A note on `sequence.json`

`sequence.json` is a real Rust-workspace format — written by the real
`spex frame-sequence` command and read by the real viewer
(`viewer/src/tileset.ts`'s `fetchSequence`) to play back a real
multi-frame point-cloud animation (each `frame-NNN/` alongside it is an
ordinary tileset, sharing one coordinate offset via
`spex_tiler::build_with_offset` so the viewer can swap between them
without the point cloud's position jumping — see that function's own doc
comment). It's checked by `crates/spex-cli/tests/schema_validation.rs` the
same way `tileset.json` etc. are, not just documented in prose.

## The two show formats

`show.json` is *authored*; `show-resolved.json` is *compiled*. They are
deliberately not the same document with optional fields.

The authored one states time in **bars**, keyframes in **normalised
shot-local time** (0..1), targets as **globs** over instance ids, and shot
durations as a weight plus a min/max range. None of that is playable: it
describes a piece that can be resolved to 4:00, 10:00, 60:00 or endless.
`spex show-build` picks one duration and one seed and resolves all of it —
absolute seconds, expanded instance-index lists, integer repeat counts,
tier-filtered shot list — so that nothing is left to decide per frame. A
player that resolves anything at play time is a player that can drift, and
four cuts resolved independently would be four edits rather than four
readings of one document.

Bars, not seconds, in the authored form: the piece is written at 84 bpm in
4/4 and requires every cut to land on a bar line. One bar is 20/7 s, so in
seconds that rule cannot be stated exactly — the canonical cut is 84 bars,
which is 240.000 s only because 84 x 20/7 is.

`shows/die-geschichtliche-matrix.show.json` is the real document (Act I so
far; the remaining acts are authored in Phase 5). It and a deliberately
minimal one are validated against the schema by
`crates/spex-show/tests/documents.rs`, which also checks the document
against `spex_show::model` — a format with two readers needs both to agree.
Real `spex show-build` output is validated against the resolved schema by
`crates/spex-cli/tests/schema_validation.rs`, which also asserts that two
runs at the same seed are byte-identical.

## Notes for anything reading these directly

- `graph.json`'s `GraphNode.parent` models trees/forests only — a single
  optional parent id, not a general edge list. See `docs/ARCHITECTURE.md`
  for why.
- Coordinates in `tileset.json` and `nodes.json` are in the *same*
  offset-relative frame — `nodes.json[].center` and `tileset.json`'s
  `bounds`/`octree/*.bin` positions are directly comparable without any
  further transform.
- A fan-out-capped tree (see `docs/ARCHITECTURE.md`) introduces synthetic
  nodes with ids like `<parent-id>__more` and a `metadata.collapsedCount` —
  these aren't in the original `graph.json`, only in `nodes.json`, since
  they're an artifact of layout, not capture.
- Optional fields that are absent from a file (rather than explicitly
  `null`) parse the same way — e.g. a hand-written `graph.json` can omit
  `title`/`metric_label` entirely, or omit a node's `metric`/`metadata`.
