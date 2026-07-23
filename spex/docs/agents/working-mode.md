# Working mode

## Real data only — and where it actually lives

Every demo in this project uses real data: a real `ps`/`brew deps --tree`/
`npm ls --json`/`sqlite3`/`cargo tree` run, a real downloaded dataset (the
Chinook sample database, The Economist's Big Mac Index, Alpha Vantage stock
prices, a real Wikipedia link crawl), or a real string format parsed exactly
(SMILES for `spex molecule`). This is not a style preference — it's been an
explicit, repeated correction from the user: fabricated "illustrative"
numbers are not acceptable even for a quick demo.

This produces two different, deliberate patterns depending on where the data
comes from:

- **`demos/` is entirely gitignored.** It's local, regenerated output —
  never commit anything under it directly.
- **Data fetched from a stable external source gets committed as a snapshot
  under `scripts/`**, separate from the ephemeral `demos/` output — e.g.
  `scripts/stock-data/*.json` (raw Alpha Vantage responses), the downloaded
  Chinook `.sqlite` file, `scripts/wikipedia-crawl-data/frankfurt-depth3-fanout3.json`.
  A `scripts/gen_*_demo.py` script then reads the *committed snapshot* and
  writes `demos/<name>/graph.json` — it does **not** re-fetch live on every
  `walkthrough.sh` run. This matters for two real reasons, not just
  tidiness: it keeps `walkthrough.sh`/CI fast and reliable (no network
  flakiness, no API key needed at generation time), and some of these
  sources genuinely rate-limit (see below).
- A separate `scripts/gen_*_crawl.py`-style **live-fetch tool** may exist
  alongside the snapshot-reading one specifically to *regenerate* the
  snapshot when needed (e.g. `gen_wikipedia_crawl.py` vs.
  `gen_wikipedia_demo.py`) — the live tool is not run routinely.

## A concrete external-API gotcha, hit twice

Python's `urllib` got silently blocked or stalled by two different real
services this project talked to (Yahoo Finance, then Wikipedia's MediaWiki
API) in a way plain `curl` was not — likely a TLS-fingerprint or
client-library-based throttle, not a real rate limit. The fix both times was
shelling out to `curl` via `subprocess.run()` instead of using `urllib`
directly. If a Python data-fetch script mysteriously hangs or gets rejected
against a real public API, check this class of issue *before* assuming it's
a genuine rate limit — test the same request with a bare `curl` first to
tell the two apart. (A genuine rate limit looks different: `curl` gets
throttled too, under sustained load, not on the first request.)

## Commit discipline

- One real feature or fix per commit. Don't bundle unrelated changes.
- Build and test *before* committing — see `verification.md`.
- Commit messages explain **why**, not just what changed — the diff already
  shows what changed; the message is for the reasoning that isn't visible in
  the diff (a root cause, a trade-off, what was measured to confirm the fix
  actually works).
- Push after each commit once a "commit → verify → push" rhythm is
  established with the user in a session — don't let real, verified work
  sit unpushed for many commits in a row.
- Never commit secrets (API keys, tokens). If a real API key is used
  interactively to fetch a one-time data snapshot, verify with a
  repo-wide `grep` for the literal key before committing anything that
  touched it.

## Known recurring gotchas

- **Stale background server processes.** Killing/restarting a `spex serve`
  or `spex gallery` background process with `pkill -f <pattern>` has failed
  to actually kill it more than once in this project's history — always
  verify with `ps -p <pid> -o pid,lstart` (compare against the binary's and
  `demos/`'s mtime) before trusting that a running server reflects your
  latest build. This has directly caused false "it's not working" moments
  where the real cause was a stale process from a previous step still bound
  to the port.
- **"The bug is probably in the data, not the code" is often wrong — check
  first.** When a user reported the `neovim-deps` demo looked "strange"
  (packet stuck after one hop), the real `brew deps --tree neovim` data was
  completely correct; the bug was in `buildPrimaryPath()`'s child-selection
  logic. Confirm the underlying real data independently (re-run the source
  command, look at the raw captured JSON) *before* assuming either the data
  or the code is at fault — don't guess.
- **`Graph` is tree/forest only** — no cycles, no shared parents. This is a
  real, load-bearing constraint that every adapter with genuinely
  non-tree-shaped source data has had to work around explicitly (not
  silently drop data): `brew-deps` duplicates a package under every branch
  that depends on it; `sql-schema`/`deb-deps` keep only a table/package's
  *first* foreign key/dependency alternative as the tree edge (additional
  real relationships still recorded in metadata, just not drawn as a second
  edge); `spex molecule` keeps a ring-closure bond as `ring_bond_to`
  metadata on both endpoint atoms rather than a second parent. See
  `adapters.md` for the pattern to follow for a new adapter that hits this.
