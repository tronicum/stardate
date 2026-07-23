# Adding a new input adapter

An "adapter" turns some real, external source into a `spex_graph::Graph` —
that's the entire contract. It never touches `spex-tiler`, `spex-server`, or
`viewer/`; those are shared downstream of "here is a `Graph`."

## The shape every adapter follows

1. **One file**: `crates/spex-cli/src/<name>.rs`, exposing `pub fn
   run(...) -> Result<Graph>` (see `brew_deps.rs`, `cargo_deps.rs`,
   `npm_deps.rs`, `sql_schema.rs`, `molecule.rs` for real examples of the
   two main flavors below).
2. **Get real data one of two ways**:
   - Shell out to a real external tool via `std::process::Command` and
     parse its actual text/JSON output (`brew deps --tree`, `cargo tree`,
     `npm ls --json --all`, `dpkg -s`, `sqlite3` via `PRAGMA` queries). If
     the tool has a real structured-JSON mode, prefer it over scraping
     human-readable tree art (`npm_deps.rs` parses real JSON;
     `brew_deps.rs`/`cargo_deps.rs` have to scrape box-drawing characters
     because neither `brew`/`cargo` has a clean JSON tree mode).
   - Parse a real external *format* directly, no subprocess needed
     (`molecule.rs`'s SMILES parser). Still real data — a real molecule's
     real canonical SMILES string — just no external process.
3. **Build `GraphNode`s**: `id`, `label`, `parent: Option<String>` (`None` =
   root; a forest of multiple roots is valid), `metric: Option<f64>` (drives
   color — pick something real and meaningful: subtree size for a
   dependency tree, atomic number for a molecule, real installed size for
   `deb-deps`), `metadata` (free-form `serde_json::Map` — put anything real
   and useful here, even if it's not driving color).
4. **Wire it into `crates/spex-cli/src/main.rs`**: add the module
   (`mod <name>;`), a `Command::<Name> { ... }` clap variant, its arm in the
   `match` in `main()`, and a `cmd_<name>` function that calls `run()`,
   prints a short real-data summary (e.g. "captured N packages"), and
   writes the graph JSON.
5. **Add it to `scripts/walkthrough.sh`** if it should regenerate on every
   walkthrough run — gate it on the required tool being present
   (`command -v <tool> >/dev/null 2>&1`) so a missing tool just skips that
   step gracefully rather than failing the whole script. If the adapter
   fetches live external data with real rate limits, don't wire it into
   `walkthrough.sh` directly — see `working-mode.md`'s committed-snapshot
   pattern instead.

## `Graph`'s tree-only constraint is not optional — plan for it

`Graph`/`GraphNode` model trees/forests only: no cycles, no node with more
than one parent. Real-world data is very often *not* naturally tree-shaped
(a package can be a dependency of several things; a molecule has rings; a
Wikipedia page's link graph is enormously cyclic). Decide *explicitly* how
your adapter handles this — don't silently drop real relationships:

- **Duplicate under every real parent** (what `brew-deps` does) — simplest,
  but inflates node count for heavily-shared dependencies.
- **Keep only the first real relationship as the tree edge, record the rest
  as metadata** (what `sql-schema`'s first-FK rule and `deb-deps`'s
  first-`|`-alternative rule do) — good when there's a natural "primary"
  relationship to pick.
- **Keep the extra edge as metadata on both endpoints, not a second parent**
  (what `molecule.rs`'s `ring_bond_to` does for a ring-closure bond) — good
  when the extra edge is real information worth keeping visible (e.g. on
  hover) without needing to render as a second edge.
- **Cap fan-out and dedupe against a visited-set** if the source graph is
  large/cyclic enough that an uncapped walk would be unbounded (what the
  Wikipedia crawl adapter does — a real page can have hundreds of outbound
  links, and the link graph loops back on itself within a couple of hops).
  The layout's own `MAX_CHILDREN_SHOWN` fan-out cap (in `spex-graph`) is a
  second, generic safety net every adapter gets automatically regardless —
  don't rely on it alone if your source data can be unboundedly large
  *before* layout even runs.

## Testing pattern

Unit-test against a **realistic, hand-written sample of the real external
tool's actual output shape** — not a fabricated/guessed schema. E.g.
`deb_deps.rs`'s tests use a real `dpkg -s curl` output sample; `las.rs`'s
tests write a real LAS file via the `las` crate's own writer and read it
back. If you can get real output from the actual tool (even a small
example), use that as the literal fixture text rather than inventing a
plausible-looking one.
