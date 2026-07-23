# AGENTS.md

Entry point for any agent — a Claude Code subagent, a fork, or another AI
tool entirely — picking up work in this repository. This file is short on
purpose; it's a map, not the content.

## What this is, in one paragraph

`spex` is a point-cloud explorer: it turns LiDAR/scan files (PLY/XYZ/CSV/LAS/
LAZ) into a streamable octree tileset viewable in a browser. It doubles as a
generic tree/graph explorer — "input adapters" (traceroute, process tree,
package dependency trees, SMILES molecules, a Wikipedia link crawl, ...)
capture real-world trees into one common JSON format, laid out radially in
3D, and rendered through the *exact same* point-cloud pipeline as the LiDAR
case. There is no adapter-specific viewer code; everything becomes points.
See `docs/ARCHITECTURE.md` for the full reasoning and a worked example.

## Read these, in this order

1. **This file** — where to look for what, and the short version of how work
   happens here.
2. **`CLAUDE.md`** — the canonical technical reference: build/test commands,
   crate-by-crate architecture, tileset file formats, known limitations.
   Terse and reference-only by design; if you need the "why" behind
   something it points to, that's (3).
3. **`docs/ARCHITECTURE.md`** — the narrative version: a real worked example
   and the reasoning behind non-obvious design choices (radial layout, the
   fan-out cap, why three views of one model, the tree-only `Graph`
   trade-off).
4. **`TODOs.md`** — the single source of truth for project status. Numbered
   milestones (each with what was actually *verified*, not just built),
   what's actively being worked, and a pruned backlog of ideas actually
   worth doing next. **Check here before re-implementing something** — a
   surprising amount of "obvious next feature" ideas are already done, or
   already deliberately deferred with a documented reason.
5. **`docs/agents/`** — this repo's specific operating playbooks: how
   verification actually happens here, how adapters are built, how to spend
   context/token budget well. Written from what concretely worked (and
   didn't) across many real sessions building this project with AI agents —
   not generic advice. Start with `docs/agents/working-mode.md`.
6. **`.claude/agents/`** — two custom subagent definitions scoped to this
   repo (`spex-verifier`, `spex-adapter-writer`) that already know the
   playbooks in (5), so a coordinator can delegate without re-explaining
   them every time.

## The short version of how we work here

- **Real data only.** Every demo uses something actually captured or
  downloaded — real `ps`/`brew`/`npm`/`sqlite3`/`cargo tree` output, a real
  public dataset, a real API response, a real SMILES string for a real
  molecule. Never fabricated numbers, even for "just illustrative" demos.
  See `docs/agents/working-mode.md` for the committed-snapshot pattern this
  produced.
- **Small, verified, pushed commits.** One real feature or fix per commit,
  built and tested *before* committing, with a message explaining *why*, not
  just *what*. Don't batch unrelated changes into one commit.
- **Verify with the real thing, not just `cargo test`.** Tests prove the
  code is internally consistent; they don't prove a browser feature
  actually renders, or that a CLI command's output is sensible to a human.
  See `docs/agents/verification.md` for the verification ladder used every
  time in this project.
- **Delegate expensive verification, keep synthesis in the main thread.** A
  headless-browser session or a long real-data fetch produces a lot of
  one-time-use tool output. Push that into a fork or subagent and let it
  report back a short summary — see `docs/agents/context-budget.md`.
- **Don't idle-wait when there's real backlog work and nothing blocking
  it.** `TODOs.md`'s "Doing / next up" and "Backlog" sections distinguish
  ready-to-pick-up work from work that's intentionally blocked (needs a
  real Linux box, needs the user's call on data-scraping ethics, needs a
  bigger architecture discussion before starting). Keep pulling from the
  former; don't route around the latter unilaterally.
