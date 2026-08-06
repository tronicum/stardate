# TODOs

A rough kanban, not a process — tiles move left to right as we get to them.
See `CLAUDE.md` for how the pieces fit together.

For `spex-ankerstein` specifically, real GitHub Milestones now exist
(Phase 1-5, https://github.com/tronicum/stardate/milestones) mirroring
`docs/ANKERSTEIN-ENGINE.md`'s phase breakdown — a live, checkable-without-
me tracker, complementary to this file's own prose history rather than a
replacement for it.
Full milestone rationale/verification detail lives in `docs/MILESTONES-LOG.md` — this file is now a lean index; look there before assuming something wasn't verified.

## Milestones (rough)

Full rationale/verification narrative for every milestone lives in [`docs/MILESTONES-LOG.md`](docs/MILESTONES-LOG.md) — this table is just a fast-to-scan index.

| # | Milestone | Status | Note |
|---|---|---|---|
| M1 | Point cloud MVP | ✅ done | PLY/XYZ → octree tileset → LOD viewer, end to end. |
| M2 | Graph abstraction | ✅ done | `spex-graph` model, 3 real adapters, 3 views, `demos/` convention. |
| M3 | Robustness | ✅ done | Fan-out cap, ring-stagger, real line-edges (M25), jitter fix (M37) all shipped. |
| M4 | More adapters | ✅ done | disk-usage, sql-schema, deb-deps, cargo-deps, npm-deps, molecule; apt still open. |
| M5 | Docs & handoff | ✅ done | CLAUDE.md, AGENTS.md, README, `scripts/walkthrough.sh`. |
| M6 | Self-explanatory demos | ✅ done | `Graph.title`/`metric_label`, colored terminal view, browser legend. |
| M7 | Web gallery | ✅ done | `spex gallery` — front-page index of all demos. |
| M8 | Navigable CLI, deeper docs, formal spec | ✅ done | Docs, formal spec, and `spex nav` all shipped. |
| M9 | Demoscene cycle mode | ✅ done | Gallery auto-rotates and jumps between demos on a timer. |
| M10 | ASCII renderer + reusable pipeline test | ✅ done | `spex ascii` + Berlin→Tegernsee→Neuss end-to-end test fixture. |
| M11 | ASCII crop fix + richer browser tooltips | ✅ done | Ascii crops to content; tooltips show full metadata, not just label. |
| M12 | Animated packet | ✅ done | Marker travels node-to-node, speed scaled to real hop distance. |
| M13 | SQL schema adapter | ✅ done | `spex sql-schema` introspects a real SQLite DB via `sqlite3`. |
| M14 | Packet-hit tooltip flash | ✅ done | Packet reaching a node briefly shows that node's full tooltip. |
| M15 | Floating demoscene-style title | ✅ done | Header title floats + glow-cycles blue/yellow/pink, pure CSS. |
| M16 | Real Chinook dataset for sql-schema | ✅ done | Swapped hand-typed fixture for real MIT-licensed 11-table Chinook DB. |
| M17 | Big Mac Index demo | ✅ done | 43 real Big Mac prices (2000-2026) from The Economist, as a chain. |
| M18 | Static GitHub Pages export | ✅ done | `spex export-static` + `pages.yml` auto-deploy, relative paths throughout. |
| M19 | Deutsche Bahn mode | ✅ done | TSP route + simulated delays/cancellations, fixed-seed reproducible. |
| M20 | Debian/RPM (dpkg) adapter | 🟡 code done, unverified live | `dpkg -s` works in unit tests; no live Debian/Ubuntu box to verify against. |
| M21 | `spex nav` search-filter | ✅ done | `/` filters demo list live; verified in a real tmux pty. |
| M22 | cargo-tree adapter | ✅ done | `spex cargo-deps` runs real `cargo tree`, dogfooded on spex-graph itself. |
| M23 | Lower-left debug panel for packet demos | ✅ done | Monospace live readout of hop/progress/status, shown only when packet active. |
| M24 | Stock-price demo (Tesla/VW/BYD) | ✅ done | Real weekly closes via Alpha Vantage, windowed to 104 weeks. |
| M25 | Real line edges | ✅ done | Crisp WebGL lines node-to-parent, layered on top of existing point-trails. |
| M26 | `spex nav` real ANSI-colored detail view | ✅ done | Real terminal truecolor via `ansi-to-tui`, verified in a real tmux pty. |
| M27 | Packet follows the heaviest branch | ✅ done | Fixed `neovim-deps` dead-end-leaf bug by following highest-metric child. |
| M28 | npm dependency adapter | ✅ done | `spex npm-deps` parses real `npm ls --json`; 96 real packages dogfooded. |
| M29 | `spex graph-diff` | ✅ done | Terminal diff between two `graph.json` captures; verified on real `ps-tree` snapshots. |
| M30 | German cities real-TSP demo | ✅ done | Frankfurt + 5 cities, brute-force exact shortest path — first solved TSP. |
| M31 | Browsable ASCII view (`ascii.html`) | ✅ done | Generated once per tileset at `graph-layout` time, linked from every gallery card. |
| M32 | 3 degrees of Wikipedia from Frankfurt | ✅ done | Real BFS crawl of Wikipedia links, fan-out capped and deduped. |
| M33 | Molecule adapter (`spex molecule`) | ✅ done | Real SMILES parsing; ring-closure bonds kept as metadata, not a second parent. |
| M34 | Packet full DFS sweep | ✅ done | `buildFullSweepPath` covers the whole tree via a real Euler-tour, not one branch. |
| M35 | Diff/temporal viewer coloring | ✅ done | `graph-diff --merge` tags added/removed/changed/unchanged with fixed colors. |
| M36 | Real LAS/LAZ point cloud input | ✅ done | Verified against a real 3.8M-point, real-RGB LiDAR scan (PDAL test data). |
| M37 | Layout jitter overshoot fix | ✅ done | Found real sibling-blob collisions on a 1066-node `ps-tree` capture; jitter now capped to slice width. |
| M38 | Animated ASCII (`spex ascii --animate`) | ✅ done | Real turntable-orbit animation, live terminal or self-contained HTML. |
| M39 | Animated ASCII wired into the gallery | ✅ done | `ascii-animated.html` rides along every tileset automatically, 24 frames @ 8fps. |
| M40 | "2027: A Brick Odyssey" first brick spike | ✅ done | Real 1×1 LDraw brick; bounds matched real 8×8mm footprint/9.6mm height almost exactly. |
| M40 addendum | Real baked-in lighting | ✅ done | Lambertian+specular baked into point RGB; fixed a real LDraw Y-down vs spex Y-up bug. |
| M41 | Monolith ("the 2001 moment") | ✅ done | 9-part stack (1:4:8.8 or 1:4:9.2, not exact 1:4:9); fixed a real inter-part stacking gap. |
| M42 | Auto-rotate toggle in the WebGL viewer | ✅ done | Slow hands-off spin checkbox; damped ~2.5-3s coast-down confirmed as wanted feel. |
| M43 | `spex-brick-mesh` intermediate format | ✅ done | Resolve-once mesh cache; byte-identical geometry stats after refactor. |
| M44 | `spex-brick-scene` format (real LDraw model file) | ✅ done | Parses car.ldr/pyramid.ldr; fixed a real ldraw.org HTTP 429 via backoff + local mirror. |
| M45 | Monolith assembly reveal animation | ✅ done | `spex frame-sequence` tiles N frames with one shared offset; reverted an early canvas-HTML false start. |
| M46 | New crate `spex-ldraw` (cache/colors) | ✅ done | Disk cache → local mirror → live HTTP with exponential backoff on 429. |
| M47 | `spex brick-part` CLI command | ✅ done | Real `1x1-brick` bounds (8.0×11.2×8.0mm) match M40 exactly. |
| M48 | `spex brick-model` CLI command | ✅ done | Real car.ldr render matches M44 exactly (61 placements/26 distinct parts). |
| M49 | `spex brick-assembly` CLI command | ✅ done | Generalizes M45's reveal animation to any scene, not just the monolith. |
| M50 | Retired `unibrick/` Python entirely | ✅ done | Deleted Python pipeline + its JSON schemas; full `cargo test --workspace` green. |
| M98 | `spex ankerstein-part` CLI command | ✅ done, CI-green | Rendered bounds match seed catalog exactly (gk-cube-full ~25×25×25mm); real functional run still pending (no local Rust toolchain). |
| M99 | Prism/wedge geometry | ✅ done, CI-green | `generate_prism` adds a real 45° roof-slope stone (`gk-prism-45`, 50×50×50mm bbox), cited to George Hardy's *Anker Stone Building Sets*. |
| M100 | `spex ankerstein-model` CLI command | ✅ done, CI-green | Scene/assembly rendering; caught and fixed a real test-math bug (bbox width is 50mm, not 37.5mm). |
| M101 | Arch geometry | ⏸ deferred | No free source gives real voussoir dimensions; CVA Stone Catalog is member-only — invented angles rejected per the real-data-only standard. |
| M104 | Gallery + docs wiring | ✅ done | No code needed — `spex gallery`/`spex demos` already directory-shape-agnostic; documented in README/ANKERSTEINE.md. |

## Demo data provenance

Standard: real tool + real underlying data, not just a real *tool* pointed at fabricated sample data. Where a real dump isn't available, be explicit about which parts are real vs invented (`decix-trace` and `disk-usage` are the bar — real hops/sizes, nothing made up).

| Demo | Tool | Data | Status |
|---|---|---|---|
| `decix-trace` | real `traceroute` | real network hops on this machine | ✅ fully real |
| `disk-usage` | real `du` | real filesystem sizes on this machine | ✅ fully real |
| `my-shell` | real `ps` | real process tree on this machine | ✅ fully real |
| `neovim-deps` | real `brew deps --tree` | real installed package graph | ✅ fully real |
| `pstree` | none (`pstree-demo`) | fabricated, explicitly labeled "not read from any real machine" | ⚠️ intentional synthetic fallback, not meant to be upgraded |
| `sql-schema` | real `sqlite3` (`PRAGMA`/`COUNT(*)`) | real Chinook sample DB (downloaded once from `lerocha/chinook-database`, MIT-licensed) — 11 tables, real row counts up to 8,715 | ✅ fully real |
| `traveling-salesman` | none (generator script) | real city coordinates + real haversine distances; latency numbers and router hostnames/IPs are invented, honestly labeled illustrative | ⚠️ real geography, fabricated network numbers — no real dump to copy from for a fictional route, but worth a second look |
| `berlin-tegernsee-neuss` | none (generator script) | same shape as `traveling-salesman`, smaller | ⚠️ same caveat |
| `bigmac` | none (generator script downloads a real CSV) | real Big Mac Index prices, published twice a year by The Economist (`TheEconomist/big-mac-data`) | ✅ fully real |
| `stock-tsla`/`stock-vow3`/`stock-byd` | none (generator script reads a committed snapshot) | real weekly closes via Alpha Vantage `TIME_SERIES_WEEKLY`, windowed to the most recent 104 weeks (real full history is 800-1100+ points, verified too dense to stay legible/fast) | ✅ fully real |

Apply this standard to new demos by default (stock-price, Kevin Bacon movie-cast) — look for a real downloadable/copyable dataset dump before inventing numbers.

## Board

### Done
- Point cloud MVP: `spex info/convert/serve`, octree LOD streaming viewer
- `spex-graph` intermediate format + generic radial 3D layout
- Adapters: `trace` (real), `brew-deps` (real), `ps-tree` (real, `--root` scoping), `disk-usage` (real, `du`-based), `pstree-demo` (fabricated fallback)
- Views: `graph-print` (terminal), web hover-tooltip labels, JSON (free — it's the intermediate format itself)
- Fan-out cap/collapse safeguard (generic, lives in the layout, protects every adapter)
- Ring-stagger: alternating siblings offset radially to ease crowding in capped high-fanout rings (better, not perfect — see backlog)
- `demos/<name>/{graph.json,tileset/}` convention + `spex demos` listing (with a hint pointing at the walkthrough script when empty)
- `CLAUDE.md`, `AGENTS.md`, `README.md` refresh (now covers both pipelines)
- `scripts/walkthrough.sh` — generates all 5 demos in one go (skips gracefully if a tool like `brew`/`traceroute` is missing), so a fresh clone gets a working tour immediately
- Test coverage: `spex-server` (0 → 3 tests, router serving/fallback/404), `spex-tiler` edge cases (empty input, exact-budget boundary); 30 tests total across the workspace now
- Fixed a real bug: piping `graph-print`/any stdout command into `head` etc. used to panic ("Broken pipe") — reset SIGPIPE to default Unix behavior instead
- `Graph.title`/`metric_label` (what a graph is, what the metric means) — every adapter now sets both
- Terminal view: header (title + node count), TTY-aware ANSI truecolor per line (same gradient as the browser, `NO_COLOR`-respecting), and a summary footer (metric range + hottest node) — understandable standalone, no browser needed
- Browser view: persistent title + color-gradient legend bar (via new `meta.json`), always visible — no more "what am I even looking at" without hovering
- `spex gallery [dir] [--port]` — front-page index of every demo as a card, each linking to `/d/<name>/`; see [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `docs/ARCHITECTURE.md` — the narrative: core idea, a worked real example, and the "why" behind non-obvious choices.
- `spec/*.schema.json` + `spec/README.md` — formal JSON Schema for every generated file format, enforced by real CLI-driven tests; see [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `spex nav [dir]` — k9s-style interactive TUI browser over demos; verified for real in a tmux pty. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Demoscene cycle mode: gallery "▶ cycle through demos" auto-rotates + jumps between demos on a timer, 100% reuse of existing tileset code. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `spex ascii <tileset-dir> [--width]` — colored ASCII-art snapshot, pinhole-camera projection, verified in a real pty. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `crates/spex-cli/tests/end_to_end_journey.rs` — reusable `run_full_pipeline()` harness, instantiated with the Berlin→Tegernsee→Neuss fixture.
- `demos/traveling-salesman` — a bigger 7-city baseline for a nicer-looking demo than the 3-node journey.
- Fixed a real bug: `spex ascii` used to render blank rows when a sparse cloud only lit up part of the grid; now crops to content + 1-cell margin. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Browser hover tooltip now shows every metadata field, not just `label (metric)`. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Animated packet in the browser viewer: a glowing marker travels the primary path, speed scaled to real hop distance, toggleable. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `spex sql-schema <db> -o <graph.json>` — real `sqlite3` introspection, row count drives color, FKs recorded as metadata. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `demos/sql-schema/` upgraded to the real MIT-licensed Chinook sample DB (11 tables, real row counts up to 8,715). See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `demos/bigmac` — real Big Mac Index prices from The Economist as a chain, wired into `walkthrough.sh`. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `spex export-static` + `.github/workflows/pages.yml` — fully static, relative-path gallery export, deployed on every push to `main`. See [detail](docs/MILESTONES-LOG.md#board-detail-archive) (includes a real regression caught/fixed in the live gallery server).
- `scripts/gallery.sh [port]` — one-shot regenerate-and-refresh-server helper. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Packet-hit tooltip flash: reaching a node briefly shows its full tooltip regardless of cursor position. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Floating demoscene-style title (`#graph-title`): slow bob + blue→yellow→pink glow, pure CSS. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Richer traveling-salesman trace: 2 synthetic router hops per city pair, real haversine sub-distances; also fixed both TSP demos being missing from `walkthrough.sh`. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `demos/deutsche-bahn` — TSP route + simulated on-time/delayed/cancelled status, fixed seed. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `spex nav` `/` search-filter — live-filters the demo list; verified in a real tmux pty. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `spex cargo-deps <package>` — real `cargo tree` parsing, dogfooded on the workspace itself (30 real crates). See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Lower-left debug panel: live hop/progress/status readout while the animated packet is active. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Stock-price demo unblocked with a real Alpha Vantage key, windowed to 104 weeks after the unwindowed full history proved genuinely unreadable. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Real line edges (`THREE.LineSegments`) added alongside the existing point-trail edges. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `spex nav` real ANSI-colored detail view via `ansi-to-tui`, verified with a direct cell-color assertion. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Packet heaviest-branch fix: `neovim-deps`' dead-end 1-hop path fixed by following the highest-metric child. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `spex npm-deps` — real `npm ls --json` parsing, dogfooded on `viewer/`'s own 96-package lockfile. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `spex graph-diff <old> <new>` — terminal diff by node id, verified against two real `ps-tree` captures.
- `demos/german-tsp` — Frankfurt + 5 German cities, brute-force-exact solved shortest path. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Browsable ASCII view (`ascii.html`) generated at `graph-layout` time, linked from every gallery card. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `demos/wikipedia-crawl` — real BFS crawl of Wikipedia's link graph from Frankfurt, depth 3, fan-out capped/deduped. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- `spex molecule` — real SMILES parser, ring closures kept as metadata not a second parent, 4 known molecules built in. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Packet full DFS sweep (`buildFullSweepPath`) — real Euler-tour traversal covering the whole tree. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- Diff/temporal viewer coloring (`graph-diff --merge`) — added/removed/changed/unchanged tags drive fixed colors. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).

### Doing / next up
- [ ] 🟡 **`spex deb-deps` — verify against a real live Debian/Ubuntu system.** Code shells out to real `dpkg -s` and passes unit tests, but this machine has no Debian box/working colima networking to verify live against. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- [ ] MySQL/DB2 SQL adapter — `sql_schema.rs`'s SQLite version is the template (same table/FK/row-count shape); swap `sqlite3` CLI calls for the real driver/CLI (`mysql`/`db2` client) when there's a real instance to point at
- [ ] RPM adapter (`rpm -qR`) — same shape/blocker as the `spex deb-deps` verification item above (needs a real Debian/Ubuntu-or-RPM Linux box). See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- [ ] Commit a real, small `.las` test fixture into the repo — M36's LAS/LAZ verification was done against files fetched ad hoc, never kept. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- [x] ✅ Real complex point-cloud test — done (M36), verified against a real 3.8M-point real-RGB LiDAR scan. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- [ ] LEGO/Klemmbaustein price-history side of the "Baumeister" spark idea — paused, brickinsights.com's chart data isn't scrapable from static HTML; worth trying Rebrickable or a proper open-licensed dataset instead. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- [ ] **Klemmbaustein brick voxel renderer, planning stage only.** Real geometry already confirmed to exist via Rebrickable + LDraw; see `BRICKs.md`. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).

- [ ] **`spex-ankerstein` — a real Richter's Anchor Stone Building Set renderer.** Spec in `docs/ANKERSTEIN-ENGINE.md` (provisional milestones M98-M108). M98 (CLI part render), M99 (prism geometry), M100 (scene/model render) all done and CI-green; M101 (arch geometry) explicitly deferred (no free source has real dimensions); M104 (gallery/docs wiring) done. See the Milestones table above for per-milestone status, and [full detail](docs/MILESTONES-LOG.md#board-detail-archive) for the licensing caveats, the CI setup, the set/inventory layer, and a real test-math bug caught and fixed along the way.

### Backlog (ideas discussed, not built — pruned to the ones actually worth doing next)
- [x] ✅ **Big meta idea: German cities real-TSP + degrees-of-Wikipedia — both parts done (M30, M32).** See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- [x] ✅ Molecule adapter — done (M33). Still open: multi-step reaction visualization. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- [ ] "Morph between two graph states" — lerp every matching node's position/color between graph A and graph B instead of walking a single chain
- [ ] Animated ASCII (M38/M39) + WebGL packet animation, combined — a graph-aware variant of `--animate`, well-scoped but not started. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- [x] ✅ Animated ASCII default width tuned for common screen sizes — done, bumped 100→140 columns. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- [x] ✅ Full coverage of branching trees — done (M34). Still open: multiple simultaneous packets instead of one sweeping serially.
- [x] 🟡 Layout polish: jitter-caused overlap fixed (M37); the deeper "not enough real arc length at this radius" case (a pathological many-nested-levels case) is still open. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- [ ] Consider actually removing the point-trail edge points now that real lines exist — bigger/riskier change, deliberately not done yet
- [ ] More package-manager adapters (apt) — same shape as `brew-deps`/`cargo-deps`/`npm-deps`
- [ ] DAG/shared-dependency merging — `Graph` is tree-only today, so a package used by two things gets duplicated instead of merged
- [x] ✅ Diff/temporal mode, viewer half — done (M35). Still open: the "morph between two graph states" animated-lerp idea above.
- [ ] Real ICMP raw-socket probing for `trace` (needs sudo/capabilities — using UDP traceroute today)
- [ ] `spex nav` animated-ascii preview — needs a timer-driven redraw in `nav.rs`'s input loop, not started. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- [ ] Out-of-core / streaming point-cloud loading — not urgent yet (M36's 3.8M points loaded in ~1.6s), but the natural next stress point once a genuinely huge file is in hand. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).
- [x] Evaluated, **not adopted**: `spex ascii` improvement lead https://github.com/1480c1/aalib — not a 3D renderer, monochrome, strictly less capable than the existing pure-Rust ASCII renderer. See [detail](docs/MILESTONES-LOG.md#board-detail-archive).

### Icebox (from `spex-tiler`'s known limits — not urgent)
- Out-of-core tiling for point clouds too large for memory
- Point buffer compression
