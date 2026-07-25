# The Fugen Engine — implementation spec

**Work:** *Die Geschichtliche Matrix* — a generative WebGL piece built from
real Klemmbaustein geometry, arguing that counting, building and computing
are one human instinct.
**Engine:** the Fugen Engine — a vector-accurate brick renderer, a runtime
show engine, and a procedural fugue, all built *inside* `spex`, reusing what
M1–M50 already shipped.
**Archive signature:** IA-2026-002
**Spec revision:** **rev 3**, 2026-07-25 · **Premiere:** **Wednesday, 23 June 2027** —
the centenary of GB 263865, the eight-studs-in-two-rows patent. See
[`decisions.md`](decisions.md) D1a.

---

## Read in this order

| File | What it is |
|---|---|
| **this file** | The working rules, the scope, the milestone index |
| [`00-context.md`](00-context.md) | Where the project stands after M1–M50, the six gaps, and the architecture |
| [`screenplay.md`](screenplay.md) | **The work itself** — the shot list, in bars |
| [`phase1-renderer.md`](phase1-renderer.md) | M51–M59 — real triangles and real edges. *"Vektorgenau"* |
| [`phase2-show.md`](phase2-show.md) | M60–M66 — the screenplay as data, one clock, runtime transforms |
| [`phase3-audio.md`](phase3-audio.md) | M67–M71 — a generated four-voice fugue, in the browser |
| [`phase4-kit.md`](phase4-kit.md) | M72–M77 — the brick construction kit, the Atlas, the flags |
| [`phase5-work.md`](phase5-work.md) | M78–M85 — authorship, act by act |
| [`phase6-wasm.md`](phase6-wasm.md) | M86–M90 — one implementation, two targets |
| [`phase7-ship.md`](phase7-ship.md) | M91–M97 — determinism, compatibility, deployment |
| [`budgets.md`](budgets.md) | Device, memory, long-run and AI budgets. **Numbers that fail a build** |
| [`licensing.md`](licensing.md) | Licensing, attribution, and the ethics constraints |
| [`plan.md`](plan.md) | The calendar, the human gates, the risks, the definition of done |
| [`decisions.md`](decisions.md) | **What was decided, when, and what it rules out.** Append-only |
| [`epics-sprints.md`](epics-sprints.md) | The same plan as a backlog: 15 epics, 16 sprints, every one pointing at real M-numbers |
| [`backlog.md`](backlog.md) | Ideas captured but not decided. Nothing here is in the plan |
| [`d5-entscheidungsvorlage.md`](d5-entscheidungsvorlage.md) | Decision paper (German): what is actually demanded for Hilary Fisher Page — plaque, exhibition, or a foundation for pedagogical play |
| [`d6-entscheidungsvorlage.md`](d6-entscheidungsvorlage.md) | Decision paper (German): which three Atlas sites the 4:00 cut shows. Five variants, criteria, recommendation |
| [`../human-todo.md`](../human-todo.md) | Wikipedia / Wikidata / Commons corrections Stefan makes by hand (German). Not engine work |
| [`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md) | **Review 01** — seven specialist reviews, with a decision and a reason for every finding |

Also required background, outside this directory: `AGENTS.md`, `CLAUDE.md`,
`TODOs.md`, `BRICKs.md`, and `docs/agents/`. This spec assumes all five.

## Why the spec is split

`docs/FUGEN-ENGINE.md` was 130 KB — roughly 33 k tokens re-read at the start
of every session, to work on one milestone. A phase file is about 4 k. That
was the single largest context saving available, so rev 3 is the split. The
monolithic file remains as a stub pointing here.

## How to work through this

1. **One milestone = one or more small commits**, never one giant commit.
   `docs/agents/working-mode.md`'s commit discipline applies verbatim.
2. **Verify before committing**, using `docs/agents/verification.md`'s ladder.
   Every milestone names which rungs are mandatory for it. **Rung 5 (real
   headless Chromium) is mandatory for every viewer-visible milestone** — this
   is a cinematic project; "it compiles" proves nothing about it.
3. **The build order is now three deep.** `wasm-pack build` → `npm run build`
   → `cargo build --release`. Skipping any step is the new "I changed it and
   nothing happened".
4. **Real data only.** Real LDraw geometry, the real LDConfig colour table,
   real World Heritage data, real published flag construction specifications,
   real patent numbers. Where a real value cannot be obtained, the milestone
   says so and the code records the gap.
5. **Additive, not destructive.** The point-cloud and graph pipelines stay
   exactly as they are and keep working for every existing demo. The mesh
   renderer is a *second* mode selected by the presence of a manifest.
   `spex brick-part` / `brick-model` / `brick-assembly` / `brick-cinematic`
   keep working unchanged for the entire plan.
6. **Update `TODOs.md` as you go**, in the M1–M50 style: what was built, what
   was *verified*, with which real numbers. `TODOs.md` stays the source of
   truth for status; this directory is the plan.
7. **Do not batch milestones.** Ship M51 before starting M52. Every milestone
   is written so the repository is in a working, demoable state at its end.
8. **New ideas go to `TODOs.md`'s backlog, not into the current milestone.**
   Rev 1 grew three milestones this way before a line of code was written.

## Scope

**In**, and now comfortably so, because the premiere moved to the 2027
centenary ([`plan.md`](plan.md) §1): all seven phases, the Atlas at 40 sites,
the wasm phase, and all four cuts — 4:00, 10:00, 60:00 and endless.

**The deliverable that matters** is the **4:00 canonical cut with three Atlas
sites, complete by early May 2027** — seven weeks before the premiere. Everything after that gate is
improvement, and the extra nine months are buffer and apparatus, not licence
to reopen Phase 1.

**Out of scope, permanently:** physics simulation (every settle and collapse
is authored easing), ray tracing, video export, WebGPU this cycle
([`budgets.md`](budgets.md) §7), and any use of the trademarked brand name,
any official set's design, or any scraped commercial catalogue.

## Milestone index

| Phase | M | Title | File |
|---|---|---|---|
| **P1** | M51 | BFC-correct geometry and real edge extraction | [phase1](phase1-renderer.md) |
| | M52 | The mesh bundle format (`spex-mesh`) | |
| | M53 | `spex mesh-part` / `mesh-model` | |
| | M54 | The viewer's mesh render mode | |
| | M55 | Instanced rendering | |
| | M56 | The real LDraw material system | |
| | M57 | **Crisp edges and conditional edges** — the *vektorgenau* milestone | |
| | M58 | Post-processing and lighting | |
| | M59 | Mesh LOD and culling | |
| **P2** | M60 | The `show.json` format | [phase2](phase2-show.md) |
| | M61 | `spex-show`: compiler and duration resolver | |
| | M62 | The clock and the timeline evaluator | |
| | M63 | The camera director | |
| | M64 | Runtime choreography | |
| | M65 | Dissolve, materialise, point↔mesh crossfade | |
| | M66 | `spex show` / `show-export`, URL parameters, HUD | |
| **P3** | M67 | `fugue.json` and the subject | [phase3](phase3-audio.md) |
| | M68 | The counterpoint generator | |
| | M69 | The WebAudio engine | |
| | M70 | Scheduler and runtime realisation | |
| | M71 | Audio↔visual binding, autoplay policy, mixer | |
| **P4** | M72 | `spex-build`: grid-legal parametric construction | [phase4](phase4-kit.md) |
| | M73 | `spex-heritage`: the World Heritage index | |
| | M74 | The Atlas site models | |
| | M75 | `spex-flag`: flags as brick mosaics | |
| | M76 | The wave (tier-3 / installation only) | |
| | M77 | Atlas autopilot — the XML-driven pipeline | |
| **P5** | M78–M82 | Act I · Act II · Act III · Atlas · Act IV + Der Kick | [phase5](phase5-work.md) |
| | M83 | The four cuts | |
| | M84 | Titles, credits, attribution, `LICENSING.md` | |
| | M85 | Seeded editions | |
| **P6** | M86 | `spex-wasm`: the boundary and the toolchain | [phase6](phase6-wasm.md) |
| | M87 | The timeline evaluator in wasm, zero-copy | |
| | M88 | `spex-build` / `spex-ldraw` / `spex-flag` in the browser | |
| | M89 | The fugue in wasm; DSP in an AudioWorklet | |
| | M90 | The wasm phase gate: measure, document, decide | |
| **P7** | M91–M97 | Determinism · compatibility · accessibility · deployment · single-file edition · docs · archive record | [phase7](phase7-ship.md) |

## Revision history

| Rev | Date | What changed |
|---|---|---|
| 1 | 2026-07-25 | First full spec, M51–M97, in one file. Planning input, not an approved plan |
| 2 | 2026-07-25 | Seven specialist reviews (`../FUGEN-ENGINE-REVIEW-01.md`) and the binding amendments they produced: fourteen technical blockers, the device budgets, the screenplay re-authored in bars, ten historical corrections, and a scope decision |
| **3** | **2026-07-25** | Split into this directory. Premiere rebased to the **2027 centenary of GB 263865**, which restores full scope: Phase 6, M77, Atlas tier C and the 60:00 cut are back in. Human gates are now scheduled slots. Definition of done gains signed artefacts where an automated check is impossible |

Planned for rev 4: the corrections currently carried as per-phase amendment
blocks get folded into the milestone text itself, once P0's four spike
questions are answered.

---

*Iunctura Archiv · Signatur IA-2026-002*
*"Die Römer fügten Ziegel. Wir fügen Kunststoff. Die Sprachmodelle fügen Token."*
