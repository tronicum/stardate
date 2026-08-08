# M66 — the first time anyone watched it

**8 August 2026.** `spex show`, the eight URL parameters, the HUD — and the
first end-to-end performance of Act I.

Status block: [`phase2-show.md`](../fugen/phase2-show.md). How to run one:
[`screening.md`](../fugen/screening.md). Screenshots:
[`m66-s01-point.png`](../fugen/screenshots/m66-s01-point.png),
[`m66-s02-swarm.png`](../fugen/screenshots/m66-s02-swarm.png),
[`m66-s05-monolith.png`](../fugen/screenshots/m66-s05-monolith.png).

---

## The thing this milestone actually did

Every milestone from M60 to M65 built a part of a machine nobody had run. The
format, the resolver, the evaluator, the camera, the choreography, the
dissolve — each verified, each with a passing status block, none of them ever
asked to produce the piece. M66 wired them together and pressed play.

Eight defects came out. Not one of them was in the part that milestone had
built. Every single one lived in a seam.

That is worth saying plainly, because it is the whole content of the day:
**components that are individually correct compose into something that is
not**, and no amount of per-milestone verification finds that. Only running it
does.

## Two of them were in the screenplay, not the code

The dissolve tracks were **inverted — all three**. A1-S04 dissolved the
monolith it was supposed to be assembling. A1-S06's Stonehenge "rose from the
ground" by vanishing.

The screenplay was written before the dissolve had a sign. M65 decided
0 = solid, 1 = gone, and did so months after the document was authored; the
document had assumed the opposite and there was no reason for anyone to
notice, because a dissolve track that says `1` is a perfectly valid dissolve
track. The resolver accepted it. The schema validated it. The evaluator
evaluated it. Six shots, every number correct, and the object on screen was the
wrong one.

There is no test for this that is not a picture of the right object.

## Three were about *when*, not what

### A seek fired no generator

A1-S04's assembly is declared by a `seed` cue at the shot's first frame. Seek
to 0.2 seconds into that shot and the nine parts never fly: `fireCues` moves
its cursor past everything a jump skipped, deliberately, because arriving at
3:00 must not sound every accent of the first three minutes.

That reasoning is right and it is about **accents**. A `seed` cue is not an
event that happens at a moment. It is a statement that this shot has an
assembly in it — it is *state*. M62's own header already draws exactly this
distinction, about the endless cycle's seed advance: "a state change, not a
sequence of events with individual meaning". The distinction existed in prose
for four milestones before anything acted on it.

So the rule is by kind now. `seed` and `hud` are re-applied for whatever shot
you land in; `audio` and `marker` are not.

### Every frame looked like a seek

`SEEK_THRESHOLD_SEC` was 0.5 s, with a comment saying half a second is "well
past any real frame (even this container's software rasteriser manages
~4 fps)".

The first full screening ran at **2.1 fps**. 0.45 s a frame — under the
threshold, but only just, and any frame that went over it was silently
reclassified as a jump. The result was not a stutter or a crash. It was a piece
that fired **no cue at all**, and a director HUD that reported a four-voice
fugue as having no voices.

The number was measured once, against a machine, and then became an assumption
about all machines. The deeper repair is not the new number: a player that
seeks *calls* `resetCueCursor`, explicitly, so the heuristic was only ever
needed for the case nobody announces — a backgrounded tab, which jumps by
minutes. It was doing a job that had already been done, and doing it wrong.

### Pausing stopped the camera following the timeline

M63 gated the camera on `playing`, for a good reason: a camera that follows
both a timeline and a mouse follows neither.

But pause-and-seek is what *scrubbing* is. A paused player seeked to t=0 showed
the second shot's framing while the HUD read `A1-S01, 0.00 s`. Every number on
screen agreed with every other number on screen and the picture was of a
different shot.

"Is it playing" was the wrong question. The right one is **"is show time
moving"** — playback or a seek, either way the timeline owns the camera; the
mouse wins only when time is standing still. Which, incidentally, is what
`?free=1` was always doing correctly, in the branch right above it.

## One was a crash that could not happen at t=0

`InstanceGroup.mesh.material` is an **array** — one entry per submesh, because
a part can carry a moulded accent colour beside the instance's own colour. I
handed the array to `DissolveController`, which reads `material.userData`, and
an array has none.

It threw on the first frame in which any scene was visible. Act I opens with
**no scene on screen at all** — A1-S01 is two bars of black and one HUD pixel —
so the opening frame rendered perfectly, the harness reported success, and the
exception arrived six seconds later, in a rAF callback, where it looked like
noise.

## And two were about light

These are the two I would not have found without looking, and they are the two
I am most glad about.

### The opening frame was not black

The screenplay's first word of direction is *Black*. What rendered was a
mid-grey slab across the bottom half of the screen.

The obvious suspect is the albedo, and the albedo is innocent: the palette's
`grundUnten` is linear 0.0015. The next suspect is the environment, so I set
`envMapIntensity = 0` — and nothing changed, which with `scene.environment`
rather than a per-material `envMap` is apparently what that uniform does.

Then I measured, twice, the way M59 taught this project to: render the frame
with the ground and without it, and separately with the environment and
without. Hiding the ground took that pixel from sRGB **70 to 15**. Removing the
environment, ground still present, did the same.

**It was Fresnel.** Every dielectric in `MeshStandardMaterial` has an F0 of
0.04, and Fresnel takes that to nearly 1.0 at grazing incidence. A1-S01 looks
*along* a plane eight scene-diagonals wide from 4.8 mm above it. Almost every
ground pixel in that frame is a near-total mirror of M56's synthetic
environment, and the albedo never enters the arithmetic at all.

The fix was to stop pretending the plate is a surface. It exists to receive a
shadow; `ShadowMaterial` receives a shadow and draws nothing else. M54's
argument for a lit ground — "a brick with a contact shadow reads as an object"
— is an argument about previewing a model, and a screening is not a preview.

Measured after: the same pixel, with the ground and without it, (16,24,36) both
ways.

### The swarm was black points on a black background

A1-S02 is two bars of a swarm. It rendered nothing at all.

Everything was right. 1 261 points, opacity 1, visible, in the scene graph,
parent visible, spread 12.7 mm, the projection constant correct. And a point
is not lit — it has no normal that matters at one pixel and no shading model
behind it — so it draws at its own colour, and the brick is LDraw Black, linear
0.011, which against this piece's background is nothing.

M65 had tested the crossfade on a **red** brick.

The cloud now takes the material's `EDGE` value: the number LDConfig already
publishes for the question "how should this colour read when it is a line
rather than a surface", which is exactly a one-pixel point's problem, and the
same value M57's outline pass has been using since it was written. It is a real
number from the library rather than a fudge factor, and it keeps a Terrakotta
brick pouring Terrakotta.

The point-size floor went from 1 px to 2 in the same breath: at A1-S02's
viewing distance the physical size works out to 0.95 px, so every point clamped
to the floor, and a two-bar dolly had nothing visible in it.

## What I changed about how this gets checked

**The clean loop is now a pixel test, and it is not a test for equality.**
Render the frame at t=0, let the clock really wrap, render t=0 again, compare.
That catches the whole class of defect where a shot-scoped track leaves shared
state behind — A1-S06 raises the vignette and nothing at t=0 lowers it, because
the opening shot has no reason to mention a vignette.

But it cannot be a test for bit-equality, because `post.ts` dithers and grains
from wall-clock time **on purpose**. Two renders of identical state differ by
±1 everywhere by design. So the criterion is amplitude: the dither floor is 2,
a state leak is tens of levels. Before the fixes above it measured **76**. It
now measures **2**.

**And AC1 is two tests, not one, because two questions were hiding in it.**
Whether the arithmetic is right is a question about the evaluator and needs no
frames — 121 samples through the cut, milliseconds. Whether cues fire and the
piece loops is a question about *playback*, and there is no honest way to ask
it except by playing: a sweep of seeks fires no audio cue at all, by design, and
the first version of this harness duly reported four voices as none.

That last one is the whole day in miniature. The instrument was wrong, the
reading was clean, and the only reason I knew was that I already knew what the
answer had to be.
