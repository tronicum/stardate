# Committed real point-cloud test fixtures

## `autzen-trim.las`

A real, small (~200KB, 6,002 points) stride-sampled subset of the Autzen
Stadium LiDAR dataset (Eugene, OR), the same real dataset already used for
M36's LAS/LAZ verification (see `TODOs.md`) but never previously committed
to the repo.

- **Source**: [`PDAL/data`](https://github.com/PDAL/data)'s
  `autzen/autzen.laz` (10,653,336 points, real airborne LiDAR captured by
  Watershed Sciences, Inc. in 2010 for libLAS data-testing purposes).
- **License**: CC BY 4.0 — see [`PDAL/data`'s
  `LICENSE`](https://github.com/PDAL/data/blob/main/LICENSE). Attribution:
  PDAL project / Watershed Sciences, Inc.
- **How it was trimmed**: every 1775th real point (stride-sampled across
  the full file, not just the first N) was kept, using the `las` crate's
  `Reader`/`Writer` — this preserves the dataset's real spatial extent and
  shape (buildings, stadium, terrain) rather than collapsing to a thin
  scan-line strip the way taking only the first N points would.
- **Real coordinates preserved as-is**: Oregon State Plane, US survey
  feet — `spex info` on this file reports real bounds
  `[635586.11, 848902.03, 406.96]` to `[638993.57, 853524.93, 587.86]`,
  matching the source survey's actual extent.

Verified with `spex info`/`spex convert` — both work correctly on real
external LAS data. `spex ascii`'s terminal preview currently renders this
one blank; that's a separate, real finding (not specific to this fixture)
tracked on its own, not a problem with the fixture itself.
