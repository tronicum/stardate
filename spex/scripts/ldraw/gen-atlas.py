#!/usr/bin/env python3
"""M74 — turns cited site sheets into buildable Atlas recipes.

    python3 scripts/ldraw/gen-atlas.py                    # every sheet in heritage/
    python3 scripts/ldraw/gen-atlas.py --tier A           # one tier
    python3 scripts/ldraw/gen-atlas.py --site jelling     # one site

ONE GENERATOR, FORTY SITES
--------------------------
The alternative was forty scripts like `gen-stonehenge.py`, and forty scripts
is forty places for the metre-to-stud conversion to drift. So a site is a
*sheet* -- `heritage/<slug>.json` -- and this file is the only thing that turns
metres into studs and plates. Adding a site is a data edit.

The sheets are the same shape as `flags/<iso2>.json`, and for the same reason:
a declarative description of a real thing, with the document its numbers came
from written next to them.

EVERY DIMENSION CARRIES ITS SOURCE
----------------------------------
`dimensions` maps a name to `{"m": 110, "source": "..."}`. Massing parameters
name a dimension rather than repeating a number, so a figure appears once and
its citation cannot drift away from it. A parameter may also be a bare number
-- for proportions nobody publishes, like how thick to make a facade -- and
every one of those is COUNTED AND REPORTED. The Atlas's honesty is a number:
how many of its dimensions come from a source and how many are the modeller's
judgement.

`--strict` fails the build if any site's uncited count exceeds its own declared
`uncitedBudget`, so a sheet cannot quietly accumulate invented figures.

THE SCALE IS PER SITE, AND THAT IS THE POINT
--------------------------------------------
`phase4-kit.md` budgets <= 8 000 placements per site "so the 40-site Atlas
stays under ~250 000". A single studs-per-metre for all forty cannot do that:
the Great Wall is kilometres and Jelling's church is twenty metres. So each
sheet declares its own `studsPerMetre`, chosen so the site lands under budget,
and this script reports the count. Stonehenge is the one deliberate exception
and says so in its own recipe.
"""
import argparse
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
SHEETS = REPO / "heritage"
OUT = REPO / "recipes" / "heritage"

PLATES_PER_STUD = 20 / 8  # 1 stud = 20 LDU, 1 plate = 8 LDU

# Which massing parameters are lengths, and what unit each becomes.
STUD_PARAMS = {
    "widthM": "widthStuds", "depthM": "depthStuds", "diameterM": "diameterStuds",
    "spanM": "spanStuds", "runM": "runStuds", "baseM": "baseStuds",
    "radiusM": "radiusStuds", "spacingM": "spacingStuds", "gapM": "gapStuds",
    "setbackM": "setbackStuds",
}
PLATE_PARAMS = {
    "heightM": "heightPlates", "riseM": "risePlates", "tierHeightM": "tierHeightPlates",
    "postHeightM": "postHeightPlates", "capHeightM": "heightPlates",
}
PASSTHROUGH = {"bond", "color", "stepped", "architrave", "columns", "tiers", "count", "tilePart", "cells", "thicknessStuds"}

# A `Wall` builds in whole BRICK courses, so a wall under three plates emits
# nothing at all -- silently. The Grand-Place's paving was authored 0.35 m
# thick, became one plate, and produced zero placements: the plan came out as
# four ranges around an empty rectangle, and nothing anywhere said why. A
# ground plane is a tile layer and wants `Mosaic`, which is one plate by
# construction; this constant is here so the generator can say so out loud.
MIN_WALL_PLATES = 3

# Everything in this kit stacks in whole brick courses. A `Wall` authored 1.6 m
# tall at 1 stud/m is 4 plates, which the engine builds as TWO courses -- 6
# plates, 2.4 m. Authoring the next thing at 1.6 m then floats or overlaps it,
# and eight of the nine Tier B sites failed exactly this way.
#
# So a height is snapped up to a whole course here, once, and a sheet says
# `{"onTopOf": 3}` instead of repeating a number that was never the real one.
PLATES_PER_COURSE = 3


def snap_course(plates):
    """DOWN to a whole brick course, because that is what the engine does.

    `brick_courses` in `crates/spex-build/src/primitives.rs` is
    `total_plates / BRICK_PLATES` -- integer division, truncating. A wall
    authored 4 plates tall is ONE course, three plates, and the fourth plate
    is silently dropped.

    This rounded UP for one commit, and the Parthenon caught it: the stylobate
    was authored 1.6 m, became 4 plates, and the engine built 3 -- while
    `onTopOf` put the colonnade at 6. Three plates of air under every column,
    544 floating placements, and both roundings looked reasonable in isolation.
    A generator that disagrees with its engine about how tall a wall is will
    disagree about everything stacked on it.
    """
    if plates <= 0:
        return 0
    return (plates // PLATES_PER_COURSE) * PLATES_PER_COURSE


class Uncited(Exception):
    pass


class Sheet:
    def __init__(self, path):
        self.path = path
        self.d = json.loads(path.read_text())
        self.spm = self.d["studsPerMetre"]
        self.dims = self.d.get("dimensions", {})
        # Two counts, not one. A figure that says how big a thing is and a
        # figure that says where it stands are different kinds of claim: the
        # first is a fact about the monument, the second is a site plan. Mixing
        # them buries the number that matters -- Jelling's first run reported
        # 34 uncited figures, of which 25 were coordinates.
        self.uncited = []          # (where, value) -- sizes
        self.uncited_pos = []      # (where, value) -- positions
        self.used_dims = set()
        # Absolute top of each converted step, in plates above ground, so a
        # later step can stand on it without anybody doing the arithmetic by
        # hand and getting the course rounding wrong.
        self.tops = []
        # Heights the engine truncates to a whole course. Not an error -- a
        # brick is three plates and that is the material -- but an author who
        # is not told will keep authoring 1.6 m stylobates and wondering.
        self.rounded = []

    def metres(self, value, where, position=False):
        """A dimension name resolves to its cited value; a bare number is recorded.

        `{"dim": "southMoundDiameter", "x": 0.5}` is a cited dimension times a
        factor -- a radius from a diameter, a side from a perimeter. The factor
        is arithmetic on a cited figure and keeps the citation attached, which
        writing 35.0 would not.
        """
        if isinstance(value, dict):
            name = value["dim"]
            if name not in self.dims:
                raise Uncited(f"{self.path.name}: {where} names dimension {name!r}, which the sheet does not define")
            self.used_dims.add(name)
            return float(self.dims[name]["m"]) * float(value.get("x", 1.0))
        if isinstance(value, str):
            if value not in self.dims:
                raise Uncited(f"{self.path.name}: {where} names dimension {value!r}, which the sheet does not define")
            self.used_dims.add(value)
            return float(self.dims[value]["m"])
        (self.uncited_pos if position else self.uncited).append((where, float(value)))
        return float(value)

    # A SIZE and a POSITION convert differently, and conflating them was a real
    # defect: the first Grand-Place had `max(1, ...)` on both, so a facade at
    # -1.5 m landed at +1 stud instead of -2 and the whole west range was built
    # on top of the paving. 176 overlaps, all of them one clamp.
    #
    # A size clamps to one, because a wall zero studs wide is not a wall. A
    # position must not clamp at all, because half of any site plan is negative.
    def studs(self, m):
        return max(1, round(m * self.spm))

    def plates(self, m):
        return max(1, round(m * self.spm * PLATES_PER_STUD))

    def studs_at(self, m):
        return round(m * self.spm)

    def plates_at(self, m):
        return round(m * self.spm * PLATES_PER_STUD)


def convert_params(sheet, params, where):
    out = {}
    for k, v in params.items():
        if k in STUD_PARAMS:
            out[STUD_PARAMS[k]] = sheet.studs(sheet.metres(v, f"{where}.{k}"))
        elif k in PLATE_PARAMS:
            out[PLATE_PARAMS[k]] = sheet.plates(sheet.metres(v, f"{where}.{k}"))
        elif k == "column":
            out["column"] = convert_params(sheet, v, f"{where}.column")
        elif k in PASSTHROUGH:
            out[k] = v
        else:
            raise Uncited(f"{sheet.path.name}: {where} has parameter {k!r}, which is not a length and not a known passthrough")
    return out


# How tall each primitive actually builds, in plates, given its converted
# params. Not a second opinion about the geometry -- every one of these is the
# primitive's own `extent()` rule from `crates/spex-build/src/primitives.rs`,
# and the pairing is checked by building the recipe and comparing.
def built_plates(primitive, params):
    if primitive in ("Wall", "Column"):
        return snap_course(params.get("heightPlates", 0))
    if primitive in ("Ziggurat",):
        return params.get("tiers", 1) * snap_course(params.get("tierHeightPlates", PLATES_PER_COURSE))
    if primitive in ("Pyramid",):
        # Closes to a cap: base/2 tiers of one course each.
        return max(1, params.get("baseStuds", 1) // 2) * PLATES_PER_COURSE
    if primitive == "Dome":
        h = params.get("heightPlates")
        if h is None:
            h = round(params.get("radiusStuds", 1) * 2.5)
        return snap_course(h)
    if primitive == "Stair":
        return snap_course(params.get("risePlates", 0))
    if primitive == "Arch":
        # `Arch::extent` is `rise_plates + BRICK_PLATES`: the posts rise to
        # `rise_plates` and the lintel is a course on top of that. Forgetting
        # the lintel put the Colosseum's upper arcades a course too low and
        # overlapped 640 placements.
        return snap_course(params.get("risePlates", 0)) + PLATES_PER_COURSE
    if primitive == "Colonnade":
        col = params.get("column", {})
        return snap_course(col.get("heightPlates", 0))
    if primitive == "Mosaic":
        return 1
    return 0


def convert(sheet):
    steps = []
    for i, part in enumerate(sheet.d["massing"]):
        where = f"massing[{i}] {part['primitive']}"
        at = part.get("at", {})
        # `Mosaic` takes a 2D array of colours; a paved square or a glazing
        # grid is a uniform one, and spelling out 7 480 identical entries in a
        # sheet a person has to read would be absurd. `fillWidthM`/`fillDepthM`
        # is that array, written once.
        if part["primitive"] == "Mosaic" and "fillWidthM" in part.get("params", {}):
            pr = dict(part["params"])
            w = sheet.studs(sheet.metres(pr.pop("fillWidthM"), f"{where}.fillWidthM"))
            dp = sheet.studs(sheet.metres(pr.pop("fillDepthM"), f"{where}.fillDepthM"))
            colour = pr.pop("color")
            pr["cells"] = [[colour] * w for _ in range(dp)]
            pr.setdefault("tilePart", "3070b.dat")
            part = {**part, "params": pr}
        step = {
            "primitive": part["primitive"],
            "at": {
                "xStuds": sheet.studs_at(sheet.metres(at["xM"], f"{where}.at.xM", position=True)) if at.get("xM") else 0,
                "zStuds": sheet.studs_at(sheet.metres(at["zM"], f"{where}.at.zM", position=True)) if at.get("zM") else 0,
                "yPlates": y_plates_of(sheet, at, where),
            },
            "params": convert_params(sheet, part.get("params", {}), where),
        }
        # The dropped remainder, reported rather than discovered later.
        hp = step["params"].get("heightPlates", 0)
        if part["primitive"] in ("Wall", "Column") and hp % PLATES_PER_COURSE:
            sheet.rounded.append((where, hp, snap_course(hp)))
        if part["primitive"] == "Wall" and step["params"].get("heightPlates", 0) < MIN_WALL_PLATES:
            raise Uncited(
                f"{sheet.path.name}: {where} is {step['params']['heightPlates']} plate(s) tall, "
                f"under a brick course ({MIN_WALL_PLATES}). A Wall builds in whole courses and this one "
                f"would emit NOTHING, silently. Use Mosaic for a one-plate layer."
            )
        if at.get("deg"):
            step["at"]["orientationDeg"] = int(at["deg"]) % 360
        # A LINE is expanded into explicit steps rather than asked of the
        # engine. `ArrangeOn` offers a circle and an arc, and the Pont du Gard
        # is straight: bending 35 arches onto a 400 m arc to get a row put
        # every one of them off the deck under it, 4 414 floating placements.
        # Thirty-five steps in a generated recipe is not a cost anybody pays.
        if part.get("count") and part.get("arrangeOn", {}).get("kind") == "line":
            arrange = part["arrangeOn"]
            pitch = sheet.metres(arrange["pitchM"], f"{where}.arrangeOn.pitchM")
            axis = arrange.get("axis", "x")
            for k in range(part["count"]):
                one = json.loads(json.dumps(step))
                if axis == "x":
                    one["at"]["xStuds"] = step["at"]["xStuds"] + sheet.studs_at(pitch * k)
                else:
                    one["at"]["zStuds"] = step["at"]["zStuds"] + sheet.studs_at(pitch * k)
                steps.append(one)
            sheet.tops.append(-step["at"]["yPlates"] + built_plates(step["primitive"], step["params"]))
            continue
        if part.get("count"):
            step["count"] = part["count"]
            arrange = part["arrangeOn"]
            a = {"kind": arrange["kind"], "radiusStuds": sheet.studs(sheet.metres(arrange["radiusM"], f"{where}.arrangeOn.radiusM"))}
            for extra in ("startDeg", "endDeg"):
                if extra in arrange:
                    a[extra] = arrange[extra]
            step["arrangeOn"] = a
        # Ledger for `onTopOf`: where this step's top sits, in plates above
        # ground. `yPlates` is negative-up in LDraw's frame, so the top is the
        # magnitude of the base plus what the primitive actually builds.
        base = -step["at"]["yPlates"]
        sheet.tops.append(base + built_plates(step["primitive"], step["params"]))
        steps.append(step)
    return steps


def y_plates_of(sheet, at, where):
    """`at.yM` in metres, or `{"onTopOf": i}` — the real top of an earlier step."""
    y = at.get("yM")
    if isinstance(y, dict) and "onTopOf" in y:
        i = y["onTopOf"]
        if i >= len(sheet.tops):
            raise Uncited(f"{sheet.path.name}: {where} stands onTopOf step {i}, which comes later or does not exist")
        return -(sheet.tops[i] + int(y.get("plusCourses", 0)) * PLATES_PER_COURSE)
    if y:
        return -sheet.plates_at(sheet.metres(y, f"{where}.at.yM", position=True))
    return 0


def note(sheet):
    lines = [
        f"Generated by scripts/ldraw/gen-atlas.py from heritage/{sheet.path.name}. Do not hand-edit.",
        f"Wikidata {sheet.d['qid']}, tier {sheet.d['tier']}. Scale {sheet.spm} studs/m.",
        "",
        "CITED DIMENSIONS:",
    ]
    for name in sorted(sheet.used_dims):
        d = sheet.dims[name]
        lines.append(f"  {name} = {d['m']} m -- {d['source']}")
    unused = sorted(set(sheet.dims) - sheet.used_dims)
    if unused:
        lines.append("  (defined but unused by the massing: " + ", ".join(unused) + ")")
    if sheet.uncited:
        lines += ["", f"UNCITED SIZES, {len(sheet.uncited)} -- the modeller's judgement, not a source:"]
        for where, v in sheet.uncited:
            lines.append(f"  {where} = {v} m")
    else:
        lines += ["", "Every SIZE in this massing comes from a cited dimension."]
    if sheet.rounded:
        lines += ["", f"COURSE ROUNDING, {len(sheet.rounded)} height(s) truncated to whole brick courses:"]
        for where, want, got in sheet.rounded:
            lines.append(f"  {where}: {want} plates authored, {got} built ({want - got} dropped)")
    if sheet.uncited_pos:
        lines += ["", f"THE SITE PLAN is the modeller's arrangement: {len(sheet.uncited_pos)} uncited coordinate(s).",
                  "Relative positions here are read off published plans and aerial views by eye, not measured."]
    if sheet.d.get("note"):
        lines += ["", sheet.d["note"]]
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tier")
    ap.add_argument("--site")
    ap.add_argument("--strict", action="store_true")
    args = ap.parse_args()

    OUT.mkdir(parents=True, exist_ok=True)
    paths = sorted(SHEETS.glob("*.json"))
    if not paths:
        sys.exit(f"no site sheets in {SHEETS}")

    total_uncited, total_pos, total_cited, failures = 0, 0, 0, []
    written = 0
    for path in paths:
        sheet = Sheet(path)
        if args.tier and sheet.d["tier"] != args.tier:
            continue
        if args.site and sheet.d["slug"] != args.site:
            continue
        try:
            steps = convert(sheet)
        except Uncited as e:
            failures.append(str(e))
            print(f"  ! {e}", file=sys.stderr)
            continue

        recipe = {
            "version": 1,
            "id": f"heritage-{sheet.d['slug']}",
            "title": f"{sheet.d['name']} ({sheet.d['qid']}, Tier {sheet.d['tier']})",
            "_note": note(sheet),
            "scale": {"studsPerMetre": sheet.spm, "note": sheet.d.get("scaleNote", "")},
            "palette": sheet.d["palette"],
            "steps": steps,
        }
        out = OUT / f"{sheet.d['slug']}.json"
        out.write_text(json.dumps(recipe, indent=2, ensure_ascii=False) + "\n")
        written += 1
        total_uncited += len(sheet.uncited)
        total_pos += len(sheet.uncited_pos)
        total_cited += len(sheet.used_dims)
        budget = sheet.d.get("uncitedBudget", 0)
        flag = ""
        if len(sheet.uncited) > budget:
            flag = f"   <- OVER its uncitedBudget of {budget}"
            failures.append(f"{sheet.d['slug']}: {len(sheet.uncited)} uncited figures against a budget of {budget}")
        print(f"{sheet.d['slug']:<22} tier {sheet.d['tier']}  {len(steps):>3} step(s)  "
              f"{len(sheet.used_dims):>2} cited, {len(sheet.uncited):>2} uncited size(s), "
              f"{len(sheet.uncited_pos):>2} plan coord(s){flag}")

    print(f"\n{written} recipe(s) written to {OUT.relative_to(REPO)}")
    print(f"{total_cited} cited dimension(s) used, {total_uncited} uncited size(s), "
          f"{total_pos} site-plan coordinate(s) across the set")
    if failures and args.strict:
        sys.exit(f"\n--strict: {len(failures)} problem(s)")


if __name__ == "__main__":
    main()
