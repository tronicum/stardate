#!/usr/bin/env python3
"""Generates the real animated version of "the 2001 moment"
(`gen_monolith_demo.py`'s static monolith): the same 9 real LDraw parts (7
"Brick 1x4" `3010.dat` + 2 "Plate 1x4" `3710.dat`, real Black), but this
time starting scattered/floating apart and converging into the finished
stacked monolith — a real, honestly-labeled *stylized reveal* (each part's
starting position is a deterministic scatter, not a physics simulation;
see `start_translations()`), not a claim of simulated physical assembly.

Unlike an earlier, since-reverted attempt at this feature, this one goes
through spex's *actual* point-cloud pipeline end to end, exactly like every
other unibrick demo: each animation frame is written as a real `.xyz` point
cloud (same format `gen_brick_demo.py`/`gen_monolith_demo.py` already
write), then handed to the real `spex frame-sequence` command — which tiles
every frame into a real octree tileset via `spex-tiler` (the same tiler
every other demo uses), keeping every frame's tileset in one shared
coordinate frame so the real WebGL viewer can play them back as one
animation (see `crates/spex-cli/src/frame_sequence.rs` and
`viewer/src/main.ts`'s sequence-playback support) without the point cloud
jumping position between frames. No bespoke renderer, no new spex-server
code, no server-external artifact — this is the real point/voxel cloud
pipeline, same as everything else here.

Each output point's *local* (untransformed) sampled position and baked
shading are computed exactly ONCE (not re-sampled per frame) — since every
part only ever translates, never rotates, a triangle's normal (and thus
its baked shading) and a sampled local point's coordinates never change
across frames; only the per-part translation offset added to it does. This
is what keeps the animation's points moving smoothly frame to frame
instead of shimmering (a fresh independent random sample every frame would
put each frame's points at unrelated locations on the surface, which reads
as noise, not motion).
"""
import math
import os
import random
import subprocess
import sys

from brickmesh import get_or_resolve_mesh, load_ldraw_colors, mesh_triangles
from ldraw import LDU_TO_MM, triangle_area, triangle_normal
from sampling import sample_point_in_triangle, shade_color

BRICK_1X4 = "3010.dat"
PLATE_1X4 = "3710.dat"
BLACK_COLOR_CODE = 0

BRICK_HEIGHT_LDU = 24
PLATE_HEIGHT_LDU = 8
DEFAULT_STACK = [BRICK_1X4] * 7 + [PLATE_1X4] * 2

DEFAULT_FRAME_COUNT = 30
DEFAULT_POINT_COUNT = 6000
DEFAULT_FPS = 6.0
FLOAT_HEIGHT_LDU = 420.0  # how far "up" (real LDraw -Y) each part starts before settling
SCATTER_RADIUS_LDU = 260.0  # deterministic sideways scatter so parts visibly converge from different directions

SPEX_BINARY = os.path.join(os.path.dirname(__file__), "..", "target", "release", "spex")


def part_height_ldu(part_file):
    return BRICK_HEIGHT_LDU if part_file == BRICK_1X4 else PLATE_HEIGHT_LDU


def ease_in_out_cubic(t):
    return 4 * t**3 if t < 0.5 else 1 - ((-2 * t + 2) ** 3) / 2


def final_translations(stack):
    """Same real stacking math as `gen_monolith_demo.py`'s `resolve_stack`:
    each part's bottom surface lands exactly at the top of everything
    stacked below it, no gaps."""
    translations = []
    height_from_base = 0
    for part_file in stack:
        h = part_height_ldu(part_file)
        translations.append((0.0, -float(height_from_base + h), 0.0))
        height_from_base += h
    return translations


def start_translations(stack, final):
    """A real, deliberately-labeled *stylized* starting layout — not a
    physics simulation. Each part starts `FLOAT_HEIGHT_LDU` further "up"
    (more negative Y, LDraw's own real -Y-is-up convention before this
    project's final output flip) than its own final position, plus a
    deterministic sideways scatter (seeded per part index, so re-running
    this script produces the identical starting layout every time) so
    parts visibly converge from different directions rather than all
    dropping straight down in a boring vertical line."""
    starts = []
    for i, (fx, fy, fz) in enumerate(final):
        rng = random.Random(f"monolith-assembly-{i}")
        angle = rng.uniform(0, 2 * math.pi)
        radius = SCATTER_RADIUS_LDU * (0.4 + 0.6 * rng.random())
        starts.append((fx + radius * math.cos(angle), fy - FLOAT_HEIGHT_LDU, fz + radius * math.sin(angle)))
    return starts


def sample_scene_once(stack, color_code, point_count):
    """Samples every real output point exactly once, in each placement's
    own local (untransformed) frame, with shading baked in immediately
    (translation-invariant, since it only depends on the local triangle
    normal). Returns `[(placement_index, local_point, shaded_rgb), ...]`
    — real face-area weighted both across placements and within each
    placement's own triangles, same sampling principle as `sampling.py`."""
    meshes = {part_file: get_or_resolve_mesh(part_file, color_code) for part_file in set(stack)}
    per_placement_triangles = [mesh_triangles(meshes[part_file]) for part_file in stack]
    per_placement_weights = [
        [triangle_area(tri) for tri, _ in tris] for tris in per_placement_triangles
    ]
    placement_totals = [sum(w) or 1.0 for w in per_placement_weights]
    grand_total = sum(placement_totals) or 1.0

    colors = load_ldraw_colors()
    samples = []
    for _ in range(point_count):
        r = random.random() * grand_total
        acc = 0.0
        placement_idx = len(per_placement_triangles) - 1
        for i, total in enumerate(placement_totals):
            acc += total
            if r <= acc:
                placement_idx = i
                break
        tris = per_placement_triangles[placement_idx]
        weights = per_placement_weights[placement_idx]
        r2 = random.random() * placement_totals[placement_idx]
        acc2 = 0.0
        tri_idx = len(tris) - 1
        for i, w in enumerate(weights):
            acc2 += w
            if r2 <= acc2:
                tri_idx = i
                break
        tri, tri_color_code = tris[tri_idx]
        local_point = sample_point_in_triangle(tri)
        normal = triangle_normal(tri)
        base_rgb = colors.get(tri_color_code, ("Unknown", (200, 200, 200)))[1]
        samples.append((placement_idx, local_point, shade_color(base_rgb, normal)))
    return samples


def write_frame_xyz(samples, translations, out_path):
    """Writes one real frame as a plain `.xyz` point cloud — the same
    format/Y-flip convention every other unibrick script writes, so `spex
    frame-sequence` (which shells out to the same real tiling code as `spex
    convert`) renders it unchanged."""
    with open(out_path, "w") as f:
        for placement_idx, (lx, ly, lz), (r, g, b) in samples:
            tx, ty, tz = translations[placement_idx]
            x_mm = (lx + tx) * LDU_TO_MM
            y_mm = -(ly + ty) * LDU_TO_MM
            z_mm = (lz + tz) * LDU_TO_MM
            f.write(f"{x_mm:.4f} {y_mm:.4f} {z_mm:.4f} {r} {g} {b}\n")


def main():
    point_count = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_POINT_COUNT
    frame_count = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_FRAME_COUNT
    out_dir = sys.argv[3] if len(sys.argv) > 3 else "out/monolith-assembly"
    frames_xyz_dir = sys.argv[4] if len(sys.argv) > 4 else "in/monolith-assembly-frames"

    random.seed(1337)  # reproducible run to run, same spirit as the fixed-seed German-cities-TSP demo

    print(
        f"sampling {point_count} real points once across {len(DEFAULT_STACK)} real parts, "
        f"building {frame_count} real assembly frames...",
        file=sys.stderr,
    )
    final = final_translations(DEFAULT_STACK)
    start = start_translations(DEFAULT_STACK, final)
    samples = sample_scene_once(DEFAULT_STACK, BLACK_COLOR_CODE, point_count)

    os.makedirs(frames_xyz_dir, exist_ok=True)
    frame_paths = []
    for f_idx in range(frame_count):
        t = f_idx / (frame_count - 1) if frame_count > 1 else 1.0
        eased = ease_in_out_cubic(t)
        translations = [
            tuple(s + (fi - s) * eased for s, fi in zip(start[i], final[i])) for i in range(len(DEFAULT_STACK))
        ]
        frame_path = os.path.join(frames_xyz_dir, f"frame-{f_idx:03d}.xyz")
        write_frame_xyz(samples, translations, frame_path)
        frame_paths.append(frame_path)
    print(f"wrote {frame_count} real frame point clouds to {frames_xyz_dir}/", file=sys.stderr)

    spex_binary = os.environ.get("SPEX_BINARY", SPEX_BINARY)
    cmd = [spex_binary, "frame-sequence", *frame_paths, "-o", out_dir, "--fps", str(DEFAULT_FPS)]
    print(f"running: {' '.join(cmd)}", file=sys.stderr)
    subprocess.run(cmd, check=True)

    print(f"real tileset sequence ready — view it with: spex serve {out_dir}")


if __name__ == "__main__":
    main()
