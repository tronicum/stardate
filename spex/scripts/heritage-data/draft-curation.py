#!/usr/bin/env python3
"""Drafts `curation.json` by matching a written candidate list against the snapshot.

    python3 scripts/heritage-data/draft-curation.py \
        scripts/heritage-data/wikidata-whs-2026-08-12.json \
        -o scripts/heritage-data/curation.draft.json

This script does NOT decide anything. Every entry below was written by hand;
all the script does is resolve each one to a QID against the real snapshot and
report which candidates it could not find, so that a human review is a review
of text rather than of a fuzzy-matching algorithm's confidence.

The output is deliberately named `.draft.json`. `spex heritage-list --buildable`
reads `curation.json`, and the step between the two files is a person reading
every line. The exclusion list in particular is an ethical constraint on the
work and is not delegable — see `crates/spex-heritage/src/curation.rs`.
"""
import argparse
import json
import pathlib
import sys
import unicodedata

# ---------------------------------------------------------------- exclusions
#
# Sites of atrocity, genocide, slavery and mass death are not rendered as toy
# bricks. The first five are named in `phase4-kit.md`; the rest were found by
# reading the snapshot, and finding them is the reason the list is drafted
# against real data rather than typed from the spec.
EXCLUSIONS = [
    ("Auschwitz", "atrocity",
     "The German Nazi concentration and extermination camp. A site of industrialised mass murder is not a subject for toy bricks, under any framing."),
    ("Hiroshima Peace Memorial", "atrocity",
     "The ruin left by the atomic bombing, preserved as it stood. The building is a record of the deaths beneath it."),
    ("Gorée", "atrocity",
     "A centre of the Atlantic slave trade, and a memorial to it."),
    ("Robben Island", "atrocity",
     "A political prison; the site is inscribed for what was done to the people held there."),
    ("Bikini Atoll", "atrocity",
     "A nuclear test site: the population was displaced and the atoll is still uninhabitable."),
    ("Memorial sites of the Genocide", "atrocity",
     "The Rwandan genocide memorials at Nyamata, Murambi, Gisozi and Bisesero. Not named in the spec; found in the snapshot, and it belongs here for exactly the same reason Auschwitz does."),
    ("Murambi Genocide Memorial", "atrocity",
     "A second Rwandan record in the snapshot; excluded on its own QID so a merge or split upstream cannot quietly re-admit it."),
    ("Cambodian Memorial Sites", "atrocity",
     "Centres of repression under the Khmer Rouge. Not named in the spec; found in the snapshot."),
]

# ------------------------------------------------------------- buildable set
#
# (name fragment, tier, justification). The justification says what the
# existing primitives would actually build -- Wall, Column, Arch, Colonnade,
# Dome, Stair, Mosaic -- because a site that needs an eighth primitive is a
# site this Atlas cannot honestly claim.
#
# `worship?` in a justification marks a site that is or contains an active
# place of worship. Those are excluded by default and need Stefan's explicit,
# recorded decision; they are drafted here so the decision has something to be
# about, and `review.md` lists them separately.
BUILDABLE = [
    # -- Tier A: the three Postilla addressee states -------------------------
    ("Stonehenge", "A", "Trilithons and a continuous lintel ring: Column plus a lintel course. Already built as recipes/stonehenge.json, 28 090 instances from cited stone dimensions."),
    ("Grand-Place", "A", "A closed square of guildhall facades: Wall for the elevations, Colonnade at ground level, Stair to the Town Hall. Gables are stepped Wall courses."),
    ("Jelling", "A", "Two barrows as Dome, two runestones as small blocks, one church as Wall plus roof. The smallest Tier A site and the clearest test of the vocabulary."),
    # -- Tier B: the nine of the ten-minute cut ------------------------------
    ("Great Wall", "B", "Wall and Stair, repeated -- the primitive is the site's name. Watchtowers are Column plus Wall."),
    ("Memphis and its Necropolis", "B", "The Giza pyramids: the Pyramid primitive, stepped in whole courses, which is what a brick pyramid honestly is."),
    ("Historic Centre of Rome", "B", "The Colosseum is Arch on Colonnade, four storeys, which is precisely what the building is. Already partly built as recipes/rom.json."),
    ("Acropolis", "B", "The Parthenon is Colonnade plus Stair plus a pediment of stepped Wall courses."),
    ("Taj Mahal", "B", "Dome on a plinth, four Columns as minarets, Arch iwans on each face."),
    ("Machu Picchu", "B", "Terraces as Stair, buildings as Wall. worship? -- the Intihuatana is a ritual stone, not an active place of worship."),
    ("Borobudur", "B", "Stepped terraces (Stair) with stupas (Dome). worship? -- an active Buddhist pilgrimage site."),
    ("Cologne Cathedral", "B", "Twin towers as Column, nave as Wall, the whole thing as Arch. worship? -- an active cathedral."),
    ("Pont du Gard", "B", "Three tiers of Arch. Nothing else. The cleanest possible demonstration of one primitive."),
    # -- Tier C: candidates from the spec, resolved against the snapshot -----
    ("Petra", "C", "Rock-cut facades as Colonnade and Arch against a Wall face."),
    ("Angkor", "C", "Towers as stepped Column, galleries as Colonnade, causeways as Wall. worship? -- parts are in active Buddhist use."),
    ("Chichen Itza", "C", "El Castillo is the Pyramid primitive with Stair on all four faces."),
    ("Persepolis", "C", "The Apadana is Colonnade on a Stair-approached terrace."),
    ("Mesa Verde", "C", "Cliff Palace as stacked Wall and Column under an overhang."),
    ("Bauhaus", "C", "Wall and glazing grid as Mosaic. A modular building rendered by a modular system, which is the thesis stated twice."),
    ("Zollverein", "C", "Shaft 12's headframe as Column and truss, halls as Wall."),
    ("Speicherstadt", "C", "Brick warehouses: Wall and Arch, and the single most literally brick-built site on the list."),
    ("Kronborg", "C", "Bastioned castle: Wall, Column, Stair. Hamlet's Elsinore, and Danish, which the Postillen make relevant."),
    ("Belfries", "C", "The Bruges belfry as a stepped Column. Belgian, likewise."),
    ("Wieliczka", "C", "Timbered chambers as Wall and Column. worship? -- the Chapel of St Kinga holds services."),
    ("Alhambra", "C", "Courts as Colonnade, the Court of the Lions as Column repeated, Arch throughout."),
    ("Mont-Saint-Michel", "C", "The abbey as Wall on a Stair-terraced rock. worship? -- an active monastic community."),
    ("Sydney Opera House", "C", "The shells are the one site here the vocabulary genuinely strains at -- a Dome is a hemisphere and these are not. Drafted as a candidate for exactly that reason: if it cannot be built honestly it should be dropped and said so."),
    ("Brasilia", "C", "Niemeyer's Congress: Dome, inverted Dome, twin Wall slabs."),
    ("Rapa Nui", "C", "Moai as Column with a pukao course. Ahu platforms as Wall."),
    ("Timbuktu", "C", "Djinguereber and Sankore as Wall with projecting toron. worship? -- active mosques."),
    ("Bagan", "C", "Stupas as Dome on stepped Stair terraces, repeated at scale. worship? -- active."),
    ("Himeji Castle", "C", "The keep as stacked Wall with tiered roofs; roofs as stepped courses."),
    ("Vatican City", "C", "Bernini's colonnade is Colonnade, four columns deep, which is the primitive's definition. worship? -- the entire state."),
    ("Old Town of Segovia", "C", "The aqueduct is Arch on Arch, 167 of them."),
    ("Ironbridge", "C", "The bridge as Arch in cast iron -- the first of its kind, and the industrial rhyme to Act III."),
    ("Volklingen", "C", "Blast furnaces as Column and Stair. worship? -- no."),
    ("Sigiriya", "C", "The rock as a mass, the water gardens as Mosaic, the lion staircase as Stair."),
    ("Island of Meroe", "C", "Nubian pyramids: the Pyramid primitive, steeper than Giza's, repeated across a field."),
    ("Great Zimbabwe", "C", "Dry-stone enclosures: Wall, in the most literal sense on this list -- coursed, mortarless, modular."),
    ("Fujian Tulou", "C", "Circular earthen dwellings: Wall bent into a ring, which is a real test of whether the primitive can curve."),
    ("Citadel, Ancient City and Fortress Buildings of Derbent", "C", "Wall running from the citadel to the sea."),
]


def fold(s):
    """Accent- and case-insensitive, so 'Goree' finds 'Gorée'."""
    return "".join(
        c for c in unicodedata.normalize("NFKD", s.lower()) if not unicodedata.combining(c)
    )


def find(sites, fragment):
    """Every site whose name contains the fragment. Never picks for you."""
    f = fold(fragment)
    return [s for s in sites if f in fold(s["name"])]


def resolve(sites, entries, kind):
    out, missing, ambiguous = [], [], []
    for entry in entries:
        fragment = entry[0]
        hits = find(sites, fragment)
        if not hits:
            missing.append(fragment)
            continue
        # Prefer an exact fold match; otherwise the shortest name, which for
        # this list is reliably the site rather than a component of it.
        exact = [h for h in hits if fold(h["name"]) == fold(fragment)]
        chosen = exact[0] if exact else min(hits, key=lambda h: len(h["name"]))
        if len(hits) > 1 and not exact:
            ambiguous.append((fragment, [h["name"] for h in hits]))
        out.append((chosen, entry))
    print(f"{kind}: {len(out)} resolved, {len(missing)} not found", file=sys.stderr)
    for m in missing:
        print(f"  NOT FOUND  {m}", file=sys.stderr)
    for frag, names in ambiguous:
        print(f"  ambiguous  {frag!r} -> chose {names[0]!r} of {len(names)}", file=sys.stderr)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("snapshot")
    ap.add_argument("-o", "--out", required=True)
    ap.add_argument("--reviewer", default="UNREVIEWED — a draft, not a curation")
    args = ap.parse_args()

    sites = json.loads(pathlib.Path(args.snapshot).read_text())["sites"]
    print(f"snapshot: {len(sites)} sites", file=sys.stderr)

    excluded = [
        {"id": site["id"], "name": site["name"], "reason": reason, "note": note}
        for site, (_, reason, note) in resolve(sites, EXCLUSIONS, "exclusions")
    ]
    buildable = [
        {"id": site["id"], "name": site["name"], "justification": just, "tier": tier}
        for site, (_, tier, just) in resolve(sites, BUILDABLE, "buildable")
    ]

    # A site cannot be both. If a fragment matched into the exclusion set as
    # well, the exclusion wins and the buildable entry goes -- but loudly.
    ex_ids = {e["id"] for e in excluded}
    clash = [b for b in buildable if b["id"] in ex_ids]
    for b in clash:
        print(f"  CLASH      {b['id']} {b['name']} is both; exclusion wins", file=sys.stderr)
    buildable = [b for b in buildable if b["id"] not in ex_ids]

    worship = [b for b in buildable if "worship?" in b["justification"]]
    print(
        f"\n{len(buildable)} buildable ({len(worship)} touch an active place of worship "
        f"and are excluded by default until explicitly decided), {len(excluded)} excluded",
        file=sys.stderr,
    )

    doc = {
        "version": 1,
        "reviewed": "",
        "reviewer": args.reviewer,
        "buildable": buildable,
        "excluded": excluded,
    }
    pathlib.Path(args.out).write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n")
    print(f"wrote {args.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
