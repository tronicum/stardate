export interface Bounds {
  min: [number, number, number];
  max: [number, number, number];
}

export interface NodeMeta {
  id: string;
  bounds: Bounds;
  pointCount: number;
}

export interface Tileset {
  version: number;
  offset: [number, number, number];
  bounds: Bounds;
  pointCount: number;
  nodes: NodeMeta[];
}

export async function fetchTileset(baseUrl: string): Promise<Tileset> {
  const res = await fetch(`${baseUrl}/tileset.json`);
  if (!res.ok) throw new Error(`failed to fetch tileset.json: ${res.status}`);
  return res.json() as Promise<Tileset>;
}

/** Written by `spex frame-sequence` (see unibrick/gen_monolith_assembly.py):
 * N real point-cloud tilesets, sharing one coordinate offset, meant to be
 * played back as frames of one real animation — e.g. parts converging into
 * an assembly. `frames` are directory names, each fetchable the normal way
 * via `fetchTileset`/`fetchNodePoints` at `${baseUrl}/${frame}`. */
export interface SequenceManifest {
  version: number;
  frameCount: number;
  fps: number;
  frames: string[];
}

/** Returns `null` if sequence.json is absent — the common case (a plain
 * single tileset, or a graph layout). Present only for a real multi-frame
 * point-cloud animation. */
export async function fetchSequence(baseUrl: string): Promise<SequenceManifest | null> {
  const res = await fetch(`${baseUrl}/sequence.json`);
  if (!res.ok) return null;
  return res.json() as Promise<SequenceManifest>;
}

/** Combines several frames' bounds into one — used to pick a single stable
 * camera framing across an entire sequence (a frame's own bounds can
 * legitimately differ a lot in size, e.g. scattered vs. assembled), rather
 * than reframing the camera on every frame swap. */
export function mergeBounds(list: Bounds[]): Bounds {
  const min: [number, number, number] = [Infinity, Infinity, Infinity];
  const max: [number, number, number] = [-Infinity, -Infinity, -Infinity];
  for (const b of list) {
    for (let i = 0; i < 3; i++) {
      min[i] = Math.min(min[i], b.min[i]);
      max[i] = Math.max(max[i], b.max[i]);
    }
  }
  return { min, max };
}

/** Optional per-node metadata written by `spex graph-layout` (absent for plain
 * point-cloud tilesets from `spex convert`) — the human-readable companion to
 * the raw points: each tree node's label, metric, and source metadata. */
export interface NodeLabel {
  id: string;
  label: string;
  parent: string | null;
  center: [number, number, number];
  metric: number | null;
  metadata: Record<string, unknown>;
}

/** Returns `[]` if nodes.json is absent, so plain point-cloud tilesets work unchanged. */
export async function fetchNodeLabels(baseUrl: string): Promise<NodeLabel[]> {
  const res = await fetch(`${baseUrl}/nodes.json`);
  if (!res.ok) return [];
  return res.json() as Promise<NodeLabel[]>;
}

/** Optional whole-graph description written by `spex graph-layout` (absent
 * for plain point-cloud tilesets) — what this scene is and what the color
 * gradient means, for a persistent on-screen header/legend. */
export interface GraphMeta {
  title: string | null;
  metricLabel: string | null;
  nodeCount: number;
  metricMin: number | null;
  metricMax: number | null;
}

/** Returns `null` if meta.json is absent, so plain point-cloud tilesets work unchanged. */
export async function fetchGraphMeta(baseUrl: string): Promise<GraphMeta | null> {
  const res = await fetch(`${baseUrl}/meta.json`);
  if (!res.ok) return null;
  return res.json() as Promise<GraphMeta>;
}

export interface NodePoints {
  positions: Float32Array;
  colors: Uint8Array;
  count: number;
}

/** Parses the node binary format written by spex-tiler: u32 LE count, then
 * per point 3x f32 LE position + 3x u8 RGB (15 bytes/point). */
export async function fetchNodePoints(baseUrl: string, id: string): Promise<NodePoints> {
  const res = await fetch(`${baseUrl}/octree/${id}.bin`);
  if (!res.ok) throw new Error(`failed to fetch node ${id}: ${res.status}`);
  const buf = await res.arrayBuffer();
  const view = new DataView(buf);
  const count = view.getUint32(0, true);
  const positions = new Float32Array(count * 3);
  const colors = new Uint8Array(count * 3);
  let offset = 4;
  for (let i = 0; i < count; i++) {
    positions[i * 3 + 0] = view.getFloat32(offset, true);
    offset += 4;
    positions[i * 3 + 1] = view.getFloat32(offset, true);
    offset += 4;
    positions[i * 3 + 2] = view.getFloat32(offset, true);
    offset += 4;
    colors[i * 3 + 0] = view.getUint8(offset);
    offset += 1;
    colors[i * 3 + 1] = view.getUint8(offset);
    offset += 1;
    colors[i * 3 + 2] = view.getUint8(offset);
    offset += 1;
  }
  return { positions, colors, count };
}
