"""Area-weighted surface sampling + baked-in lighting — pure geometry/color
functions with no LDraw-specific or brick-mesh-format-specific code. Takes
any flat `[(triangle_verts, color_code), ...]` list (real, from a resolved
LDraw part, a placed assembly, or someday a completely different mesh
source) and turns it into a colored point cloud. This is also the seam
where a future true mesh/vector renderer would diverge from the point-cloud
pipeline — both would start from the same resolved triangles this module
consumes.
"""
import math
import random

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


def _triangle_area(tri):
    (x0, y0, z0), (x1, y1, z1), (x2, y2, z2) = tri
    ux, uy, uz = x1 - x0, y1 - y0, z1 - z0
    vx, vy, vz = x2 - x0, y2 - y0, z2 - z0
    cx, cy, cz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
    return 0.5 * math.sqrt(cx * cx + cy * cy + cz * cz)


def _triangle_normal(tri):
    (x0, y0, z0), (x1, y1, z1), (x2, y2, z2) = tri
    ux, uy, uz = x1 - x0, y1 - y0, z1 - z0
    vx, vy, vz = x2 - x0, y2 - y0, z2 - z0
    nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
    length = math.sqrt(nx * nx + ny * ny + nz * nz) or 1.0
    return (nx / length, ny / length, nz / length)


def sample_surface(triangles, point_count, colors):
    """Samples `point_count` points across `triangles` (real face area
    weighted, so density stays even regardless of triangle size), and bakes
    each point's real Lambertian+specular shading (see `shade_color`)
    directly into its stored RGB. Returns `[(position, (r,g,b)), ...]`.

    `triangles` is `[(triangle_verts, color_code), ...]` — the same shape
    whether it came straight from `ldraw.resolve_geometry`, a loaded/placed
    `brickmesh`, or a merged multi-part assembly."""
    weights = [_triangle_area(tri) for tri, _ in triangles]
    normals = [_triangle_normal(tri) for tri, _ in triangles]
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
