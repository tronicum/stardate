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
| `.ldraw-cache/meshes/*.json` (a `spex-brick-mesh`) | [`brickmesh.schema.json`](brickmesh.schema.json) | `unibrick/brickmesh.py` (`get_or_resolve_mesh`) | `unibrick/gen_brick_demo.py`, `unibrick/gen_monolith_demo.py` |

`octree/<node-id>.bin` (the point data itself) is a small binary format, not
JSON — see the "Tileset format" section of `CLAUDE.md`: `u32` LE point
count, then per point `3x f32` LE position + `3x u8` RGB (15 bytes/point).

## Versioning

`tileset.json` has an explicit `version` field (currently `1`) since
`spex-tiler` is the most likely of these to gain a breaking format change
(compression, out-of-core support — see `TODOs.md`). The graph-pipeline
files (`graph.json`, `nodes.json`, `meta.json`) don't version yet; treat
them as v0/unstable until this note is removed. `brickmesh.json` also has
an explicit `version` (currently `1`) for the same reason — it's the
youngest of these formats and the most likely to grow a placement/rotation
field once a real multi-orientation assembly needs one (see BRICKs.md).

## A note on `brickmesh.json`

Unlike the four files above, `spex-brick-mesh` files aren't produced or
consumed by the Rust workspace at all — they're a private cache internal to
`unibrick/`'s Python scripts (`unibrick/.ldraw-cache/meshes/*.json`, itself
gitignored, same as the rest of `.ldraw-cache/`), not part of the tileset a
demo ships. It's spec'd here anyway, per this project's "everything gets a
formal schema once its shape is settled" rule (see `BRICKs.md`), and
because a future true mesh/vector renderer (a real, bigger alternative to
point-cloud sampling, see `BRICKs.md`) would consume this same format
directly. It isn't checked by `crates/spex-cli/tests/schema_validation.rs`
(no Rust reader exists yet) — `unibrick/brickmesh.py`'s `validate_mesh()` is
the real structural check actually run against generated output today.

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
