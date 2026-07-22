# How spex works, and why

`CLAUDE.md` is the terse reference (commands, file-by-file responsibilities).
This document is the narrative: what problem this solves, how data actually
flows through it end to end with a real example, and — for the parts that
aren't obvious — why they're built the way they are.

## The idea

spex started as a literal point cloud explorer: turn a LiDAR/scan file
(PLY/XYZ) into a streamable, level-of-detail 3D scene you can fly around in a
browser. The generalization came from one observation: **any tree can become
a point cloud.** Give each node in a tree a small cluster of points ("blob")
positioned in 3D space, and each edge a thin trail of points connecting two
blobs, and you've turned an abstract structure — a network path, a process
tree, a package dependency graph, a directory's disk usage — into exactly the
same kind of data a LiDAR scanner produces. The renderer never needs to know
the difference.

That's the whole architecture in one sentence: **input adapters turn
real-world trees into points; one shared pipeline turns points into an
explorable 3D scene.** Nothing downstream of "here is a list of points" cares
where they came from.

## Why points, specifically

Three reasons this pays off rather than being a neat trick:
1. The octree tiler and LOD-streaming viewer already existed for real point
   clouds — reusing them for graphs was free.
2. Points scale trivially (millions of them stream and cull cheaply); a
   graph with a pathologically wide node (see the fan-out cap, below) doesn't
   need special-case rendering, just more points.
3. Because *only* points are used — no lines, no meshes — adding a new kind
   of data source never requires touching the renderer. An adapter's entire
   job is to produce a `spex_graph::Graph`; everything after that is shared
   code that was written once.

## Walking the pipeline with a real example

`spex trace www.de-cix.net -o demos/decix-trace/graph.json` runs a real
traceroute and writes the **intermediate format** — this is the one thing
every adapter produces and every view consumes:

```json
{
  "title": "traceroute to www.de-cix.net",
  "metric_label": "avg RTT (ms)",
  "nodes": [
    { "id": "source", "label": "you", "parent": null, "metric": null, "metadata": {} },
    { "id": "hop-1", "label": "fritz.box", "parent": "source", "metric": 4.975,
      "metadata": { "hop": 1, "hostname": "fritz.box", "ip": "192.168.178.1", "rttSamplesMs": [5.91, 4.706, 4.309] } }
  ]
}
```

`title`/`metric_label` describe the whole graph (what is this, what does the
number mean); each node has a `parent` (trees/forests only — see
"non-goals" below), an optional `metric` (whatever numeric weight matters —
here, average round-trip time), and free-form `metadata` carrying whatever
the adapter captured beyond that.

At this point you can already understand the data with zero further steps:
`spex graph-print demos/decix-trace/graph.json` renders it as a colored
ASCII tree with a header and a summary footer — this is a deliberate second
first-class view, not an afterthought (more in "three views," below).

`spex graph-layout` turns the graph into points (`spex_graph::layout::build()`):
- **Position**: a recursive radial placement. The root sits at the origin;
  each depth level is a wider, higher ring (`radius = depth * 8`,
  `z = depth * 4`); siblings split their parent's angular slice evenly, with
  a small per-node jitter (seeded from a hash of the node's id, so it's
  reproducible) so that a plain chain — like this traceroute, one hop after
  another — spirals upward instead of sitting on a flat ray.
- **Color**: each node's `metric` is normalized against the graph's min/max
  and mapped through a blue → yellow → red gradient. The slowest hop in this
  trace (19.6ms) renders red; the fastest (5.0ms) renders blue.
- **Points**: each node becomes ~300 points scattered in a small jittered
  sphere (a "blob"); each edge becomes ~60 points interpolated between the
  two blobs' centers, dimmed, forming a visible trail. This is how a tree's
  *structure* survives being flattened into an undifferentiated point list —
  it's encoded entirely in position and color, not in any special "this is a
  tree" data structure the renderer has to understand.

The result — a `Vec<Point>` plus a `Vec<LayoutNodeInfo>` (each node's
resolved position, for labels) — goes into `spex_tiler::build()`, the exact
same function a real LiDAR file would go through: recursively partition by
bounding-box octant, reservoir-sample a fixed point budget per node for that
LOD level, write `tileset.json` + `octree/<node-id>.bin`. For a 10-node trace
(3,540 points) this produces a single octree node — LOD only kicks in once a
tileset exceeds the per-node budget, e.g. the full 1,000+ process `ps-tree`
demo. The CLI writes two more small files alongside it: `nodes.json` (each
node's position + label + metric + metadata, for the viewer's hover
tooltips) and `meta.json` (the graph's title/metric range, for the viewer's
always-visible header/legend).

`spex serve` (one demo) or `spex gallery` (all of them, with a front-page
index) then just... serves those files. The browser viewer streams octree
nodes with a priority-queue LOD selector, same as it would for a real scan.

## Design decisions, and why

**Radial placement, not force-directed.** A physics-simulated layout (nodes
repel, edges attract, settle into equilibrium) would look organic but is
iterative, non-deterministic across runs unless carefully seeded, and adds
real complexity for uncertain visual benefit on trees (as opposed to general
graphs, where force-directed layouts earn their keep). A tree already has an
unambiguous "distance from root" — radius encodes it directly, cheaply, and
deterministically. Two runs of `spex graph-layout` on the same `graph.json`
produce byte-identical output.

**The fan-out cap exists because something broke.** The first real
`ps-tree` capture (no `--root` scoping) found that `launchd` has 926 direct
children on this machine. Splitting one angular ring 926 ways produced a
solid, unreadable ring, and framerate dropped from 100+ fps to single
digits. The fix — cap any node at 20 visible children, collapse the rest
into one synthetic "+N more" sibling, ranked by `metric` so the heaviest
children survive — lives in the shared layout code, not in `ps-tree`
specifically, so *any* future adapter that hands the layout a
high-fan-out node gets the same protection automatically. This is the
project's one real "hit a wall, fixed it structurally" moment so far.

**Three views of the same data, not one.** A person staring at a terminal,
a person in a browser, and another program consuming the output all want
different things from the same `Graph`. Rather than pick one and bolt the
others on, `format_tree()` (terminal), the viewer (browser), and the JSON
itself (machine) are three independent renderings of one model — adding
`title`/`metric_label` to `Graph` once made all three simultaneously more
understandable, instead of needing three separate fixes.

**Why `demos/` is gitignored entirely.** Captured graphs contain real data
from whichever machine ran the adapter — real network hops, real process
names and PIDs, a real installed package list. None of that belongs in
version control by default; `scripts/walkthrough.sh` regenerates a full set
of demos on any machine in seconds, so nothing of value is lost by not
committing them.

## Non-goals (on purpose, for now)

- **Trees/forests only** — `GraphNode.parent` is a single optional string,
  not a general edge list. A real dependency graph can have a package
  required by two different things; today that package is duplicated as two
  separate nodes rather than merged into one with two incoming edges.
  Modeling general graphs (cycles, shared parents) is a bigger change to the
  layout algorithm (which assumes a tree when computing angular slices) and
  is intentionally deferred (see `TODOs.md`).
- **No real ICMP probing** — `trace` uses standard UDP traceroute, which
  needs no elevated privileges. True ICMP echo probing would need a raw
  socket (sudo/capabilities) for a marginal difference in the captured path.
- **Points only, no lines** — edges are point-trails, not a real line
  primitive, specifically so the viewer never needs a second rendering path.
