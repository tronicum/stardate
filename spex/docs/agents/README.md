# Agent operating playbooks

These aren't generic "how to be a good AI agent" advice — they're the
concrete, project-specific patterns that came out of actually building
`spex` over many real sessions: what caused rework, what the user corrected,
what turned out to work well and got repeated on purpose.

- **[working-mode.md](working-mode.md)** — the real-data policy, commit
  discipline, and the recurring gotchas (stale background servers, "the bug
  is in the code, not the data" as a default assumption to check rather than
  trust) that have bitten this project more than once.
- **[adapters.md](adapters.md)** — the checklist for adding a new input
  adapter (a new `crates/spex-cli/src/*.rs` module that turns some real
  external tool's output into a `spex_graph::Graph`), and the constraints
  `Graph`'s tree-only model forces on every one of them.
- **[verification.md](verification.md)** — the verification ladder used
  before every commit in this project: build, test, functional CLI check,
  and — for anything viewer-visible — a real headless-browser check, plus
  the viewer-rebuild-order gotcha that has caused "my change isn't showing
  up" confusion more than once.
- **[context-budget.md](context-budget.md)** — what's expensive to do in a
  main agent thread versus what's cheap, and how this project's pace has
  actually shifted (in real sessions) as available budget got tighter.

If you're a subagent dispatched from a coordinator working on this repo,
you likely don't need to read all of these — `.claude/agents/spex-verifier.md`
and `.claude/agents/spex-adapter-writer.md` already fold the relevant parts
into their own instructions.
