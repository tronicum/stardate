# The plan, the risks, and the definition of done

*Rebased to the 2027 centenary. Rev 3 replaces rev 1's 26-week calendar.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**
The reasoning behind every change here: [`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md) §6.

---

## 1. The date changed, and it changed everything

Rev 1 planned 26 weeks to 24 January 2027. An independent production review
put the honest figure at **41 weeks** and forced a scope decision: ship the
4:00 cut with three sites, cut the wasm phase, the Atlas autopilot, tier C
and the 60-minute installation.

**That decision is now reversed, because the deadline moved for a good
reason.**

The work has a real anniversary to premiere against: **2027 is the centenary
of the 2×4 studded brick as a documented standard.** The archive's own
research (`claude/quellenlage-alle-artikel.md`) identifies **GB 263865** —
Batima, Belgian priority **31 December 1925** — as the patent that contains
"eight studs in two rows", i.e. the 2×4 canon, and calls it *"das wichtigste
Dokument"* of the whole chain. That was right.

**RESOLVED, 2026-07-25 — the premiere is Wednesday, 23 June 2027.**
GB 263865 A, *Improvements in building blocks*, applicant J. Girlot, priority
31 December 1925, filed 31 December 1926, **published 23 June 1927**. Its
abstract is the reason it is the right document: *"A set of toy bricks formed
with two rows of pegs a on one face and corresponding rows of recesses b on
the opposite face… the quarter brick having four pegs, the half brick six, and
the full brick eight."* Eight studs in two rows, in print, in 1927.

The earlier candidates are both past: BE 311029's priority is 6 June 1923, and
GB 217243 (the *solid* carton-pierre block, "J. Girlot, assignee of L. Cousin")
was published 21 May 1925. See [`decisions.md`](decisions.md) D1a for the full
table and for the sourcing caution that came with it.

This gives roughly **52 weeks of build plus real buffer** instead of 26 weeks
with none. Consequences, all adopted:

- **Full scope is back in.** Phase 6 (wasm), M77 (the XML autopilot), Atlas
  tier C and the 60-minute installation cut return to the plan.
- **The 4:00 canonical cut still exists by early May 2027** — seven weeks
  before the premiere, with Phase 6, Phase 7 and the extended cuts sitting in
  between as compressible work. The slack is real but it is not nine months:
  a slip in Phase 5 eats the wasm phase first, then tier C, then the
  60-minute cut, in that order.
- **The October 2026 collision dissolves.** `masterplan-iunctura-site.md`
  promised a running loop by end of October 2026; that becomes an honest
  **Act I technical preview** instead of a broken promise.
- **The Postillen move with it (D3, decided).** Sealed letters arriving in
  the centenary year, referring to the centenary, are a far stronger object
  than letters arriving in October 2026 about a work that does not exist yet.
  Printing, sealing, translation and diplomatic post now schedule backwards
  from 23 June 2027 — and if D4's pre-announced protocol holds, the email
  goes out earlier still, with X+90 landing after the premiere.

## 2. The calendar

| Weeks | Dates | Phase | Milestones | Gate |
|---|---|---|---|---|
| 1–3 | Jul 27 – Aug 16 2026 | **P0 Spikes** | — | Three questions answered before any production code (the fourth, the premiere date, is resolved — see §1): (a) fugue spike — theory + counterpoint + MIDI export, **listened to by a human**; (b) reconstruct the three patent bricks from their drawings (D2); (c) does mesh beat points on the monolith, yes or no |
| 4–12 | Aug 17 – Oct 18 2026 | **P1 Renderer** | M51–M59 | A 1×1 brick renders with catalogue-quality edges at 60 fps, and the 200 k-instance synthetic scene holds its budget. **Safari on real hardware by week 8.** |
| 13–18 | Oct 19 – Nov 29 2026 | **P2 Show engine** | M60–M66 | Act I plays end to end from `show.json`, silent, at all four durations |
| 19–24 | Nov 30 2026 – Jan 10 2027 | **P3 Audio** | M67–M71 | Four voices, in tune, in time, generated in the browser, in sync |
| 25–33 | Jan 11 – Mar 14 2027 | **P4 Kit / Atlas / flags** | M72–M77 | Sites and flags build from recipes; the autopilot's classifier is *scored* against the hand-curated tiers before it ships |
| 34–40 | Mar 15 – May 2 2027 | **P5 The work** | M78–M85 | **The canonical 4:00 cut exists, end to end, with sound, looping seamlessly.** The real deadline; everything after is improvement |
| 41–43 | May 3 – May 23 2027 | **P6 wasm** | M86–M90 | One implementation of the evaluator and the resolver, measurably faster |
| 44–46 | May 24 – Jun 13 2027 | **P7 Ship + extended cuts** | M91–M97, tier C, 60:00 | Deployed, documented, licensed, reproducible. The installation cut if the Atlas holds its budget |
| 47 | Jun 14 – Jun 22 2027 | **Apparatus** | — | Installation build-out, the Postillen in the post, the on-chain edition, press. Not engineering days |
| — | **Wed 23 Jun 2027** | **PREMIERE** | — | **100 years of GB 263865 — eight studs in two rows** |

**Milestone-week interim deliverable, October 2026:** the vector-accurate
monolith running in a browser. That is what goes on the USB stick if the
Postillen ship early, and it is honestly labelled a technical preview.

## 3. The human gates

The single largest omission in rev 1: **Stefan is the bottleneck, not the
agents.** Roughly twenty blocking human gates existed as acceptance criteria
with no calendar slot, and rev 1's risk register did not list him at all.

**Every gate below is a scheduled, named slot.** A missed gate is a schedule
slip, not a silent block. Batch them into one weekly review.

| When | Gate | Cost |
|---|---|---|
| Week 2 | Patent-brick geometry decision (§4) | 1 h |
| **Week 3** | **Listen to the fugue spike.** If it is dead, the fallback (hand-composed subject *and* exposition, generated episodes only) is decided here — not in month six | 2 h |
| Week 3 | Mesh-vs-points verdict on the monolith | 1 h |
| Weeks 5, 9, 12 | Look at the M57 edge screenshots at 0.5× and 50× | 30 min each |
| Week 8 | Safari on real hardware | 2 h |
| Week 18 | Watch Act I, silent | 1 h |
| Week 24 | Listen to the full generated score | 2 h |
| Week 28 | Review the heritage curation **and the exclusion list** — signed and dated | 3 h |
| Week 30 | Flag contact sheet, every flag, with its ΔE | 2 h |
| Weeks 34–40 | Watch each act as it lands, with sound | 1 h × 5 |
| Week 40 | Watch the whole canonical cut | 1 h |
| Week 47 | Watch all four cuts | 3 h |
| Standing, from week 18 | Weekly full-run capture, reviewed | 30 min/wk |

Plus the items only Stefan can do at all, from
`claude/masterplan-iunctura-site.md`: DNS, Pinata/IPFS, the Ethereum wallet,
the physical Postillen, and the LEGO Group contact.

**Data acquisition is no longer among them, in the sense of blocking anything
(D10).** Stefan sources the underlying data legally and supplies it; no
milestone waits on an authorisation. What the spec still owns is what the work
*publishes*: provenance, attribution, the reconstruction declaration, and the
exclusion list — none of which are acquisition questions.

## 4. Decisions taken, and what is still open

Settled on 2026-07-25 — see [`decisions.md`](decisions.md) for the full
reasoning and what each one rules out:

- **D1 — the premiere is 23 June 2027**, the centenary of GB 263865's
  publication: eight studs in two rows, in print, in 1927. **Resolved.**
- **D2 — the three patent bricks are reconstructed from their real published
  drawings, and declared as reconstructions everywhere they appear.** They
  live in `ldraw-scenes/reconstructions/`, never mixed with resolved library
  parts. The real-data rule continues to govern everything the work presents
  as *evidence* — dates, patent numbers, colours, dimensions, metadata.
- **D3 — the Postillen move to 2027.** October 2026 becomes an honest Act I
  technical preview instead of a delivery date.
- **D4 (provisional) — the protocol is announced in advance, by email**, so
  that silence becomes a measurable result rather than a failure state. This
  deletes the success ladder and the Louis Cousin genealogy from
  `masterplan-bewegung-postillen.md`.
- **D10 — data acquisition is Stefan's lane and out of scope here.** The
  heritage tooling becomes an importer of a supplied snapshot rather than an
  acquisition tool reasoning about terms of use. Provenance, attribution and
  the exclusion list all stay: acquisition is his, publication is the work's.

**D5** (the Fisher Page ask becomes a foundation, staged) and **D6** (the three
Atlas sites are Stonehenge, Mohenjo-daro and a dougong hall) are now closed
too. **Only D7 — is the generated fugue musically alive? — and D8 — does the
mesh renderer beat the point renderer? — remain, and both are answered by the
P0 spikes in weeks 1–3.** Nothing else blocks the start.

## 5. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **The generated fugue is technically correct and musically dead** | medium | **high** | Spike it in weeks 1–3, off the critical path, and *listen* in week 3. Fallback decided there: hand-composed subject and exposition, generated episodes only. Do not carry a bad score into Phase 5 |
| **Stefan's gates slip and silently block** | **high** | **high** | §3's named slots; a missed gate is a tracked slip |
| Conditional edges eat a week | medium | medium | Time-boxed to 3 days; hard edges alone already achieve ~80 % of the look, and the screen-space outline path is needed regardless |
| The mesh renderer looks worse than the point renderer | low | high | Decided in **week 3**, before anything depends on it. The point pipeline is still there |
| Missing patent-brick geometry | **certain, already identified** | medium | §4(b), decided in week 2 |
| Atlas tier C is too much authoring | medium | low | Now genuinely optional; the 4:00 cut needs three sites |
| M77's classifier is unreliable | medium | low | Fails closed; provisional recipes require human promotion. A bad classifier costs review time, never a bad frame |
| Safari | high | medium | Week 8, on real hardware. Budget a week |
| wasm toolchain friction | medium | low | Phase 6 sits after the deliverable exists |
| **Scope creep from the piece being interesting** | **high** | **high** | The four cuts, the shot list and the milestone table are the scope. New ideas go to `TODOs.md`'s backlog, not into the current milestone. Rev 1 already grew M77, M88 and M89b this way |
| **The extra year invites gold-plating** | **new, high** | medium | The May 2027 gate is the deadline that matters. Weeks 41+ are for the extended cuts and the apparatus, not for revisiting Phase 1 |

## 6. Definition of done

The work is finished when all of the following are true, and not before.
Rev 1's list contained items that would silently never have been checked;
each of those now has either a real check or a **signed artefact**.

| # | Criterion | How it is verified |
|---|---|---|
| 1 | `spex show-build … --duration 240` and `spex show` play the canonical cut end to end, with sound, zero console errors, and the counters recorded. **Frame rate is a gate only on M92's named hardware** — nowhere in this pipeline has a GPU, so a number from anywhere else is not evidence | Automated headless run + counters ([`budgets.md`](budgets.md) §6) |
| 2 | The 10:00, 60:00 and endless cuts each resolve to their exact duration and play end to end | Automated |
| 3 | Frame 0 and the final frame of every cut are pixel-identical, and the audio meets itself at the seam | Automated frame hash — **and the final pixel is drawn in a pass that bypasses the composer**, or this silently fails on a different DPR |
| 4 | The Kick is 2 beats in every cut, and its audio onset and first frame are within one frame | Measured, recorded |
| 5 | Same seed → frame-identical runs; different seeds → visibly different | Hash 20 sampled frames; sample 25 seeds, not 1 000 |
| 6 | Every rendered brick is real LDraw geometry; every colour a real LDraw code; every site's metadata CC0-sourced; every flag matches a cited construction spec | **A generated `PROVENANCE.md`**, built from the bundles' own metadata at `show-build` time, listing every part, colour, site and flag with its source. Not a promise — an artefact |
| 7 | Every attribution required by [`licensing.md`](licensing.md) appears in the repo **and on screen** | The credits are generated from the same `PROVENANCE.md`. A missing source fails the build |
| 8 | The exclusion list has been reviewed for this release | **A dated, signed file** naming the reviewer — who is not the author |
| 9 | `cargo test --workspace` green; `npx tsc --noEmit` clean; `./scripts/walkthrough.sh` regenerates every pre-existing demo unchanged | Automated |
| 10 | `TODOs.md` carries an entry for every milestone M51–M97 in the established style, stating what was *verified*, with real numbers | Reviewed at the phase gate |
| 11 | A human has watched all four cuts, with sound, and said what they thought | §3's week-47 slot, recorded in `TODOs.md` |
| 12 | The premiere date is the real centenary of GB 263865's acceptance | §4(a) |
