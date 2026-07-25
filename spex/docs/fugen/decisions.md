# Decisions

*What was decided, when, by whom, and what it rules out. Append-only.*

**Part of [`docs/fugen/`](README.md) — the Fugen Engine implementation spec, rev 3.**

A decision recorded here is settled. Reopening one is allowed, but it happens
as a new entry that supersedes the old one — never as a silent change of mind
in some other file. The point is that a session six months from now can tell
the difference between "we chose this" and "nobody thought about it".

---

## D1 — The premiere is the centenary of GB 263865

**Date:** 2026-07-25 · **Decided by:** Stefan · **Status:** adopted, date pending

The work premieres on the hundredth anniversary of the acceptance of
**GB 263865** (Batima, Belgian priority 31 December 1925) — the specification
the archive's own research identifies as containing "eight studs in two rows",
the 2×4 canon, and calls *"das wichtigste Dokument"* of the whole chain.

The Belgian priority centenary (31 December 2025) has already passed, which is
why the British acceptance in 1927 is the right anchor: it is the centenary of
the *standard*, not of a filing.

### D1a — the date, resolved: **23 June 2027**

**Resolved:** 2026-07-25, from the patents' own front-page data (Google
Patents / Espacenet), not from a secondary source.

| Patent | Title | Applicant | Priority | Filed | **Published** | Centenary |
|---|---|---|---|---|---|---|
| **BE 311029** | — | — | **6 Jun 1923** | — | — | 2023 — passed |
| **GB 217243 A** | *Improvements in toy building blocks* | **J. Girlot (assignee of L. Cousin)** | 6 Jun 1923 | 6 Jun 1924 | **21 May 1925** | 2025 — passed |
| **GB 263865 A** | *Improvements in building blocks* | **J. Girlot** | 31 Dec 1925 | 31 Dec 1926 | **23 Jun 1927** | **23 Jun 2027** |

**The premiere is Wednesday, 23 June 2027** — 47 weeks from this spec, and
the hundredth anniversary of the publication of the 2×4 stud canon.

GB 263865's own abstract is the reason this is the right document and not
merely the conveniently dated one:

> "A set of toy bricks formed with two rows of pegs *a* on one face and
> corresponding rows of recesses *b* on the opposite face… the quarter brick
> having four pegs, the half brick six, and the full brick eight."

Eight pegs in two rows. That is the 2×4, in print, in 1927 — and the archive's
own note calling GB 263865 *"das wichtigste Dokument"* was right.

**A correction to the record, and a caution.** A search-engine AI summary
asserted that "the UK patent related to Batima is GB 217243 (**not**
GB 263865)". That exclusion is false: **both exist, both are Girlot, and they
are two different inventions.** GB 217243 (1923/25) is *solid* blocks in
carton pierre, interlocking on their upper and lower faces. GB 263865
(1925/27) is the pegs-and-recesses brick with eight studs in two rows. The
archive had already distinguished them correctly; the summary had not.

**A second caution, which matters more.** GB 217243's front page reads
"J. Girlot (assignee of **L. Cousin**)" — so Louis Cousin as the inventor of
the 1923 block rests on the patent record itself, not only on a Wikipedia
article this project may have written. That is a real relief, because citing
our own Wikipedia edits back to ourselves would be circular sourcing.

But **GB 263865 names Girlot alone.** The 2×4 canon patent does not name
Cousin. Any claim that Louis Cousin invented the 2×4 stud brick is therefore
an overclaim and must not appear in the work, the Postillen, or any Wikipedia
edit. The defensible statement is narrower and still strong:

> Louis Cousin's interlocking block is patented in 1923. The eight-stud,
> two-row brick is published under Joseph Girlot's name in 1927. Both are
> Batima. Neither is in Billund.

**Still to pull, for the archive rather than for the date:** the full
specification PDFs of both, and the "Complete Accepted" line from each front
page, to sit alongside `FR588985A` in `quellenregister.json`.

**What this rules out:** the 26-week calendar of rev 1 and the scope cut of
rev 2. Phase 6, M77, Atlas tier C and the 60:00 cut are back in
([`plan.md`](plan.md) §1).

---

## D2 — The three patent bricks are reconstructed, and declared as such

**Date:** 2026-07-25 · **Decided by:** Stefan · **Status:** adopted

Batima (1924), Kiddicraft (1940) and the 1949 brick have no LDraw geometry,
and the project's real-data rule forbids inventing geometry. Stefan's ruling:
**"Das Ganze ist ein Gesamtkunstwerk"** — the reconstruction is itself part of
the work, not a hole in its evidence.

**Therefore:** three `.dat` files are hand-authored from the real published
patent drawings — the archive already holds `FR588985A` at 900×602 and a CC0
photograph of real Batima bricks, which is enough to reconstruct responsibly.

**The condition that makes this honest, and it is not optional.** Each file
carries, in its own header: that it is a reconstruction by the Iunctura
Archiv, which drawing it was reconstructed from, and that it is **not** an
LDraw library part. The same statement appears in `PROVENANCE.md`, in the
credits, and in the shot's own lower third if the reconstruction is visible
at a scale where its detail could be mistaken for measurement.

The distinction being preserved: a **reconstruction openly declared** is a
legitimate artistic act with a citable source. A reconstruction presented as
a measured part would be a fabricated number, which is the one thing this
project does not do. The Gesamtkunstwerk argument licenses the first and not
the second.

**Consequence:** `ldraw-scenes/reconstructions/` as a separate directory, so
no reconstruction can ever be mistaken for a resolved library part by a tool
or by a reader. The real-data rule continues to govern everything the work
presents as *evidence*: dates, patent numbers, colours, dimensions, heritage
metadata.

---

## D3 — The Postillen move to 2027

**Date:** 2026-07-25 · **Decided by:** Stefan · **Status:** adopted

Sealed letters that arrive in the centenary year, and refer to the centenary,
are a stronger object than letters arriving in October 2026 about a work that
does not yet exist.

**Consequences to schedule backwards from the premiere date (D1):** printing
on Bütten, wax seals and numbering, translation into each recipient's own
language, and diplomatic-post lead times. `masterplan-bewegung-postillen.md`'s
Phase A–C timeline needs rebasing once D1's date is known.

**What this frees:** October 2026 is no longer a delivery date. It becomes an
honest **Act I technical preview** — the vector-accurate monolith running in a
browser — which resolves the collision between this plan and
`masterplan-iunctura-site.md`.

---

## D4 — The protocol is announced in advance, by email

**Date:** 2026-07-25 · **Decided by:** Stefan · **Status:** provisional — confirm before anything is sent

Two reviewers, independently, found that the Postillen as drafted read as a
petition rather than as conceptual art, and both identified the same cause:
**a work whose success criterion is institutional approval is a petition by
definition.** Their shared fix was to declare, publicly and in advance, that
the *replies* — including the absence of replies — are the work.

Stefan's ruling: **announce the protocol in advance by email.**

**The shape that follows, to be confirmed:**

1. The protocol is published on the archive's own site, dated, before
   anything is sent: *sent on date X; every response and every non-response
   published verbatim at X+90; the archive of the answers is the work.*
2. Each recipient's office receives that protocol **by email first**, so no
   one is documented without having been told how the documentation works.
3. The sealed physical Postilla follows.
4. At X+90 the archive publishes what came back, and what did not.

**Why this is the right shape and not merely a softer one.** Emailing the
protocol in advance is what makes silence *measurable*: a non-response to a
letter is ambiguous, but a non-response to a stated, dated, pre-announced
protocol is a result. It also means nobody is surprised by their own
appearance in the work, which is the difference between documentation and
ambush.

**The cost, stated plainly:** pre-announcing may reduce the chance of a reply.
A palace secretariat that knows its silence will be published may answer more
carefully, or may answer not at all. That trade is acceptable *only* under
this framing, where silence is a finding rather than a failure — which is why
D4 and the deletion below travel together.

**Two things this decision deletes**, from
`masterplan-bewegung-postillen.md`:

- **The success ladder.** It contradicts that document's own best line —
  *"Wir schreiben nicht, um Antworten zu bekommen."*
- **The Louis Cousin genealogy.** It fuses two different people: the
  Président de la Cour des Monnaies died in **1707** and cannot hold a 1923
  patent. (The court also became sovereign in 1552, not 1650.) It is the
  first thing a historian finds, and it discredits everything near it.

**And one separation**, recommended by both reviewers and not yet ruled on:
the **Fisher Page recognition** ask is modest, checkable and genuinely just,
and both the ODNB and the English Heritage blue plaque panel have real public
nomination processes. *Addressed to the panel it is a civic act; addressed to
the King it is the thing that makes the whole apparatus look unserious.*
Worth splitting off from the patronage letters — open for D5.

---

## D10 — Data acquisition is Stefan's lane, and out of scope for this spec

**Date:** 2026-07-25 · **Decided by:** Stefan · **Status:** adopted

> *"Korrigierte Daten kann ich legal besorgen. Out of scope."*

Sourcing the underlying data — patent full texts, the corrected attribution
record, heritage metadata, flag construction specifications — is Stefan's
responsibility and is obtained legally by him. **The spec stops caring how
data arrives.** That removes a class of work the engine plan had been quietly
carrying: chasing authorisations, negotiating terms, deciding what may be
fetched from where.

**What this takes off the plan:**

- The World Heritage Centre authorisation question leaves the human-gates
  table and the critical path. The pipeline reads whatever Stefan supplies;
  if that is a legally obtained WHC extract, it works, and if it is CC0
  Wikidata, it also works. No milestone waits on an institution.
- **D9 closes.** The Cousin/Girlot attribution correction is Stefan's to make.
  What the spec keeps is the *statement* it is allowed to render, fixed in
  [`screenplay.md`](screenplay.md) §5: Cousin's block is patented in 1923; the
  eight-stud two-row brick is published under Girlot's name in 1927. The
  hand-work itself is written up separately, in German, at
  [`../human-todo.md`](../human-todo.md).
- `spex heritage-index` / `heritage-sync` become **importers of a supplied
  snapshot**, not acquisition tools with their own terms-of-use reasoning.
  M73 and M77 shrink accordingly.

**What this explicitly does NOT take off the plan**, because it is a different
thing wearing similar clothes:

| Stays | Why |
|---|---|
| **Provenance** — every part, colour, site, flag and figure carries its source through to a generated `PROVENANCE.md` | This is a *build* property, not an acquisition question. A source that cannot be named cannot be rendered |
| **Attribution on screen** — LDraw CCAL, and whatever licence the supplied data carries | Governs what the work *publishes*, which is ours regardless of who fetched the input |
| **The reconstruction declaration** (D2) | Same reason: it is about what the frame claims, not where a file came from |
| **The exclusion list and the custodian/Indigenous/Danger clauses** | Editorial and ethical, not legal. Nobody's licence tells us whether to build Auschwitz out of bricks |
| **No UNESCO emblem, no brand name** | Marks, not data |

The line, stated once so it does not blur later: **acquisition is Stefan's;
publication is the work's.** The spec governs the second and assumes the first
is clean.

---

## D5 — What is demanded for Fisher Page: a foundation, staged

**Date:** 2026-07-25 · **Decided by:** Stefan · **Status:** adopted — confirm the wording before anything is sent

Variant **V-D** of [`d5-entscheidungsvorlage.md`](d5-entscheidungsvorlage.md).
Stefan's ruling that a plaque is too little turned out to be supported by the
record: Page's work was several years of field observation in nursery schools,
published as *Playtime in the First Five Years* (1938). The brick was an
output. **No plaque exists** — English Heritage lists none; the Croydon local
scheme is still to be checked.

1. **Postilla II (Charles III) loses the Page request entirely.** It keeps the
   cultural-history thesis and Stonehenge as its opening image. This is what
   stops the royal letter reading as a petition.
2. **ODNB suggestion and plaque nomination go out immediately**, on their own
   timeline, to the bodies that actually decide them — and stay **outside** the
   artwork. The submission is published; the answer is not pulled into D4's
   X+90 protocol. A nominating panel does not become an unwilling participant.
3. **Postilla IV (LEGO Group / LEGO Foundation) carries the real demand**, in
   this order: **a foundation for pedagogical play built on his writings —
   alternatively a permanent exhibition in the LEGO House.** This one stays
   *inside* the work and the protocol.

**Why this ask and not the plaque:** it is the only demand in the apparatus
that is constructive rather than retrospective — it requires nobody to admit
anything — and for the LEGO Foundation it is the thing they already do, in the
name of the man whose work they built on.

**Two constraints that are not optional.** The descendants sold the rights in
1981; they exist, and a foundation bearing his name is built *with* them. And
he died by suicide at 52 — recorded here so the apparatus knows not to use it:
a date, never a lever, never in causal proximity to Billund.

---

## D6 — The three Atlas sites: the joint itself

**Date:** 2026-07-25 · **Decided by:** Stefan · **Status:** adopted on a general approval — re-confirm before M74 authoring starts

Variant **V5** of [`d6-entscheidungsvorlage.md`](d6-entscheidungsvorlage.md):
**Stonehenge · Mohenjo-daro · a dougong hall (Foguang Si)** — three
civilisations answering one structural question, *how does something hold
together that is not glued*. Mortise and tenon in stone, brick bond in clay,
bracket sets in timber. Three joints, no mortar.

It is the only variant in which the Atlas argues instead of illustrating,
which matters for a work named after the joint. Two side effects decided it:
Stonehenge's return becomes a rhyme rather than a repeat (Act I shows it as an
object, the Atlas as a connection), and Mohenjo-daro's 1 : 2 : 4 brick ratio
rhymes with the monolith's own 1 : 4 : 9.

**Carried risk:** Foguang Si needs a `Bracket` primitive in M72. If that costs
more than three days, the third site falls back to a simpler dougong hall or
to Teotihuacán. The first two are unaffected.

**Honest note on how this was adopted:** confirmed by a general *"passt alles
erstmal"* rather than a variant-by-variant answer. If the choice turns out to
matter more than expected during M74, re-ask before authoring rather than
assuming.

---

## Open, awaiting the next session

| # | Question | Who | Blocks |
|---|---|---|---|
| ~~D9~~ | ~~Correct the live Wikipedia attribution record.~~ **Closed by D10** — Stefan's lane; the hand-work is written up at [`../human-todo.md`](../human-todo.md). The spec keeps only the wording it is allowed to render | — | — |
| **D7** | Is the generated fugue musically alive? **The spike is built and rendered** — `scripts/fugue-spike/`, 49.6 s, the exposition on bars 5/7/11/14 with a rule-checked, octave-invertible countersubject. Everything verifiable was verified: no parallel fifths or octaves, entries on their bars, every note in range. Whether it is *alive* is not measurable here | Stefan — 20 min with headphones | The whole audio phase, and whether the fallback (hand-composed subject and exposition) is triggered |
| **D8** | Does the mesh renderer beat the point renderer on the monolith? | Stefan, after the P0 spike | Whether Phase 1 proceeds as specified |
