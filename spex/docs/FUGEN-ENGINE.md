# FUGEN-ENGINE.md — moved

The Fugen Engine implementation spec (M51–M97) now lives in
**[`docs/fugen/`](fugen/README.md)**, split one file per phase.

**Start at [`docs/fugen/README.md`](fugen/README.md).**

## Why

The monolithic version of this document was 130 KB — roughly 33 k tokens
re-read at the start of every session, to work on a single milestone. A phase
file is about 4 k. That was the largest single context saving available on
this project, so rev 3 is the split.

## Where things went

| Was | Is |
|---|---|
| §0 working rules, §0.1 amendments | [`fugen/README.md`](fugen/README.md) + per-phase amendment blocks |
| §1 where we are, §2 architecture | [`fugen/00-context.md`](fugen/00-context.md) |
| §3 Phase 1 (M51–M59) | [`fugen/phase1-renderer.md`](fugen/phase1-renderer.md) |
| §4 Phase 2 (M60–M66) | [`fugen/phase2-show.md`](fugen/phase2-show.md) |
| §5 Phase 3 (M67–M71) | [`fugen/phase3-audio.md`](fugen/phase3-audio.md) |
| §6 Phase 4 (M72–M77) | [`fugen/phase4-kit.md`](fugen/phase4-kit.md) |
| §7 Phase 5 (M78–M85) | [`fugen/phase5-work.md`](fugen/phase5-work.md) |
| §8 the screenplay | [`fugen/screenplay.md`](fugen/screenplay.md) — **re-authored in bars** |
| §9 Phase 6 (M86–M90) | [`fugen/phase6-wasm.md`](fugen/phase6-wasm.md) |
| §10 the plan, §13 risks, §14 done | [`fugen/plan.md`](fugen/plan.md) — **rebased to the 2027 centenary** |
| §11 Phase 7 (M91–M97) | [`fugen/phase7-ship.md`](fugen/phase7-ship.md) |
| §12 licensing | [`fugen/licensing.md`](fugen/licensing.md) |
| *new in rev 3* | [`fugen/budgets.md`](fugen/budgets.md) — device, memory, long-run and AI budgets |

The review that produced rev 2's corrections is unchanged and still at
[`FUGEN-ENGINE-REVIEW-01.md`](FUGEN-ENGINE-REVIEW-01.md).
