# Running a screening

**M66.** How to build, play, and hand on a performance of *Die Geschichtliche
Matrix* — and what every URL parameter does.

Part of [`docs/fugen/`](README.md). The milestone's own status block is in
[`phase2-show.md`](phase2-show.md); what went wrong while building it is in
[the diary](../diary/2026-08-08-m66-the-first-screening.md).

---

## The three commands

```sh
# 1. Compile the screenplay. --duration is repeatable: each value is one cut
#    of the same document, and they all share one bundles/ directory, because
#    the geometry does not change with the length — only the timeline does.
spex show-build shows/die-geschichtliche-matrix.show.json -o demos/matrix \
  --duration 240 --duration 120 --endless --skip-unbuildable

# 2. Play it.
spex show demos/matrix                       # opens a browser
spex show demos/matrix --cut endless --director   # same, with the HUD

# 3. Or write it out as plain files, no server involved.
spex show-export demos/matrix -o /tmp/matrix-static
```

A built show directory holds:

```
demos/matrix/
  cuts.json                     which cuts exist — what ?duration= chooses from
  show-resolved.json            the first cut (always this name, whatever its length)
  show-resolved-120.json        every further cut
  show-resolved-endless.json
  bundles/<scene>/              one mesh bundle per scene, shared by all cuts
```

`cuts.json` is written even when there is one cut, so a reader has one shape to
handle rather than two.

## The URL is the interface

A screening is reproducible if and only if you can write it down. Every
parameter below is optional, every one is also a `spex show` flag where it
makes sense, and nothing is silently ignored — a value that cannot be honoured
lands in the warnings, which the console prints and `?director=1` displays.

| Parameter | Meaning |
|---|---|
| `?t=<sec>` | Start at this show time. Clamped to the cut's length. |
| `?duration=<label>` | Which cut. The labels are whatever `cuts.json` holds — `240`, `600`, `3600`, `endless` are the four the piece is authored for. An unbuilt label plays the default cut **and says so**. |
| `?seed=<n>` | Edition seed, overriding the document's. Drives every runtime generator: on Act I that is A1-S04's assembly, whose nine parts start from different places. |
| `?quality=low\|medium\|high` | Skips the two-second benchmark and forces a tier. |
| `?mute=1` | Start with no `AudioContext`. |
| `?free=1` | The mouse drives the camera; the timeline keeps running. |
| `?loop=0` | Play once and hold the final frame, even on an `endless` cut. |
| `?director=1` | The director HUD: shot id and title, movement, tier, bar count, show time, fps, draw calls, instance count, the voices that have entered, the shot's own `note`, and any warnings. |

Two of these mean something less obvious than they look.

**`?mute=1` is not a volume control.** There is no sound until M71. What the
parameter really decides is **which clock the show reads**: with an
`AudioContext` the `ShowClock` runs on the audio hardware's own oscillator, so
a visual accent cannot drift against the sound it is an accent for; without
one it runs on `performance.now()`. `?director=1` reports which
(`clock audio` / `clock performance`).

There is a second reason it matters, and M66 found it the hard way: a browser
creates an `AudioContext` **suspended** until a user gesture, and a suspended
context's `currentTime` does not advance. A clock reading it does not tick. So
the player only adopts a context once it is actually running, re-anchoring show
time as it does — and until then, or forever in a headless session with no
audio device, the piece plays on `performance.now()`.

**`?free=1` and pausing are not the same thing.** Free hands the camera to the
mouse permanently; the timeline underneath keeps running, keeps firing cues,
keeps advancing. Pausing hands the camera over *only while show time is
standing still*: seek while paused and the camera follows the timeline to where
that shot is framed, which is what makes scrubbing a paused show legible.

## Static export

`spex show-export` writes:

```
<out>/
  index.html      with <meta name="spex-base" content="show">
  assets/         the viewer's own bundle
  show/           the show directory, verbatim
```

The meta tag is the whole trick. It tells the viewer where its data lives
relative to the document, so the same directory plays from a domain root and
from a project-pages subpath without being rebuilt — the same relative-path
discipline `export-static` already enforces for the gallery.

**`file://` does not work, and cannot.** The viewer is an ES module, and Chrome
refuses module scripts from `file://` under the same-origin rule (the document's
origin is `null`). This is a property of module scripts, not of this export:
every build of this viewer since M01 has had it. Any static web server does
work, including `python3 -m http.server` in the output directory. See
[`phase2-show.md`](phase2-show.md) for the acceptance criterion this rewrote.

## What the HUD can draw

The show's `hud` tracks and cues address elements by name. Four have real
layout and anything else becomes a generic card — deliberately, so that a
document from a later version of the piece degrades to "that card is plainer
than intended" rather than to a blank page.

| Element | What it is |
|---|---|
| `seed-point` | A1-S01's single Terminalgrün point at frame centre — **one device pixel**, which is why it is a HUD element and not geometry. The last frame of the piece must be identical to the first, and a projected millimetre is a different number of pixels on every screen. |
| `caption` | Lower centre. A1-S06's `c. 2500 BCE`. |
| `monolith-metrics` | Hairline, lower right. |
| `credits` | The crawl. Built here, driven by M84. |
| *(anything else)* | A card, lower left. M80's Atlas chronicle cards land here. |

Movement title cards are not a track: they are derived from the movement
boundaries the resolved document already carries. A track would mean the same
four keyframes copied into every act, which is four numbers per act that can
disagree with where the act actually starts.
