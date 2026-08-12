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



# ------------------------------------------------------- Tier D: the next 50
#
# Chosen for one reason and stated as such: each is a piece of ARCHITECTURE
# made of discrete repeated modules, which is what this Atlas can honestly
# build. Geographically spread on purpose -- an Atlas of the world's heritage
# that is four fifths European is a claim about Europe, not about heritage.
#
# `worship?` marks a site that is or contains an active place of worship.
TIER_D = [

    ("Hadrian's Wall", "D", "Hadrian's Wall: Wall and milecastle, the same two primitives 117 km apart."),
    ('Tower of London', "D", 'Concentric curtain Walls around a keep; the White Tower is a Wall block with corner Columns.'),
    ('Bath', "D", 'The Royal Crescent is a Colonnade bent into an arc -- the one building on this list that is literally an arrangeOn arc.'),
    ('Blenheim Palace', "D", 'Baroque courtyard: Wall ranges, a Colonnade portico, Stair to the terrace.'),
    ('Palace of Westminster', "D", 'worship? -- no. Victoria Tower and the Elizabeth Tower as Columns, the river front as Wall.'),
    ('Edinburgh', "D", 'The New Town is a grid of terraces: Wall, repeated, which is what a Georgian terrace is.'),
    ('Grachtengordel', "D", 'The canal ring: gabled Wall fronts in a row, stepped courses for the gables.'),
    ('Rietveld Schr', "D", 'The Schroeder House is planes and lines -- Wall slabs meeting at edges. De Stijl is a modular grammar and this is the test of whether the kit can speak it.'),
    ('Van Nelle', "D", 'Curtain wall as Mosaic, floor slabs as Wall. A factory built as a system.'),
    ('Roskilde', "D", "worship? -- an active cathedral and Denmark's royal burial church. Brick Gothic: Wall and Arch."),
    ('Bryggen', "D", 'The Hanseatic wharf at Bergen: rows of gabled timber Wall fronts.'),
    ('Palace and Park of Versailles', "D", 'The garden front is 580 m of repeated bay: Wall plus Colonnade, at the largest repetition count on the list.'),
    ('Chartres', "D", "worship? -- an active cathedral. Two unmatched spires as Columns of different heights, which is the building's own signature."),
    ('Cité de Carcassonne', "D", '52 towers on a double curtain: Column on Wall, arrangeOn an arc.'),
    ('Arles', "D", "The Roman amphitheatre: Arch on Arch, the Colosseum's argument at a size the kit handles comfortably."),
    ('Le Havre', "D", "Perret's rebuilt city is a reinforced-concrete module repeated across a whole town centre. If any site on this list IS the thesis, it is this one."),
    ('Works of Antoni Gaud', "D", "The Sagrada Familia's towers are Columns; the rest of Gaudi is ruled surfaces the kit cannot express, and that is worth showing."),
    ('Toledo', "D", 'worship? -- the cathedral is active. Alcazar as a Wall block with four corner Columns.'),
    ('Escurial', "D", "worship? -- an active monastery. A grid of courtyards: Wall, at Herrera's deliberate monotony."),
    ('Alcobaça', "D", 'worship? -- an active monastery. Cistercian plan: Wall, Arch, Colonnade cloister.'),
    ('Aachen Cathedral', "D", 'worship? -- active. The Palatine Chapel is an octagon: Colonnade on an arrangeOn circle under a Dome.'),
    ('Würzburg', "D", 'worship? -- no. The Residenz is a Wall range with a Stair the building is famous for.'),
    ('Bamberg', "D", 'worship? -- the cathedral is active. A hill town of Wall fronts.'),
    ('Wartburg', "D", "Wall and Column on a ridge -- a castle is the primitive set's home ground."),
    ('Berlin Modernism Housing Estates', "D", "Taut's Hufeisensiedlung is one dwelling repeated 1 000 times around a horseshoe. Modular housing rendered by a modular system."),
    ('Museum Island', "D", "The Altes Museum's front is eighteen Ionic columns: one Colonnade, cited."),
    ('Regensburg', "D", 'worship? -- the cathedral is active. The Steinerne Bruecke is sixteen Arches, cited and countable.'),
    ('Fagus', "D", "Gropius's glass curtain wall as Mosaic on a Wall frame -- the building that starts modern architecture, and a grid."),
    ('Historic Centre of Vienna', "D", 'worship? -- the Stephansdom is active. The Ringstrasse blocks as Wall ranges.'),
    ('Semmering', "D", 'Sixteen viaducts and fifteen tunnels: Arch, in the Alps.'),
    ('Venice', "D", "The Doge's Palace is a Colonnade under a Wall -- the load-bearing inversion the building is famous for."),
    ('Florence', "D", "Brunelleschi's dome is an octagonal Dome on a Wall drum; the campanile is a Column."),
    ('Piazza del Duomo, Pisa', "D", 'worship? -- the cathedral is active. Four buildings, one of them leaning, all Colonnade and Arch.'),
    ('Siena', "D", "The Torre del Mangia is a Column on the Campo's Wall front."),
    ('Sassi', "D", 'Matera: rock-cut dwellings stacked as Wall on Wall, the oldest continuously inhabited modular city on the list.'),
    ('Diocletian', "D", 'Split: a Roman palace with a town grown inside it -- Colonnade, Arch, Wall.'),
    ('Dubrovnik', "D", "A complete circuit of Wall with towers. The primitive set's purest single-site demonstration after the Pont du Gard."),
    ('Imperial Palaces of the Ming', "D", 'The Forbidden City: courtyard after courtyard of Wall and Stair, 980 buildings on one axis.'),
    ('Temple of Heaven', "D", 'worship? -- ceremonial rather than congregational. The Hall of Prayer is three Domes on a three-tier Stair terrace.'),
    ('Ancient City of Ping Yao', "D", '6 km of Ming city Wall with 72 watchtowers, cited and countable.'),
    ('Buddhist Monuments in the Horyu-ji', "D", 'worship? -- active. The five-storey pagoda is five Wall boxes with stepped roofs.'),
    ('Historic Monuments of Ancient Kyoto', "D", 'worship? -- active. Kinkaku-ji is three storeys of Wall on a plinth.'),
    ('Shirakawa-g', "D", 'Gassho-zukuri farmhouses: a steep roof as stepped Wall courses, repeated across a village.'),
    ('Hwaseong Fortress', "D", 'Suwon: 5.7 km of Wall with cited gates and bastions.'),
    ('Fatehpur Sikri', "D", 'worship? -- the Jama Masjid is active. A whole city of red sandstone Colonnade and Wall, abandoned complete.'),
    ('Qutb complex', "D", "worship? -- the mosque is a ruin. A 72.5 m fluted Column, tapering -- the kit's tallest single primitive."),
    ('Red Fort', "D", 'Wall, 2.4 km of it, with the Diwan-i-Am as Colonnade.'),
    ('Group of Monuments at Mahabalipuram', "D", 'The Five Rathas are monolithic temples, each a stepped Wall mass.'),
    ('Ayutthaya Historical Park', "D", 'Prangs as stepped Columns on Wall platforms.'),
    ('Complex of H', "D", 'worship? -- imperial rather than congregational. Hue: a walled citadel with gates.'),
    ('Shibam', "D", 'Mud-brick towers up to eight storeys -- the oldest skyscraper city, and literally built of bricks.'),
    ('Aït Benhaddou', "D", 'Earthen kasbah: Wall towers stacked up a hillside.'),
    ('Medina of Marrakesh', "D", 'worship? -- the Koutoubia is active. The minaret is a Column of cited height.'),
    ('Old Towns of Djenn', "D", 'worship? -- the Great Mosque is active and the largest mud-brick building in the world.'),
    ('Monolithic churches in Lalibela', "D", 'worship? -- active. Churches cut DOWN into rock: the kit builds solids, not voids, and this is the site that says so.'),
    ('Aksum', "D", 'Stelae as Columns, the tallest 33 m and fallen.'),
    ('Baalbek', "D", "The Temple of Jupiter's six standing Columns, 20 m tall, cited."),
    ('Teotihuacan', "D", 'The Pyramid of the Sun and the Moon on the Avenue of the Dead: Pyramid, twice, on an axis.'),
    ('Historic Centre of Mexico City', "D", 'worship? -- the cathedral is active. Built on Tenochtitlan, which is the argument.'),
    ('Pre-Hispanic Town of Uxmal', "D", 'The Pyramid of the Magician is elliptical in plan -- the second site to ask spex-build for an ellipse.'),
    ('Tikal', "D", 'Temple I as a steep stepped Pyramid with a roof comb.'),
    ('Old Havana', "D", 'Colonial Wall fronts and Colonnade arcades, a whole quarter of them.'),
    ('Historic Center of Quito', "D", 'worship? -- active churches. A grid on a hillside: Wall and Stair.'),
    ('Historic Town of Ouro Preto', "D", 'Baroque hill town: Wall fronts and two churches.'),
    ('Independence Hall', "D", 'A brick Wall front with a Column steeple -- and the building where a constitution was argued, which this piece has views about.'),
    ('Cahokia', "D", 'Monks Mound is the largest earthwork in the Americas: a four-terrace Pyramid of earth.'),
    ('Taos Pueblo', "D", 'Adobe dwellings stacked five storeys -- Wall on Wall, continuously inhabited for a thousand years.'),
    ('Old Quebec', "D", 'Quebec: a walled town, Wall and Column, the only one north of Mexico.'),
]

BUILDABLE = BUILDABLE + TIER_D

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
