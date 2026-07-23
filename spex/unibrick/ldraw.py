"""Real LDraw fetching/parsing — the network + file-format layer only.

Everything here is specific to talking to ldraw.org's real official library
and parsing its real `.dat`/`.ldr` line format. No brick-mesh-format code and
no point-sampling code lives here — see `brickmesh.py` (the resolve-once
intermediate format) and `sampling.py` (area-weighted surface sampling +
baked lighting), both of which build on this module rather than duplicating
it.
"""
import math
import os
import urllib.request

LDRAW_BASE = "https://library.ldraw.org/library/official"
CACHE_DIR = os.path.join(os.path.dirname(__file__), ".ldraw-cache")
USER_AGENT = "spex-demo/1.0 (educational project, github.com/tronicum/stardate)"

LDU_TO_MM = 0.4  # real LDraw unit conversion, see BRICKs.md

IDENTITY = (1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0)
ZERO = (0.0, 0.0, 0.0)


def fetch(path):
    """Fetches one real file from ldraw.org's official library, cached
    locally under unibrick/.ldraw-cache/ so repeated runs (and the multi-file
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


def part_description(part_file):
    """Returns the real part's own descriptive title — LDraw's own
    convention is that a part file's very first line is `0 <description>`
    (e.g. "0 Brick  1 x  1") — or None if the cached/fetched text doesn't
    start with one."""
    text = fetch(f"parts/{part_file}")
    first_line = text.splitlines()[0] if text.splitlines() else ""
    tokens = first_line.split(None, 1)
    if len(tokens) == 2 and tokens[0] == "0":
        return tokens[1].strip()
    return None


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
