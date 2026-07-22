# TODOs

A rough kanban, not a process — tiles move left to right as we get to them.
See `CLAUDE.md` for how the pieces fit together.

## Milestones (rough)

- **M1 — Point cloud MVP** ✅ done: convert/serve/view real point clouds (PLY/XYZ → octree tileset → LOD viewer)
- **M2 — Graph abstraction** ✅ done: `spex-graph` model, 3 real adapters, 3 views (terminal/web/json), `demos/` convention
- **M3 — Robustness** 🟡 in progress: fan-out cap + ring-stagger done; real line-edges still open
- **M4 — More adapters** 🟡 in progress: disk-usage done, others open
- **M5 — Docs & handoff** ✅ done: CLAUDE.md, AGENTS.md, README, `scripts/walkthrough.sh`
- **M6 — Self-explanatory demos** ✅ done: `Graph.title`/`metric_label`, colored+summarized terminal view, browser header/legend
- **M7 — Web gallery** ✅ done: `spex gallery` — front-page index of all demos, click into any of them
- **M8 — Navigable CLI, deeper docs, formal spec** ✅ done: docs, formal spec, and `spex nav` all shipped
- **M9 — Demoscene cycle mode** ✅ done: gallery "▶ cycle through demos" auto-rotates + jumps between demos on a timer
- **M10 — ASCII renderer + reusable pipeline test** ✅ done: `spex ascii`, and a Berlin→Tegernsee→Neuss fixture as a template for end-to-end pipeline tests
- **M11 — ASCII crop fix + richer browser tooltips** ✅ done: `spex ascii` crops to content instead of mostly-blank space; browser hover tooltip now shows full metadata, not just `label (metric)`
- **M12 — Animated packet** ✅ done: a marker travels node-to-node along a graph's primary chain in the browser viewer, speed scaled to real hop distance, toggleable
- **M13 — SQL schema adapter** ✅ done: `spex sql-schema` introspects a real SQLite database via `sqlite3`, one node per table, real row counts + FKs
- **M14 — Packet-hit tooltip flash** ✅ done: the animated packet reaching a node briefly shows that node's full hover tooltip, regardless of mouse position

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
- `spex gallery [dir] [--port]` — front-page index (dark-themed HTML, generated server-side, no templating dep) listing every demo as a card (title, node/point counts), each linking to `/d/<name>/`; same embedded viewer SPA loads any demo based on the URL (`TILESET_BASE` now resolved from `location.pathname` instead of hardcoded) — `spex serve <tileset-dir>` (single-demo mode) is unchanged
- `docs/ARCHITECTURE.md` — the narrative: the core idea (any tree becomes a point cloud), a worked example with real data through every pipeline stage, and the "why" behind the non-obvious choices (radial not force-directed layout, the fan-out cap's origin story, three independent views of one model)
- `spec/*.schema.json` + `spec/README.md` — formal JSON Schema (2020-12) for `graph.json`/`tileset.json`/`nodes.json`/`meta.json`, each self-contained (no external `$ref`s). Enforced, not just documented: `crates/spex-cli/tests/schema_validation.rs` runs the real CLI end to end and validates every generated file against its schema (plus an `--ignored` spot-check against real, messier demo data — nulls, arrays in metadata, dotted hostnames — to confirm the schemas hold beyond the clean synthetic case)
- `spex nav [dir]` — k9s-style interactive browser (`ratatui`+`crossterm`) over the same `discover_demos()` data as `spex demos`/`spex gallery`: move through the list, `enter`/`v` for a scrollable detail view (the real tree, ANSI-stripped), `w` spawns a real detached `spex serve` on a deterministic per-demo port. Terminal safety (raw mode + alt screen, restored even on panic) verified for real in a tmux pty: launched, navigated, viewed a tree, opened a web view (confirmed reachable via `curl`), quit, and confirmed the shell prompt came back clean and still accepted commands afterward.
- Demoscene cycle mode: a "▶ cycle through demos" link on the gallery page picks a random demo and appends `?cycle=1`; the viewer detects that flag, turns on `OrbitControls.autoRotate` for a slow continuous spin, shows a "next demo in Ns" countdown, and after 20s re-fetches the gallery page's own HTML (scraping `/d/<name>/` links out of it — no new API endpoint needed), picks a different random demo, and navigates there. 100% reuse of existing tileset-loading/rendering code — the only new things are the timer, the auto-rotate flag, and the countdown UI.
- `spex ascii <tileset-dir> [--width]` — colored ASCII-art snapshot (inspired by https://github.com/tronicum/aa-bb-blkstn-cc): a real pinhole-camera projection of the tileset's actual points (matching the viewer's default framing, so the snapshot looks like the browser's first view), z-buffered per cell, luminance mapped through a light/dark glyph ramp, colored via the same ANSI-truecolor technique as `graph-print`. `spex-tiler` gained `read_points()` (the read-side counterpart to `build()`) to make this possible — works on any tileset, literal point clouds included. Verified for real in a pty (`script`) that color escapes actually appear, and that the rendered shape visibly matches the browser view.
- `crates/spex-cli/tests/end_to_end_journey.rs` — a reusable `run_full_pipeline()` harness (build a small `Graph` fixture → real `graph-layout`/`graph-print`/`ascii` via the built binary → structured `PipelineArtifacts` to assert on), instantiated with a small illustrative fixture: a simulated packet journey Berlin→Tegernsee→Neuss (real coordinates, honestly-labeled illustrative latency — also materialized as `demos/berlin-tegernsee-neuss/`). Template for testing any other adapter/pipeline end to end, not just this one fixture.
- `demos/traveling-salesman` — a bigger 7-city baseline (Neuss→Hamburg→Kiel→Berlin→Sonneberg→Bayreuth→Tegernsee, real haversine distances, illustrative latency) for a nicer-looking demo than the 3-node journey.
- Fixed a real bug found by actually using the tool: `spex ascii` rendered the full field-of-view grid even when a sparse point cloud only lit up a small region of it — one real render had content on 15 of 62 rows, which scrolled clean off a short terminal and looked like nothing rendered at all. Now crops to content + 1-cell margin.
- Browser hover tooltip now shows every metadata field (multi-line, same compact array-collapsing as `graph-print`'s terminal view), not just `label (metric)` — the richer traveling-salesman metadata (distance, coordinates, notes) is now actually visible in the browser instead of only in the JSON/terminal.
- Animated packet in the browser viewer (`viewer/src/packetAnimation.ts` + `main.ts`): `buildPrimaryPath()` walks a graph from its root always taking the first child (the complete path for a chain like traceroute/journey demos, an honest partial path down one branch for a tree), a small glowing marker travels along it with speed scaled to each hop's real 3D distance (long hops take longer, not every hop the same duration), loops back to the start on reaching the end, toggleable via a new "Animate packet" checkbox (default on). Zero backend changes — reuses the already-fetched `nodes.json` data. Verified in a real headless-Chromium session: the marker visibly moved between two screenshots taken ~2.5s apart, the toggle hid it, and a plain point-cloud tileset (no `nodes.json`) still rendered with no new console errors.
- `spex sql-schema <db> -o <graph.json>` (`crates/spex-cli/src/sql_schema.rs`): shells out to the real `sqlite3` CLI (`sqlite_master`, `PRAGMA table_info`, `PRAGMA foreign_key_list`, real `SELECT COUNT(*)`) — one node per table, row count drives color, columns + all FKs recorded as metadata, first FK becomes the tree parent (tables with none are forest roots — `Graph`'s forest support, not a synthetic root). `demos/sql-schema/` is a small hand-built real SQLite db (customers/products/orders/order_items, a few real rows each), created by `scripts/walkthrough.sh` (gated on `sqlite3` being on PATH) — start-small groundwork for a MySQL/DB2 adapter later (same shape, different query mechanism). Verified: unit tests against a real temp sqlite db, full pipeline (`sql-schema` → `graph-print` → `graph-layout` → `ascii`), and a real headless-Chromium screenshot of the browser view, all correct with no console errors.
- `scripts/gallery.sh [port]` — one-shot "refresh and view": re-runs `walkthrough.sh` to regenerate `demos/`, then kills any gallery server already listening on the port (the gallery's demo list is fixed at process startup, so an already-running one won't pick up new/changed demos — this is exactly what happened when `sql-schema` was added and a stale gallery on :8080 didn't show it) and starts a fresh one. Verified: ran it against a live stale gallery process, confirmed the old pid was killed and the new server listed all 6 current demos including the just-added one.
- Packet-hit tooltip flash (`viewer/src/main.ts`): reaching a node (`packetT` wrapping to a new segment in `updatePacket()`) sets `packetHitNode`/`packetHitTimer`; a new `updatePacketHitLabel()` (called each frame after `updatePacket()`) projects that node's real screen position and shows its existing tooltip `<div>` (same element/content `updateLabels()` already builds for mouse hover) for ~1.2s, independent of cursor position — reuses the hover system instead of building a second one. Gated on the same "Labels (hover)" checkbox; toggling "Animate packet" off also clears any in-flight flash. Verified in headless Chromium with the mouse parked off-scene (so only the packet-hit path could show a label): the Hamburg node's full tooltip (label, metric, distance, lat/lon, note) appeared and was screenshotted, no console errors.
- Richer traveling-salesman trace (`scripts/gen_traveling_salesman.py`): the 7-city chain now has 2 synthetic router-like hops between each city pair (19 nodes total instead of 7) — fabricated hostname/IP per hop, but real haversine sub-distances between real interpolated lat/lon points, and each city-pair's original illustrative latency split proportionally across its sub-hops so the numbers stay coherent. Wired into `scripts/walkthrough.sh` (gated on `python3`) so it regenerates on a fresh clone instead of being a hand-authored one-off file. Also fixed `berlin-tegernsee-neuss` the same way — it was never in `walkthrough.sh` either, which is exactly how both got permanently deleted mid-session by an `rm -rf demos` (`demos/` is gitignored by design, so there was no backup); both are now real generated-on-demand steps in the script, not one-off files that can vanish again. Verified: full `walkthrough.sh` run from an empty `demos/` regenerates all 8 demos including these two, full test suite green, and a headless-Chromium check confirmed the packet's hit-tooltip flash works correctly on the new intermediate hop nodes too (not just city nodes).

### Doing / next up
- [ ] Historical stock-price demo for fun (Volkswagen or Tesla) — daily/weekly closes as a chain, metric = price or volume, colored by gain/loss; a good second real exercise for the animated-packet chain view once real data is wired up
- [ ] Debian/RPM package-dependency adapter (`dpkg -s`/`apt-cache depends` or `rpm -qR`) — same shape as `brew-deps`, start small (one real package's direct deps, not a full recursive apt/dnf tree); not testable on this dev machine (macOS, no dpkg/rpm on PATH) — needs a Linux box or a container
- [ ] MySQL/DB2 SQL adapter — `sql_schema.rs`'s SQLite version is the template (same table/FK/row-count shape); swap `sqlite3` CLI calls for the real driver/CLI (`mysql`/`db2` client) when there's a real instance to point at
- [ ] "Deutsche Bahn mode" for the traveling-salesman demo, for fun: random per-hop delays, occasional cancelled/skipped hops, maybe a "replacement bus" fallback edge — layered on top of `scripts/gen_traveling_salesman.py`'s existing hop generation

### Backlog (ideas discussed, not built — pruned to the ones actually worth doing next)
- [ ] Animated packet on branching trees (ps-tree, brew-deps) — today it only walks one branch (first-child-always); multiple simultaneous packets or a full DFS sweep would cover the whole tree
- [ ] Layout polish: ring-stagger helps but capped rings can still overlap at some angles/zoom levels — worth a proper multi-ring rewrite if it keeps bothering us
- [ ] Real line-edges in the viewer (replace point-trails with an actual WebGL line primitive)
- [ ] More package-manager adapters (npm, `cargo tree`, apt) — same shape as `brew-deps`
- [ ] DAG/shared-dependency merging — `Graph` is tree-only today, so a package used by two things gets duplicated instead of merged
- [ ] Diff/temporal mode — re-run an adapter later and visualize what changed (traceroute path drift, process churn)
- [ ] Real ICMP raw-socket probing for `trace` (needs sudo/capabilities — using UDP traceroute today)
- [ ] `spex nav`: real ANSI-colored detail view (needs an ANSI→ratatui-styled-text conversion; plain text for now), `/` search-filter on the demo list

### Icebox (from `spex-tiler`'s known limits — not urgent)
- LAS/LAZ point cloud input
- Out-of-core tiling for point clouds too large for memory
- Point buffer compression
