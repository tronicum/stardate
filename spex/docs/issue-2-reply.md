# Re: batch-render curated LDraw parts into a standalone static parts viewer

Short answer: yes to all three asks, and none of them is hard. But there is a
licensing layer under this that has to be settled first, and one of the
problems is **ours**, not yours. Taking that first, because it changes what
the batch tool has to do.

*Not legal advice — I'm not a lawyer, and neither is Stefan. What follows is
the published terms with sources, plus what we think they mean for this
specific use. Where something is genuinely unresolved I've said so rather
than picked an answer.*

---

## 1. spex itself is CC BY-NC-SA 4.0, and that is the binding constraint

This is the one that actually decides whether you can do what you're asking.
`export-static` ships a copy of the viewer's JS bundle into your output
directory, so embedding it means **redistributing spex**, not merely using
it.

**NonCommercial.** You describe the archive as science and non-profit, which
is almost certainly fine — but note the CC test is not "is the operator a
non-profit". It is whether the use is *"primarily intended for or directed
toward commercial advantage or monetary compensation"*
([CC BY-NC-SA 4.0 §1(l)](https://creativecommons.org/licenses/by-nc-sa/4.0/legalcode)).
A charitable foundation running ads would fail it; an unfunded hobby site
passes. If the site carries advertising, paid membership, or a commercial
sponsor's placement, ask Stefan for a separate grant rather than assuming.

**ShareAlike is the part people get wrong, in both directions.** It means the
viewer bundle you redistribute stays CC BY-NC-SA 4.0 and must say so. It does
**not** reach the rest of your site: putting our viewer on a page next to your
own content makes a *collection*, not an *adaptation*, and §3(b) only bites on
adaptations. Your catalogue text, your CSS, your Rebrickable-derived data —
untouched. If you *fork the viewer source* and ship your changes, that fork is
an adaptation and does have to carry the licence.

Practically: a line in your colophon naming spex, linking the repo, naming
CC BY-NC-SA 4.0 and linking it, and saying whether you modified it.

**A note for Stefan rather than for you:** CC licences are not designed for
software, and Creative Commons
[say so themselves](https://creativecommons.org/faq/#can-i-apply-a-creative-commons-license-to-software).
CC BY-NC-SA on a Rust workspace plus a TS viewer will keep generating this
exact conversation. Worth considering a dual licence, or a written exception
for archival/scientific reuse.

## 2. The LDraw terms have moved, and our own metadata is stale

We have been writing `"LDraw Parts Library (ldraw.org), CCAL 2.0"` into every
`mesh.json`. That is **out of date**, and the correction matters for you
because you would be republishing it.

- The current Contributor Agreement (rev. 2024-06-06) releases work under
  **CC BY 4.0**, or CC0 at the author's option. The CCAL 2.0 agreement is
  marked *"Legacy CA … No longer being used."*
  ([contributor agreement](https://ldraw.org/ldraw-org-contributor-agreement))
- `CAreadme.txt` in the current library ZIP: *"'CCAL version 2.0' should be
  read to mean CC BY 2.0"*, and modifications dated on or after **2023-03-05**
  are CC BY 4.0. So the library is a **CC BY 2.0 / CC BY 4.0 mix, file by
  file**.
- `ldraw.org/legal-info` still shows the 2007 text and is superseded on these
  points. Don't cite it.

**Two things follow that a batch tool must handle.**

**a) Some parts are not redistributable at all.** The header spec permits
`0 !LICENSE Not redistributable : see NonCAreadme.txt`
([header spec](https://www.ldraw.org/article/398.html)). A batch job over a
curated list of 100–200 parts will eventually hit one. spex does not read
`!LICENSE` today — it is not in the parser at all — so it would silently
convert and publish a file whose own header forbids it. **We'll fix that**;
see §5.

**b) Your renders are fine; our tilesets are not obviously fine.** LDraw draws
the line at 2D:

> *"What is not considered a Derivative work? — Rendered images generated from
> the LDraw library. Rendering here covers any conversion of a 3D model file
> into a 2D image."*

So a screenshot carries no obligation. But a **3D→3D** conversion does — the
published example is POV-Ray files, and *"Alternative libraries of parts …
converted from the LDraw Parts Library … must be considered a Derivative
Work."* A spex tileset or mesh bundle is exactly that: converted 3D data you
would be shipping as files.

**Flagged as genuinely unresolved:** LDraw's published text never names mesh,
glTF, OBJ or point-cloud formats. POV-Ray is the only 3D→3D example given.
The general rule reads onto ours, and we're treating it as derivative and
attributing, which is the safe reading — but LDraw has not said so, and if
this matters to the archive it is worth a mail to the SteerCo.

Attribution required is five items: name the creator, URL for the work, name
the licence, URL for the licence, and **note that it has been modified** —
which converting to a tileset plainly is.

## 3. Rebrickable: no licence, and that cuts both ways

The whole published grant is:

> *"All downloads on this page are free for use for any purposes, as long as
> it's awesome. If you do make use of them for public consumption, a mention
> of Rebrickable would be nice."* ([downloads](https://rebrickable.com/downloads/))

and, for the API, *"may be used for any purpose, including commercial"*
([terms](https://rebrickable.com/terms/)).

Attribution is **not required** — "would be nice", "highly appreciated". Credit
them anyway; it costs a line.

**The gap:** the wording grants *use*, and says nothing about **redistribution**
of the CSVs or of derived datasets. If you are only using the catalogue to
*choose* which 200 parts to render, that is plainly use and there is no issue.
If you intend to publish a derived dataset alongside the viewer, that is not
covered by anything they've published, and a short mail to Rebrickable is
cheaper than guessing. (Separately: MOCs and MOC images *are* restricted —
"may NOT be re-used on other sites without explicit permission from the MOC
designer". Catalogue parts are not MOCs, so this shouldn't touch you.)

## 4. The trademark

LEGO's [Fair Play](https://www.lego.com/en-us/legal/notices-and-policies/fair-play)
page requires, for non-commercial sites referring to their products:

> LEGO® is a trademark of the LEGO Group of companies which does not sponsor,
> authorize or endorse this site

Plus: never the logo; the mark as an adjective, never a noun and never
pluralised; same typeface as surrounding text, not emphasised; `®` on every
use; not in a domain name. Their own caveat is worth quoting too — *"a
disclaimer will not serve to undo an improper trademark use."*

Note the Fair Play page says **nothing about 3D models, CAD files or rendered
geometry**. It is written about words and photographs. That gap is real and I
can't close it for you.

spex's own house rule (`BRICKs.md`) is that the mark never appears in code,
commands, filenames or on-screen text at all — we say *Klemmbaustein*. You may
find that easier than getting the `®` right in fifty places.

---

## 5. What we're changing on our side

Three defects, all found while writing this:

1. **`tileset.json` carries no attribution whatsoever.** The mesh path writes
   an `attribution` block; the point-cloud path — `brick-part`, which is
   exactly what your batch tool would call — writes none. So today spex would
   hand you LDraw-derived 3D data with no credit in it.
2. **The attribution string that does exist says `CCAL 2.0`**, which is legacy
   wording for CC BY 2.0 and wrong for anything touched since 2023-03-05.
   It should name the per-file licence and carry the licence URL and the
   modification note.
3. **`!LICENSE` is not parsed at all**, so `Not redistributable` parts are
   converted silently.

Until (3) lands, if you batch-render from a curated list, `grep -L "!LICENSE"`
your inputs and eyeball anything that says `Not redistributable`. It is a
small set.

## 6. The three feature asks

- **Batch mode** — reasonable, and (3) above should land inside it rather than
  beside it: a batch job that skips a non-redistributable part and *tells you
  which* is strictly better than one that needs a separate lint.
  Skip-and-report on a missing part, non-zero exit only if nothing rendered.
- **`export-static` metadata** — today it derives the card title from the demo
  directory name and there are no per-demo thumbnails; `render_gallery_html()`
  is `pub` precisely so you can render your own index instead. Per-demo title
  and thumbnail is a small, welcome addition.
- **Prebuilt binaries** — none published today. `cargo build --release` is the
  only route. Worth doing, and it's a release-workflow question rather than a
  code one.

One thing worth saying plainly: none of §§1–4 is a reason not to do this. They
are four short paragraphs in a colophon and one lint in a batch tool.
