# Fugue spike — S0, answering D7

**The question:** *is the generated fugue musically alive?* It is the
top-listed risk in [`../../docs/fugen/plan.md`](../../docs/fugen/plan.md) §5,
and the one thing in this project that no amount of testing can settle. So it
gets answered in week 3, by ear, before Phase 2 and Phase 3 are authored on
top of it — not in month six.

**This is a throwaway.** The real implementation is `crates/spex-fugue`
(M67/M68). Nothing here is meant to survive into it except the authored
subject and whatever verdict comes back.

```sh
python3 fugue_spike.py -o out/       # writes out/fugue-spike-exposition.{mid,wav}
```

Only `numpy` is required. The synth is a plain additive organ written into the
script, so the WAV is listenable without a soundfont; the `.mid` is a real
type-1 SMF for opening in a notation program.

## What it actually generates

The **exposition**, on the screenplay's own bars — entries at bars 5, 7, 11
and 14, closing at bar 17, which is the end of Act I. 84 bpm, D dorian,
four voices, 49.6 seconds.

| | |
|---|---|
| **Subject** (authored) | D4 A4 G4 F4 E4 · F4 G4 A4 G4 F4 |
| **Tonal answer** | A4 D5 C5 B4 · C5 D5 E5 D5 C5 |
| **Countersubject** (generated, rule-checked) | D5 E5 D5 B4 D5 F5 G5 F5 G5 E5 D5 C5 D5 E5 C5 |

The subject is the one musical decision this project does not delegate. Its
head leaps tonic → dominant, which is exactly the case that **forces** a tonal
answer rather than a plain transposition — deliberately, so that if the real
generator ever takes the easy path, it is audible rather than theoretical.

The countersubject is *not* hand-written. It comes out of a small constrained
search: mostly stepwise, contrary motion preferred, no voice crossing, within
a tenth of the subject, **no parallel fifths or octaves — and none either when
it is dropped an octave**, which is M68's invertibility requirement
demonstrated at small scale. The script prints both checks.

## What to listen for

Four things, in this order. The first two are the verdict; the rest is detail.

1. **Does the subject have a face?** After the second entry you should be able
   to recognise it coming back. If it is forgettable at 0:20, no amount of
   correct counterpoint will save it, and the subject gets rewritten — which
   is cheap now and expensive in March.
2. **Does the tonal answer sound like an answer**, or like a transposition
   that stumbled? It enters at bar 7, about 0:15.
3. **The two episodes** (bars 9–11 and bar 13). They are sequences on the
   subject's head with no new material. Do they carry, or do they sag into
   filler? Sagging episodes are the most common way a generated fugue stops
   sounding like one.
4. **The four voices at bar 14.** The texture should thicken without turning
   to mud.

## What a "no" costs, and what it triggers

The fallback is already decided in [`../../docs/fugen/plan.md`](../../docs/fugen/plan.md)
§5: **hand-composed subject *and* exposition, generated episodes only.** That
keeps the runtime generation — and therefore the seeded editions and the
endless cut — while taking the part machines are worst at away from them.
Deciding that in week 3 costs a rewrite of `FugueSpec`'s plan section. Deciding
it in month six costs Phase 5.

## What this spike does *not* answer

It has no development, no stretto, no pedal point, no percussive layer, and no
modulation beyond the episodes' own sequences. It is the first 17 of 84 bars.
A good exposition does not guarantee a good fugue — but a bad one guarantees a
bad one, which is why this is the part that gets heard first.

**And one honest limit:** everything above was verified numerically — no
parallels, entries on the right bars, every note inside its voice's real
range. Whether it is *alive* is not a property anything here can measure. That
is the whole reason the gate is a human with 20 minutes and a pair of
headphones.
