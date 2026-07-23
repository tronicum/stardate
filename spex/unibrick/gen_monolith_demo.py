#!/usr/bin/env python3
"""Generates "the 2001 moment": a real Klemmbaustein monolith, assembled
from a real vertical stack of real LDraw parts, approximating the film's
canonical 1:4:9 proportions as closely as real LEGO height units allow —
see BRICKs.md and TODOs.md's M41 entry for the honest math on why *exact*
1:4:9 isn't achievable with real brick/plate heights (72mm isn't a whole
multiple of the real 3.2mm plate height), and the real closest-buildable
choice this script makes instead.

Reuses every real-geometry function via `brickmesh.py`/`sampling.py` rather
than duplicating them — and, unlike the first version of this script,
resolves each *distinct* real part exactly once (via
`brickmesh.get_or_resolve_mesh`, cached as a `spex-brick-mesh`) no matter
how many times it's placed in the stack: this 9-part assembly only ever
resolves 2 real LDraw files (one brick, one plate) over the network, then
cheaply translates (`brickmesh.place_mesh`) the same resolved geometry into
each of the 9 real stacked positions.
"""
import sys

from brickmesh import get_or_resolve_mesh, load_ldraw_colors, place_mesh
from ldraw import LDU_TO_MM
from sampling import sample_surface

BRICK_1X4 = "3010.dat"
PLATE_1X4 = "3710.dat"
BLACK_COLOR_CODE = 0  # real LDraw color code for "Black", #1B2A34

BRICK_HEIGHT_LDU = 24  # a real brick is 24 LDU (9.6mm) tall
PLATE_HEIGHT_LDU = 8  # a real plate is 8 LDU (3.2mm) tall

# The real, closest-buildable approximation of the film's canonical 1:4:9
# monolith proportions: width=1 stud (8mm) exactly matches a real 1x4
# part's footprint; height=9 studs (72mm) isn't a whole multiple of the
# real 3.2mm plate height, so this stacks 7 real bricks + 2 real plates
# (23 plate-units = 73.6mm, ratio 1:4:9.2) rather than pretend-round to an
# impossible exact 72mm. The other honest option (7 bricks + 1 plate,
# 70.4mm, ratio 1:4:8.8) is equally close; this one was picked because a
# monolith reads as more imposing slightly tall than slightly short.
DEFAULT_STACK = [BRICK_1X4] * 7 + [PLATE_1X4] * 2


def part_height_ldu(part_file):
    return BRICK_HEIGHT_LDU if part_file == BRICK_1X4 else PLATE_HEIGHT_LDU


def resolve_stack(stack, color_code):
    """Resolves every *distinct* real part in `stack` exactly once (a
    `spex-brick-mesh` per part file, reused for every placement of that
    part), then places each occurrence bottom-to-top so each part's own
    real bottom surface exactly touches the real top surface of everything
    stacked below it — a real vertical assembly, not just repeated
    identical geometry at the origin.

    Every real LDraw part here shares the same real local convention (top
    surface, where studs are, at local y=0; bottom surface, the tube
    opening, at local y=+height) - confirmed directly against the real
    3010.dat/3710.dat files, not assumed. So a part's translation needs to
    place its own bottom (local y = +height) at -height_from_base (the top
    of everything already stacked): translation_y + height = -height_from_base,
    i.e. translation_y = -(height_from_base + height) - the "+ height" term
    (this part's OWN height, not just what's below it) is what a first,
    buggy version of this function omitted, producing a real visible gap
    the moment two different part heights (brick vs. plate) were mixed -
    same-height parts (brick-on-brick) happened to stack correctly even
    with the bug, which is why it wasn't caught immediately."""
    meshes = {part_file: get_or_resolve_mesh(part_file, color_code) for part_file in set(stack)}
    triangles = []
    height_from_base = 0
    for part_file in stack:
        h = part_height_ldu(part_file)
        # LDraw's own Y-down convention (see ldraw.py): stacking "up" means
        # moving toward more negative Y, so subtract, not add.
        translation = (0.0, -float(height_from_base + h), 0.0)
        triangles.extend(place_mesh(meshes[part_file], translation))
        height_from_base += h
    return triangles, height_from_base


def main():
    point_count = int(sys.argv[1]) if len(sys.argv) > 1 else 20000
    out_path = sys.argv[2] if len(sys.argv) > 2 else "out/monolith.xyz"

    distinct = sorted(set(DEFAULT_STACK))
    print(
        f"resolving {len(distinct)} distinct real LDraw part(s) for a {len(DEFAULT_STACK)}-part real stack "
        f"(cached as spex-brick-mesh files, reused across placements)...",
        file=sys.stderr,
    )
    triangles, total_height_ldu = resolve_stack(DEFAULT_STACK, BLACK_COLOR_CODE)
    total_height_mm = total_height_ldu * LDU_TO_MM
    print(
        f"resolved {len(triangles)} real triangles across {len(DEFAULT_STACK)} real parts "
        f"(real total height {total_height_mm:.1f}mm, real footprint 8x32mm, "
        f"ratio 1:4:{total_height_mm / 8.0:.2f})",
        file=sys.stderr,
    )

    colors = load_ldraw_colors()
    points = sample_surface(triangles, point_count, colors)

    import os

    os.makedirs(os.path.dirname(out_path) or ".", exist_ok=True)
    with open(out_path, "w") as f:
        for (x, y, z), (r, g, b) in points:
            f.write(f"{x * LDU_TO_MM:.4f} {-y * LDU_TO_MM:.4f} {z * LDU_TO_MM:.4f} {r} {g} {b}\n")

    print(f"wrote {len(points)} real surface-sampled points (real Black monolith) to {out_path}")


if __name__ == "__main__":
    main()
