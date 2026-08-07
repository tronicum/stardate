# Phase 5 - the work itself (M78-M85)

*Authorship. Each milestone is 'this act now plays end to end, in time, with sound'.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**
Read [`README.md`](README.md) first: it carries the working rules, the scope decision and the milestone index.
Review record and the reasoning behind every rev-2 change: [`../FUGEN-ENGINE-REVIEW-01.md`](../FUGEN-ENGINE-REVIEW-01.md).

---


By this point the engine exists. Phase 5 is authorship: `show.json` grows
movement by movement, and each milestone is "this act now plays end to end,
in time, with sound".

| M | Deliverable | Gate |
|---|---|---|
| **M78** | **Act I — Archäologie der Fuge** plays end to end | §8's A1 shot list realised; monolith height 73.6 mm on screen; the fugue's first two entries land on their scored bars |
| **M79** | **Act II — Der Core Standard** | the bulla→coin metamorphosis reads as one continuous transformation, not a cut; 4-voice exposition completes exactly at the act boundary |
| **M80** | **Act III — Die Fuge** | the three patent bricks (1923 / 1939 / 1949) render from real geometry, the clutch-power macro shot is legible, stretto begins on cue |
| **M81** | **The Atlas movement** | tier-A sites + flags + chronicle cards, with the site→flag→card choreography working generically for any site, so tiers B and C are data, not code |
| **M82** | **Act IV — Der Token, and Der Kick** | mesh→point dissolve, the neon grid, and an 800 ms Kick that is frame-exact against its audio accent |
| **M83** | **The four cuts** | 4:00, 10:00, 60:00 and endless all resolve, run, and loop seamlessly (§8.4) |
| **M84** | **Titles, credits, attribution, `docs/LICENSING.md`** | LDraw CCAL attribution, Wikidata CC0, OSM ODbL, the no-UNESCO-emblem rule, the no-brand-name rule, all stated in the work and in the repo |
| **M85** | **Seeded editions** | `?seed=` visibly and audibly varies the piece within authored bounds, deterministically — the on-chain-edition prerequisite |

Each of M78–M82 follows the same internal shape and the same verification
ladder (1, 2, 3, 5 **mandatory** — an act is by definition a change to the picture; 6 at the end of the phase):

1. Author the act's shots in `show.json` from §8's shot list.
2. Build or generate every scene it needs.
3. Realise the camera track.
4. Bind the act's audio cues.
5. Headless-capture the act at 2 fps for a contact sheet and at 60 fps for
   the two or three moments that must be exact.
6. **Watch it.** A human watches the act, in a browser, with sound, before
   the milestone closes. Record what they said in `TODOs.md`. This is not
   ceremony: every visual decision in this document is a guess until
   somebody looks at it.

### M85 in detail — seeded editions

What the seed is allowed to vary (authored bounds, never structure):

- the Atlas's site *selection and order* (from the reviewed buildable set);
- the fugue's episode material and the exact voicing of entries — never the
  subject, never the section plan;
- brick colour palette within a per-act permitted set of real LDraw codes;
- camera orbit start angles and the scatter seed of every assembly;
- the wind seed for the flags.

What the seed must never vary: act order, act durations, the Kick, the loop
seam, the subject, or any factual content.

**Acceptance criteria.** Two different seeds produce visibly and audibly
different runs; the same seed twice produces frame-identical runs (verified
by hashing 20 sampled frames); every seed in [0, 999] resolves without error
(run all thousand headlessly at 1 fps, assert no exceptions).

---
