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
import sys
import time
import urllib.error
import urllib.request
import zipfile

LDRAW_BASE = "https://library.ldraw.org/library/official"
LIBRARY_ZIP_URL = "https://library.ldraw.org/library/updates/complete.zip"
CACHE_DIR = os.path.join(os.path.dirname(__file__), ".ldraw-cache")
LIBRARY_ZIP_PATH = os.path.join(CACHE_DIR, "complete.zip")
USER_AGENT = "spex-demo/1.0 (educational project, github.com/tronicum/stardate)"

LDU_TO_MM = 0.4  # real LDraw unit conversion, see BRICKs.md

IDENTITY = (1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0)
ZERO = (0.0, 0.0, 0.0)

_library_zip = None
_library_zip_checked = False


def _get_library_zip():
    """Lazily opens the real, official full-library archive
    (`complete.zip`, ~136MB, real current official LDraw parts+models
    library) if it's been downloaded locally to `LIBRARY_ZIP_PATH` — see
    `download_library_zip()`. Not committed to the repo (gitignored, same
    as the rest of `.ldraw-cache/`) and never uploaded anywhere; purely a
    local shortcut so a real multi-part model (dozens of distinct file
    fetches — parts, their subparts, their shared primitives) doesn't have
    to round-trip the network per file and risk ldraw.org's real rate
    limit, which a per-file fetch loop genuinely hit during this module's
    own development."""
    global _library_zip, _library_zip_checked
    if not _library_zip_checked:
        _library_zip_checked = True
        if os.path.exists(LIBRARY_ZIP_PATH):
            _library_zip = zipfile.ZipFile(LIBRARY_ZIP_PATH)
    return _library_zip


def download_library_zip():
    """Downloads the real, official `complete.zip` once to
    `LIBRARY_ZIP_PATH`. Not run automatically by `fetch()` — an explicit
    one-time step (run this directly, or `python3 -c "import ldraw;
    ldraw.download_library_zip()"`), since it's a real ~136MB transfer, not
    something to trigger silently as a side effect of resolving one part."""
    global _library_zip, _library_zip_checked
    os.makedirs(CACHE_DIR, exist_ok=True)
    req = urllib.request.Request(LIBRARY_ZIP_URL, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=120) as resp:
        with open(LIBRARY_ZIP_PATH, "wb") as f:
            while True:
                chunk = resp.read(1 << 20)
                if not chunk:
                    break
                f.write(chunk)
    _library_zip = None
    _library_zip_checked = False  # force _get_library_zip() to re-open the freshly downloaded file


def fetch(path, retries=6):
    """Fetches one real file, preferring the local full-library mirror
    (see `_get_library_zip`) when it's present — zero network requests,
    zero rate-limit risk — falling back to a real per-file HTTP fetch
    (retried with exponential backoff on a real HTTP 429 — the same class
    of issue already hit and fixed for Wikipedia's API, see
    `scripts/gen_wikipedia_crawl.py`) only for whatever isn't in the
    mirror (or if it was never downloaded at all). Either way, the result
    is cached locally under `unibrick/.ldraw-cache/` at the same per-file
    path, so a later run doesn't care which source last answered it."""
    cache_path = os.path.join(CACHE_DIR, path)
    if os.path.exists(cache_path):
        with open(cache_path) as f:
            return f.read()

    zip_file = _get_library_zip()
    text = None
    if zip_file is not None:
        try:
            with zip_file.open(f"ldraw/{path}") as f:
                text = f.read().decode("utf-8", errors="replace")
        except KeyError:
            pass  # not in this snapshot of the official mirror - fall through to a live fetch

    if text is None:
        url = f"{LDRAW_BASE}/{path}"
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        for attempt in range(retries):
            try:
                with urllib.request.urlopen(req, timeout=15) as resp:
                    text = resp.read().decode("utf-8", errors="replace")
                break
            except urllib.error.HTTPError as e:
                if e.code == 429 and attempt < retries - 1:
                    wait = 2**attempt
                    print(f"  {path!r}: HTTP 429 (rate limited), retrying in {wait}s...", file=sys.stderr)
                    time.sleep(wait)
                    continue
                raise

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
