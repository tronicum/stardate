# Spending context/token budget well on this project

## What's expensive

- **Headless-browser verification** (Playwright/Chromium sessions,
  screenshots, DOM queries). Real and necessary for viewer changes, but the
  raw tool output is large and only useful once. Always delegate this to a
  fork or `.claude/agents/spex-verifier.md` — see `verification.md`.
- **Rebuilding the viewer + Rust workspace repeatedly.** `npm run build`
  then `cargo build --release` for every small viewer tweak adds up; batch
  related viewer changes before rebuilding when reasonable.
- **Live external-data fetches**, especially ones that need retries/backoff
  against a real rate limit (the Wikipedia crawl's depth-5 attempt burned a
  lot of real wall-clock time before being scoped down to depth 3 — see
  `working-mode.md`). If a fetch is going to be reused as a demo's data
  source, capture it once and commit the snapshot rather than re-running it
  live every session.
- **Large file reads** and **full `git log`/diff dumps** when only a small,
  known slice is actually needed.

## What's cheap and worth doing directly, without delegating

- Doc edits (`TODOs.md`, `CLAUDE.md`, `README.md`, these `docs/agents/`
  files) — no build/test cycle at all.
- Planning/backlog updates in `TODOs.md` — genuinely free; a good thing to
  reach for specifically when budget is tight but there's still real value
  to add (see below).
- Small, targeted Rust edits with a fast `cargo test -p <one-crate>` cycle.
- `cargo clippy --workspace --all-targets` — fast, and has caught real
  (if minor) issues worth fixing (a redundant `.trim()`, a needless
  `.collect()`, a needless range loop) with essentially zero verification
  cost beyond the lint pass itself plus a quick `cargo test --workspace`.

## What's *not* worth it just to "keep moving"

Busywork with a large diff and no real functional value is a bad trade even
when there's spare budget to spend — it's real review/merge cost for the
user later, for no benefit. Concretely: this codebase has never been run
through `cargo fmt` and deliberately uses a denser style in places (long
single-line expressions) than default rustfmt width would produce; running
`cargo fmt` "just to use up remaining budget" would touch hundreds of
pre-existing lines for pure style with zero functional change — that's not
a good use of either budget or the user's later review time. Prefer small,
real, checkable improvements (a stale doc claim, a genuinely-scoped backlog
item, a real clippy warning) over motion for its own sake.

## How this project's pace has actually shifted as budget got tight, in real sessions

Sessions in this project have converged on a real pattern, not just a
theory: when told the session's token budget is low, **keep shipping**
rather than pause-and-summarize (see the `feedback_burn_budget_keep_going`
memory — unused budget resets on a schedule and is lost, not banked, so
idling to "conserve" it produces no benefit). But the *kind* of work shifts
as the remaining budget gets tighter:

- **Plenty of budget** → bigger, well-scoped features with full
  verification ladders (a new adapter, a new viewer capability), each its
  own commit.
- **Budget getting tight** → smaller, cheaper-to-verify units: doc
  freshness fixes, clippy cleanup, tuning an existing constant with a quick
  before/after check, wiring an already-built feature into one more place
  it should reach (e.g. adding an existing static feature's link to the
  gallery).
- **Budget critically low** → docs/planning only (this file's "cheap"
  list) — genuinely free, and TODOs.md backlog entries written *now* are
  exactly what makes the next session (however much budget it has) able to
  start executing immediately instead of re-discovering what's next.

Real blockers are not something to route around just because there's
budget pressure to "go faster" — a task needing a real Linux box, an
ethical call on scraping a site's ToS, sudo/elevated privileges, or a
bigger architecture discussion stays flagged and blocked regardless of how
much budget remains. Working mode adapts to budget; correctness and
authorization boundaries don't.
