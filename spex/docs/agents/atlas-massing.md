# Building an Atlas sheet

A playbook for adding a World Heritage site to `heritage/` — written from
twenty-two of them, and from the roughly forty failed generations it took to
get there. Everything here is a defect that really happened, in the order it
tends to happen.

The loop is: **write the sheet → `gen-atlas.py --strict` → `atlas-diagnose.py`
→ fix → repeat until `sauber`.** Nothing ships with a nonzero problem count.
`gen-atlas.py` rejects a sheet outright and *leaves the previous recipe on
disk* — so a brick count that refuses to change means the sheet was rejected,
not that the edit did nothing. Read the rejection line.

## The citation contract

`dimensions` holds figures **quoted from a source, verbatim, with the source
string next to them**. Everything else — every number the modeller chose — is
counted by the generator as an uncited size and charged against
`uncitedBudget`. The budget is not a quality gate; it is a *disclosure*. A
sheet with one cited dimension and twelve uncited ones is honest as long as
the note says so, and Machu Picchu (1 of 25) and Petra (1, and it is a
volume) both ship on exactly that basis.

Two rules that keep it honest:

- **Arithmetic on a source is not a source.** Mont-Saint-Michel's article
  gives a 960 m circumference and no diameter. The 305.6 m the model uses is
  division, plus an assumption that the mount is a circle, which it is not.
  That step goes in the note, never into `dimensions`.
- **A figure for one thing is not a figure for another.** Sigiriya's only
  horizontal dimension is the extent of the *frescoes* on the west face —
  140 m. The sheet uses it as the rock's footprint and says that it is a
  substitution.

What a source gives you is often not buildable at all. The Iron Bridge's
article offers 1 700 components, 384.6 t of iron and a 5.3 t heaviest member:
counts and masses, and **a mass cannot be built**.

**Get the QID right.** The sheet's `qid` must be the **World Heritage Site**,
not the building. Q10285 is the Colosseum; the site is Q18448486, the Historic
Centre of Rome. Ten of the first seventeen sheets had the building's QID, and
the only symptom was a blank inscription year. `bauchbinden-daten.py` warns
per sheet when a QID is absent from the snapshot — run it after adding one.

## What the primitives actually do

Most failed generations were one of these six.

**An `Arch` is `span` PLUS twice `thicknessStuds` wide.** A 5.1 m arch on
3-stud piers is 11.1 m wide. Segovia laid 24 of them on a 7.6 m pitch and
overlapped its neighbours 1 996 times; the Pont du Gard taught the same lesson
first. Pitch the arcade off the *total*, not the span.

**`onTopOf` miscomputes for an `Arch`.** An Arch carries its origin
differently from a Wall, and `{"onTopOf": n}` put the Iron Bridge's ribs at
−24 plates where 25 was wanted. Give an Arch an explicit `yM`.

**A deck over an arch touches it only at the crown.** Everything else floats.
Real bridges use spandrels, and this vocabulary has no spandrel solid — the
Iron Bridge's deck cost 210 floating bricks, and spandrel walls made it 4 841.
Five attempts in, the right answer was to build the arch and write down why
there is no deck.

**A column can't carry a beam by itself.** A `Colonnade` with
`architrave: true` and a Wall laid across the capitals both leave nothing
under the beam's midspan. The Parthenon and Petra's upper order both needed a
real entablature course between them.

**A platform is not a baseplate.** Paro placed at x=28 on a 26 m ahu stands
beside it, in the air: 660 floating bricks. Check every `at` against the
extent of the thing beneath it.

**`arrangeOn` knows `line` and `circle`/`arc`. There is no `rect`.** Angkor's
four corner towers are four steps. Inventing a kind fails with a `KeyError`
on the missing radius, which reads like a bug and is not one.

## Scale is a decision with a cube in it

`studsPerMetre` sets the brick count by the **cube**. A solid `Ziggurat` is
the most expensive body in the set: Mont-Saint-Michel was 203 578 bricks at
0.8 and 8 402 at 0.3. Pick the scale from what has to be *legible* in the
shot, then check the count.

Two failure modes at the small end:

- **Under one brick course, a `Wall` emits nothing, silently.** Sigiriya's
  2 m garden slab at 0.35 studs/m is 0.7 studs; `validate()` rejects it. Use
  `Mosaic` for a genuinely one-plate layer.
- **Rounding closes gaps.** Angkor's 4.5 m wall is 1.575 studs at 0.35 and
  rounds to 2, so its four wall runs overlapped at the corners. Leave a
  stud of slack at seams below ~1 stud/m.

And at the top end: `arc_point` snaps to the half-stud grid, so two neighbours
on a small circle round onto the same studs. Borobudur's inner terraces carry
8, 6 and 4 stupas against the monument's 72 — the count is in the note, and
the model does not pretend to it.

## The note is part of the deliverable

Every sheet's `note` says what the model does **not** do, in specifics: 24 of
Segovia's 167 arches, 400 m of the Great Wall's 8 850 km, Chichén Itzá's 365
steps left unbuilt because one step is 0.26 m and under a plate, Angkor's moat
left out because at 0.35 studs/m it is more baseplate than temple.

**Ship the sites the vocabulary fails at, and say so.** Sydney is three domes
of the right size in the right places and is not the Sydney Opera House —
Utzon's shells are leaning sections of one 75.2 m sphere and a `Dome` cap can
neither lean nor be a section of a sphere it does not contain. Leaving it out
would claim the Atlas can build everything it contains. Putting it in states
what it cannot, and costs a shot rather than a lie.

## Pictures

The gallery's exposure slider bottoms out at 0.4, and at that setting **Tan
(19) and White (15) blow out to pure light** under the bloom — the Taj Mahal
rendered as a single white blob while Menkaure in Reddish Brown stayed legible
in the same frame. It is the bloom, not the exposure. Light Bluish Gray (71)
is the safe stone. Watch for a blanket swap flattening two roles onto one
colour, as it did to the Acropolis's marble and rock.

`atlas-plate.mjs` drives the exposure control and dollies with real wheel
events; `bauchbinde.py` burns in the lower third — site, inscription year,
criteria, state party, all from the CC0 Wikidata snapshot and never from the
World Heritage Centre, whose terms forbid republication. A missing year prints
as a missing year.
