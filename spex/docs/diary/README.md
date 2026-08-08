# Development diary

Three places in this repository say what happened, and they are not
interchangeable.

- **`docs/fugen/phase*.md`** carries each milestone's status block: what was
  built, which acceptance criteria hold, and the numbers. It is a *record*,
  written to be checked against.
- **`TODOs.md`** carries the same thing compressed to one paragraph per
  milestone, so a reader can scan the whole project in ten minutes.
- **This diary** is the part neither of those has room for: **what went wrong,
  in the order it went wrong, and what it turned out to mean.**

The distinction matters because the interesting content of this project is
almost never the feature. It is the defect underneath it. A status block can
say "AC1 passes, 17 309 → 0 lit pixels" — it cannot say that the first three
runs passed every number while the screenshots showed an object that had not
moved, and that the reason was the same reason three times over.

**One entry per milestone, written the day it lands, and only where there is
something to say.** A milestone that went exactly as planned does not get an
entry; it gets a status block and nothing else. Nobody is served by a diary
of successes.

The rule the entries follow: **a defect goes in the diary when the way it was
found is more instructive than the defect itself.**

| Entry | Milestone | The short version |
|---|---|---|
| [`2026-08-08-m65-dissolve-and-crossfade.md`](2026-08-08-m65-dissolve-and-crossfade.md) | M65 | Four defects, every one found by a picture that the numbers had already passed |
