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

`octree/<node-id>.bin` (the point data itself) is a small binary format, not
JSON — see the "Tileset format" section of `CLAUDE.md`: `u32` LE point
count, then per point `3x f32` LE position + `3x u8` RGB (15 bytes/point).

## Versioning

`tileset.json` has an explicit `version` field (currently `1`) since
`spex-tiler` is the most likely of these to gain a breaking format change
(compression, out-of-core support — see `TODOs.md`). The graph-pipeline
files (`graph.json`, `nodes.json`, `meta.json`) don't version yet; treat
them as v0/unstable until this note is removed.

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
