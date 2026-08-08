# Verification

`cargo test` proves the code is internally consistent. It does **not** prove
a browser feature actually renders, that a CLI command's output looks
sensible to a human, or that a real external file/service really behaves
the way a unit test's synthetic fixture assumes.

So there's a ladder. **Rev 2 (2026-08) made most of it conditional**, because
running all of it before every commit cost more than it caught: across the
whole of Phase 1 the full-regeneration rung found nothing at all, while the
screenshot rung found three real bugs — but only on the commits where the
picture was *supposed* to change.

## The ladder

| # | Step | When |
|---|---|---|
| 1 | `cargo build --release` | **Always.** |
| 2 | A **real functional CLI check** — run the built binary against real input and read the output | **Always.** Cheapest real check there is. |
| 3 | `cargo test --workspace` | **Always, once**, immediately before committing. |
| 4 | `graph-layout` + `spex ascii` on the result | When the change touches the graph pipeline. |
| 5 | A real headless-Chromium screenshot, looked at | **When the picture is supposed to change.** See below. |
| 6 | `./scripts/walkthrough.sh` — regenerate every demo | At the **end of a phase**, or when something shared changed (a format, the tiler, the server). |

## What rung 6 did not cover, until Phase 3 ran it

**`walkthrough.sh` regenerated no mesh demo and no show demo at all.** Every
example it built was a graph or a point cloud — and Phase 3 changed the
instanced attribute layout, the LOD re-pack, the post chain's bloom, the
dissolve shader and the edge pass. The gate that exists to catch a shared
change breaking a demo nobody starts by hand any more was not covering the
demos the change was in.

It builds `mesh-model car`, `mesh-part 3001.dat` and the show directory now.
The phase-3 run that found this was completed by hand — `dissolve.mjs`,
`assembly.mjs`, `crossfade.mjs`, `showrun.mjs`, `lodprobe.mjs` — and all of
them passed once the second finding below was dealt with.

**And three probes had been reporting `FAIL` since M66 for two 404s that are
the design working.** The viewer picks between its three render modes by
*asking* — `show-resolved.json`, then `mesh.json` — and taking whichever
answers; a 404 there is a fact, and every module involved says so. The browser
logs it as a console error anyway, so any probe gating on "zero console errors"
fails on a plain mesh bundle. `scripts/viewer-shot/absence.mjs` matches on the
**URL of the failed response** rather than on the console text, which is the
same sentence for every 404 there has ever been, and discounts only the paths
whose absence is documented.

Both are the same lesson from opposite ends: **a gate nobody runs is a gate
that stops being true**, and it stops being true quietly.

Rung 3 used to appear twice, at the start and again as a final gate. Once is
enough: it is the same command against the same tree.

## When rung 5 is actually mandatory

Not "any viewer-visible change" — that rule produced dozens of screenshots of
frames that were meant to look identical, each costing a minute or more on a
software rasteriser.

**Mandatory** when the change is supposed to alter what a frame looks like: a
new render pass, a material, geometry, lighting, an effect. Those are the
milestones the pictures belong to anyway.

**Not required** when the change is supposed to leave the picture alone — a
refactor, an instancing change, a data-format change, a performance fix. For
those, assert the *counters* instead (`__spexMesh.stats`, draw calls, zero
console errors), which `scripts/viewer-shot/probe.mjs` reads without ever
rendering a frame to disk. If a "should look identical" change is risky, one
screenshot at a small viewport is a cheap tiebreak — that is a judgement
call, not a rule.

A blank frame is a failure to launch, not a passing check. Always read the
console-error count, whichever rung you are on.

## Measure, don't assert

**Eight of Phase 1's ~30 acceptance criteria were factually wrong**, and every
one failed the same way: it asserted a frame rate or a bound that the
environment or the mathematics could not deliver.

- `≥ 55 fps` and `≥ 60 fps` in a container with **no GPU**. Chromium falls
  back to SwiftShader, roughly two orders of magnitude slower than the
  slowest real hardware. The same document had already rejected
  `--disable-gpu` as a proxy for exactly this reason, and then three
  milestones asked for frame rates anyway.
- `draw calls ≤ 3 × distinct part count` — impossible, because colour is a
  material binding: one part in seven colours is seven instanced meshes.
- `< 4 ms` for a transform pass "on the development machine", measured on a
  shared container.
- "the conditional-edge **count** changes as the camera orbits" — it does
  not, and a correct renderer is why: a cylinder shows exactly two
  silhouette edges from every direction. Only the *identity* rotates.

**So: an acceptance criterion says "measure X and record the number" by
default.** A hard bound belongs only where exceeding it should actually stop
the build — a memory ceiling, a determinism check, a format invariant. If a
bound needs particular hardware, the criterion names the hardware and does
not apply anywhere else.

This is not lowering the bar. Every one of those eight was *measured*, and
the measurements were the valuable part: 29 draw calls at 50 000 instances,
0.26 % LOD error over a dolly, 96 % fewer triangles at LOD1. It was the
assertions that were worthless.

## Unit tests: one per bug that really happened

Rust tests are cheap and fast and have caught real parser bugs — keep
writing them. But write them **against something that went wrong or could
plausibly go wrong in a specific way**, not for coverage. The tests that
earned their place in this project all name a real defect:

- the same LDraw primitive classified differently depending on its reference
  chain (a stud vs. a wall),
- a speckle colour's `VALUE` read as the brick's because the line was not
  split at `MATERIAL` first,
- a mirrored reference matrix that must still face outward,
- the manifest not being byte-identical across two runs.

A test whose failure would not tell you anything you did not already know is
a test that only costs time.

## Two gotchas that are still real

**The build order is three deep.** `viewer/` is embedded into the `spex`
binary at *Rust compile time* via `rust-embed`, not read at runtime:

```sh
cd viewer && npm run build       # produces viewer/dist
cd ..      && cargo build --release   # re-embeds it
```

Skipping the second step is the single most repeated source of "I changed
the viewer but nothing's different" in this project's history. With WASM it
becomes `wasm-pack build` → `npm run build` → `cargo build --release`.

**A stale `spex serve` serves stale content.** If the process started before
your latest build, every check against it is meaningless.

```sh
ps aux | grep "target/release/spex" | grep -v grep
```

`pkill -f` has failed to actually kill it more than once — and note that a
`pkill -f "release/spex serve"` pattern can match the shell running it and
kill your own command. Verify the PID is gone before re-checking.

## Delegate expensive verification, keep the verdict

A headless-browser session or a long real-data fetch produces a lot of tool
output that is useful exactly once. Push it into a fork or the
`.claude/agents/spex-verifier.md` subagent with **concrete pass/fail
criteria** — "confirm the debug panel's hop denominator matches N", "sample
the tooltip at two points ~1 s apart and confirm the content differs" — and
let it report a short summary. An open-ended "does it look right?" is both
more expensive and less trustworthy than a specific check.
