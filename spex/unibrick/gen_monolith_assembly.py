#!/usr/bin/env python3
"""Generates the real animated version of "the 2001 moment"
(`gen_monolith_demo.py`'s static monolith): the same 9 real LDraw parts (7
"Brick 1x4" `3010.dat` + 2 "Plate 1x4" `3710.dat`, real Black), but this
time starting scattered/floating apart and converging into the finished
stacked monolith — a real, honestly-labeled *stylized reveal* (each part's
starting position is a deterministic scatter, not a physics simulation;
see `start_translations()`), not a claim of simulated physical assembly.

Writes a single self-contained animated HTML file (same spirit as `spex
ascii --animate --out file.html`'s pre-baked-frames-cycled-by-setInterval
technique — see M38/M39 in TODOs.md — generalized here from ASCII text
frames to real colored points on an HTML5 canvas, with real mouse-drag
orbit + scroll-zoom controls, since a full physics/vertex-lerp animation
inside spex's actual WebGL viewer would require the octree/tileset format
itself to track per-point identity across frames, a much bigger change to
a shared, heavily-tested core format — see BRICKs.md). No spex-tiler/
spex-server/viewer changes needed at all: this is a fully standalone
artifact, openable directly from disk, no server required.

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
import json
import math
import os
import random
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

DEFAULT_FRAME_COUNT = 36
DEFAULT_POINT_COUNT = 6000
FLOAT_HEIGHT_LDU = 420.0  # how far "up" (real LDraw -Y) each part starts before settling
SCATTER_RADIUS_LDU = 260.0  # deterministic sideways scatter so parts visibly converge from different directions

PLAY_SECONDS = 6.0
INTRO_HOLD_SECONDS = 1.0
END_HOLD_SECONDS = 2.5


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
    (more negative Y, LDraw's own real -Y-is-up-facing convention before
    this project's final output flip) than its own final position, plus a
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


def build_frames(stack, color_code, point_count, frame_count):
    """Builds `frame_count` real point-cloud snapshots — an eased
    (slow-start, fast-middle, slow-end) interpolation of each placement's
    translation from its stylized start to its real final stacked
    position — converted to real millimeters with the same real LDraw
    Y-down -> spex Y-up flip every other unibrick script applies at output
    time. Returns `(frames, bounds)`, `bounds` being the real min/max
    across *every* frame combined (not just the final one) so a single
    camera framing comfortably contains the scattered start too — same
    "one shared window across all frames" principle as `spex ascii
    --animate`'s turntable orbit (M38)."""
    final = final_translations(stack)
    start = start_translations(stack, final)
    samples = sample_scene_once(stack, color_code, point_count)

    frames = []
    min_c = [math.inf, math.inf, math.inf]
    max_c = [-math.inf, -math.inf, -math.inf]
    for f in range(frame_count):
        t = f / (frame_count - 1) if frame_count > 1 else 1.0
        eased = ease_in_out_cubic(t)
        translations = [
            tuple(s + (fi - s) * eased for s, fi in zip(start[i], final[i])) for i in range(len(stack))
        ]
        frame_points = []
        for placement_idx, (lx, ly, lz), rgb in samples:
            tx, ty, tz = translations[placement_idx]
            x_mm = (lx + tx) * LDU_TO_MM
            y_mm = -(ly + ty) * LDU_TO_MM
            z_mm = (lz + tz) * LDU_TO_MM
            frame_points.append([round(x_mm, 3), round(y_mm, 3), round(z_mm, 3), *rgb])
            for axis, v in enumerate((x_mm, y_mm, z_mm)):
                min_c[axis] = min(min_c[axis], v)
                max_c[axis] = max(max_c[axis], v)
        frames.append(frame_points)
    bounds = {"min": min_c, "max": max_c}
    return frames, bounds


HTML_TEMPLATE = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<title>Klemmbaustein monolith — assembly reveal</title>
<style>
  html, body { margin: 0; height: 100%; background: #0b0e12; overflow: hidden; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
  canvas { display: block; width: 100vw; height: 100vh; cursor: grab; }
  canvas:active { cursor: grabbing; }
  #caption {
    position: absolute; bottom: 16px; left: 16px; color: #e6e6e6; background: rgba(0,0,0,0.55);
    padding: 10px 14px; border-radius: 8px; font-size: 13px; line-height: 1.5; max-width: 60vw; pointer-events: none;
  }
  #caption b { color: #fff; }
</style>
</head>
<body>
<canvas id="c"></canvas>
<div id="caption">
  <b>__TITLE__</b><br/>
  __CAPTION__<br/>
  drag to orbit &middot; scroll to zoom
</div>
<script>
const FRAMES = __FRAMES_JSON__;
const BOUNDS = __BOUNDS_JSON__;
const INTRO_MS = __INTRO_MS__, PLAY_MS = __PLAY_MS__, END_HOLD_MS = __END_HOLD_MS__;

const canvas = document.getElementById('c');
const ctx = canvas.getContext('2d');
function resize() {
  canvas.width = window.innerWidth * devicePixelRatio;
  canvas.height = window.innerHeight * devicePixelRatio;
}
resize();
window.addEventListener('resize', resize);

const center = [
  (BOUNDS.min[0] + BOUNDS.max[0]) / 2,
  (BOUNDS.min[1] + BOUNDS.max[1]) / 2,
  (BOUNDS.min[2] + BOUNDS.max[2]) / 2,
];
const diag = Math.hypot(
  BOUNDS.max[0] - BOUNDS.min[0],
  BOUNDS.max[1] - BOUNDS.min[1],
  BOUNDS.max[2] - BOUNDS.min[2],
) || 1;

let theta = 0.7, phi = 0.35, radius = diag * 1.6;
let dragging = false, lastX = 0, lastY = 0;
let autoRotate = true;

canvas.addEventListener('pointerdown', (e) => { dragging = true; autoRotate = false; lastX = e.clientX; lastY = e.clientY; });
window.addEventListener('pointerup', () => { dragging = false; });
window.addEventListener('pointermove', (e) => {
  if (!dragging) return;
  theta -= (e.clientX - lastX) * 0.006;
  phi = Math.max(-1.4, Math.min(1.4, phi + (e.clientY - lastY) * 0.006));
  lastX = e.clientX; lastY = e.clientY;
});
canvas.addEventListener('wheel', (e) => {
  radius = Math.max(diag * 0.2, Math.min(diag * 6, radius * (1 + e.deltaY * 0.001)));
  e.preventDefault();
}, { passive: false });

function cross(a, b) { return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]; }
function normalize(v) { const l = Math.hypot(v[0], v[1], v[2]) || 1; return [v[0]/l, v[1]/l, v[2]/l]; }

function frameIndexForElapsed(elapsed) {
  const cycle = INTRO_MS + PLAY_MS + END_HOLD_MS;
  const t = elapsed % cycle;
  if (t < INTRO_MS) return 0;
  if (t < INTRO_MS + PLAY_MS) {
    const frac = (t - INTRO_MS) / PLAY_MS;
    return Math.min(FRAMES.length - 1, Math.floor(frac * FRAMES.length));
  }
  return FRAMES.length - 1;
}

let lastTime = performance.now();
function render(now) {
  requestAnimationFrame(render);
  const dt = (now - lastTime) / 1000;
  lastTime = now;
  if (autoRotate) theta += dt * 0.2;

  const camPos = [
    center[0] + radius * Math.cos(phi) * Math.cos(theta),
    center[1] + radius * Math.sin(phi),
    center[2] + radius * Math.cos(phi) * Math.sin(theta),
  ];
  const forward = normalize([center[0]-camPos[0], center[1]-camPos[1], center[2]-camPos[2]]);
  const right = normalize(cross(forward, [0, 1, 0]));
  const up = cross(right, forward);
  const focal = canvas.height / (2 * Math.tan(0.5 * Math.PI / 3));

  ctx.fillStyle = '#0b0e12';
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  const frame = FRAMES[frameIndexForElapsed(now)];
  const projected = [];
  for (const p of frame) {
    const dx = p[0]-camPos[0], dy = p[1]-camPos[1], dz = p[2]-camPos[2];
    const vx = dx*right[0]+dy*right[1]+dz*right[2];
    const vy = dx*up[0]+dy*up[1]+dz*up[2];
    const vz = dx*forward[0]+dy*forward[1]+dz*forward[2];
    if (vz <= 1) continue;
    const scale = focal / vz;
    const sx = canvas.width/2 + vx*scale;
    const sy = canvas.height/2 - vy*scale;
    projected.push([sx, sy, vz, p[3], p[4], p[5]]);
  }
  projected.sort((a, b) => b[2] - a[2]);
  const pr = Math.max(1.1, diag * 0.01 * devicePixelRatio);
  for (const [sx, sy, vz, r, g, b] of projected) {
    ctx.fillStyle = `rgb(${r},${g},${b})`;
    ctx.beginPath();
    ctx.arc(sx, sy, Math.max(1, pr * (diag * 0.6 / vz)), 0, Math.PI * 2);
    ctx.fill();
  }
}
requestAnimationFrame(render);
</script>
</body>
</html>
"""


def render_assembly_html(frames, bounds, title, caption):
    """Plain token substitution rather than `%`-style/`str.format`
    templating — the embedded CSS/JS is full of literal `%` (percentages,
    the modulo operator) and `{}` (every JS block), both of which collide
    with those formatting mini-languages. `__TOKEN__` markers can't
    collide with real HTML/CSS/JS syntax."""
    html = HTML_TEMPLATE
    for token, value in (
        ("__TITLE__", title),
        ("__CAPTION__", caption),
        ("__FRAMES_JSON__", json.dumps(frames, separators=(",", ":"))),
        ("__BOUNDS_JSON__", json.dumps(bounds, separators=(",", ":"))),
        ("__INTRO_MS__", str(int(INTRO_HOLD_SECONDS * 1000))),
        ("__PLAY_MS__", str(int(PLAY_SECONDS * 1000))),
        ("__END_HOLD_MS__", str(int(END_HOLD_SECONDS * 1000))),
    ):
        html = html.replace(token, value)
    return html


def main():
    point_count = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_POINT_COUNT
    frame_count = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_FRAME_COUNT
    out_path = sys.argv[3] if len(sys.argv) > 3 else "out/monolith-assembly.html"

    random.seed(1337)  # reproducible run to run, same spirit as the fixed-seed German-cities-TSP demo

    print(
        f"sampling {point_count} real points once across {len(DEFAULT_STACK)} real parts, "
        f"building {frame_count} real assembly frames...",
        file=sys.stderr,
    )
    frames, bounds = build_frames(DEFAULT_STACK, BLACK_COLOR_CODE, point_count, frame_count)

    html = render_assembly_html(
        frames,
        bounds,
        "Klemmbaustein monolith — assembly reveal",
        "9 real LDraw parts (7&times; Brick 1&times;4, 2&times; Plate 1&times;4), "
        "a stylized convergence reveal, not a physics simulation.",
    )

    os.makedirs(os.path.dirname(out_path) or ".", exist_ok=True)
    with open(out_path, "w") as f:
        f.write(html)

    size_kb = os.path.getsize(out_path) / 1024
    print(f"wrote a real {frame_count}-frame assembly animation ({size_kb:.0f} KB) to {out_path}")


if __name__ == "__main__":
    main()
