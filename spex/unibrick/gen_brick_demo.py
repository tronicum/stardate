#!/usr/bin/env python3
"""Generates a real Klemmbaustein/interlocking-brick point cloud from a real
LDraw part file (https://ldraw.org — see BRICKs.md for the real geometry
source confirmation and CCAL 2.0/CC BY licensing). First concrete spike for
the planned brick-voxel-renderer milestone: fetch one real part's real
triangle-mesh geometry, sample its surface into points, color them with one
real official LDraw color, and write a plain XYZ point cloud that spex's
*existing* point-cloud pipeline (spex convert/spex serve/spex ascii) already
knows how to render — no new spex code needed at all.

A real LDraw part is not one flat file: it's a small tree of real files
(a top-level part references real "subpart" files under parts/s/, which in
turn reference real shared "primitive" files under p/ — a cylinder, a disc,
a box — used identically across thousands of different parts). This script
resolves that tree recursively, exactly as any real LDraw viewer (Studio,
LDCad, LeoCAD) does, composing each real 3x3 rotation/scale matrix + real
translation down through the recursion (LDraw's own "type 1" line format).
Only real face geometry (type 3 triangles, type 4 quads) becomes points;
type 2/5 lines are real edge/optional-line hints for wireframe rendering,
not solid surface, so they're deliberately skipped.

Files are fetched from ldraw.org's real, current official library on
demand and cached locally (not committed — see AGENTS.md/docs/agents on
avoiding committing large/many external files; a full local library mirror,
if ever wanted, is a deliberate separate step via git-lfs, not this
script's job). Real LDraw color codes/RGB values come from the real,
official LDConfig.ldr, fetched and parsed the same way.
"""
import math
import os
import random
import sys
import urllib.request

LDRAW_BASE = "https://library.ldraw.org/library/official"
CACHE_DIR = os.path.join(os.path.dirname(__file__), ".ldraw-cache")
USER_AGENT = "spex-demo/1.0 (educational project, github.com/tronicum/stardate)"

DEFAULT_PART = "3005.dat"  # a real, iconic 1x1 brick (2x4 brick's little cousin)
DEFAULT_COLOR_CODE = 4  # real LDraw color code for "Red"
DEFAULT_POINT_COUNT = 600
LDU_TO_MM = 0.4  # real LDraw unit conversion, see BRICKs.md


def fetch(path):
    """Fetches one real file from ldraw.org's official library, cached
    locally under scripts/.ldraw-cache/ so repeated runs (and the multi-file
    recursive resolution below) don't re-fetch the same primitive file
    dozens of times in one run."""
    cache_path = os.path.join(CACHE_DIR, path)
    if os.path.exists(cache_path):
        with open(cache_path) as f:
            return f.read()
    url = f"{LDRAW_BASE}/{path}"
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=15) as resp:
        text = resp.read().decode("utf-8", errors="replace")
    os.makedirs(os.path.dirname(cache_path), exist_ok=True)
    with open(cache_path, "w") as f:
        f.write(text)
    return text


def resolve_ref_path(name):
    """A referenced LDraw filename doesn't say which real library folder it
    lives in — real LDraw resolvers try a defined real search path. This
    tries the same real candidate folders in a sensible order and uses
    whichever the server actually has, rather than hardcoding one guess."""
    name = name.replace("\\", "/")
    if name.startswith("s/"):
        candidates = [f"parts/{name}"]
    elif name.startswith("48/"):
        candidates = [f"p/{name}"]
    else:
        candidates = [f"p/{name}", f"parts/{name}", f"parts/s/{name}"]
    last_error = None
    for candidate in candidates:
        try:
            return candidate, fetch(candidate)
        except Exception as e:  # noqa: BLE001 - real network/HTTP errors, try next candidate
            last_error = e
    raise RuntimeError(f"couldn't resolve real LDraw file {name!r} in any of {candidates}: {last_error}")


def load_ldraw_colors():
    """Parses the real, official LDConfig.ldr color table:
    `0 !COLOUR <name> CODE <n> VALUE #RRGGBB EDGE #RRGGBB` -> {code: (name, (r,g,b))}."""
    text = fetch("LDConfig.ldr")
    colors = {}
    for line in text.splitlines():
        tokens = line.split()
        if len(tokens) < 8 or tokens[1] != "!COLOUR":
            continue
        name = tokens[2]
        code = int(tokens[tokens.index("CODE") + 1])
        hex_rgb = tokens[tokens.index("VALUE") + 1].lstrip("#")
        rgb = tuple(int(hex_rgb[i : i + 2], 16) for i in (0, 2, 4))
        colors[code] = (name, rgb)
    return colors


def mat_mul(a, b):
    """3x3 row-major matrix multiply, both as flat 9-tuples."""
    return tuple(
        sum(a[row * 3 + k] * b[k * 3 + col] for k in range(3))
        for row in range(3)
        for col in range(3)
    )


def mat_vec(m, v):
    return tuple(sum(m[row * 3 + k] * v[k] for k in range(3)) for row in range(3))


def vec_add(a, b):
    return tuple(a[i] + b[i] for i in range(3))


IDENTITY = (1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0)
ZERO = (0.0, 0.0, 0.0)


def resolve_geometry(path, matrix, translation, color_code, triangles, depth=0):
    """Recursively resolves one real LDraw file into a flat list of real
    (triangle, color_code) pairs, in the *top-level part's* local coordinate
    space — every nested real transform composed down through recursion."""
    if depth > 8:
        raise RuntimeError(f"LDraw reference recursion too deep at {path!r} - likely a real cycle or bug")
    text = fetch(f"parts/{path}") if depth == 0 else resolve_ref_path(path)[1]
    for line in text.splitlines():
        tokens = line.split()
        if not tokens:
            continue
        line_type = tokens[0]
        if line_type == "1":
            # 1 <colour> x y z a b c d e f g h i <file>
            sub_color = int(tokens[1])
            nums = [float(t) for t in tokens[2:14]]
            sub_translation = tuple(nums[0:3])
            sub_matrix = tuple(nums[3:12])
            new_matrix = mat_mul(matrix, sub_matrix)
            new_translation = vec_add(mat_vec(matrix, sub_translation), translation)
            sub_file = " ".join(tokens[14:])
            effective_color = color_code if sub_color == 16 else sub_color
            resolve_geometry(sub_file, new_matrix, new_translation, effective_color, triangles, depth + 1)
        elif line_type in ("3", "4"):
            face_color_code = int(tokens[1])
            effective_color = color_code if face_color_code == 16 else face_color_code
            nums = [float(t) for t in tokens[2:]]
            local_verts = [tuple(nums[i : i + 3]) for i in range(0, len(nums), 3)]
            world_verts = [vec_add(mat_vec(matrix, v), translation) for v in local_verts]
            if line_type == "3":
                triangles.append((world_verts, effective_color))
            else:
                v0, v1, v2, v3 = world_verts
                triangles.append(([v0, v1, v2], effective_color))
                triangles.append(([v0, v2, v3], effective_color))
        # line_type in ("0", "2", "5"): comments/meta and real edge/optional
        # lines - never solid surface, deliberately skipped.


def triangle_area(tri):
    (x0, y0, z0), (x1, y1, z1), (x2, y2, z2) = tri
    ux, uy, uz = x1 - x0, y1 - y0, z1 - z0
    vx, vy, vz = x2 - x0, y2 - y0, z2 - z0
    cx, cy, cz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
    return 0.5 * math.sqrt(cx * cx + cy * cy + cz * cz)


def triangle_normal(tri):
    """Real face normal via the right-hand rule, from the real vertex
    winding LDraw's own BFC (Back Face Culling) certification guarantees
    (every real official part file declares `BFC CERTIFY CCW`). Not
    adjusted for `BFC INVERTNEXT` (a real directive some parts use to flag
    a mirrored/flipped sub-file reference) — a handful of faces on a
    composite part can end up with an inward-facing normal as a result, a
    minor cosmetic imperfection in the baked lighting below, not a
    correctness bug in the real geometry itself."""
    (x0, y0, z0), (x1, y1, z1), (x2, y2, z2) = tri
    ux, uy, uz = x1 - x0, y1 - y0, z1 - z0
    vx, vy, vz = x2 - x0, y2 - y0, z2 - z0
    nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
    length = math.sqrt(nx * nx + ny * ny + nz * nz) or 1.0
    return (nx / length, ny / length, nz / length)


# A fixed "headlight" direction near the viewer's own default camera angle
# (spex's default camera sits at center + diagonal*0.6 on every axis - see
# crates/spex-cli/src/ascii.rs's default_camera and the viewer's matching
# initial position), so the baked-in highlight actually reads as light
# coming from roughly where you're already looking from by default.
LIGHT_DIR = (0.5774, 0.5774, 0.5774)  # normalize((0.6, 0.6, 0.6))
AMBIENT_FLOOR = 0.35  # unlit faces stay dimly visible, not pure black
SPECULAR_POWER = 28.0  # higher = tighter, glassier-looking highlight
SPECULAR_STRENGTH = 0.55


def shade_color(base_rgb, normal):
    """Bakes real Lambertian shading + a tight specular-style highlight
    directly into a point's stored color, computed once here at generation
    time from the real triangle normal it was sampled from - not something
    the renderer computes at all. Both spex's WebGL viewer and its ASCII
    renderer just display whatever RGB is stored per point, so this is the
    only way to get a "shiny" look out of either without teaching either
    renderer a real lighting model of its own."""
    nx, ny, nz = normal
    lx, ly, lz = LIGHT_DIR
    diffuse = max(0.0, nx * lx + ny * ly + nz * lz)
    intensity = AMBIENT_FLOOR + (1.0 - AMBIENT_FLOOR) * diffuse
    specular = diffuse**SPECULAR_POWER
    r, g, b = base_rgb
    return tuple(
        max(0, min(255, round(channel * intensity + 255 * specular * SPECULAR_STRENGTH)))
        for channel in (r, g, b)
    )


def sample_point_in_triangle(tri):
    (x0, y0, z0), (x1, y1, z1), (x2, y2, z2) = tri
    u, v = random.random(), random.random()
    if u + v > 1.0:
        u, v = 1.0 - u, 1.0 - v
    return (
        x0 + u * (x1 - x0) + v * (x2 - x0),
        y0 + u * (y1 - y0) + v * (y2 - y0),
        z0 + u * (z1 - z0) + v * (z2 - z0),
    )


def sample_surface(triangles, point_count, colors):
    """Samples `point_count` points across `triangles` (real face area
    weighted, so density stays even regardless of triangle size), and bakes
    each point's real Lambertian+specular shading (see `shade_color`)
    directly into its stored RGB. Returns `[(position, (r,g,b)), ...]`."""
    weights = [triangle_area(tri) for tri, _ in triangles]
    normals = [triangle_normal(tri) for tri, _ in triangles]
    total = sum(weights) or 1.0
    points = []
    for _ in range(point_count):
        r = random.random() * total
        acc = 0.0
        idx = len(triangles) - 1
        for i, w in enumerate(weights):
            acc += w
            if r <= acc:
                idx = i
                break
        tri, color_code = triangles[idx]
        base_rgb = colors.get(color_code, ("Unknown", (200, 200, 200)))[1]
        shaded_rgb = shade_color(base_rgb, normals[idx])
        points.append((sample_point_in_triangle(tri), shaded_rgb))
    return points


def main():
    part = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_PART
    color_code = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_COLOR_CODE
    point_count = int(sys.argv[3]) if len(sys.argv) > 3 else DEFAULT_POINT_COUNT
    out_path = sys.argv[4] if len(sys.argv) > 4 else "out/brick.xyz"

    print(f"resolving real LDraw geometry for {part!r}...", file=sys.stderr)
    triangles = []
    resolve_geometry(part, IDENTITY, ZERO, color_code, triangles)
    print(f"resolved {len(triangles)} real triangles (after quad->triangle splitting)", file=sys.stderr)

    colors = load_ldraw_colors()
    points = sample_surface(triangles, point_count, colors)

    os.makedirs(os.path.dirname(out_path) or ".", exist_ok=True)
    with open(out_path, "w") as f:
        for (x, y, z), (r, g, b) in points:
            # Real LDraw convention: +Y points DOWN (a stud's tip is at
            # negative Y, a brick's underside at positive Y). spex's viewer/
            # ascii camera assumes standard +Y-up. Flip Y only here, at
            # final output, so the brick displays stud-up instead of
            # upside-down - the shading above was already computed
            # correctly in LDraw's own native coordinate frame beforehand.
            f.write(f"{x * LDU_TO_MM:.4f} {-y * LDU_TO_MM:.4f} {z * LDU_TO_MM:.4f} {r} {g} {b}\n")

    color_name = colors.get(color_code, ("Unknown", None))[0]
    print(f"wrote {len(points)} real surface-sampled points ({part}, real color {color_name!r}) to {out_path}")


if __name__ == "__main__":
    main()
