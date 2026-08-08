# M65 — four defects the numbers could not see

**8 August 2026.** Dissolve, materialise, and the point↔mesh crossfade.
Status block: [`phase2-show.md`](../fugen/phase2-show.md). Screenshots:
[`m65-dissolve-062.png`](../fugen/screenshots/m65-dissolve-062.png),
[`m65-crossfade-mid.png`](../fugen/screenshots/m65-crossfade-mid.png).

---

## What it was for

Act I opens on a single point that becomes a solid brick. Act IV ends with
every brick on screen becoming points again. Those are the two ends of the
piece and they are the same gesture run in opposite directions, so the engine
has to be able to cross between its own two render modes *on screen* — not cut
between them, not fade to black and back. This milestone is that crossing, plus
the erosion that carries objects out of a shot.

None of that is hard. What follows is not about the difficulty.

## The shape of the thing

Every one of the four defects had the same shape, and it took me until the
third to see it:

> **Something that draws the object, other than its lit surface, did not know
> about the effect.**

Each time, the numbers passed. Each time, the picture was wrong. And each time
I had already written the measurement that would have caught it — I just had
not pointed it at the right thing.

### One. The outline survived its own object

The first full run of `dissolve.mjs` reported the object going from 17 309 lit
pixels to 13 637 across a three-second dissolve. A 21 % reduction, described in
the log as a dissolve. Every intermediate frame was smooth. No console errors.

The screenshot at full dissolve showed **a perfect wireframe of the monolith
hanging in space**, every stud and tube outlined, with no surface anywhere
inside it.

M57's edge pass is its own `ShaderMaterial` with its own instance-matrix
texture. It had never heard of `aDissolve`. And the picture it produced was
not ugly — it was *striking*, an X-ray of a brick stack — which is exactly why
it would have survived a casual look. It is a good image and it is not this
shot's image.

### Two. Then the shadow survived

Edges fixed. Now 5 591 pixels remained at full dissolve, and one column of the
report had gone **negative**: `meanDelta -6.6`. The object was making the frame
*darker* than the empty scene.

That number cannot mean anything else. A shadow of nothing. three.js renders
shadow maps with its own depth material, which also had never heard of
`aDissolve`, so a fully eroded brick went on occluding the light exactly as if
it were solid.

I want to be clear about the sequencing here, because it is the point of the
entry: **I only saw this because I had already been forced to look at a
picture once.** Had the edges been correct from the start, the run would have
reported "17 309 → 5 591, smooth, no errors" and I would have moved on.

### Three. And then they eroded at different rates

Both fixed. Full dissolve now reaches zero. The halfway frame showed bricks
turned into **wire cages** — surface almost entirely gone, outline barely
touched.

The cause is a distribution, not a bug in the ordinary sense. The surface
thresholds against smoothed two-octave value noise, which is concentrated
around 0.5. The edge pass thresholded against a uniform per-edge hash, which is
flat on [0,1). Both are perfectly good random thresholds. At threshold 0.56 the
first has lost most of its mass and the second has lost 56 % of its edges, and
the object comes apart in the wrong order.

The fix was to export the noise from one module and have all three shaders —
surface, shadow, outline — sample the same field. Not because sharing is tidy,
but because *two implementations of a hash function is the most reliable way
for "the same fragments" to stop being true.*

### Four. The crossfade was off by a third of the object, and it was bloom

Different harness, same day. At crossfade value 0.5 the point cloud's screen
box came out **36 % narrower and 30 % shorter** than the mesh's, offset by a
third of the object's width. That is not noise. It looks exactly like a
coordinate-frame bug — a missing mirror, a scale, a transposed matrix — and I
spent a while looking for one.

It was **bloom**. A lit brick's specular blooms several pixels past its own
silhouette. A point cloud at the same nominal opacity is far dimmer per pixel
and barely blooms at all. The measurement was comparing **two glows, not two
shapes.** With bloom disabled for the box pass the error fell to 4 %.

Three smaller confounds fell out of the same investigation, and all three are
worth writing down:

- The mesh's box included **the shadow it casts**; the cloud casts none.
- `renderer.shadowMap.enabled = false` changed the numbers by **exactly zero**,
  because it needs a material recompile — which is the identical silent no-op
  M58's `--no-shadows` flag already produced once. The fix is to hide the
  *ground* instead: there is no shadow if there is nothing for it to fall on,
  and nothing to forget.
- `gl_PointSize` assumed **metres** in a millimetre scene, so every point
  clamped at the 14 px ceiling and a 1×1 brick rendered as one solid red blob.

## What it changed

**The measurement harness is now three runs, not one.** The dissolve is
measured with motion blur off, the pictures are taken with it on, and the
blurred numbers are reported alongside rather than quietly excluded — because
a smear legitimately grows an object's footprint while the object shrinks, and
measuring a collapse through a blur is measuring the blur.

**Every box measurement renders the frame twice** — once with the subject and
once without — and counts only what differs. This was already M59's fix for the
dolly confound; M65 is the third milestone to need it. It should have been the
default from the start.

**And one acceptance criterion was rewritten rather than met.** AC2 asked that
the two representations' bounding boxes agree within 1 %. They cannot, and not
because they disagree: a filled silhouette and a *finite sample of a surface*
have different pixel statistics by construction. The outermost of ~1 200
samples lands a few pixels inside the true silhouette, and how far inside
depends on sampling density, not on alignment; drawn at their real size the
points instead stick out by their own radius. The two bracket the truth from
either side — −0.52 % / −3.03 % at real size, −1.04 % / −3.90 % at one pixel —
and the unbiased statistic, the centroid of the lit pixels, agrees to
**0.37 % / 2.18 %**.

The harness now asserts only against gross misalignment and *reports*
everything finer. Tuning the point size until a number came out under 1 %
would have been fitting the instrument to the answer, and the whole verification
doctrine of this project (`docs/agents/verification.md`, rev 2) exists because
eight of Phase 1's acceptance criteria were written before anyone had measured
the thing they were about.

## Two constants that came from looking, not from arithmetic

`POINT_RADIUS_MM` started at 0.35 and is 0.08. At 0.35 a brick's 1 261 points
overlap into a solid mass at close range — a picture of a red brick, not of a
swarm. A point has to read as a point.

And the spread is now **relative**, 1.6 × the part's own radius, because a flat
26 mm is a gentle loosening on a 200 mm monolith and throws an 8 mm brick clean
off the frame.

Neither of those is discoverable from a passing test.

## The line I want to keep

Rung 5 of the verification ladder — take a screenshot and *look at it* — was
made conditional in the rev-2 rewrite, on the evidence that across all of
Phase 1 it found three bugs and cost a minute a frame on a software
rasteriser. That was the right call and I would make it again.

M65 found four in one day, and every single one had passed the numbers first.

The rule stands as written: a screenshot is mandatory **when the change is
supposed to alter what a frame looks like**. What M65 adds is how far that
reaches. It is not only the render pass you wrote. It is the outline, the
shadow, the post chain, and the second representation of the same object — all
of which draw your change, and none of which you were thinking about when you
wrote it.
