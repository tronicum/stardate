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

## Measuring what a material looks like

```
node scripts/viewer-shot/swatch.mjs http://127.0.0.1:8096/
```

Projects every instance's centre to screen space, samples the rendered frame
there, and prints the sRGB triple plus its luma. "Does chrome read as metal?"
is a question about relative luminance, and a screenshot only answers it if
someone looks carefully at the right pixel.

This is how M56's environment was built. Four attempts, each wrong in a way
only measurement showed: chrome at `95,99,108` (grey plastic) against a
near-black studio floor; real LDraw Red clipping to `255,146,108` (orange)
once the environment was raised to fix it; and — the one nobody would have
guessed — red rendering `255,120,88` **with every direct light switched off**,
against `40,0,0` from the entire rig, which is how the three small "highlight"
cards turned out to be the scene's actual lighting.

## Orbiting, for the conditional-edge test

```
node scripts/viewer-shot/orbit.mjs http://127.0.0.1:8093/ /tmp/m57
```

Twelve angles, one screenshot each, plus 0.5x and 50x the default camera
distance. Prints *which* conditional edges pass the silhouette test at each
angle — the set, not its size. That distinction is the point: a cylinder shows
exactly two silhouette edges from every direction, so the count is constant
even when everything works, and only the identity of the two rotates. Exits
non-zero if the set never changes, which would mean the test is not running.

## The bloom ramp and the empty scene

```
node scripts/viewer-shot/bloomramp.mjs http://127.0.0.1:8093/ /tmp/m58
```

Ramps the bloom threshold 1.0 → 0.2 over 30 captured frames and prints each
frame's mean luminance, then hides every object and checks the frame is
neither black nor NaN. Exits non-zero if the ramp is flat, non-monotonic, or
the empty scene comes back black.

A flat luminance curve is the interesting failure: it means bloom is reading a
signal that was already clipped to 0..1 before it got there, which is the
whole reason the post chain renders into a HalfFloat target with tone mapping
off.

## LOD transitions

```
node scripts/viewer-shot/dolly.mjs http://127.0.0.1:8091/ /tmp/m59
```

Pulls the camera back over 60 frames and asks whether the level switches are
visible. It renders **each frame twice** — once as selected, once with every
instance pinned to LOD0, same camera — because comparing consecutive frames
cannot separate a LOD pop from the shot simply changing. The first version did
compare consecutive frames and reported a 6.49 % "jump" that was the ground
receding.

It also prints the LOD population per frame and fails if it never changes: a
dolly where nothing switched would pass the luminance test trivially and prove
nothing. That assertion is what caught the harness bug where
`c.position.copy(t).add(c.position.clone()…)` parked the camera on the object
every frame — JavaScript runs the `copy` before evaluating the argument.

## The show engine, measured without a picture

```
spex show-build shows/die-geschichtliche-matrix.show.json -o /tmp/show --duration 240 --no-bundles
node scripts/viewer-shot/showprobe.mjs /tmp/show/show-resolved.json
```

M62's clock and timeline evaluator have no render pass, so there is nothing to
screenshot — but they are still browser code, and three of their four
acceptance criteria are only answerable in a real browser. `showprobe.mjs`
bundles `viewer/src/show/` with esbuild, injects it into Chromium, and reports
four numbers: seek determinism (an FNV hash over every value the evaluator
emits, seeked versus played-through), clock drift against
`audioContext.currentTime`, allocations per frame from Chromium's heap
sampler, and the endpoint/monotonicity behaviour of every easing curve.

**The allocation number is only worth reading because of the positive
control.** Alongside the measurement the probe runs a loop that allocates one
small object per frame, so the report says both "evaluate is at the noise
floor" and "here is what the noise floor can see" (28 B/frame). The first
version of that control kept its objects in a local array and reported only
`array.length` — V8's escape analysis then allocated nothing at all, and 6 000
allocations measured 1.4 kB. A control the optimiser may delete measures the
optimiser.

**The drift figure is not a hardware figure.** This container has no audio
device, so Chromium's `AudioContext` runs on a synthesised clock, and the
~90 ms/minute divergence between it and `performance.now()` should not be
quoted as what a real machine does. What it does establish is that the two
clocks are independent — which is the reason the show reads the one the sound
is on. Show time against its own chosen source measures 0.0000 ms, and that
part is arithmetic rather than hardware: time is derived from an anchor, never
accumulated frame by frame.

## DER KICK

```
spex mesh-model ldraw-scenes/monolith.ldr -o /tmp/mono
spex serve /tmp/mono --port 8098 --no-open &
node scripts/viewer-shot/kick.mjs http://127.0.0.1:8098/ /tmp/m63
```

Drives M63's camera director through the piece's final two beats — a 10⁴
pull-back — and reads pixels at every frame. Writes three keyframes and
reports the collapse, the depth range, and `?free=1`.

**It renders its own frames rather than using the viewer's loop.** That loop
calls `controls.update()`, which rewrites `camera.position` from
OrbitControls' internal spherical coordinates, so anything the harness wrote
would be gone before it reached the screen. Each frame is therefore set,
updated, `post.render`ed and `readPixels`ed synchronously, and the viewer's
own loop never gets a turn in between.

**Each frame is rendered twice, with the bricks and without.** Counting bright
pixels instead measures the ground plane, whose lit area changes as the camera
recedes — the first version reported the object *growing by 1181 px* while it
was shrinking. Same confound as `dolly.mjs`, same fix.

**And the zoom is driven three times**, because motion blur legitimately grows
an object's footprint while the object shrinks: blurred for the pictures,
unblurred for the collapse measurement, and once more under the spec's
`[d/1e4, d*1e4]` depth range for comparison. Measuring a collapse through a
blur is measuring the blur.

## The runtime assembly

```
spex mesh-model car -o /tmp/car
spex serve /tmp/car --port 8100 --no-open &
node scripts/viewer-shot/assembly.mjs http://127.0.0.1:8100/ /tmp/m64
```

Checks M64's four things: that the TypeScript splitmix64 reproduces the Rust
one bit for bit against `docs/fugen/fixtures/assembly-scatter.json`, that the
runtime assembly lands where the baked `brick-assembly` demo does, what the
per-frame transform pass costs, and whether `0 STEP` order actually looks
different from index order.

**The fixture is the point of the first check.** Both implementations are
compared to a committed file that neither generates at test time, because two
implementations compared only against each other can drift together.

**`m.lod()?.update()` after `writer.flush()` is load-bearing**, and the first
version of this harness did not do it. Since M59, `InstanceWriter` writes only
into `group.matrices` and the LOD selector is what copies that into the meshes
the GPU reads. Without the update every number this harness prints still
passed and every screenshot was of a car that had never moved — which is how
that latent bug in `flush()` was found.

Use `car` and not the monolith for AC3: `ldraw-scenes/monolith.ldr` is
hand-authored and has no `0 STEP` lines, so there is no real build order to
stagger by.

## M66 — the show

Three scripts, because one page cannot answer all three questions.

- **`showrun.mjs <url> <out>`** — AC1 and AC3. The arithmetic is asked of the
  evaluator with no frames at all; whether cues *fire* and the piece loops is
  asked by playing the endless cut once through in real time, because a sweep
  of seeks fires no cue by design. Each URL parameter gets its own page load —
  sharing one would let a parameter pass because of what a previous one left
  behind. Writes `m66-showrun.json` and, if the loop is not clean, the two
  frames it compared.
- **`showframes.mjs <url> <out>`** — rung 5. The pictures, shot *without*
  `?director=1`, whose HUD covers most of the frame. One extra frame with it.
- **`showexport.mjs <dir>`** — AC2. Serves a `spex show-export` output at a
  domain root and again under a deep subpath from the same bytes, then tries
  `file://`, and reports what each one did.

`showprobe2.mjs` is a scratch diagnostic rather than a test: it prints the post
chain, camera and ground state at t=0 before and after a real loop, and renders
the same frame with and without the ground and with and without the
environment. It is what found the Fresnel answer to "why is the opening frame
not black".

**`showvideo.mjs <url> <out> [cut] [frames] [fps]`** is not a test — it is the
one that produces something to *watch*, for people who are not sitting at the
machine that renders it. On a software rasteriser at two or three frames a
second there is no screen-recording a show live, so it pauses the clock and
steps it: seek, render, capture, repeat, with show time advancing a fixed
amount per captured frame. The recording is therefore a property of the piece
and not of the machine. The trade is that anything derived from *frame* time
rather than show time — the grade pass's dither, a materialise flash decay —
gets stepped along with it. Assemble with ffmpeg:

```sh
node scripts/viewer-shot/showvideo.mjs http://127.0.0.1:8120/ /tmp/vid endless 300 15
ffmpeg -framerate 15 -i /tmp/vid/f%04d.png -c:v libx264 -pix_fmt yuv420p -crf 20 out.mp4
```
