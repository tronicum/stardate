"""The `spex-brick-scene` intermediate format: a real multi-part assembly,
parsed directly from a real, official LDraw *model* file's own type-1
placement lines (`https://library.ldraw.org/library/official/models/*.ldr`
— e.g. `car.ldr`, `pyramid.ldr`, both real official LDraw sample models
authored by James Jessiman, LDraw's original creator) — as opposed to
`gen_monolith_demo.py`'s hand-written stack, this is the real "build
instructions" half of `BRICKs.md`'s plan: real part placements (which real
part, at which real position/orientation) sourced from data LDraw itself
ships, not invented by this project.

A **licensing note, checked directly against ldraw.org rather than
assumed**: individual part files (`parts/*.dat`) each carry an explicit
`0 !LICENSE Redistributable under CCAL version 2.0` header, confirmed
covered by the real "LDraw.org Parts Library Agreement" (CC BY 2.0). The
official *model* files under `models/` (this module's source) carry no
such header, and ldraw.org's own Legal Info page
(https://www.ldraw.org/legal-info.html) doesn't explicitly address
`models/` the way it does `parts/` — so their redistribution status is
genuinely unconfirmed, not just unresearched. This project treats them the
same way as the parts cache: fetched on demand, cached locally under
`.ldraw-cache/` (gitignored, never committed), and only ever used to
*derive* a locally-generated point cloud — never redistributed as a model
file itself. Re-verify at ldraw.org before any real redistribution of the
model files themselves, same standing rule as the parts library.
"""
import json
import os

from ldraw import fetch

SCENE_VERSION = 1
ATTRIBUTION = (
    "Placement data parsed from a real official LDraw sample model "
    "(https://ldraw.org/library/official/models/), authored by James "
    "Jessiman. Unlike LDraw's parts library, these particular model files "
    "carry no explicit CCAL 2.0/CC BY license header — their redistribution "
    "status is unconfirmed, not assumed; see brickscene.py's module "
    "docstring. Referenced part geometry itself remains under the LDraw "
    "Parts Library's own confirmed CCAL 2.0/CC BY 2.0 terms."
)

SCENE_CACHE_DIR = os.path.join(os.path.dirname(__file__), ".ldraw-cache", "scenes")


def fetch_model(model_file):
    """Fetches one real official LDraw sample model file (e.g. `car.ldr`),
    cached the same way `ldraw.fetch()` caches any other real path."""
    return fetch(f"models/{model_file}")


def parse_scene(model_file):
    """Parses a real LDraw model file's own lines directly into a
    `spex-brick-scene` dict — every real type-1 line becomes one
    placement (`partFile`, `colorCode`, `translation`, `matrix`), and each
    real `0 STEP` boundary increments a `buildStep` counter carried on
    every placement added after it (LDraw's own real "build in stages"
    convention — not used for anything yet, but real, free provenance
    worth keeping rather than discarding).

    Deliberately does *not* recurse into each referenced part's own
    geometry (that's `brickmesh.get_or_resolve_mesh`'s job, called once
    per *distinct* placement) — a scene only records *where* real parts
    go, at the granularity the source model file itself uses."""
    text = fetch_model(model_file)
    lines = text.splitlines()

    description = None
    author = None
    for line in lines:
        tokens = line.split(None, 1)
        if len(tokens) != 2 or tokens[0] != "0":
            continue
        rest = tokens[1].strip()
        if rest.startswith("//"):
            continue
        if rest.lower().startswith("author:"):
            author = rest.split(":", 1)[1].strip()
        elif rest.lower().startswith("name:"):
            continue  # just restates the model's own filename
        elif description is None:
            description = rest

    placements = []
    build_step = 0
    for line in lines:
        tokens = line.split()
        if not tokens:
            continue
        if tokens[0] == "0":
            if len(tokens) >= 2 and tokens[1] == "STEP":
                build_step += 1
            continue
        if tokens[0] != "1":
            continue
        # 1 <colour> x y z a b c d e f g h i <file>
        color_code = int(tokens[1])
        nums = [float(t) for t in tokens[2:14]]
        placements.append(
            {
                "partFile": " ".join(tokens[14:]),
                "colorCode": color_code,
                "translation": nums[0:3],
                "matrix": nums[3:12],
                "buildStep": build_step,
            }
        )

    return {
        "version": SCENE_VERSION,
        "sourceModel": model_file,
        "sourceDescription": description,
        "sourceAuthor": author,
        "units": "ldu",
        "attribution": ATTRIBUTION,
        "placements": placements,
    }


def validate_scene(scene):
    """A hand-rolled structural check against `spec/brickscene.schema.json`
    — same rationale as `brickmesh.validate_mesh`: no `jsonschema`
    dependency added just for this, but enough to catch a malformed scene
    before it's written or consumed. Raises ValueError on the first
    problem found."""
    required = ("version", "sourceModel", "units", "attribution", "placements")
    for key in required:
        if key not in scene:
            raise ValueError(f"brickscene missing required field {key!r}")
    if scene["version"] != SCENE_VERSION:
        raise ValueError(f"brickscene version {scene['version']!r} != supported {SCENE_VERSION!r}")
    if scene["units"] != "ldu":
        raise ValueError(f"brickscene units {scene['units']!r} != 'ldu'")
    if not isinstance(scene["placements"], list) or not scene["placements"]:
        raise ValueError("brickscene placements must be a non-empty list")
    for i, p in enumerate(scene["placements"]):
        for key in ("partFile", "colorCode", "translation", "matrix", "buildStep"):
            if key not in p:
                raise ValueError(f"brickscene placement {i} missing {key!r}")
        if len(p["translation"]) != 3:
            raise ValueError(f"brickscene placement {i} translation must have 3 components")
        if len(p["matrix"]) != 9:
            raise ValueError(f"brickscene placement {i} matrix must have 9 components")


def save_scene(scene, path):
    validate_scene(scene)
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    with open(path, "w") as f:
        json.dump(scene, f)


def load_scene(path):
    with open(path) as f:
        scene = json.load(f)
    validate_scene(scene)
    return scene


def _scene_cache_path(model_file):
    return os.path.join(SCENE_CACHE_DIR, model_file.replace("/", "_") + ".json")


def get_or_parse_scene(model_file):
    """The real "resolve once" entry point for scenes, mirroring
    `brickmesh.get_or_resolve_mesh`: returns a cached parsed scene if this
    model was already parsed by a previous run, otherwise parses it (real
    network fetch via `fetch_model`, cached raw text too) and caches the
    parsed result."""
    cache_path = _scene_cache_path(model_file)
    if os.path.exists(cache_path):
        return load_scene(cache_path)
    scene = parse_scene(model_file)
    save_scene(scene, cache_path)
    return scene


__all__ = [
    "SCENE_VERSION",
    "ATTRIBUTION",
    "fetch_model",
    "parse_scene",
    "validate_scene",
    "save_scene",
    "load_scene",
    "get_or_parse_scene",
]
