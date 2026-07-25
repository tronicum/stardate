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
Dokument"* of the whole chain. A British specification filed on that priority
was accepted in **1927**.

> **ACTION, week 1, blocking the premiere date:** pull GB 263865's own
> "Complete Accepted" date from Espacenet or Google Patents and record it in
> `TODOs.md`. The premiere is that date + 100 years. Everything below assumes
> a premiere in the second half of 2027; the exact day comes from the patent,
> not from us. The Belgian priority centenary (31 December 2025) has already
> passed, which is precisely why GB 263865's 1927 acceptance is the right
> anchor.

This gives roughly **52 weeks of build plus real buffer** instead of 26 weeks
with none. Consequences, all adopted:

- **Full scope is back in.** Phase 6 (wasm), M77 (the XML autopilot), Atlas
  tier C and the 60-minute installation cut return to the plan.
- **The 4:00 canonical cut still exists by early May 2027** — nine months
  before it is needed, which is the correct amount of slack for a piece whose
  hardest risk ("is the fugue musically dead?") is a taste judgement.
- **The October 2026 collision dissolves.** `masterplan-iunctura-site.md`
  promised a running loop by end of October 2026; that becomes an honest
  **Act I technical preview** instead of a broken promise.
- **The Postillen should probably move too.** Sealed letters arriving in the
  centenary year, referring to the centenary, are a far stronger object than
  letters arriving in October 2026 about a work that does not exist yet.
  **Stefan's call, and worth making early** — it changes the printing,
  sealing and diplomatic-post lead times.

## 2. The calendar

| Weeks | Dates | Phase | Milestones | Gate |
|---|---|---|---|---|
| 1–3 | Jul 27 – Aug 16 2026 | **P0 Spikes** | — | Four questions answered before any production code: (a) GB 263865's acceptance date; (b) fugue spike — theory + counterpoint + MIDI export, **listened to by a human**; (c) do the three patent bricks have usable geometry (see §4); (d) does mesh beat points on the monolith, yes or no |
| 4–12 | Aug 17 – Oct 18 2026 | **P1 Renderer** | M51–M59 | A 1×1 brick renders with catalogue-quality edges at 60 fps, and the 200 k-instance synthetic scene holds its budget. **Safari on real hardware by week 8.** |
| 13–18 | Oct 19 – Nov 29 2026 | **P2 Show engine** | M60–M66 | Act I plays end to end from `show.json`, silent, at all four durations |
| 19–24 | Nov 30 2026 – Jan 10 2027 | **P3 Audio** | M67–M71 | Four voices, in tune, in time, generated in the browser, in sync |
| 25–33 | Jan 11 – Mar 14 2027 | **P4 Kit / Atlas / flags** | M72–M77 | Sites and flags build from recipes; the autopilot's classifier is *scored* against the hand-curated tiers before it ships |
| 34–40 | Mar 15 – May 2 2027 | **P5 The work** | M78–M85 | **The canonical 4:00 cut exists, end to end, with sound, looping seamlessly.** The real deadline; everything after is improvement |
| 41–43 | May 3 – May 23 2027 | **P6 wasm** | M86–M90 | One implementation of the evaluator and the resolver, measurably faster |
| 44–47 | May 24 – Jun 20 2027 | **P7 Ship** | M91–M97 | Deployed, documented, licensed, reproducible |
| 48–52 | Jun 21 – Jul 25 2027 | **Extended cuts** | tier C, 60:00 | The installation cut, if the Atlas holds its performance budget |
| — | Jul – premiere 2027 | **Buffer + apparatus** | — | Installation build-out, Postillen, on-chain edition, press. Not engineering weeks |

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
| Week 1 | GB 263865 acceptance date → the premiere date | 1 h |
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
the physical Postillen, the LEGO Group contact — and, decided once and early,
whether to pursue World Heritage Centre data authorisation
([`licensing.md`](licensing.md)), which has institutional lead times measured
in months.

## 4. Two decisions that must be taken in week 1–2

**(a) The premiere date.** GB 263865's own acceptance date, from the patent.
Blocking everything downstream that says "2027".

**(b) The three patent bricks have no LDraw geometry.** Batima (1924) and
Kiddicraft (1940) are not in the LDraw library, and the project's own rule 4
forbids inventing geometry — so A3-S02 and A3-S03, and therefore M80's gate,
are currently unmeetable and nobody had noticed. Two honest options:

1. **Hand-author three `.dat` files** from the real published patent
   drawings, labelled unambiguously as this project's own reconstructions,
   with the drawing cited in each file's header. The archive already has
   `FR588985A` at 900×602 and a CC0 photograph of real Batima bricks — enough
   to reconstruct responsibly.
2. **Restage A3-S02/S03** around parts that exist, and carry the patent
   evidence as overlay and citation rather than as geometry.

Option 1 is stronger and is the recommendation, *provided* the reconstruction
is declared everywhere it appears. Decide before P1 ends, because it changes
what M74's recipes need.

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
| 1 | `spex show-build … --duration 240` and `spex show` play the canonical cut end to end, with sound, ≥ 55 fps at 1080p, zero console errors | Automated headless run + counters ([`budgets.md`](budgets.md) §6) |
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
