# Budgets — device, memory, long-run, and AI

*Numeric limits the project designs to, rather than discovers.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**
Sources: the browser-platform review and the agentic-coding review in [`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md) §2 and §7.

---

## 1. Device budgets

Every viewer-visible milestone asserts against these. They are not targets to
approach; they are limits that fail a build.

| | M1 MacBook Air | Integrated Intel | Mid Android tablet |
|---|---|---|---|
| Resolution / DPR | 1600×1000, **DPR cap 1.5** | 1080p, DPR 1 | 0.6× scale, DPR 1 |
| Frame target | 60 fps | 60 fps | **30 fps** |
| Triangles/frame incl. shadow | 3.0 M | 1.5 M | 0.5 M |
| Draw calls | ≤ 150 | ≤ 120 | ≤ 80 |
| Instance-matrix bytes uploaded/frame | ≤ 512 KB | ≤ 512 KB | ≤ 128 KB |
| Drawn edge segments | ≤ 300 k | ≤ 120 k | 0 (post-process outline only) |
| GPU total | ≤ 500 MB | ≤ 350 MB | ≤ 200 MB |
| JS heap steady state | ≤ 250 MB, drift ≤ 10 MB/h | same | ≤ 150 MB |
| Post chain | bloom ½-res, SMAA, SSAO off above 50 k instances | bloom ¼-res only | one ¼-res bloom |

**Where the memory actually goes at 250 000 instances** — the numbers that
make the table above non-negotiable:

| Item | Cost |
|---|---|
| `instanceMatrix` (16 × f32) | **16.0 MB CPU + 16.0 MB GPU** |
| …re-uploaded every frame via a blanket `needsUpdate` | **960 MB/s**, 2–4 ms of a 16.7 ms budget, plus driver buffer ghosting that fragments over days |
| Per-instance dissolve scalar | 1.0 MB |
| Instance ids as JS strings + `Map` | **32 MB — discard after glob resolution** |
| Shadow maps, 2048² × 3 cascades | 48 MB |
| Composer half-float targets at DPR 2 | **~140 MB** — the sneakiest line; the DPR cap alone saves 60 MB |
| Octree points at the current 5 M budget | 75 MB GPU + 75 MB retained CPU |
| `mesh.json`'s `instances[]` as JSON | **37 MB text → ~120 MB parsed heap → 0.8–1.5 s main-thread parse** |

The last line is why instances are binary (10 B each = 2.5 MB, see
[`phase1-renderer.md`](phase1-renderer.md) B11). It is also what makes the
single-file edition possible at all.

**Edges.** WebGL2 has no instancing-of-instances. Fat-line quads at 250 k
instances × ~150 edges × 4 vertices is **150 M vertices/frame** — dead on
arrival, and at crowd distance every outline merges into a black mass anyway.
Real LDraw type-2/5 edges are for hero shots only (above ~40 px projected
height, ≤ 3 k bricks); everything else gets a screen-space
depth+normal-discontinuity outline whose cost is independent of instance
count.

## 2. Cheapest big wins, ranked

1. **Dirty-set evaluation.** Evaluating all 250 k instances is 8–15 ms, not
   the 2 ms M87 claims. Evaluating only the instances an active track
   actually touches — typically under 20 k — is **under 1 ms**. *The single
   largest correction in this spec.*
2. **`attribute.addUpdateRange(offset, count)`** (three r159+; `updateRanges`
   in r185). 16 MB/frame → ~300 KB. The wasm evaluator returns per-group
   dirty *ranges*, not just a changed flag.
3. **Visible-instance compaction in wasm.** `InstancedMesh` culling is
   all-or-nothing; pack visible instances contiguously and set `mesh.count`.
   Turns a 250 k Atlas into a 20 k draw.
4. **Binary instance encoding.** Removes 37 MB of JSON, 120 MB of heap, and a
   second of time-to-first-frame.
5. **`setPixelRatio(Math.min(devicePixelRatio, 1.5))`.** One line; ~3× fill
   rate on a high-DPR tablet. `main.ts:300` currently uses the raw ratio.
6. **LOD2 (12-triangle OBB) as the crowd default**, LOD0 budget ≤ 3 k
   instances, LOD1 ≤ 20 k. 250 k × 12 = 3.0 M triangles — exactly the M1
   budget.
7. **Screen-space outline** instead of geometric edges at crowd distance.
8. **Fix `fetchNodePoints`' per-point `DataView` loop** (`tileset.ts:117`,
   ~100 ms/MB on the main thread): pad the record 15 → 16 B, use typed-array
   views, move to a Worker with transferables.
9. **Rewrite `selectNodes`** (`lod.ts:53`) — it allocates a `Frustum`, a
   `Matrix4`, a `Box3` per node, a heap and a `Set` *every frame*. Persistent
   structures, run at 10 Hz.

**Off-thread:** the audio scheduler and (if taken) M89b's DSP go in an
`AudioWorklet` — mandatory. A Worker handles bundle fetch/decode/unpack and
endless-mode score generation. **Do not use `OffscreenCanvas`**: it breaks
OrbitControls and DOM-label sync, and Safari's worker WebGL2 is unreliable.
No `SharedArrayBuffer` — it needs COOP/COEP, which the `file://` single-file
edition cannot have. Transferables only.

## 3. What breaks in a long run

An installation tab runs for days. Every item here is either in the current
viewer or in rev 1 of this spec.

| # | Failure | Mandated mitigation |
|---|---|---|
| 1 | `hudEl.innerHTML = …` every frame (`main.ts:419`) — 216 000 HTML parses/hour, detached nodes, GC sawtooth | Pre-built nodes, `textContent`, ≤ 4 Hz |
| 2 | `updateLabels()` projects every label and writes `style.display` on every element every frame — forced style recalc, O(N) | Spatial cull; touch only changed elements |
| 3 | `setInterval` for the audio scheduler — Chrome intensive-throttles hidden tabs to 1/min while `AudioContext` keeps running: starvation, gaps, then a multi-hour jump on return | Drive the scheduler from an `AudioWorkletProcessor` message pump; pause `ShowClock` on `visibilitychange` |
| 4 | **f32 absolute time** — three days is 259 200 s, where f32 resolves to 16 ms | **f64 for absolute time everywhere**; f32 only for shot-local normalised `t` |
| 5 | Per-frame allocation across the wasm boundary (`Box<[f32]>`, `String`) | Shared preallocated scratch buffers; call `take_cues` only when the dirty flags say so |
| 6 | `memory.grow` detaching every `Float32Array` view | Linear memory with **`initial == maximum`** so a grow traps instead of silently detaching; JS checks `view.buffer !== memory.buffer` immediately before `render` as a belt |
| 7 | `advanceToFrame()` disposes and recreates every node geometry per sequence frame — thousands of GL allocations/second | Pooled fixed-size geometries, `bufferSubData` only |
| 8 | **No `webglcontextlost` handler exists anywhere** | `preventDefault()` + a full rebuild path, tested with `WEBGL_lose_context`. Non-negotiable for an installation |
| 9 | WebAudio node churn: ~8 notes/s × 7 nodes ≈ 200 000 nodes/hour, retained by the pending-voice list and `onended` closures | Fixed voice pool, explicit `disconnect()`, assert node count flat. Three simultaneous 6 s convolvers during an IR crossfade is ~30 % of a tablet core — crossfade through one convolver + gain, or shorten the tail |
| 10 | Endless-mode fugue memory: 90 000 bars over three days | ≤ 64-bar ring buffer; assert `memory.buffer.byteLength` constant over a 6-hour soak |

## 4. Safari and mobile, specifically

- iOS `AudioContext` **suspends on any interruption and will not resume
  without a fresh gesture**; `sampleRate` is hardware-locked; contexts leak
  unless `close()`d. **The "▶ begin" gate must be re-armable mid-run.**
- **No `EXT_disjoint_timer_query_webgl2`** in Safari — GPU-time
  instrumentation silently returns nothing there. Gate the assertion on
  extension presence.
- iPad tab memory ceiling ~1–1.5 GB, killed silently. The 37 MB JSON parse
  peak alone was a real risk.
- Safari drops WebGL contexts far more aggressively than Chrome on tab switch.
- `performance.now()` is coarsened to 1 ms without cross-origin isolation, so
  the ≤ 3 ms onset assertions cannot be measured in Safari at all.
- Composer float targets: use `HalfFloatType`; `EXT_color_buffer_float` is not
  reliably present on iOS.
- Tile-based mobile GPUs punish render-target switches — each post pass costs
  a full tile flush. Low tier = **one** bloom pass, no SSAO, no shadows.

**Test Safari on real hardware in week 6, not week 25.**

## 5. Loading

- Time-to-first-paint **< 500 ms**: a static title-card HTML paints before
  the three.js module loads. The autoplay gate buys the rest for free.
- First rendered frame ≤ 2 s after the gesture on M1, ≤ 5 s on a tablet.
- gzip/brotli at the HTTP layer, plus position quantisation to i16 on the
  20/8 LDU grid — **exact, not lossy**, because the grid is exact.
- **Skip Draco** (a 200 KB decoder for ~2 MB of geometry is negative ROI).
  Meshopt's 25 KB decoder only if geometry passes 10 MB.
- Hashed filenames, `Cache-Control: immutable, max-age=31536000`;
  `mesh.json` short-cache.
- **M95's single-file edition is viable only for the 240 s, tier-A cut.**
  Inline *gzipped* bytes and inflate with `DecompressionStream('gzip')` —
  base64 of raw wasm alone would be ~4 MB. Realistic budget: wasm 900 KB +
  tree-shaken three ~250 KB + geometry ~400 KB + instances ~100 KB + score
  50 KB ≈ **1.7 MB gz ≈ 2.3 MB base64**. The 40-site Atlas will not fit, and
  M95 must say so rather than discover it in week 26.

## 6. Measurement plan

**Assert counters in CI on every viewer-visible milestone. Never assert fps
in CI.**

| Instrument | Assertion |
|---|---|
| `renderer.info.render.calls` / `.triangles` | ≤ the tier limits in §1 |
| `renderer.info.memory.geometries` / `.textures` | **delta == 0** over a 5-minute soak — a free leak canary |
| `performance.measureUserAgentSpecificMemory()` | heap < 300 MB, **delta < 10 MB over 10 min** |
| `PerformanceObserver({type:'long-animation-frame'})` | **zero** LoAF entries > 50 ms during playback (supersedes `longtask`) |
| Chrome DevTools Performance panel, Memory checkbox | the JS-heap sawtooth returns to the same baseline; allocation-instrumentation-on-timeline is how "zero allocations per frame" is actually verified |
| Memory panel comparison snapshots | no detached DOM (the `#labels` divs, the HUD) |
| `wasmMemory.buffer.byteLength` | constant; a `memory.grow` in a soak run is a **hard CI failure** |
| `AudioContext.baseLatency` / `outputLatency` | recorded; `AudioWorkletProcessor.process()` invocations vs. expected — **zero dropouts** over 10 min |
| `gl.bufferSubData` wrapped in a counting proxy (test build) | bytes-uploaded-per-frame against §1. *This is the assertion that stops win #2 from silently reverting.* |
| `EXT_disjoint_timer_query_webgl2` | GPU frame time — Chrome desktop only, gate on extension presence |

fps is asserted only on the named real hardware in M92's matrix.
`chrome://tracing` with `disabled-by-default-gpu.service` (and Metal System
Trace on macOS) for the GPU-memory-fragmentation soak.

**Rejected:** `--disable-gpu` as a Low-tier proxy. That is SwiftShader, ~100×
slower; it fails every fps gate for reasons unrelated to this code, and
tuning Low against it would make Low far uglier than it needs to be.

## 7. WebGPU

**Not this year. WebGL2 only, and no dual path.**

WebGPU did reach Baseline across Chrome/Edge, Firefox 141 and Safari 26 — but
Safari 26 requires macOS 26 / iOS 26, a real exclusion for an installation
machine or a 2022 Android tablet. More decisively, three's `WebGPURenderer`
is a *different* renderer with a *different* post stack (TSL nodes): the
conditional-edge shader, the PMREM environment and the whole composer chain
would be rewritten, not ported.

The one thing WebGPU genuinely buys here is compute-shader transform
evaluation with matrices never leaving VRAM — which removes the 16 MB/frame
upload entirely. But wins #1–#3 in §2 get ~95 % of that benefit for ~1 % of
the work, and they are needed on WebGL2 regardless. Keep material and edge
code in plain GLSL-shaped chunks so a later port stays cheap. Revisit only
past ~500 k instances, or if GPU-side culling becomes the bottleneck.

---

## 8. The AI budget

| Phase | Input tokens | Output | Model mix | API cost | Human h |
|---|---|---|---|---|---|
| P1 Renderer | 90–200 M | 3–6.5 M | 40 % Opus / 50 % Sonnet / 10 % Copilot | $350–780 | 60–90 |
| P2 Show | 70–160 M | 2.3–5 M | 25 / 65 / 10 | $220–500 | 40–60 |
| P3 Audio | 55–130 M | 1.9–4 M | 35 / 55 / 10 | $210–480 | 50–80 |
| P4 Kit / Atlas | 70–160 M | 2.3–5 M | 10 / 75 / 15 | $170–390 | 40–60 |
| P5 The work | 90–200 M | 2.9–6 M | 30 / 60 / 10 | $310–700 | 80–120 |
| P6 wasm | 40–90 M | 1.3–2.8 M | 25 / 70 / 5 | $130–290 | 25–40 |
| P7 Ship | 25–60 M | 0.8–1.8 M | 10 / 70 / 20 | $70–170 | 40–60 |
| **Total** | **450 M – 1.0 B** | **15–31 M** | ~25 % Opus | **$1 460–3 310** | **335–510** |

Midpoint ≈ 700 M in / 22 M out / ~$1 750 / ~420 human hours (~16 h/week).

**Cheapest for this project shape: a subscription tier (~$1 200 over 26
weeks), with API keys held for overflow in P1 and P5.** Token burn here is
dominated by *cache reads on a stable repo* — the same `CLAUDE.md`, the same
crate tree, thousands of turns — which is exactly what subscription pricing
absorbs and per-token pricing punishes. The binding constraint is the 5-hour
rolling limit during screenshot-heavy weeks, not dollars. A local-model
hybrid saves $150–300 at best, because local tokens displace the *cheap*
tokens.

### Routing criterion

**Cost-of-wrong-choice ÷ speed-of-feedback-loop** — not task difficulty.

- **Frontier (Opus-class)** where the loop is slow, invisible or
  architectural: M51's BFC winding, M57's conditional-edge shader math, M60's
  schema design (every later milestone inherits it), M61's resolver, M68's
  counterpoint, M83's loop seam, M87's wasm boundary — and *any* debugging
  session whose symptom is a picture.
- **Mid (Sonnet-class)** wherever `cargo test -p <crate>` closes the loop in
  seconds: the bundle writer, grid legality, heritage ingestion, CIELAB
  quantisation, CLI wiring, all six JSON Schemas and their validation cases,
  the mechanical wasm port. This is the bulk of the work.
- **Copilot inline** only inside one file with a human present: GLSL constant
  tuning, `tsc` error fixing, test bodies. It has no model of `CLAUDE.md`'s
  invariants — never cross-crate.
- **Local (Qwen3-Coder-30B / Devstral class, 32–64 GB)**: commit messages
  from diffs, DE↔EN prose for chronicle cards and Postillen, transcribing
  published flag construction tables into `flags/<iso2>.json`, batch renames,
  JSON reformatting. It genuinely works there; it wastes human time on
  anything needing current three.js or wasm-bindgen API recall.

### Context economy

1. **This split is win #1.** `docs/FUGEN-ENGINE.md` was 130 KB ≈ 33 k tokens
   re-read per session. A phase file is ~4 k. *Done — that is what rev 3 is.*
2. **Trim `CLAUDE.md`** (17.7 KB, re-sent every turn): keep the commands, the
   crate one-liners and the viewer-rebuild-order gotcha; move the
   paragraph-length per-adapter prose to `ARCHITECTURE.md`. Archive M1–M50
   out of the 87 KB `TODOs.md`.
3. **Verdict-only verification.** Enforce a ≤ 200-word return from
   verification forks: no screenshots, no cargo/npm logs into the
   coordinator. `2>&1 | tail -5` on builds. `walkthrough.sh` and the seed
   sweep never touch the main thread.

Session boundary = one milestone. Prefer *finish → `TODOs.md` entry → clear*
over compaction: the `TODOs.md` entry is a lossless, human-reviewed, free
summary. Compact only mid-milestone, past ~70 %.

### Where agentic coding will fail on this project

- **Shaders.** A wrong-sign cross product still renders *something*. Demand a
  **numeric proxy for every visual claim** — M57's "conditional-edge count
  changes across 12 orbit angles" is a number an agent can check. The human
  looks at the 2000×2000 PNGs at 0.5× and 50×.
- **Audio.** Agents cannot hear, and headless audio is unreliable. Substitute
  `OfflineAudioContext` renders with asserted onset sample indices. "Is the
  fugue musically dead" is 100 % human, and the week-3 listen is a **calendar
  event**, not an acceptance criterion.
- **wasm memory growth.** Silent, and it looks like a GPU bug; agents will
  "fix" it with `needsUpdate` flags. Require the detach test before any
  performance work.
- **3D spatial judgement.** Grid-legal geometry can still be ugly. Use
  `spex ascii` — this repo already owns a cheap, agent-readable 3D proxy.
- **The real-data rule.** Under pressure, agents confabulate flag ratios and
  heritage criteria. Every fact cites an in-repo source; the exclusion list
  stays human and fails closed.
