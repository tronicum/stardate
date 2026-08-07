# Epics and sprints

*A backlog view of the same plan. Nothing here supersedes the milestones — every epic and every sprint points at real M-numbers in [`README.md`](README.md)'s index.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**
Source of truth for scope and dates stays [`plan.md`](plan.md); this file is the same content shaped for a backlog tool.

Produced 2026-07-25 by a Product Owner pass (epics) and a Scrum Master pass
(cadence, sprints, protocol), both working from the spec rather than from each
other.

---

## 0. Traceability — every milestone has exactly one epic and one sprint

| Milestones | Epic | Sprint |
|---|---|---|
| M51 M52 M53 | E1 | S1 |
| M54 M56 | E2 | S2 |
| M55 | E3 | S2 |
| M57 M58 | E2 | S3 |
| M59 | E3 | S3 |
| M60 M61 | E4 | S4 |
| M62 | E5 | S4 |
| M63 M64 M65 M66 | E5 | S5 |
| M67 M68 | E6 | S6 |
| M69 | E7 | S6 |
| M70 M71 | E7 | S7 |
| M72 M73 | E8 | S8 |
| M74 M75 | E9 | S9 |
| M76 | E9 | S10 |
| M77 | E10 | S10 |
| M78 M79 M80 | E11 | S11 |
| M81 M82 | E11 | S12 |
| M83 M84 | E12 | S13 |
| M85 | E13 | S13 |
| M86–M90 | E14 | S14 |
| M91–M97 | E15 | S15 |

---

# Part A — Epics

## E1 — Real LDraw geometry & mesh bundle · **M51, M52, M53**

**Value:** real vertex and edge geometry can be extracted from LDraw parts and packed into an inspectable binary bundle — the raw material everything else renders from.
**DoD:** BFC-correct winding and real edge extraction match today's bounding box exactly (M51); the bundle validates against its schema and welding cuts vertex count ≥ 40 % (M52); `spex mesh-model` reproduces the established real part/instance counts, byte-identical, servable with no server change (M53).
**Deps:** none — additive to the point pipeline.
**Gates:** none directly; it precedes the week-3 mesh-vs-points verdict (D8).
**Risk:** medium — winding bugs are invisible until the geometry is lit.
**Cuttable:** no. Every later visual milestone depends on it.

## E2 — Cinematic single-object rendering, *vektorgenau* · **M54, M56, M57, M58**

**Value:** a single brick renders with real materials, crisp catalogue-quality edges and filmic post — the first "this looks real" moment.
**DoD:** mesh mode renders the monolith solid, with no interior faces, and reports its own counters — instances, parts, triangles drawn vs unique, draw calls — from a real headless screenshot (M54; the original "≥ 55 fps" was corrected: the headless harness is SwiftShader, which this phase had already rejected as a performance proxy, so fps is asserted on real hardware in M92 instead); ≥ 5 real LDConfig finishes resolve correctly including chrome and transparent (M56); a continuous uniform-width outline with no z-fighting, edge cost ≤ 25 % of frame time (M57); the post chain holds 60 fps on High and Low with no black-frame regressions (M58).
**Deps:** E1.
**Gates:** weeks 5, 9, 12 — M57 edge screenshots at 0.5× and 50×, 30 min each.
**Risk:** **high** — the visual signature of the whole piece, and "conditional edges eat a week" is a named risk.
**Cuttable:** no.

## E3 — Crowd-scale rendering · **M55, M59**

**Value:** tens of thousands of bricks render within the device budget — what makes the Atlas possible at all.
**DoD:** the 50 k-instance scene renders with a draw-call count **independent of instance count** — measured: 29 calls at 50 000 instances vs 67 at 61, and the transform-pass cost recorded (M55; "≥ 60 fps, ≤ 3× part count" was corrected — one part in seven colours is seven instanced meshes, so the bound counted the wrong thing, and fps belongs to M92 for the reason at M54's AC2); three LODs with stud/tube removal gated on the reference path, and the 200 k-instance synthetic scene at ≥ 45 fps (M59).
**Deps:** E1, E2.
**Risk:** medium — LOD tuning.
**Cuttable:** partially. It matters mainly for Atlas tiers B and C and the 60:00 cut; tier A alone needs little of it.

## E4 — Show document format & duration resolver · **M60, M61**

**Value:** the piece becomes one declarative document that resolves deterministically into four durations — the mechanism every cut, seed and re-authoring depends on.
**DoD:** both schemas validate real output (M60); the resolver hits its target duration within 1 ms across 200 random configurations, filters tiers correctly, and builds deterministically (M61).
**Deps:** E1.
**Risk:** medium — a schema and an algorithm every later milestone inherits.
**Cuttable:** no.

## E5 — Runtime show playback · **M62, M63, M64, M65, M66**

**Value:** Act I plays end to end from `show.json`, silent, at all four durations — a runtime-evaluated show rather than baked animation.
**DoD:** seek-then-play matches continuous playback with zero allocations after warm-up (M62); the Kick zoom collapses artefact-free (M63); runtime choreography matches the baked assembly within 0.01 mm (M64); dissolve and crossfade ship at ≤ 15 % frame cost (M65); the full cut runs with zero console errors and every URL parameter verified (M66).
**Deps:** E4, E2, E3.
**Gates:** week 18 — watch Act I, silent, 1 h.
**Risk:** medium — five-milestone integration risk.
**Cuttable:** no. This *is* the Phase 2 gate.

## E6 — Generated fugue score · **M67, M68**

**Value:** a real, contrapuntally correct four-voice score exists for the canonical cut — the musical spine.
**DoD:** the schema validates and 20 real theory assertions pass (M67); zero parallel fifths or octaves, correct exposition order, genuine stretto overlap, and an exported `.mid` **listened to by a human** (M68).
**Deps:** none — a parallel track from day one.
**Gates:** week 3 — the fugue spike listen, 2 h (P0, precedes this epic); week 24 — listen to the full score, 2 h.
**Risk:** **high** — the top-listed project risk, and D7 is still open.
**Cuttable:** never fully. The fallback is pre-decided: hand-composed subject *and* exposition, generated episodes only.

## E7 — Runtime audio engine & sync · **M69, M70, M71**

**Value:** the score synthesises live and locks to the same clock as the visuals — "Act I plays with sound" becomes real.
**DoD:** the audio graph is clip-free at under 10 % CPU (M69); onsets within 3 ms of the score and no stuck notes after 100 seeks (M70); audio-visual drift ≤ 20 ms over 60 minutes, and the Kick within one frame (M71).
**Deps:** E6, E5.
**Risk:** medium — Safari's autoplay and interruption behaviour.
**Cuttable:** no. Sound is in the definition of done.

## E8 — Construction kit & heritage data · **M72, M73**

**Value:** sites can be composed from parametric, grid-legal primitives against a real, ethically curated World Heritage dataset.
**DoD:** every primitive emits real placements and `validate()` reports zero undeclared illegality (M72); ≥ 900 real sites in the snapshot, ≥ 40 curated as buildable, and a non-empty, reviewed exclusion list (M73).
**Deps:** E1, E4.
**Gates:** week 28 — heritage curation **and** exclusion list, reviewed and signed, 3 h.
**Risk:** medium — the exclusion list's correctness is ethical rather than technical, mitigated by failing closed.
**Cuttable:** not for tier A. What is cuttable is the scale beyond it.

## E9 — Atlas sites & flags · **M74, M75, M76**

**Value:** the World Heritage sites and their flags actually stand on screen — the Atlas movement becomes demoable.
**DoD:** each tier's recipes build, validate and render, no site over 8 000 placements, contact sheet reviewed per tier (M74); the Dannebrog cross and the Union Flag saltire match their published construction within one stud, ΔE > 12 flagged (M75); the flags' per-frame wave-evaluation cost and draw calls are recorded, and they are perfectly flat at zero wind (M76; frame rate belongs to M92).
**Deps:** E3, E8.
**Gates:** week 30 — the flag contact sheet with every ΔE, 2 h.
**Risk:** medium — Foguang Si's `Bracket` primitive is time-boxed at three days before the fallback (D6).
**Cuttable:** tiers B and C yes, tier A no. The wave (M76) is installation-only and fully cuttable.

## E10 — Atlas autopilot · **M77**

**Value:** the Atlas becomes a living index that proposes new sites from the real feed instead of a fixed hand-curated list.
**DoD:** the classifier scores ≥ 9/12 against the hand-curated ground truth or is fixed before close; fails closed on unclassified and low-confidence input; output stays provisional until a human promotes it; an OSM-massed recipe for ≥ 3 real sites, each carrying its ODbL attribution.
**Deps:** E8, E9.
**Risk:** low — a bad classifier costs review time, never a bad frame.
**Cuttable:** yes, cleanly. It loses future scalability, not the piece.

## E11 — The authored film: Acts I–IV + Atlas · **M78, M79, M80, M81, M82**

**Value:** the piece itself exists, act by act, playing end to end in time and with sound.
**DoD:** each act authored from the shot list, cameras and cues bound, contact sheets captured, and **watched by a human with sound** before close — monolith at 73.6 mm (M78), a continuous bulla→coin metamorphosis (M79), real patent-brick geometry and an on-cue stretto (M80), generic site→flag→card choreography (M81), a frame-exact Kick (M82).
**Deps:** E2, E3, E5, E7, E9.
**Gates:** weeks 34–40 — watch each act with sound, 1 h × 5.
**Risk:** **high** — every upstream risk surfaces here, and this is the real May 2027 deadline.
**Cuttable:** no.

## E12 — The four cuts & attribution · **M83, M84**

**Value:** one show resolves cleanly into all four durations with a seamless loop, fully credited.
**DoD:** 4:00, 10:00, 60:00 and endless all resolve, run and loop seamlessly (M83); LDraw, Wikidata and OSM attribution plus the no-emblem and no-brand-name rules stated in-work and in-repo (M84).
**Deps:** E11.
**Gates:** week 40 — watch the whole canonical cut, 1 h.
**Risk:** low–medium.
**Cuttable:** 60:00 and endless are the explicit compressible tail. The 4:00/10:00 pair and the attribution are not.

## E13 — Seeded editions · **M85**

**Value:** every run is a unique, deterministic, verifiable variation — the on-chain-edition prerequisite.
**DoD:** two seeds differ visibly and audibly; the same seed twice is frame-identical; every seed in [0, 999] resolves without error.
**Deps:** E12.
**Risk:** low.
**Cuttable:** yes. The premiere works on one fixed seed.

## E14 — WebAssembly port · **M86, M87, M88, M89, M90**

**Value:** resolver, evaluator, kit and fugue generator exist once, compiled to wasm — closing the Rust/TypeScript drift and unlocking zero-copy evaluation at 250 k instances.
**DoD:** `resolve_show` byte-identical to the CLI, wasm under 400 KB gzipped, existing demos still load with the wasm module absent (M86); the wasm evaluator matches the TypeScript one exactly **before** the TypeScript is deleted, under 2 ms for 250 k instances (M87); build, flag and ldraw run browser-side with no fetching (M88); the generated score byte-identical and no memory growth in endless mode (M89a); an honest before/after verdict recorded (M90).
**Deps:** E4, E5, E6, E8.
**Risk:** low — deliberately sequenced after the algorithms are settled.
**Cuttable:** entirely, and it is the first thing a slip eats.

## E15 — Ship · **M91–M97**

**Value:** the piece becomes reproducible, cross-browser, accessible, deployed, distributable as one file, and provably authentic.
**DoD:** a committed frame- and audio-hash regression fixture (M91); a real fps and load matrix including Safari (M92); reduced motion, keyboard control and a watchable Low tier (M93); a static export with no backend (M94); the single-file edition at ≤ 12 MB, tier A only (M95); docs synced (M96); a provenance record carrying seed, commit and frame hashes (M97).
**Deps:** E12, E13, E14.
**Gates:** week 8 — Safari on real hardware, 2 h; week 47 — watch all four cuts, 3 h.
**Risk:** medium — Safari is a named high-likelihood risk, and the 12 MB single-file budget only fits tier A.
**Cuttable:** no. This *is* the definition of done.

## Releases — the moments something demoable exists

| # | What exists | Roughly | Epics required |
|---|---|---|---|
| 1 | **A brick renders with real edges** | week 12 | E1, E2 |
| 2 | **Act I technical preview, silent, all four durations** — replaces the old October 2026 promise (D3) | week 18 | E1–E5 |
| 3 | **Act I plays with sound** | weeks 24–33 | E1–E9 |
| 4 | **The canonical 4:00 cut, complete, looping** — *the real deadline* | week 40, May 2027 | E1–E9, E11, E12 (E10/E13 optional) |
| 5 | **A faster, unified engine** | week 43 | E14 |
| 6 | **Premiere-ready: all cuts, deployed, provable** | weeks 46–47 | E15 (+E10, E13 if retained) |

## Not an epic — tracked differently

- **`TODOs.md` entries per milestone** (DoD #10) — a close-out checklist item inside the milestone, not a backlog item.
- **The verification ladder** (`docs/agents/verification.md`) — a per-commit practice.
- **The weekly full-run capture review** (standing from week 18, 30 min/week) — a recurring calendar slot.
- **Docs and playbook upkeep** — folded into the milestone that changes the thing being documented; M90 and M96 are the only dedicated sync points.
- **New ideas surfacing mid-build** — [`backlog.md`](backlog.md), per rule 8, never into the running milestone.
- **AI model routing** ([`budgets.md`](budgets.md) §8) — an execution policy.

---

# Part B — Sprints

## 1. Cadence: 3-week sprints, 16 of them, plus a one-week apparatus block

**Not weekly:** with one human at ~16 h/week who is also the reviewer for every
gate, a weekly planning-review-retro cycle burns hours that belong to the ~20
named gates and to *watching output*, not to managing process.
**Not fortnightly:** the AI-hour ranges in [`budgets.md`](budgets.md) §8 and
[`plan.md`](plan.md) §2's phase lengths (3, 9, 6, 6, 9, 7, 3, 3 weeks) divide
cleanly into three-week blocks, and P0 is already a three-week spike block.
Three weeks gives agents enough runway to close a milestone's whole
verification ladder without the human refereeing mid-flight, and still catches
a slip within one cycle — because the §3 gates are checked weekly regardless of
sprint boundary.

**One deliberate exception:** Phase 5 (weeks 34–40) runs at **two weeks**
(S12, S13). Seven weeks does not divide by three, and more importantly the
watching gates there are weekly — a three-week sprint would put a whole act
between reviews.

**A sprint review here is the human executing the gate(s) that fall in that
window** — watching Act I silent, listening to the fugue, looking at the ΔE
contact sheets — plus a 15-minute skim of the `TODOs.md` entries and the
`spex ascii` / `walkthrough.sh` output for milestones with no named gate.
**It is not** a slide deck, a demo *to* anyone, or an estimate-versus-actual
ritual. There is no stakeholder audience and no velocity to report.

## 2. The sprints

Standing from week 18: a 30 min/week full-run capture review, not repeated per row.

| # | Dates | Phase | Goal | Epics / Milestones | Gates in window | Demoable at the end |
|---|---|---|---|---|---|---|
| **S0** | 27 Jul – 16 Aug 26 | P0 | Answer D7 and D8 before any production code | spikes only | wk2 patent-brick decision 1 h · wk3 fugue listen 2 h + mesh-vs-points verdict 1 h | Go/no-go on the mesh renderer and the generated fugue, recorded in [`decisions.md`](decisions.md) |
| **S1** | 17 Aug – 6 Sep 26 | P1 | Real geometry in, real edges extracted | **E1: M51 M52 M53** | wk5 edge screenshots 0.5 h | `mesh-part` / `mesh-model` emit real triangles with BFC-correct winding |
| **S2** | 7 – 27 Sep 26 | P1 | Mesh mode renders, instanced, materials real | **E2: M54 M56 · E3: M55** | wk8 **Safari on real hardware** 2 h · wk9 screenshots 0.5 h | Viewer mesh mode live; Safari smoke-tested |
| **S3** | 28 Sep – 18 Oct 26 | P1 | Vektorgenau edges, crowd budget holds | **E2: M57 M58 · E3: M59** | wk12 screenshots 0.5 h | A 1×1 brick at catalogue quality, 60 fps; 200 k instances in budget |
| **S4** | 19 Oct – 8 Nov 26 | P2 | `show.json` compiles, durations resolve, the clock runs | **E4: M60 M61 · E5: M62** | — | A show file resolves to an exact duration and ticks |
| **S5** | 9 – 29 Nov 26 | P2 | Act I plays silent, all durations | **E5: M63 M64 M65 M66** | wk18 **watch Act I silent** 1 h | Act I end to end from `show.json`, silent |
| **S6** | 30 Nov – 20 Dec 26 | P3 | Subject and counterpoint generate; audio engine boots | **E6: M67 M68 · E7: M69** | — | A generated four-voice score exists as data and MIDI |
| **S7** | 21 Dec 26 – 10 Jan 27 | P3 | The score plays in the browser, in sync | **E7: M70 M71** | wk24 **listen to the full score** 2 h | Four voices, in tune, in time, bound to the visuals |
| **S8** | 11 – 31 Jan 27 | P4 | The kit builds; the heritage index imports | **E8: M72 M73** | — | `spex-build` produces grid-legal models from a recipe |
| **S9** | 1 – 21 Feb 27 | P4 | Atlas sites and flags render | **E9: M74 M75** | wk28 **heritage + exclusion review, signed** 3 h · wk30 **flag ΔE contact sheet** 2 h | Stonehenge, Mohenjo-daro, the dougong hall, and the first flags |
| **S10** | 22 Feb – 14 Mar 27 | P4 | The wave; the autopilot scored | **E9: M76 · E10: M77** | — | The classifier's accuracy against the hand-curated set, reported |
| **S11** | 15 Mar – 4 Apr 27 | P5 | Acts I–III authored, with sound | **E11: M78 M79 M80** | wk34–36 **watch each act with sound** 1 h × 3 | Three acts play, scored, in real time |
| **S12** | 5 – 18 Apr 27 | P5 | Atlas act and Act IV authored | **E11: M81 M82** | wk37–38 **watch with sound** 1 h × 2 | The full show; Der Kick lands on two beats |
| **S13** | 19 Apr – 2 May 27 | P5 | **The canonical 4:00 cut exists and loops** | **E12: M83 M84 · E13: M85** | wk40 **watch the whole cut** 1 h | *The real deadline:* 4:00, with sound, seamless loop |
| **S14** | 3 – 23 May 27 | P6 | The wasm port, measured | **E14: M86–M90** | M90's own verdict is the gate | Identical output from the wasm evaluator and resolver, faster |
| **S15** | 24 May – 13 Jun 27 | P7 | Ship, plus the extended cuts if the budget holds | **E15: M91–M97** + tier C + 60:00 | — | Deployed, documented, licensed; 10:00 / 60:00 / endless if in budget |
| **Ap** | 14 – 22 Jun 27 | Apparatus | Install, Postillen, chain, press — not engineering | — | wk47 **watch all four cuts** 3 h | The premiere build |
| — | **Wed 23 Jun 27** | **PREMIERE** | 100 years of GB 263865 | | | |

## 3. Definition of Ready / Done, at sprint level

**Ready.** The milestone's section exists in its phase file with named
verification rungs; every human decision it depends on is already closed in
[`decisions.md`](decisions.md) — a sprint cannot start P1 render work while D8
is still open; and the previous milestone has its `TODOs.md` entry, because
milestones are not batched.

**Done.** Every milestone in scope has its own small commit(s); `TODOs.md`
carries an entry with real verified numbers; the CI counter assertions from
[`budgets.md`](budgets.md) §6 are green for anything viewer-visible (draw
calls, triangles, heap delta, zero long-animation-frames, no detached
geometry); `cargo test --workspace` and `npx tsc --noEmit` are clean; every
scheduled gate in the window is either completed or **logged as a slip**;
nothing viewer-visible shipped without rung 5; and no new idea got merged
instead of filed to [`backlog.md`](backlog.md).

## 4. Ceremonies, honestly scoped

| Ceremony | Verdict | Cadence |
|---|---|---|
| **Sprint planning** | survives — confirm milestone order for three weeks, confirm no blocking decision is open. Not story-pointing | 30 min per sprint |
| **Daily standup** | **dropped.** Theatre with one human and agents that do not sleep. Replaced by agents writing `TODOs.md` as they finish, read asynchronously | — |
| **Sprint review** | survives, redefined: it *is* the scheduled human gate. Sprints with no gate get a 15-minute output skim | per sprint |
| **Retrospective** | survives, folded into the next planning: what slipped, and is a cut trigger now true | 15 min per sprint |
| **Backlog grooming** | no ceremony — continuous writes to `backlog.md` | — |
| **Estimation / velocity** | **dropped entirely.** There is no team to average over | — |

## 5. Buffer and the slip protocol

The real slack sits **after week 40**. Once the 4:00 cut is done, weeks 41–47
(S14, S15, the apparatus block) are the only compressible time before a fixed
premiere. The cut order is already decided; what this adds is the **trigger**
for each:

1. **Cut E14, the wasm port (S14).** Trigger: definition-of-done items 1–5 —
   the automated show, duration, seam, Kick and determinism checks — are not
   green at the close of S13, week 40. Ship the WebGL2 path only.
2. **Cut E10, the autopilot (M77, S10).** Trigger: Phase 4 is still open past
   week 33. It fails closed by design, so cutting it costs review time, not a
   bad frame.
3. **Cut Atlas tier C.** Trigger: the week-28 and week-30 gates slip out of S9.
   Tier C is explicitly optional; the 4:00 cut needs three sites, not forty.
4. **Cut the 60:00 installation cut (S15).** Trigger: the counters in
   [`budgets.md`](budgets.md) §6 do not hold across the Atlas at install scale
   by week 46.

**A missed gate is itself the primary slip signal** — tracked the week it is
missed, not at sprint close.

## 6. Three anti-patterns specific to this project

1. **Silent gate slip becomes silent scope creep.** Agents do not stop when a
   gate is missed. Phase 2 and 3 work can proceed on top of a D7 verdict that
   was never actually recorded. *Watch:* does every downstream milestone cite a
   **closed** decision, or an assumed one?
2. **Ceremony reinflation.** Reviving standups, story points or a "demo to
   stakeholders" against an audience of one re-spends the scarcest resource —
   16 h/week — on process instead of the watching and listening the plan
   actually requires.
3. **Agent milestone batching.** Rule 7 says ship M51 before M52; agents left
   unsupervised across a three-week sprint can produce three milestones' worth
   of parallel diffs that land together at sprint end, turning one review into
   an un-gate-able pile. *Watch:* the commit history **inside** a sprint, not
   just its end state.
