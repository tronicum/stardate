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

**Open, and blocking:** the patent's own "Complete Accepted" date, from
Espacenet or Google Patents. Stefan is bringing it to the next session.
Premiere = that date + 100 years.

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

## Open, awaiting the next session

| # | Question | Who | Blocks |
|---|---|---|---|
| **D1a** | GB 263865's "Complete Accepted" date | Stefan — bringing it | The premiere date, and every 2027 date downstream of it |
| **D5** | Split the Fisher Page recognition ask from the patronage Postillen? | Stefan | Postillen texts, and their addressees |
| **D6** | Which three Atlas sites for the canonical cut — the thesis's three (a brick site, a stone-arch site, one non-European) or the Postilla states? | Stefan | M74's tier A, and the Atlas's on-screen reading |
| **D7** | Is the generated fugue musically alive? | Stefan, after the P0 spike | The whole audio phase, and its fallback |
| **D8** | Does the mesh renderer beat the point renderer on the monolith? | Stefan, after the P0 spike | Whether Phase 1 proceeds as specified |
