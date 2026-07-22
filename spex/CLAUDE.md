# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`spex` is two things layered on one pipeline:
1. A literal point cloud explorer — converts LiDAR/scan files (PLY/XYZ/CSV) into a streamable octree tileset viewable in the browser.
2. A generic tree/graph explorer — "input adapters" (traceroute, process tree, package deps, ...) capture real-world trees into one common JSON format, which gets laid out in 3D and rendered through the *same* point-cloud pipeline as #1. There is no adapter-specific viewer code; everything becomes points.

See `docs/ARCHITECTURE.md` for the narrative version of this — a worked example with real data and the reasoning behind the non-obvious design choices (radial layout, the fan-out cap, why three views of one model). This file stays terse/reference-only. See `spec/README.md` for the formal JSON Schema of every file in the "Tileset format" section below — validated against real generated output by `crates/spex-cli/tests/schema_validation.rs`, not just documented in prose.

## Commands

Build/test (Rust workspace):
- `cargo build --release` — builds the `spex` CLI (`crates/spex-cli/src/main.rs`, binary name `spex`, output at `target/release/spex`)
- `cargo test --workspace` — run all unit tests across every crate
- `cargo test -p <crate>` — test a single crate (e.g. `spex-graph`, `spex-cli`)
- Requires a reasonably recent Rust toolchain (1.9x+). Homebrew's older Rust (1.75) hits `edition2024` MSRV errors on some dependencies — `brew upgrade rust` if you see that.

Viewer (`viewer/`, TypeScript, embedded into the Rust binary at compile time):
- `cd viewer && npm install` (first time)
- `cd viewer && npm run dev` — Vite dev server; proxies `/tileset` requests to a `spex serve` instance on `:8080`, so start one first
- `cd viewer && npx tsc --noEmit` — typecheck only
- `cd viewer && npm run build` — typechecks then builds to `viewer/dist/`. **You must rebuild the Rust workspace after this** (`cargo build --release`) to pick up the new bundle — `spex-server` embeds `viewer/dist` via `rust-embed` at Rust compile time, not at runtime.
- `vite` is pinned to `^5` (not the current major) for compatibility with older Node versions — check `viewer/package.json` before bumping it.

Point cloud pipeline (literal point clouds — `.ply`/`.xyz`/`.csv`/`.txt`):
```
spex info <file>                          # point count/bounds, no conversion
spex convert <input> -o <tileset-dir>     # build octree tileset
spex serve <tileset-dir>                  # serve + open browser
```

Graph/tree pipeline (traceroute, processes, packages, ...):
```
spex trace <host> -o <graph.json>                  # real traceroute
spex ps-tree [--root <pid>] -o <graph.json>        # real process tree via `ps`; --root scopes to one subtree
spex brew-deps <formula> -o <graph.json>           # real `brew deps --tree`
spex sql-schema <db-file> -o <graph.json>          # real SQLite schema via `sqlite3`: tables + FKs + row counts
spex pstree-demo -o <graph.json>                   # fabricated example tree (offline/synthetic fallback only)

spex graph-print <graph.json>                      # terminal view: ASCII tree, metric + metadata inline
spex graph-layout <graph.json> -o <tileset-dir>    # 3D layout -> octree tileset + nodes.json + meta.json
spex serve <tileset-dir>                           # same web viewer; hover a blob for its label/metric
spex demos [dir=demos]                             # list demos/<name>/{graph.json,tileset/} + view commands for each
spex gallery [dir=demos] [--port]                  # web front page: every demo as a clickable card, browse at /d/<name>/
spex export-static [dir=demos] -o <output-dir>     # same gallery, written as plain static files (no server) for e.g. GitHub Pages
spex nav [dir=demos]                               # k9s-style interactive browser: move/enter/w(eb)/q, no browser needed to explore
spex ascii <tileset-dir> [--width]                 # colored ASCII-art snapshot of a tileset's real points, no browser needed
```
Demo artifacts live under `demos/<name>/{graph.json, tileset/}` by convention — distinct from `in`/`out`, which are for literal point cloud files.

## Architecture

### Crate layout (`crates/`)
- **spex-core** — shared primitives: `Point` (position+color), `Aabb`, octree node-id helpers (`"r"`, `"r0"`, `"r03"`, ...). No I/O.
- **spex-io** — point cloud file readers (hand-rolled PLY ascii/binary_little_endian parser, XYZ/CSV), dispatched by extension in `read_points()`.
- **spex-graph** — the tree/graph abstraction layer, and the one crate every adapter and every view depends on: `Graph` (`title`/`metric_label` describing the whole graph, plus `nodes: Vec<GraphNode>`) / `GraphNode` (`id`/`label`/`parent`/`metric`/`metadata` — trees/forests only, no cycles or multiple parents), `format_tree()` (the terminal ASCII view: header, TTY-aware ANSI-colored tree, summary footer), and `layout::build()` (radial 3D layout → `Vec<Point>` + per-node `LayoutNodeInfo`).
- **spex-tiler** — builds the octree tileset from a flat `Vec<Point>`, used identically by both pipelines: recursively partitions by bounding-box octant, reservoir-samples a fixed point budget per node for that LOD level, writes non-overlapping `octree/<node-id>.bin` files + a `tileset.json` manifest. `build()` returns the coordinate offset it subtracted — callers with other data in the same original coordinate space (e.g. spex-graph's node centers) need it to stay aligned with the tileset's points. `read_points()` is the read-side counterpart — reads any tileset back into a flat `Vec<Point>` (used by `spex ascii`).
- **spex-server** — axum server. Single-tileset mode (`build_router`): serves a tileset directory as static files, falls back to the viewer's built assets (embedded via `rust-embed` from `viewer/dist`). Gallery mode (`build_gallery_router`): one static `nest_service` per demo at `/d/<name>/tileset` (the demo set is fixed at startup — no dynamic axum path params needed), a pre-rendered gallery page (plain `format!`-built HTML, no templating dep) at `/`, same viewer-asset fallback for everything else including `/d/<name>/` itself. `render_gallery_html()` and `write_viewer_assets()` are `pub` so `spex-cli`'s `export-static` can reuse them outside a running server — all links/asset paths are relative (`d/<name>/`, `./assets/...`), so the same output works served from a domain root or a subpath (a GitHub Pages project site).
- **spex-cli** — the `spex` binary. Each adapter is its own module (`trace.rs`, `ps_tree.rs`, `brew_deps.rs`, `disk_usage.rs`, `sql_schema.rs`, `pstree_demo.rs`) that shells out to a real tool, parses its text output, and returns a `spex_graph::Graph`. `sql_schema.rs` shells out to `sqlite3` (schema/FK/row-count introspection via `PRAGMA` queries) — a table's first foreign key becomes its tree parent, tables with none are forest roots (same shared-parent limitation as `brew-deps`, see Known limitations). `nav.rs` is the `spex nav` TUI (ratatui/crossterm): list + detail view over `discover_demos()`, `w` spawns a real `spex serve` as a detached subprocess (own binary, `std::env::current_exe()`) on a deterministic per-demo port rather than running the server in-process. `ascii.rs` is `spex ascii`: a simple pinhole camera (matches the viewer's default framing) projects `spex_tiler::read_points()`'s output into a z-buffered character grid, luminance→glyph ramp + ANSI truecolor per cell (same technique as `spex_graph::display`). `export_static.rs` is `spex export-static`: writes every demo's tileset + a self-contained per-demo copy of the viewer under `<out>/d/<name>/`, plus a gallery `index.html` at the root, via `spex_server::write_viewer_assets`/`render_gallery_html` — no server involved, for hosting on a static host like GitHub Pages (see `.github/workflows/pages.yml`). `main.rs` wires clap subcommands to all of these plus the point-cloud/tileset/serve commands. `tests/schema_validation.rs` and `tests/end_to_end_journey.rs` are black-box integration tests (spawn the built binary — spex-cli has no lib target) — the latter's `run_full_pipeline()` helper is a reusable template for exercising any adapter/pipeline end to end.
- **viewer/** — Vite + TypeScript + three.js SPA (not a Cargo crate). Same bundle serves both server modes: `TILESET_BASE` is resolved from `location.pathname` (`/d/<name>/` → `/d/<name>/tileset`, otherwise plain `/tileset`). Fetches `tileset.json`, streams octree nodes via a priority-queue LOD selector (screen-space-error-ordered, frustum-culled, point-budgeted — mirrors Potree/3D-Tiles refinement), and optionally fetches `nodes.json` (hover tooltips) and `meta.json` (persistent title + color-gradient legend). `packetAnimation.ts`'s `buildPrimaryPath()` (always-first-child walk from the root) drives a marker mesh animated node-to-node along a graph's primary chain, toggleable via the "Animate packet" control.

### The two pipelines share everything downstream of "a list of points"
- Literal point clouds: file → `spex-io::read_points()` → `Vec<Point>` → `spex-tiler::build()` → tileset → `spex-server`/viewer.
- Graphs/trees: adapter → `spex_graph::Graph` (JSON) → `spex_graph::layout::build()` → `Vec<Point>` + `LayoutNodeInfo` → `spex-tiler::build()` → tileset (+ `nodes.json`, written separately by the CLI) → the *same* `spex-server`/viewer.

Adding a new adapter never touches `spex-tiler`, `spex-server`, or `viewer/` — it only needs to produce a `spex_graph::Graph`.

### Layout algorithm (`spex-graph/src/layout.rs`)
Recursive radial placement: root at the origin, each depth level a wider/higher ring (`radius = depth * RADIUS_STEP`, `z = depth * HEIGHT_STEP`), siblings splitting their parent's angular slice evenly, plus a small per-node deterministic angle jitter (seeded from an FNV hash of the node id) so linear chains spiral instead of sitting on a flat ray. Each node becomes a point-cluster "blob" (fixed point count scattered in a jittered sphere) colored by a blue→yellow→red gradient normalized over `metric`; each edge becomes a sparse, dimmed point-trail between blob centers — this is how trees are visualized with only point primitives, with no line-drawing added to the viewer.

**Fan-out safeguard**: any node with more than `MAX_CHILDREN_SHOWN` (20) children keeps only the heaviest (by `metric`, descending) and collapses the rest into one synthetic `"<parent>__more"` node labeled `"+N more"`. This is what makes real, wide data (e.g. a process tree where one process has 900+ children) render as a bounded, legible fan-out instead of an unreadable ring — it's generic to the layout, not adapter-specific, so every adapter gets the guarantee automatically.

### Tileset format
- `tileset.json` — bounds, coordinate offset, total point count, and a flat list of octree nodes (id/bounds/point count).
- `octree/<node-id>.bin` — `u32` LE point count, then per point `3x f32` LE position (relative to the tileset's offset) + `3x u8` RGB (15 bytes/point).
- `nodes.json` (graph pipeline only; absent for plain point-cloud tilesets) — per-node `{id, label, parent, center, metric, metadata}`, in the same offset-relative coordinate frame as the points, used by the viewer's hover labels.
- `meta.json` (graph pipeline only) — `{title, metricLabel, nodeCount, metricMin, metricMax}`, the whole-graph description for the viewer's persistent header/legend (mirrors what `format_tree()`'s header/footer show in the terminal).

Formal JSON Schemas for all four (`graph.json` too) live in `spec/*.schema.json` — see `spec/README.md`.

## Known limitations
- No LAS/LAZ input, no out-of-core tiling for point clouds too large for memory, no buffer compression (see `crates/spex-tiler`).
- `Graph` models trees/forests only — no cycles or shared parents, so `brew-deps` duplicates a package under every branch that depends on it rather than merging it into one node, and `sql-schema` only draws a table's *first* foreign key as its tree parent (additional FKs are still recorded in metadata, just not drawn as a second parent edge).
