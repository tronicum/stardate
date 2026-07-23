#!/usr/bin/env python3
"""Generates a real point cloud from a real, official LDraw *model* file —
e.g. https://ldraw.org's own `car.ldr`/`pyramid.ldr` sample models, real
official demonstration files shipped with LDraw and authored by James
Jessiman (LDraw's original creator). This is the "build instructions" half
of `BRICKs.md`'s plan: a real assembly's parts, each at its own real
position/orientation, sourced directly from the model file's own real
type-1 placement lines (`brickscene.py`) — not a hand-written stack like
`gen_monolith_demo.py`.

Reuses `brickmesh.get_or_resolve_mesh()` per distinct (part, color) pair,
so a real model with dozens of placements (car.ldr repeats several parts
many times — six "Plate 1x2" `3024.dat` placements, four `3623.dat`, four
`4624.dat`, ...) only ever resolves each distinct part once over the
network, regardless of how many times — or at how many different real
rotations — it's placed.
"""
import os
import sys

from brickmesh import get_or_resolve_mesh, load_ldraw_colors, place_mesh
from brickscene import get_or_parse_scene
from ldraw import LDU_TO_MM
from sampling import sample_surface

DEFAULT_MODEL = "car.ldr"
DEFAULT_POINT_COUNT = 20000


def resolve_scene(scene):
    """Resolves every *distinct* real (part, color) pair referenced by the
    scene exactly once, then places each real occurrence at its own real
    translation/rotation matrix — the concrete payoff of splitting
    resolution from placement in `brickmesh.py`."""
    distinct = sorted({(p["partFile"], p["colorCode"]) for p in scene["placements"]})
    meshes = {(part, color): get_or_resolve_mesh(part, color) for part, color in distinct}
    triangles = []
    for placement in scene["placements"]:
        mesh = meshes[(placement["partFile"], placement["colorCode"])]
        triangles.extend(
            place_mesh(mesh, translation=tuple(placement["translation"]), matrix=tuple(placement["matrix"]))
        )
    return triangles


def main():
    model = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_MODEL
    point_count = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_POINT_COUNT
    out_path = sys.argv[3] if len(sys.argv) > 3 else "out/model.xyz"

    print(f"parsing real LDraw model {model!r}...", file=sys.stderr)
    scene = get_or_parse_scene(model)
    distinct_parts = {p["partFile"] for p in scene["placements"]}
    print(
        f"parsed {len(scene['placements'])} real part placements "
        f"({len(distinct_parts)} distinct real parts) from {scene['sourceDescription']!r} "
        f"(real author: {scene['sourceAuthor']})",
        file=sys.stderr,
    )

    triangles = resolve_scene(scene)
    print(f"resolved {len(triangles)} real triangles", file=sys.stderr)

    colors = load_ldraw_colors()
    points = sample_surface(triangles, point_count, colors)

    os.makedirs(os.path.dirname(out_path) or ".", exist_ok=True)
    with open(out_path, "w") as f:
        for (x, y, z), (r, g, b) in points:
            # Same real LDraw Y-down -> spex Y-up flip as gen_brick_demo.py/
            # gen_monolith_demo.py, applied only at final output.
            f.write(f"{x * LDU_TO_MM:.4f} {-y * LDU_TO_MM:.4f} {z * LDU_TO_MM:.4f} {r} {g} {b}\n")

    print(f"wrote {len(points)} real surface-sampled points ({model}) to {out_path}")


if __name__ == "__main__":
    main()
