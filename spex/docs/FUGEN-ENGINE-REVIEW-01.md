# FUGEN-ENGINE — Review 01

**Reviewed:** `docs/FUGEN-ENGINE.md` rev 1 (M51–M97), commit `b6a791e`
**Date:** 2026-07-25
**Panel:** seven specialist reviews, run independently, none seeing the others.

| # | Reviewer | Brief |
|---|---|---|
| R1 | Principal software architect | crate decomposition, signatures, wasm boundary, backwards compatibility |
| R2 | Technical artist / real-time rendering | will it actually look like a brick render |
| R3 | Screenwriter / editor (abstract short-form) | dramaturgy of §8 |
| R4 | Technical producer | 26 weeks, 47 milestones, one human |
| R5 | Creative director (generative art / installation) | is it art or a demo with a thesis pasted on |
| R6 | Historian of technology and material culture | are the historical claims true |
| R7 | Browser platform / performance expert | CPU, GPU, memory, long runs |
| R8 | Agentic-coding practitioner | token/model/cost budget for building it |

Decisions below are **ADOPT** (goes into rev 2 as binding), **ADOPT-MOD**
(adopted with a change, stated), **DEFER** (real, not now, goes to the
backlog with a reason), or **REJECT** (with a reason). Nothing is left
unanswered — an unanswered review finding is a finding that resurfaces in
week 14.

---

## 1. Blocking technical corrections (R1, R2, R7) — all ADOPT

These invalidate acceptance criteria as written. They must be fixed before
M51 is implemented, and they are the reason this review happened before any
code was written rather than after.

| # | Finding | Decision |
|---|---|---|
| B1 | **The conditional-edge test is inverted.** A type-5 line is a silhouette when both control points project to the *same* side of the edge. Rev 1 says the opposite in both M51 and M57 — which would draw exactly the cylinder tessellation and hide the silhouette, i.e. the precise failure M57's AC2 is written to catch. three.js's own `LDrawLoader` discards when `sign(d0) != sign(d1)`. | **ADOPT.** Corrected inline in rev 2. |
| B2 | **Colour is baked into geometry, so per-instance colour is impossible.** `resolve_part` substitutes LDraw code 16 during recursion, and `MeshBundleBuilder::add_part` keys on `part_file` alone — two colours of the same part collide on one key and duplicate identical geometry, defeating the instancing M55 depends on. | **ADOPT.** `resolve_part_full` preserves code 16 as `color_code: Option<u32>` (`None` = inherit); parts key on `part_file` only; `submeshes[].material: number \| null`, `null` = take the instance's material. |
| B3 | **The `ColorTable` change in M56 breaks the point pipeline**, contrary to M56's own AC3: `sampling.rs:95` and `brick.rs:216` both destructure the tuple, and an accessor method does not rescue a tuple pattern. Also `find_after(tokens,"ALPHA")` cannot parse `MATERIAL SPECKLE` — `ALPHA`/`LUMINANCE`/`VALUE` appear on both sides of the `MATERIAL` token. | **ADOPT.** `load_colors` keeps its tuple return, untouched. New `load_colors_full() -> HashMap<u32, LdrawColor>`, used only by `spex-mesh`. Parser splits at `MATERIAL` first. |
| B4 | **`spex serve <bundle-dir>` will refuse to start** — `main.rs:684` bails unless `tileset.json` or `sequence.json` exists, so M53 AC4 and M54 AC1 are false as written. | **ADOPT.** M53 adds `mesh.json` and `show-resolved.json` to that guard. |
| B5 | **M59's LOD1 is not implementable** from a flat `Vec<Triangle>` — `resolve_into` discards the reference chain, so stud/tube identification "on the reference path" has no path to gate on. | **ADOPT, moved to M51.** `PartGeometry` gains `sources: Vec<String>` and every triangle/edge a `source: u16` index. |
| B6 | **M87's wasm interface has three defects:** a `u32` bitmask caps at 32 instance groups (the Atlas has hundreds); one `matrix_ptr` cannot feed N `InstancedMesh`es without per-group offsets; and a Rust-side `memory_generation` counter cannot detect a grow triggered inside wasm-bindgen's own glue. | **ADOPT.** Group-aware offsets, a per-group dirty byte array, and the JS-side detach check `view.buffer !== memory.buffer` immediately before `render`. |
| B7 | **M87's own "never grow" mitigation contradicts its interface** — `frame_state() -> Box<[f32]>` and `take_cues() -> String` allocate every frame, which is exactly what grows linear memory. | **ADOPT.** Preallocated `frame_state_ptr`/`len` and a fixed cue ring buffer. Linear memory set `initial == maximum` so a grow traps instead of silently detaching (R7). |
| B8 | **Beat snapping and the stated timings are mutually unsatisfiable.** At 84 bpm a bar is 2.857 s. The Kick at 239.200 s for 0.800 s is neither on a beat nor a bar; the Atlas in-point is off-grid; §8 also says "336 bars" where 240 s at 84 bpm is 336 *beats* = 84 bars. | **ADOPT — the whole screenplay is re-authored in bars.** See §3. |
| B9 | **M51 AC3 is self-defeating** ("identical up to per-triangle vertex rotation" — but reversing winding is not a rotation) and AC1 confuses LDU with mm. | **ADOPT.** Compare unordered vertex-position sets; the bundle boundary is mm/Y-up and says so once. |
| B10 | **M59 AC1 depends on M74** — a week-6 milestone gated on a week-17 deliverable. | **ADOPT.** M59 verifies against a synthetic 200k-instance scene; the real-Atlas measurement becomes an AC on M81. |
| B11 | **`mesh.json`'s `instances[]` array is a 37 MB JSON blowup** at 250k instances → ~120 MB parsed heap and 0.8–1.5 s of main-thread parse. | **ADOPT.** Binary instance encoding: `(i16 x,y,z; u8 orientation; u8 material; u16 part)` = 10 B/instance = 2.5 MB. Grid legality (M72) is what makes this exact rather than lossy. This is also what makes M95's single-file edition possible at all. |
| B12 | **Colour management is wrong in the format.** `baseColor: [0.106,0.165,0.204]` is sRGB ÷ 255; three.js r152+ treats it as linear. Every material ships ~2.2× too dark. | **ADOPT.** Store linear, and state the colour space in `mesh.schema.json`. |
| B13 | **Tone mapping before bloom** makes the bloom threshold meaningless. | **ADOPT.** `NoToneMapping` on the renderer, HalfFloat targets, bloom in linear HDR, ACES in `OutputPass` last. |
| B14 | **Bloom on a black field bands** on an 8-bit backbuffer, and the spec never mentions dithering. | **ADOPT.** ±0.5/255 triangular dither plus ~1.5% fixed film grain in the output pass. Cheapest single thing separating "cheap WebGL" from "print". |

---

## 2. Performance and memory (R7) — ADOPT as binding budgets

R7's verdict, stated plainly: **the spec's own performance targets are not
reachable as designed, and two specific decisions fix most of it.**

**The two:**

1. **Dirty-set evaluation, not full evaluation.** M87's "<2 ms for 250 000
   instances" is not real — 4 M f32 stores plus quaternion→matrix is
   8–15 ms. Evaluating only the instances an active track actually touches
   (typically < 20 000) brings it under 1 ms. *Largest single correction in
   the document.*
2. **Screen-space outlines at crowd distance.** WebGL2 has no
   instancing-of-instances; geometric fat-line edges at 250 000 instances is
   ~150 M vertices/frame. Real LDraw type-2/5 edges are for hero shots
   (≤ 3 000 bricks, above ~40 px projected); everything else gets a
   depth+normal-discontinuity full-screen outline pass whose cost is
   independent of instance count.

**Binding device budgets** (rev 2 §3a):

| | M1 MacBook Air | Integrated Intel | Mid Android tablet |
|---|---|---|---|
| Resolution / DPR | 1600×1000, dpr cap **1.5** | 1080p, dpr 1 | 0.6× scale, dpr 1 |
| Target | 60 fps | 60 fps | **30 fps** |
| Triangles/frame incl. shadow | 3.0 M | 1.5 M | 0.5 M |
| Draw calls | ≤ 150 | ≤ 120 | ≤ 80 |
| Instance-matrix bytes uploaded/frame | ≤ 512 KB | ≤ 512 KB | ≤ 128 KB |
| Drawn edge segments | ≤ 300 k | ≤ 120 k | 0 (post-process only) |
| GPU total | ≤ 500 MB | ≤ 350 MB | ≤ 200 MB |
| JS heap steady state | ≤ 250 MB, drift ≤ 10 MB/h | same | ≤ 150 MB |

**Long-run failures that must be designed out** (an installation tab runs
for days; every one of these is in the current viewer or the current spec):

- `hudEl.innerHTML` every frame → 216 000 HTML parses/hour, detached nodes.
  Pre-built nodes, `textContent`, ≤ 4 Hz.
- `updateLabels()` touching every label element every frame → forced style
  recalc. Spatial cull; only write changed elements.
- `setInterval` for the audio scheduler → Chrome intensive-throttles hidden
  tabs to 1/min while `AudioContext` keeps running: scheduler starvation,
  then a multi-hour time jump on return. **Drive the scheduler from an
  `AudioWorkletProcessor` message pump** and pause `ShowClock` on
  `visibilitychange`.
- **f32 absolute time.** Three days is 259 200 s, where f32 resolution is
  16 ms. f64 for absolute time everywhere; f32 only for shot-local `t`.
- **No `webglcontextlost` handler exists.** Non-negotiable for an
  installation; test with `WEBGL_lose_context`.
- WebAudio node churn (~200 000 nodes/hour) → fixed voice pool, explicit
  `disconnect()`.
- Endless-mode fugue memory → ≤ 64-bar ring buffer, asserted constant over
  a 6-hour soak.
- `advanceToFrame()` disposes and recreates every geometry per frame →
  pooled geometries, `bufferSubData` only.

**Cheap wins, ranked:** dirty-set evaluation → `addUpdateRange` partial
uploads (16 MB/frame → ~300 KB) → visible-instance compaction in wasm
(`mesh.count`) → binary instance encoding → `setPixelRatio(min(dpr, 1.5))`
(one line, ~3× fill rate) → LOD2 as the default at crowd distance.

**CI asserts counters, never fps:** `renderer.info.render.calls/.triangles`,
`renderer.info.memory.geometries/.textures` delta == 0 over a 5-minute soak,
`performance.measureUserAgentSpecificMemory()` drift < 10 MB/10 min, zero
`long-animation-frame` entries > 50 ms, `wasmMemory.buffer.byteLength`
constant, `gl.bufferSubData` bytes/frame against the table above. M58's
`--disable-gpu` "Low tier proxy" is **rejected** — that is SwiftShader, ~100×
slower, and tuning Low against it would make Low far uglier than necessary.

**WebGPU: no, this year.** Baseline support exists (Chrome/Edge, Firefox 141,
Safari 26) but Safari 26 requires macOS/iOS 26 — a real exclusion for an
installation machine or a 2022 tablet — and three's `WebGPURenderer` is a
different renderer with a different post stack (TSL), so the conditional-edge
shader, PMREM environment and composer chain would be rewritten, not ported.
Dirty ranges + visible compaction buy ~95% of the benefit for ~1% of the
work. Keep material and edge code in plain GLSL-shaped chunks so a later port
stays cheap. **DEFER**, revisit past ~500 k instances.

---

## 3. The screenplay (R3, R5) — the largest change in rev 2

R3 and R5 arrived independently at the same diagnosis: **the film has no
memory between shots.** Every act adds, nothing is ever lost, so the piece
can be agreed with but not felt. R5 put it sharply: *"an extraordinary engine
with a screenplay attached, and the screenplay is losing."*

### ADOPT — structural

| # | Change | Reason |
|---|---|---|
| S1 | **Author the piece in bars, not seconds.** 84 bpm, 4/4 ⇒ 1 bar = 2.857 s; the canonical cut is **84 bars**, not "336 bars". Every shot duration becomes an integer bar count. | Resolves B8; makes rule 2 ("every cut lands on a bar line") true instead of aspirational. |
| S2 | **Re-budget the acts: I 17 bars, II 20, III 20, Atlas 6, IV 21.** Rev 1's 60/60/60 gives the setup act the same weight as the payoff act. | The film should get longer as it gets more urgent. |
| S3 | **Add A4-S01b, "Der letzte Stein" (2 bars).** One Terrakotta brick, alone in black, held still, colour draining, going to point-swarm last of everything. The network arrives *after* it is gone, not as its continuation. | The single change that converts an argument into an arc. It also earns the loop: the final green pixel stops being a triumphant QED and becomes elegiac, and the identical opening frame reads as *it begins again* rather than *nothing happened*. |
| S4 | **Cut the HUD numeral in A2-S03. The count overruns instead.** Tokens keep spilling past what the row can hold and past what the eye can track. | It is the only moment the film narrates an image it has already made. The overrun is also the historically actual reason for the bulla — tallying by object stopped scaling. And 14 s is 19.6 beats, so "1…24" never fit anyway. |
| S5 | **Give the piece one continuity object.** The monolith stays visibly present at the edge of frame once per act — behind the coin, behind the white studio, on the world plate — so there is something that persists and can be lost in S3's new shot. | Rev 1 mentions "the monolith held in the background all along" once, as a parenthesis. It is the only continuity object in the film. |
| S6 | **The Atlas accumulates instead of resetting.** Sites stay on the world plate; the movement builds a populated globe. Vary what is withheld (sometimes flag before build, sometimes the card only after the site is gone); every fifth site drops to macro on one joint; bind each site to a different fugue voice so four sites = one entry cycle. | Otherwise 40 repetitions of one camera grammar is a slideshow with a good render, and site 40 looks exactly like site 1. |
| S7 | **A1-S03 is the centre of the piece, not the clutch macro.** The edges must arrive in **one frame**, not fade — legibility is a threshold event. And delete the scored coincidence: hold silence for a bar and bring voice 1 in *after* the image lands. | "Sound arriving late is the difference between an event and an effect." |
| S8 | **Land the 2:00 cut two beats early, against the cadence.** | A full cadence and a hard cut coinciding is double punctuation — the most conventional gesture in the work. |
| S9 | **Retime per R3's table** (below). | No shot outside the Kick is under 6 s in rev 1; a film with no short shot cannot accent anything. |
| S10 | **The Kick becomes 2 beats (1.429 s), not 800 ms.** | 800 ms is neither one beat nor two, so it cannot be "frame-exact against its accent" — and at 10⁴ zoom, 0.8 s is a flash where the collapse should be *seen* collapsing. Still `scaling: 'fixed'` in every cut. |

**Retiming (bars @ 84 bpm):** A1-S01 2 · A1-S02 2 · A1-S03 3 · A1-S04 4 ·
A1-S05 3 · A1-S06 3 ‖ A2-S01 4 · A2-S02 4 · A2-S03 5 · A2-S04 4 · A2-S05 3 ‖
A3-S01 4 · A3-S02 4 · A3-S03 3 · A3-S04 **5** · A3-S05 4 ‖ ATL unit 2 bars ×
3 ‖ A4-S01 5 · **A4-S01b 2** · A4-S02 6 · A4-S03 4 · A4-S04 3½ · Kick ½.

Note A3-S03 shortens *because* it rhymes with A3-S02 ("rhymes must be shorter
than what they rhyme with"), and A3-S04 — the clutch macro, the title image
of the work — becomes the longest shot in its act instead of tied for third.

### ADOPT-MOD — the kitsch list (R5)

| Finding | Decision |
|---|---|
| **The waving flags are Epcot**, and they import nationalism: the "state party" is frequently not the builder (Great Wall, Machu Picchu, Timbuktu), and flying a modern sovereign flag over a pre-national ruin is a political claim made automatically, forty times, by a loop. | **ADOPT-MOD.** Flags stay — Stefan asked for them and they are load-bearing for the Postillen — but: **no poles, no wind by default**; a flag appears as a flat mosaic *and its `QuantizeReport`* (the ΔE, the colours LDraw does not have). A flag that cannot be built in the available palette is the thesis' best counter-evidence and it costs nothing to show. Wind becomes a tier-3 installation-cut option. Per-site flag suppression is **mandatory** (see R6 ethics). M76 is re-scoped and drops down the priority list. |
| **Chronicle cards are museum wall labels, forty times.** | **ADOPT-MOD.** One card per movement, not per site; the card states the *module* (which bond, which unit of measure), which is the only fact that belongs to the argument. Per-site name/year moves to a single quiet corner line. |
| **Act IV's green neural lattice is the most exhausted image in generative art.** | **ADOPT.** Keep the lattice, kill the sci-fi: render the token field as an *inventory* — a parts bin, a bill of materials, counted. More accurate about what a model actually is, and unfamiliar. |
| **"2017 · Attention Is All You Need" on screen dates the day it renders.** | **ADOPT.** Cut. The image already says it. |
| **Wax seals + IPFS + blockchain stacked as authority-cosplay; a success ladder whose maximum is institutional endorsement.** | **ADOPT — see §5.** |

### REJECT (with reason)

- **R5: "break the loop — one brick that does not return per cycle."**
  Genuinely the better artwork, and it is *rejected only as a default*: the
  pixel-identical seam is a hard technical requirement for the endless
  installation cut and for the on-chain edition's determinism. **Compromise
  adopted:** the seam stays pixel-identical, and the *accumulated dark region
  of the world plate* persists across cycles in endless mode only — the field
  around the pixel is not empty by cycle forty, but the pixel is the pixel.
  Recorded here so the trade-off is visible rather than lost.

---

## 4. Historical corrections (R6) — all ADOPT, several urgent

R6 fact-checked every dated claim. Several are wrong in ways a hostile
curator finds in an afternoon.

| Claim in rev 1 | What is actually true | Rev 2 wording |
|---|---|---|
| "Mesopotamia ~3500 BCE, first standardised mud bricks" | Mud brick is ~7th millennium BCE; moulded brick predates Uruk by ~4 000 years. What *is* true of Uruk is deliberate downsizing so two bricks fill one hand. | **"Uruk, ~3500 BCE — the brick is sized to the hand that carries it. The module becomes an administrative fact."** |
| "This is the invention of number" (A2-S03) | Schmandt-Besserat's token thesis is substantially contested (Bennison-Chapman, *CAJ* 29.2, 2019: no evidence of a unified Neolithic symbolic system). What survives: late-4th-millennium Uruk bullae with impressed counters *are* accounting devices. | Date the shot **~3300 BCE, Uruk**; caption **"An account you cannot alter without breaking it."** Drop "invention of number" entirely. |
| "Lydia ~600 BCE, first minted coin — the first fungible unit" | **Backwards.** Early electrum's gold:silver ratio varied and could be manipulated; it was a *trust* instrument precisely because it was not assayable. Fungibility arrives with Kroisos's bimetallic reform, mid-6th century. | **"Sardis, mid-6th century BCE — gold and silver at a fixed standard. Not the first coin. The first coin you did not have to test."** Truer, and it serves the standardisation thesis better. |
| "Rome ~100 CE, bipedalis" attached to the Pont du Gard | Bipedales are bonding-course plates and hypocaust spanners, not the wall module. And the Pont du Gard is c. 40–60 CE, dry-laid limestone ashlar, **largely without mortar** — no brick at all. | Split the shot: either "Pont du Gard, c. 50 CE — no mortar: the module before the mortar", or "Rome, c. 110 CE — the bipedalis bonding course, and the brick stamp that dates it by consul". The dated *figlinae* stamp is the better image anyway: a standard module with an audit trail. |
| "Stonehenge mortise-and-tenon, ~3000 BCE" | 3000 BCE is Stage 1 (ditch, bank, posts). The sarsens, lintels, mortices and tenons are **c. 2500 BCE**. | Move the date 500 years. |
| `GB 529580 · 1939` | Filed **17 April 1940**. | Use 1940 unless a 1939 priority document can be produced. |
| `BE 311029 · 1923 (Louis Cousin)` | **Not confirmed.** The traceable record is **FR 588985**, assignee "Le Batima Soc.", c. 1924. | **Do not put BE 311029 on screen.** Use FR 588985 and the assignee, or say "Batima, c. 1924". |
| The Louis Cousin genealogy in `masterplan-bewegung-postillen.md` | **Two different people fused.** Louis Cousin, Président de la Cour des Monnaies, died **1707**; he cannot hold a 1923 patent. (The Cour des Monnaies also became a sovereign court in 1552, not 1650.) The "Albert Despature-Cousin… Ahne? Zufall?" aside is apophenia. | **Cut the entire Cousin genealogy.** It is the first thing a historian finds, and it discredits everything near it. |
| "Interlego v Tyco [1988] UKPC 3 — the confirmation" | Correct citation, wrong caption: **Lego lost.** It holds that copying a drawing with skill and labour yields no new originality absent visually significant alteration. It adjudicates nothing about Page. | **"the case that ended the drawings — not the case that named the inventor."** |
| "Page is the true inventor of the interlocking brick" | Bounded version is strong; absolute version is false. Stud-and-socket blocks predate him, and the **1958 tube-and-stud coupling** is a genuine Danish invention Page's brick lacked — which is *why* Kiddicraft failed. | *"Hilary Fisher Page patented and sold the self-locking studded brick before Billund made one. LEGO's own history says so. What Billund added in 1958 — the tube inside the brick — is real, and it is what made the joint hold. Two inventions, one object. Only one of them is remembered."* Plus the receipts: permission sought from Kiddicraft in the late 1950s; rights bought from Page's successors in 1981 for £45,000 as the Tyco litigation opened. |
| The AI token as continuous with the clay token | **A pun, not a continuity.** "Token" reaches ML through compiler lexical analysis; subword tokenization (BPE, 2016) precedes *Attention Is All You Need*, which consumes it as given. | Say so *in the work*. Caption 2017 as **"the year the module became the only unit"** (attention over interchangeable positions). **Delete "Dies ist keine Metapher. Dies ist Mathematik"** — invert it: *"Dies ist eine Metapher. So bauen Menschen."* The formal argument survives being called what it is. |

### ADOPT — what is missing

Three additions that strengthen rather than decorate the thesis:

1. **Indus Valley, c. 2600–1900 BCE** (Mohenjo-daro — already a World
   Heritage Site). Fired brick at a consistent **1:2:4** ratio across a
   territory larger than Sumer, plus a binary/decimal weight system. A better
   standardisation case than Mesopotamia, and it **rhymes with the monolith's
   own 1:4:9**. It fixes the "first" problem by replacing it with a stronger
   claim.
2. **Qin China, 3rd c. BCE** — crossbow triggers and terracotta-army
   components made as interchangeable parts with workshop maker's marks.
   Interchangeable manufacture plus stamped accountability two millennia
   before Europe; it is the Roman brick stamp's twin and it demolishes the
   diffusionist reading of the timeline.
3. **Yingzao Fashi, 1103 CE (Li Jie)** — the *cai-fen* system: eight graded
   timber sections from which every member derives by ratio. The
   best-documented pre-modern parametric building standard anywhere, and it
   is a *ratio* system, which is what this work actually is.

Secondary, if the Atlas has room: the Andean **quipu** (positional base-10
accounting without writing — it productively breaks the counting→writing
teleology) and Islamic **girih** modular tiling.

### ADOPT — ethics, beyond the existing exclusion list

The exclusion mechanism (fail-closed, data-not-code, human-reviewed) is
well-reasoned. Its coverage is not.

1. **Custodian-request exclusion, not building-type exclusion.** "Active
   places of worship" misses sacred *landscapes* (Uluru–Kata Tjuta, Kakadu).
   Exclude on documented custodian objection, whatever the built form.
2. **Indigenous consent.** Where a site has an identified Indigenous
   custodian community, default to exclusion absent a documented outreach
   attempt (UNDRIP Arts. 11/31).
3. **The flag movement is the largest exposure.** Rendering a state party's
   flag beside land whose custodians contest that state's authority is a
   political act performed automatically, forty times, by a loop. **Per-site
   flag suppression is mandatory**, and no flag is ever rendered for
   transboundary or contested-sovereignty sites.
4. **Sites in Danger** are excluded by default, or admitted only with the
   danger listing on the card and **without** the build-to-a-beat animation.
   *A site assembling itself joyfully while it is being shelled is the
   failure mode.*
5. **Burial and human remains** — Stonehenge is a cremation cemetery; many
   buildable sites are tombs. Not an exclusion, a tone gate on the build shot.
6. **Render exclusion visibly.** Excluding Gorée and Auschwitz silently
   erases them. Better: an empty plinth, the card, no bricks, held for the
   full unit. Stronger artistically, and honest about why.
7. **"Reviewed by a human" must name a human who is not the author.**

---

## 5. The Bewegung (R5, R6) — ADOPT

Both reviewers, independently: as written, a curator or historian reads the
sealed-letters apparatus as a crank petition. The tells are specific and each
is fixable — sincere requests for royal patronage; wax seals plus IPFS plus
blockchain stacked as authority-cosplay; the Cousin apophenia; "this is not a
metaphor, this is mathematics"; and a success ladder whose maximum is
institutional endorsement. That last is decisive: *a work judged by whether
the powerful say yes is a petition by definition.*

The precedent for the form is excellent — mail art, On Kawara's telegrams,
Cildo Meireles's *Inserções*, Institutional Critique. One change separates
them:

> **Make the reply the work, and declare that in advance, publicly, before
> sending.** Publish all four Postillen and a fixed protocol — sent on date
> X; every response and every non-response published verbatim at X+90; the
> work is the archive of the answers. You are then not asking three kings for
> legitimacy; you are measuring what three kings do when asked, and **silence
> becomes the strongest result rather than the failure state.**

Two consequences, both adopted:

- **Delete the success ladder** from `masterplan-bewegung-postillen.md`. It
  contradicts that document's own best line — *"Wir schreiben nicht, um
  Antworten zu bekommen."*
- **Separate the Fisher Page recognition ask from the patronage asks.** It is
  modest, checkable and genuinely just, and both the ODNB and the English
  Heritage blue plaque panel have real public nomination processes.
  **Addressed to the panel it is a civic act; addressed to the King it is the
  thing that makes the whole apparatus look unserious.**

---

## 6. Schedule (R4) — ADOPT, with the scope decision taken

R4's independent estimate against rev 1's plan:

| Phase | Rev 1 | R4 | Δ |
|---|---|---|---|
| P1 Renderer | 6 w | 9 w | +3 |
| P2 Show engine | 4 w | 6 w | +2 |
| P3 Audio | 3 w | 6 w | +3 |
| P4 Kit / Atlas / flags | 4 w | 9 w | +5 |
| P5 The work | 4 w | 7 w | +3 |
| P6 wasm | 3 w | 0 (cut) | −3 |
| P7 Ship | 2 w | 4 w | +2 |
| **Total** | **26 w** | **41 w** | **+15** |

Honest completion at rev 1 scope: **May 2027**, not January.

**The week-1 decision, ADOPTED:**

> **The deliverable is the 4:00 cut with 3 Atlas sites. October ships an
> Act I preview, not the piece. Everything else is explicitly post-date.**

This is the only decision that both recovers the fifteen weeks and resolves a
collision R4 found that nobody had noticed: `claude/masterplan-iunctura-site.md`
promises a running 4:00 loop and a USB stick of the engine by **end of
October**, while rev 1 delivers 4:00 on **20 December**. Both plans assumed
the other would yield.

**Cut list, in order** (recovers ~13 weeks): Phase 6 wasm entirely → M77
autopilot → Atlas tier C (28 sites) → the 60:00 cut and everything existing
only for it (M59's LOD2/BVH) → M88, M89b, M92's 9-cell matrix (keep Safari),
M70's SMF reader, M85's 1000-seed sweep (sample 25) → tier B 12 sites → 6.

**Two risks rev 1 missed entirely:**

- **Stefan is the bottleneck, not the agents.** ~20 blocking human gates
  (listens, watches, contact sheets, curation review, promotions) exist as
  acceptance criteria with **no calendar slot**, and §13's risk register does
  not list him at all. *Fix: schedule every gate now as a named weekly slot;
  a missed gate is a schedule slip, not a silent block.*
- **The three patent bricks have no real LDraw parts.** Batima 1923 and
  Kiddicraft 1939 are not in the library, and rule 4 forbids inventing
  geometry — so M80's gate is unmeetable as written. *Decide in week 1: either
  hand-author three `.dat` files from the published drawings and label them
  clearly as ours, or restage A3-S02/S03 around parts that exist.*

**Gates move earlier:** the "is mesh better than points" gate to **week 2**
(on the monolith, yes/no); a **fugue listen in week 3** (spike theory +
counterpoint + MIDI export off the critical path in weeks 1–2, so "musically
dead" is discovered before Phase 2 is authored against it); a resolver gate in
week 7 on fake shots; **Safari on real hardware in week 6**, not week 25; and
a standing weekly full-run capture from week 10.

**§14 is partly unverifiable** and rev 2 fixes it: items 6, 7 and 9 have no
automated check and no checklist artefact; the exclusion review has no dated
signature; item 3's "pixel-identical" will not survive bloom and tone mapping
and would have been quietly relaxed. Each gets either a real check or an
explicit human-signed artefact.

---

## 7. AI budget (R8) — ADOPT

| Phase | Input tokens | Output | Model mix | API cost | Human h |
|---|---|---|---|---|---|
| P1 Renderer | 90–200 M | 3–6.5 M | 40% Opus / 50% Sonnet / 10% Copilot | $350–780 | 60–90 |
| P2 Show | 70–160 M | 2.3–5 M | 25 / 65 / 10 | $220–500 | 40–60 |
| P3 Audio | 55–130 M | 1.9–4 M | 35 / 55 / 10 | $210–480 | 50–80 |
| P4 Kit / Atlas | 70–160 M | 2.3–5 M | 10 / 75 / 15 | $170–390 | 40–60 |
| P5 The work | 90–200 M | 2.9–6 M | 30 / 60 / 10 | $310–700 | 80–120 |
| P6 wasm | 40–90 M | 1.3–2.8 M | 25 / 70 / 5 | $130–290 | 25–40 |
| P7 Ship | 25–60 M | 0.8–1.8 M | 10 / 70 / 20 | $70–170 | 40–60 |
| **Total** | **450 M–1.0 B** | **15–31 M** | ~25% Opus | **$1 460–3 310** | **335–510** |

Midpoint ≈ 700 M in / 22 M out / ~$1 750 / ~420 human hours (~16 h/week).

**Cheapest for this project shape: a subscription tier (~$1 200 over 26
weeks), with API keys held for overflow in P1 and P5.** Token burn here is
dominated by *cache reads on a stable repo* — same `CLAUDE.md`, same crate
tree, thousands of turns — which is exactly what subscription pricing absorbs
and per-token pricing punishes. The binding constraint is the 5-hour rolling
limit during screenshot-heavy weeks, not dollars.

**Routing criterion: cost-of-wrong-choice ÷ speed-of-feedback-loop**, not
task difficulty.

- **Opus** where the loop is slow, invisible or architectural: M51 BFC
  winding, M57's conditional-edge shader math, M60's schema design (every
  later milestone inherits it), M61's resolver, M68's counterpoint, M83's loop
  seam, M87's wasm boundary — and *any* debugging session whose symptom is a
  picture.
- **Sonnet** wherever `cargo test -p <crate>` closes the loop in seconds:
  bundle writer, grid legality, heritage ingestion, CIELAB quantisation, CLI
  wiring, all six JSON Schemas and their validation cases, the mechanical
  wasm port. This is the bulk.
- **Copilot inline** only inside one file with a human present: GLSL constant
  tuning, `tsc` error fixing, test bodies. It has no model of `CLAUDE.md`'s
  invariants — never cross-crate.
- **Local (Qwen3-Coder-30B / Devstral class)**: commit messages from diffs,
  DE↔EN prose for chronicle cards and Postillen, transcribing published flag
  construction tables into `flags/<iso2>.json`, batch renames, JSON
  reformatting. It genuinely works there; it will waste human time on
  anything needing current three.js or wasm-bindgen API recall.

**Context economy — the three that actually pay:**

1. **Split this spec.** `docs/FUGEN-ENGINE.md` is 130 KB ≈ 33 k tokens. Split
   into `docs/fugen/phase{1..7}.md` so a session loads ~4 k, not 33 k.
   *Biggest single win available today* — **adopted, scheduled for rev 3.**
2. **Trim `CLAUDE.md`** (17.7 KB, re-sent every turn): keep commands, crate
   one-liners, the viewer-rebuild-order gotcha; move the paragraph-length
   per-adapter prose to `ARCHITECTURE.md`. Archive M1–M50 out of the 87 KB
   `TODOs.md`.
3. **Verdict-only verification.** Enforce a ≤ 200-word return from
   verification forks; no screenshots, no cargo/npm logs into the
   coordinator; `2>&1 | tail -5` on builds; `walkthrough.sh` and the seed
   sweep never touch the main thread.

Session boundary = one milestone. Prefer *finish → TODOs entry → clear* over
compaction: the TODOs entry is a lossless, human-reviewed, free summary.

**Where agentic coding will specifically fail here** — and what the human
must do instead: a wrong-sign cross product in the edge shader still renders
*something* (demand a numeric proxy for every visual claim; the human looks
at the 2000×2000 PNGs); agents cannot hear (substitute `OfflineAudioContext`
renders with asserted onset sample indices, and make the week-13
"is the fugue dead" decision a calendar event); wasm memory growth looks like
a GPU bug and agents will "fix" it with `needsUpdate` flags; grid-legal
geometry can still be ugly (use `spex ascii` — this repo already owns a cheap
agent-readable 3D proxy); and under pressure agents confabulate flag ratios
and heritage criteria, which is exactly what the real-data rule and the
fail-closed exclusion list exist to catch.

---

## 8. What rev 2 does with all of this

1. **§0.1 Binding amendments** at the top of `FUGEN-ENGINE.md`: B1–B14, the
   device budget table, the long-run mandates, the retimed screenplay, the
   historical corrections, the scope decision.
2. **The scope decision is applied to §10's calendar**: 4:00 cut, 3 sites,
   Act I preview in October, Phase 6 and M77 explicitly post-date.
3. **§8 is re-authored in bars** with S1–S10.
4. **§12 gains** the flag-suppression rule, the custodian/Indigenous/Danger
   clauses, and the visible-exclusion plinth.
5. **§13 gains** Stefan-as-bottleneck, the missing patent-brick geometry, and
   the October collision.
6. **§14 gains** a signed artefact for every human gate.
7. **Rev 3 splits the document** into `docs/fugen/phase{1..7}.md`.

Every finding above that is not adopted is recorded with its reason, so that
a later session can reopen it deliberately rather than rediscover it by
accident.

---

*Iunctura Archiv · Signatur IA-2026-002 · Review 01 · 2026-07-25*
