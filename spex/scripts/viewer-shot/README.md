# viewer-shot — rung 5 of the verification ladder

Every viewer-visible milestone in `docs/fugen/` has to produce a real
screenshot from a real browser, plus the renderer's own counters. "It looked
right on my screen" is not a result anyone can re-check in six months; a PNG
and a number are.

## Setup

```
cd scripts/viewer-shot && npm install
```

`playwright` is pinned exactly (not `^`) because the browser binaries in this
environment are installed once, out of band, and a minor version bump makes
Playwright look for a build number that isn't there.

## Taking a shot

```
spex mesh-model car -o /tmp/car-mesh
spex serve /tmp/car-mesh --port 8092 &
node scripts/viewer-shot/shot.mjs http://127.0.0.1:8092/ /tmp/car.png --expect-mesh
```

Writes `/tmp/car.png` and `/tmp/car.json` (the counters, the HUD text, and
every console error / warning / failed request). Exits non-zero on any console
error, any failed request, or — with `--expect-mesh` — if the page did not
actually take the mesh path.

Flags: `--width`, `--height` (default 1600×1000), `--settle` (ms before the
shot, default 4000, so the fps counter has a real window behind it),
`--expect-mesh`.

**The 404s on `mesh.json`, `sequence.json`, `nodes.json` and `meta.json` are
not errors.** Their absence is how the viewer picks a render mode, so the
harness filters exactly those four out of every collector. Anything else 4xx
is a real missing asset and fails the run.

## fps is not an assertion

This container has no GPU: Chromium falls back to SwiftShader, which is
roughly two orders of magnitude slower than real hardware. A monolith that
would run at hundreds of fps reports 4 here. That is a property of the
harness, not of the renderer.

So: **assert counters, never fps.** Draw calls, instance counts, triangle
counts and "zero console errors" are all hardware-independent and mean the
same thing everywhere. Frame rate is asserted only on the named real hardware
in M92. This mirrors the rejection already recorded in
`docs/fugen/phase1-renderer.md` — SwiftShader was rejected as a Low-tier
performance proxy for the same reason.

## Isolating a rendering artefact

```
node scripts/viewer-shot/isolate.mjs http://127.0.0.1:8091/ /tmp/mono 330 150 260 600
```

Renders the same crop five times, each with exactly one contribution removed:
tightened depth range, roughness forced to 1, shadows off, instances pulled
apart. Which image changes tells you what the artefact was. This is how M54
established that the seam lines on a stacked model are geometry and not
z-fighting: a 2000× tighter depth range left them pixel-identical.

## Very large scenes

At 50 000 instances the renderer submits 11 M triangles a frame, twice with
the shadow pass. On a GPU that is unremarkable; on SwiftShader it exceeds any
screenshot timeout. Two escape hatches:

```
node scripts/viewer-shot/probe.mjs http://127.0.0.1:8095/     # counters only, no pixels
node scripts/viewer-shot/shot.mjs  ... --no-shadows           # halves the geometry submitted
```

`probe.mjs` reads the same `window.__spexMesh` hooks and never calls
`page.screenshot`, which is what makes M55's 50 000-instance numbers
obtainable here at all. `--no-shadows` is for scale scenes only — never for a
milestone's hero shot.

`--bench` adds M55's transform measurement to a normal shot: how long it takes
to rewrite and upload every instance's transform, once through `setMatrix`
(what an animation curve produces) and once through `setTransform`
(position/quaternion/scale). Both medians are printed.
