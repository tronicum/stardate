#!/usr/bin/env python3
"""S0 spike — answers D8: does the mesh renderer beat the point renderer?

Throwaway. The real thing is M51-M54 in Rust; this exists so the verdict is
made from two pictures of the same object in week 3, not from an argument.

It also pre-flights the single hardest claim in M51, before M51 is written:
that BFC winding can be composed correctly down a real LDraw reference chain,
and that type 2 / type 5 edge lines can be pulled out of real part files.
If that turns out to be wrong here, it is a bad afternoon instead of a bad
milestone.

Resolves a real .ldr scene into
  scene-mesh.json    triangles + per-face outward normals + hard/conditional edges
  scene-points.json  the same surface, area-weighted sampled with baked shading
                     (the same technique spex-ldraw::sampling already uses)

Usage:  python3 resolve_ldraw.py ../../ldraw-scenes/monolith.ldr -o out/
"""
import argparse
import json
import math
import random
import urllib.request
from pathlib import Path

BASE = "https://library.ldraw.org/library/official"
UA = "spex-brick/1.0 (educational project, github.com/tronicum/stardate)"
CACHE = Path(".ldraw-cache")
LDU_TO_MM = 0.4

# ------------------------------------------------------------ fetching ----


def fetch(path: str) -> str:
    p = CACHE / path
    if p.exists():
        return p.read_text(encoding="utf8", errors="replace")
    req = urllib.request.Request(f"{BASE}/{path}", headers={"User-Agent": UA})
    text = urllib.request.urlopen(req, timeout=30).read().decode("utf8", "replace")
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text, encoding="utf8")
    return text


def resolve_ref(name: str):
    name = name.replace("\\", "/")
    if name.startswith("s/"):
        cands = [f"parts/{name}"]
    elif name.startswith("48/"):
        cands = [f"p/{name}"]
    else:
        cands = [f"p/{name}", f"parts/{name}", f"parts/s/{name}"]
    for c in cands:
        try:
            return fetch(c)
        except Exception:
            continue
    raise FileNotFoundError(name)


# -------------------------------------------------------------- linalg ----


def mat_mul(a, b):
    return [sum(a[r * 3 + k] * b[k * 3 + c] for k in range(3))
            for r in range(3) for c in range(3)]


def mat_vec(m, v):
    return [m[r * 3] * v[0] + m[r * 3 + 1] * v[1] + m[r * 3 + 2] * v[2] for r in range(3)]


def add(a, b):
    return [a[0] + b[0], a[1] + b[1], a[2] + b[2]]


def det3(m):
    return (m[0] * (m[4] * m[8] - m[5] * m[7])
            - m[1] * (m[3] * m[8] - m[5] * m[6])
            + m[2] * (m[3] * m[7] - m[4] * m[6]))


IDENT = [1, 0, 0, 0, 1, 0, 0, 0, 1]


def normal(tri):
    (ax, ay, az), (bx, by, bz), (cx, cy, cz) = tri
    u = (bx - ax, by - ay, bz - az)
    v = (cx - ax, cy - ay, cz - az)
    n = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
    ln = math.sqrt(sum(c * c for c in n))
    return [c / ln for c in n] if ln else [0.0, 0.0, 0.0]


def area(tri):
    (ax, ay, az), (bx, by, bz), (cx, cy, cz) = tri
    u = (bx - ax, by - ay, bz - az)
    v = (cx - ax, cy - ay, cz - az)
    n = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
    return 0.5 * math.sqrt(sum(c * c for c in n))


# ------------------------------------------------------------- colours ----


def load_colours():
    out = {}
    for line in fetch("LDConfig.ldr").splitlines():
        t = line.split()
        if len(t) < 8 or t[1] != "!COLOUR":
            continue
        def after(k):
            return t[t.index(k) + 1] if k in t else None
        try:
            code = int(after("CODE"))
        except (TypeError, ValueError):
            continue
        val, edge = after("VALUE"), after("EDGE")
        if not val:
            continue
        rgb = lambda h: [int(h.lstrip("#")[i:i + 2], 16) / 255 for i in (0, 2, 4)]
        out[code] = {"value": rgb(val), "edge": rgb(edge) if edge else [0.35] * 3}
    return out


# ------------------------------------------------------------ resolving ---
# BFC, per LDraw's own spec, and the whole point of this spike:
#   - a file declares `0 BFC CERTIFY CCW` (near-universal) or `CW`
#   - `0 BFC INVERTNEXT` flips the NEXT type-1 reference only
#   - a reference matrix with a negative determinant is itself a mirroring
#     transform, which flips winding for that whole subtree
# All three compose. When the composed winding is reversed, the triangle's
# vertices are stored reversed, so the plain right-hand-rule normal comes out
# pointing outward.

STATS = {"files": 0, "uncertified": set(), "invertnext": 0, "mirrored": 0}


def resolve(part_file, matrix, translation, colour, depth, reverse, tris, edges, cedges,
            top_level=False):
    if depth > 10:
        raise RecursionError(part_file)
    text = fetch(f"parts/{part_file}") if top_level else resolve_ref(part_file)
    STATS["files"] += 1

    winding_ccw, certified, invert_next = True, False, False
    for line in text.splitlines():
        t = line.split()
        if not t:
            continue
        if t[0] == "0":
            if len(t) > 2 and t[1] == "BFC":
                if "CERTIFY" in t:
                    certified = True
                    if "CW" in t:
                        winding_ccw = False
                if "INVERTNEXT" in t:
                    invert_next = True
                    STATS["invertnext"] += 1
                if "CW" in t and "CERTIFY" not in t:
                    winding_ccw = False
                if "CCW" in t and "CERTIFY" not in t:
                    winding_ccw = True
            continue

        # a file's own declared winding flips everything below it
        rev = reverse ^ (not winding_ccw)

        if t[0] == "1" and len(t) >= 15:
            try:
                sub_col = int(t[1])
                nums = [float(x) for x in t[2:14]]
            except ValueError:
                continue
            sub_t, sub_m = nums[0:3], nums[3:12]
            mirrored = det3(sub_m) < 0
            if mirrored:
                STATS["mirrored"] += 1
            child_rev = rev ^ invert_next ^ mirrored
            invert_next = False
            resolve(" ".join(t[14:]),
                    mat_mul(matrix, sub_m),
                    add(mat_vec(matrix, sub_t), translation),
                    colour if sub_col == 16 else sub_col,
                    depth + 1, child_rev, tris, edges, cedges)
        elif t[0] in ("3", "4"):
            try:
                c = int(t[1])
                nums = [float(x) for x in t[2:]]
            except ValueError:
                continue
            pts = [mat_vec(matrix, nums[i:i + 3]) for i in range(0, len(nums) - 2, 3)]
            pts = [add(p, translation) for p in pts]
            eff = colour if c == 16 else c
            faces = [(0, 1, 2)] if t[0] == "3" else [(0, 1, 2), (0, 2, 3)]
            if len(pts) < (3 if t[0] == "3" else 4):
                continue
            for f in faces:
                tri = [pts[f[0]], pts[f[1]], pts[f[2]]]
                if rev:
                    tri = [tri[2], tri[1], tri[0]]
                tris.append((tri, eff))
        elif t[0] == "2" and len(t) >= 8:
            try:
                c = int(t[1])
                nums = [float(x) for x in t[2:8]]
            except ValueError:
                continue
            a = add(mat_vec(matrix, nums[0:3]), translation)
            b = add(mat_vec(matrix, nums[3:6]), translation)
            edges.append((a, b, colour if c == 16 else c))
        elif t[0] == "5" and len(t) >= 14:
            try:
                c = int(t[1])
                nums = [float(x) for x in t[2:14]]
            except ValueError:
                continue
            pts = [add(mat_vec(matrix, nums[i:i + 3]), translation) for i in range(0, 12, 3)]
            cedges.append((pts[0], pts[1], pts[2], pts[3], colour if c == 16 else c))

    if not certified:
        STATS["uncertified"].add(part_file)


def parse_scene(path):
    out = []
    for line in Path(path).read_text().splitlines():
        t = line.split()
        if len(t) < 15 or t[0] != "1":
            continue
        try:
            col = int(t[1])
            nums = [float(x) for x in t[2:14]]
        except ValueError:
            continue
        out.append((" ".join(t[14:]), col, nums[0:3], nums[3:12]))
    return out


# ------------------------------------------------------------- shading ---
# The same baked-shading technique spex-ldraw::sampling already uses, so the
# point render in this comparison is a fair representation of what the
# existing pipeline actually produces — not a strawman.

LIGHT = [0.5774, 0.5774, 0.5774]
AMBIENT, SPEC_POW, SPEC_STR = 0.35, 28.0, 0.55


def shade(rgb, n):
    d = max(0.0, sum(n[i] * LIGHT[i] for i in range(3)))
    inten = AMBIENT + (1 - AMBIENT) * d
    spec = d ** SPEC_POW
    return [min(1.0, c * inten + spec * SPEC_STR) for c in rgb]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("scene")
    ap.add_argument("-o", "--out", default="out")
    ap.add_argument("--points", type=int, default=180_000)
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    colours = load_colours()
    placements = parse_scene(args.scene)
    print(f"{len(placements)} placements, "
          f"{len({p[0] for p in placements})} distinct parts")

    tris, edges, cedges = [], [], []
    for part, col, tr, m in placements:
        resolve(part, m, tr, col, 0, False, tris, edges, cedges, top_level=True)

    print(f"files resolved  : {STATS['files']}")
    print(f"triangles       : {len(tris)}")
    print(f"hard edges      : {len(edges)}")
    print(f"conditional     : {len(cedges)}")
    print(f"INVERTNEXT seen : {STATS['invertnext']}")
    print(f"mirrored refs   : {STATS['mirrored']}")
    print(f"uncertified     : {sorted(STATS['uncertified']) or 'none'}")

    # LDraw is Y-down and in LDU; spex is Y-up and in mm. Convert once, here.
    #
    # FINDING, and it cost the first two renders: negating Y is a MIRROR. It
    # flips handedness, so every triangle's winding ends up backwards relative
    # to its own outward normal, and a renderer that culls by winding — which
    # is all of them — shows the inside of the far wall instead of the outside
    # of the near one. The object renders as if it were transparent.
    #
    # So the conversion must also swap two vertices of every triangle. This is
    # not optional and it is not cosmetic; it inverts backface culling for the
    # entire LDraw library. M52 must do the same thing at the bundle boundary.
    def conv(p):
        return [p[0] * LDU_TO_MM, -p[1] * LDU_TO_MM, p[2] * LDU_TO_MM]

    pos, nrm, col = [], [], []
    outward = 0
    centroid = [0.0, 0.0, 0.0]
    for tri, _ in tris:
        for v in tri:
            for i in range(3):
                centroid[i] += v[i]
    centroid = [c / (3 * len(tris)) for c in centroid]

    for tri, c in tris:
        n = normal(tri)
        mid = [sum(v[i] for v in tri) / 3 for i in range(3)]
        away = [mid[i] - centroid[i] for i in range(3)]
        if sum(n[i] * away[i] for i in range(3)) > 0:
            outward += 1
        rgb = colours.get(c, {"value": [0.8, 0.8, 0.8]})["value"]
        for v in (tri[0], tri[2], tri[1]):   # <- the mirror fix, see conv()
            pos.extend(conv(v))
            nrm.extend([n[0], -n[1], n[2]])
            col.extend(rgb)
    print(f"outward normals : {outward}/{len(tris)} "
          f"({100 * outward / len(tris):.1f}%)")

    epos, ecol = [], []
    for a, b, c in edges:
        rgb = colours.get(c, {"edge": [0.35] * 3}).get("edge", [0.35] * 3)
        epos.extend(conv(a) + conv(b))
        ecol.extend(rgb + rgb)

    (out / "scene-mesh.json").write_text(json.dumps({
        "position": pos, "normal": nrm, "color": col,
        "edgePosition": epos, "edgeColor": ecol,
        "triangleCount": len(tris), "hardEdgeCount": len(edges),
        "conditionalEdgeCount": len(cedges),
        "outwardNormalPct": round(100 * outward / len(tris), 1),
    }))

    # area-weighted surface sampling with baked shading — the existing pipeline
    rng = random.Random(0xC0FFEE)
    weights, acc = [], 0.0
    for tri, _ in tris:
        acc += area(tri)
        weights.append(acc)
    ppos, pcol = [], []
    for _ in range(args.points):
        r = rng.random() * acc
        lo, hi = 0, len(weights) - 1
        while lo < hi:
            mid = (lo + hi) // 2
            if weights[mid] < r:
                lo = mid + 1
            else:
                hi = mid
        tri, c = tris[lo]
        u, v = rng.random(), rng.random()
        if u + v > 1:
            u, v = 1 - u, 1 - v
        p = [tri[0][i] + u * (tri[1][i] - tri[0][i]) + v * (tri[2][i] - tri[0][i])
             for i in range(3)]
        rgb = colours.get(c, {"value": [0.8, 0.8, 0.8]})["value"]
        ppos.extend(conv(p))
        pcol.extend(shade(rgb, normal(tri)))
    (out / "scene-points.json").write_text(json.dumps({
        "position": ppos, "color": pcol, "count": args.points}))
    print(f"wrote {out}/scene-mesh.json and scene-points.json "
          f"({args.points} points)")


if __name__ == "__main__":
    main()
