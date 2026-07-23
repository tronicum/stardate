#!/usr/bin/env python3
"""Generates a real Klemmbaustein/interlocking-brick point cloud from a real
LDraw part file (https://ldraw.org — see BRICKs.md for the real geometry
source confirmation and CCAL 2.0/CC BY licensing). First concrete spike for
the planned brick-voxel-renderer milestone: resolve one real part's real
triangle-mesh geometry (once, cached as a `spex-brick-mesh` — see
`brickmesh.py`), sample its surface into points, color them with one real
official LDraw color, and write a plain XYZ point cloud that spex's
*existing* point-cloud pipeline (spex convert/spex serve/spex ascii) already
knows how to render — no new spex code needed at all.

A real LDraw part is not one flat file: it's a small tree of real files
(a top-level part references real "subpart" files under parts/s/, which in
turn reference real shared "primitive" files under p/ — a cylinder, a disc,
a box — used identically across thousands of different parts). `ldraw.py`
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
import os
import sys

from brickmesh import get_or_resolve_mesh, load_ldraw_colors, mesh_triangles
from ldraw import LDU_TO_MM
from sampling import sample_surface

DEFAULT_PART = "3005.dat"  # a real, iconic 1x1 brick (2x4 brick's little cousin)
DEFAULT_COLOR_CODE = 4  # real LDraw color code for "Red"
DEFAULT_POINT_COUNT = 600


def main():
    part = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_PART
    color_code = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_COLOR_CODE
    point_count = int(sys.argv[3]) if len(sys.argv) > 3 else DEFAULT_POINT_COUNT
    out_path = sys.argv[4] if len(sys.argv) > 4 else "out/brick.xyz"

    print(f"resolving real LDraw geometry for {part!r} (cached as a spex-brick-mesh)...", file=sys.stderr)
    mesh = get_or_resolve_mesh(part, color_code)
    triangles = mesh_triangles(mesh)
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
