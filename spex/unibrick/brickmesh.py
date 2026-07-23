"""The `spex-brick-mesh` intermediate format: a real LDraw part's geometry,
resolved once (network fetch + recursive reference-tree walk, both real —
see `ldraw.py`) and cached as plain JSON so re-generating the same part at a
different color or point density never re-walks the real LDraw reference
tree again. See `BRICKs.md`'s "next real design step" note (now this file)
and `spec/brickmesh.schema.json` for the formal shape.

A mesh is one part, resolved in its own local, untransformed LDraw frame
(LDU units, matrix=identity, translation=zero) — exactly what
`ldraw.resolve_geometry(part, IDENTITY, ZERO, ...)` already produces. Placing
several meshes into an assembly (see `gen_monolith_demo.py`) is a cheap
`place_mesh()` translation of already-resolved triangles, not a re-fetch —
this is what lets a 9-part stack resolve only the 2 *distinct* real part
files it actually uses, no matter how many times each is placed.
"""
import json
import os

from ldraw import IDENTITY, ZERO, fetch, load_ldraw_colors, part_description, resolve_geometry

MESH_VERSION = 1
ATTRIBUTION = (
    "Geometry resolved from the LDraw Parts Library (https://ldraw.org), "
    "CC BY 2.0 / CCAL 2.0. Not an official LDraw or LEGO product."
)

MESH_CACHE_DIR = os.path.join(os.path.dirname(__file__), ".ldraw-cache", "meshes")


def resolve_part_mesh(part_file, color_code):
    """Resolves one real LDraw part into a `spex-brick-mesh` dict — the real
    network fetch + recursive reference-tree walk happens here, exactly
    once, regardless of how many times the caller later places/recolors/
    resamples the result."""
    triangles = []
    resolve_geometry(part_file, IDENTITY, ZERO, color_code, triangles)
    return {
        "version": MESH_VERSION,
        "sourcePart": part_file,
        "sourceDescription": part_description(part_file),
        "requestedColorCode": color_code,
        "units": "ldu",
        "attribution": ATTRIBUTION,
        "triangles": [
            {"vertices": [list(v) for v in verts], "colorCode": code}
            for verts, code in triangles
        ],
    }


def validate_mesh(mesh):
    """A hand-rolled structural check against `spec/brickmesh.schema.json`
    — not a full JSON-Schema validator (no `jsonschema` dependency added
    just for this), but enough to catch a malformed mesh before it's
    written or consumed. Raises ValueError on the first problem found."""
    required = ("version", "sourcePart", "requestedColorCode", "units", "attribution", "triangles")
    for key in required:
        if key not in mesh:
            raise ValueError(f"brickmesh missing required field {key!r}")
    if mesh["version"] != MESH_VERSION:
        raise ValueError(f"brickmesh version {mesh['version']!r} != supported {MESH_VERSION!r}")
    if mesh["units"] != "ldu":
        raise ValueError(f"brickmesh units {mesh['units']!r} != 'ldu'")
    if not isinstance(mesh["triangles"], list) or not mesh["triangles"]:
        raise ValueError("brickmesh triangles must be a non-empty list")
    for i, tri in enumerate(mesh["triangles"]):
        verts = tri.get("vertices")
        if not isinstance(verts, list) or len(verts) != 3:
            raise ValueError(f"brickmesh triangle {i} must have exactly 3 vertices")
        for v in verts:
            if not isinstance(v, list) or len(v) != 3:
                raise ValueError(f"brickmesh triangle {i} has a vertex that isn't [x, y, z]")
        if "colorCode" not in tri:
            raise ValueError(f"brickmesh triangle {i} missing colorCode")


def save_mesh(mesh, path):
    validate_mesh(mesh)
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    with open(path, "w") as f:
        json.dump(mesh, f)


def load_mesh(path):
    with open(path) as f:
        mesh = json.load(f)
    validate_mesh(mesh)
    return mesh


def _mesh_cache_path(part_file, color_code):
    safe_part = part_file.replace("/", "_")
    return os.path.join(MESH_CACHE_DIR, f"{safe_part}__c{color_code}.json")


def get_or_resolve_mesh(part_file, color_code):
    """The real "resolve once" entry point: returns a cached mesh from disk
    if this exact (part, requested color) was already resolved by a
    previous run, otherwise resolves it for real over the network (via
    `ldraw.py`) and caches the result for next time."""
    cache_path = _mesh_cache_path(part_file, color_code)
    if os.path.exists(cache_path):
        return load_mesh(cache_path)
    mesh = resolve_part_mesh(part_file, color_code)
    save_mesh(mesh, cache_path)
    return mesh


def mesh_triangles(mesh):
    """Converts a mesh's JSON-friendly (list-based) triangles back into the
    `[(triangle_verts_as_tuples, color_code), ...]` shape `sampling.py`
    expects — the same shape `ldraw.resolve_geometry` produces directly."""
    return [
        (tuple(tuple(v) for v in tri["vertices"]), tri["colorCode"])
        for tri in mesh["triangles"]
    ]


def place_mesh(mesh, translation=(0.0, 0.0, 0.0), recolor_to=None):
    """Places a resolved mesh's triangles at `translation` (a plain vertex
    offset — a mesh is always stored axis-aligned/untransformed in its own
    local LDraw frame, so placement here is only ever a translation, not a
    full matrix; rotating a placed part is a real future extension, not
    needed by anything built so far). If `recolor_to` is given, any
    triangle whose color is the mesh's own `requestedColorCode` (i.e. was
    LDraw color 16, "inherit the part's overall color," at resolve time) is
    remapped to the new color — a real, honest approximation: it recolors
    whatever was "the part's own color," and leaves any genuinely
    fixed/accent-colored triangles (a minority on most simple parts) alone,
    rather than fabricating a full re-resolve against a different color."""
    tx, ty, tz = translation
    base_code = mesh["requestedColorCode"]
    placed = []
    for verts, code in mesh_triangles(mesh):
        moved = tuple((x + tx, y + ty, z + tz) for x, y, z in verts)
        out_code = recolor_to if (recolor_to is not None and code == base_code) else code
        placed.append((moved, out_code))
    return placed


def merge_colors(*color_tables):
    """Merges several `load_ldraw_colors()`-shaped dicts (in practice
    they're all the same real official LDConfig.ldr, so this just needs to
    dedupe cheaply — kept as a real function rather than inlined `dict`
    unpacking at each call site, since it documents the intent)."""
    merged = {}
    for table in color_tables:
        merged.update(table)
    return merged


__all__ = [
    "MESH_VERSION",
    "ATTRIBUTION",
    "resolve_part_mesh",
    "validate_mesh",
    "save_mesh",
    "load_mesh",
    "get_or_resolve_mesh",
    "mesh_triangles",
    "place_mesh",
    "merge_colors",
    "load_ldraw_colors",
    "fetch",
]
