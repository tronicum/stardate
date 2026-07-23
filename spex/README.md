explorer to map stuff

## spex — point cloud & tree/graph explorer

CLI + browser viewer for large point clouds, doubling as a generic tree/graph
explorer: real-world trees (network paths, process trees, package
dependencies, ...) get captured into one common format, laid out in 3D, and
rendered through the *same* point-cloud pipeline — a traceroute or a process
tree becomes a "point cloud" of small blobs and connecting trails.

See `docs/ARCHITECTURE.md` for how this actually works (a worked example + the reasoning behind it), `CLAUDE.md` for the terse reference, `TODOs.md` for what's built vs. planned.

### Point cloud pipeline

Converts `.ply`/`.xyz`/`.csv`/`.txt`/`.las`/`.laz` into a streamable octree
tileset, then serves an interactive three.js viewer with LOD streaming.

```sh
cargo build --release

spex info in/cloud.ply                      # inspect an input file
spex convert in/cloud.ply -o out/cloud-tileset   # build a tileset
spex serve out/cloud-tileset                 # serve + open the viewer
```

### Graph/tree pipeline

Input adapters capture a real tree into a small JSON format
(`id`/`label`/`parent`/`metric`/`metadata` — formally specified in
`spec/graph.schema.json`), viewable three ways: terminal (ASCII tree), web
(the same point-cloud viewer, hover a blob for its label), or machine (the
JSON itself).

```sh
# capture (pick one)
spex trace <host> -o demos/my-trace/graph.json                  # real traceroute
spex ps-tree --root <pid> -o demos/my-ps/graph.json              # real process tree (scope with --root; a bare run covers everything but is capped/collapsed for legibility)
spex brew-deps <formula> -o demos/my-deps/graph.json             # real `brew deps --tree`
spex molecule benzene -o demos/my-molecule/graph.json            # real SMILES parsing (built-in molecules, or any SMILES string)

# view
spex graph-print demos/my-trace/graph.json                       # terminal view
spex graph-layout demos/my-trace/graph.json -o demos/my-trace/tileset  # build the web view's tileset
spex serve demos/my-trace/tileset                                 # web view
spex ascii demos/my-trace/tileset                                 # colored ASCII-art snapshot, no browser needed
spex ascii demos/my-trace/tileset --animate                       # same, but a turntable-orbit animation (terminal or --out <file.html>)

spex demos                                                        # list what's captured and how to view each
spex gallery                                                      # web front page: every demo as a card, click into any of them
spex nav                                                          # k9s-style interactive browser: move, enter to view a tree, w to open its web view
```

### Viewer

The viewer (`viewer/`) is a Vite + TypeScript + three.js app, built via `npm run build`
and embedded into the `spex` binary at compile time. After changing viewer source,
rebuild it (`cd viewer && npm run build`) then `cargo build` again to pick up the
new bundle.

### Known limitations

Point clouds: no out-of-core tiling for point clouds too large for memory, no
buffer compression — see `crates/spex-tiler`. LAS/LAZ input is supported but
still loads the whole file into memory like every other format here.
Graphs: tree/forest only (no cycles or shared parents), so e.g. `brew-deps`
duplicates a package under every branch that depends on it rather than
merging it into one node.
