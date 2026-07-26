/** Loading side of the mesh bundle written by `spex mesh-part` /
 * `spex mesh-model` (see `crates/spex-mesh/src/bundle.rs` and
 * `spec/mesh.schema.json`): one small JSON manifest plus tightly packed
 * little-endian binary buffers.
 *
 * Two things this file must not re-do, because the writer already did them
 * exactly once and says so in the manifest:
 *
 *  - Positions are **millimetres, +Y up**. LDraw's own frame is LDU/Y-down,
 *    and negating Y is a mirror — so the writer also reversed every
 *    triangle's winding. That is why the renderer can leave backface culling
 *    on. A visible interior surface means the *writer* is wrong, not this.
 *  - Colours are **linear**, not sRGB. three.js r152+ treats raw component
 *    values as linear, so converting again here would make every material
 *    about 2.2x too bright.
 */

import type { Bounds } from '../tileset';

export interface Submesh {
  /** Index into `materials[]`, or `null` for LDraw colour 16 ("inherit") —
   * meaning: take the *instance's* own material. A number is a fixed accent
   * colour molded into the part itself. */
  material: number | null;
  indexOffset: number;
  indexCount: number;
}

export interface MeshPart {
  index: number;
  partFile: string;
  description: string | null;
  vertexCount: number;
  triangleCount: number;
  hardEdgeCount: number;
  conditionalEdgeCount: number;
  bounds: Bounds;
  buffers: {
    position: string;
    normal: string;
    index: string;
    hardEdge: string;
    condEdge: string;
  };
  submeshes: Submesh[];
  sources: string[];
  license: string | null;
  author: string | null;
}

export interface MeshMaterial {
  colorCode: number;
  name: string;
  /** LINEAR rgb, 0..1. Do not convert. */
  baseColor: [number, number, number];
  /** LINEAR rgb, 0..1 — LDraw's own edge colour for this material, used by
   * M57's line pass. */
  edgeColor: [number, number, number];
}

export interface InstanceEncoding {
  stride: number;
  layout: string[];
  translationUnitMm: number;
  maxTranslationErrorMm: number;
  count: number;
  file: string;
}

export interface MeshBundle {
  version: number;
  generator: string;
  unit: string;
  upAxis: string;
  colorSpace: string;
  creaseDegrees: number;
  bounds: Bounds;
  parts: MeshPart[];
  materials: MeshMaterial[];
  /** Distinct row-major 3x3 orientations, already in the output frame. */
  orientations: number[][];
  instanceEncoding: InstanceEncoding;
  instanceIds: string[];
  attribution: { geometrySource: string; colorTable: string; note: string };
}

/** Returns `null` when `mesh.json` is absent — which is every existing
 * point-cloud and graph tileset. That absence is the whole mode switch: no
 * existing demo changes behaviour, because for those this fetch 404s and the
 * viewer takes the path it always took. Same shape as `fetchSequence`. */
export async function fetchMeshBundle(baseUrl: string): Promise<MeshBundle | null> {
  const res = await fetch(`${baseUrl}/mesh.json`);
  if (!res.ok) return null;
  const bundle = (await res.json()) as MeshBundle;
  if (bundle?.version !== 1) {
    // Loud rather than silent: a bundle we can't read is a build problem, and
    // falling through to the point path would hide it behind an empty screen.
    throw new Error(`unsupported mesh bundle version ${bundle?.version}`);
  }
  if (bundle.colorSpace !== 'linear') {
    throw new Error(`mesh bundle declares colorSpace "${bundle.colorSpace}"; this viewer only reads linear`);
  }
  return bundle;
}

export interface PartBuffers {
  position: Float32Array;
  normal: Float32Array;
  index: Uint32Array;
  /** 6 floats per edge: two endpoints. Unused until M57. */
  hardEdge: Float32Array;
  /** 12 floats per edge: two endpoints then two control points. Unused until M57. */
  condEdge: Float32Array;
}

async function fetchTyped<T>(
  baseUrl: string,
  path: string,
  ctor: new (b: ArrayBuffer) => T,
): Promise<T> {
  const res = await fetch(`${baseUrl}/${path}`);
  if (!res.ok) throw new Error(`failed to fetch ${path}: ${res.status}`);
  return new ctor(await res.arrayBuffer());
}

/** Fetches every part's buffers in parallel. These are read straight into
 * typed arrays and handed to WebGL unchanged — no per-element JS loop and no
 * intermediate JSON, which is the entire reason the format is binary. */
export async function fetchMeshBuffers(
  baseUrl: string,
  bundle: MeshBundle,
): Promise<Map<number, PartBuffers>> {
  const entries = await Promise.all(
    bundle.parts.map(async (part): Promise<[number, PartBuffers]> => {
      const [position, normal, index, hardEdge, condEdge] = await Promise.all([
        fetchTyped(baseUrl, part.buffers.position, Float32Array),
        fetchTyped(baseUrl, part.buffers.normal, Float32Array),
        fetchTyped(baseUrl, part.buffers.index, Uint32Array),
        fetchTyped(baseUrl, part.buffers.hardEdge, Float32Array),
        fetchTyped(baseUrl, part.buffers.condEdge, Float32Array),
      ]);
      if (position.length !== part.vertexCount * 3) {
        throw new Error(
          `${part.partFile}: position buffer holds ${position.length / 3} vertices, manifest says ${part.vertexCount}`,
        );
      }
      if (index.length !== part.triangleCount * 3) {
        throw new Error(
          `${part.partFile}: index buffer holds ${index.length / 3} triangles, manifest says ${part.triangleCount}`,
        );
      }
      return [part.index, { position, normal, index, hardEdge, condEdge }];
    }),
  );
  return new Map(entries);
}

/** One placement, decoded from the 10-byte record. */
export interface MeshInstance {
  part: number;
  material: number;
  /** Millimetres, output frame. */
  translation: [number, number, number];
  orientation: number;
  id: string;
}

/** Decodes `instances.bin`: `i16 x, i16 y, i16 z, u8 orientation, u8 material,
 * u16 part`, little-endian, 10 bytes each.
 *
 * Translations are stored as integer counts of one LDraw unit
 * (`translationUnitMm`, 0.4 mm) rather than floats — exact for grid-legal
 * geometry, a quarter of the size, and the manifest records the largest
 * rounding error any instance actually incurred so an off-grid model is
 * visible instead of quietly approximated. */
export function decodeInstances(buffer: ArrayBuffer, bundle: MeshBundle): MeshInstance[] {
  const enc = bundle.instanceEncoding;
  const expected = enc.count * enc.stride;
  if (buffer.byteLength !== expected) {
    throw new Error(`instances.bin is ${buffer.byteLength} bytes, manifest implies ${expected}`);
  }
  const view = new DataView(buffer);
  const unit = enc.translationUnitMm;
  const out: MeshInstance[] = [];
  for (let i = 0; i < enc.count; i++) {
    const o = i * enc.stride;
    out.push({
      translation: [
        view.getInt16(o + 0, true) * unit,
        view.getInt16(o + 2, true) * unit,
        view.getInt16(o + 4, true) * unit,
      ],
      orientation: view.getUint8(o + 6),
      material: view.getUint8(o + 7),
      part: view.getUint16(o + 8, true),
      id: bundle.instanceIds[i] ?? `#${i}`,
    });
  }
  return out;
}

export async function fetchMeshInstances(
  baseUrl: string,
  bundle: MeshBundle,
): Promise<MeshInstance[]> {
  const res = await fetch(`${baseUrl}/${bundle.instanceEncoding.file}`);
  if (!res.ok) throw new Error(`failed to fetch ${bundle.instanceEncoding.file}: ${res.status}`);
  return decodeInstances(await res.arrayBuffer(), bundle);
}
