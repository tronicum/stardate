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

/** Must match `spex_mesh::FORMAT_VERSION`. */
export const MESH_FORMAT_VERSION = 2;

export interface Submesh {
  /** Index into `materials[]`, or `null` for LDraw colour 16 ("inherit") —
   * meaning: take the *instance's* own material. A number is a fixed accent
   * colour molded into the part itself. */
  material: number | null;
  indexOffset: number;
  indexCount: number;
}

/** A coarser level of one part (M59). Level 0 is the part's own buffers and
 * counts, not a member of `lods` — so ignoring this field entirely still
 * yields full geometry, which is why adding LODs needed no version bump. */
export interface MeshLod {
  level: 1 | 2;
  vertexCount: number;
  triangleCount: number;
  hardEdgeCount: number;
  conditionalEdgeCount: number;
  buffers: PartBufferPaths;
  submeshes: Submesh[];
}

export interface PartBufferPaths {
  position: string;
  normal: string;
  index: string;
  hardEdge: string;
  condEdge: string;
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
  buffers: PartBufferPaths;
  submeshes: Submesh[];
  sources: string[];
  /** Absent in bundles written before M59. */
  lods?: MeshLod[];
  license: string | null;
  author: string | null;
}

/** The particle layer of a real `MATERIAL SPECKLE` / `MATERIAL GLITTER`
 * colour. Carried through in full; M56 renders only the base material, and
 * the procedural chunk that consumes these lands with the dissolve shader
 * work — a renderer that has it needs these exact numbers, not a guess. */
export interface SpeckleParams {
  /** LINEAR rgb. */
  color: [number, number, number];
  fraction: number;
  minSize?: number;
  maxSize?: number;
  vFraction?: number;
  size?: number;
}

/** PBR parameters resolved by `spex_mesh::material::from_ldraw`.
 *
 * Every number is a calibrated artistic choice rather than a measurement —
 * the reasoning for each lives beside the constant in `material.rs`. The one
 * exception is `ior` 1.53 on transparent parts, which is polycarbonate's real
 * refractive index. */
export interface PbrParams {
  metalness: number;
  roughness: number;
  /** From the real `ALPHA`. Below 1 means genuinely transparent in LDConfig. */
  opacity: number;
  clearcoat: number;
  clearcoatRoughness: number;
  transmission: number;
  ior: number;
  iridescence: number;
  iridescenceIOR: number;
  /** Real `LUMINANCE` / 255 — non-zero only for glow-in-the-dark colours. */
  emissiveIntensity: number;
  speckle?: SpeckleParams;
}

export type Finish =
  | 'solid' | 'chrome' | 'pearlescent' | 'rubber'
  | 'matte_metallic' | 'metal' | 'speckle' | 'glitter';

export interface MeshMaterial {
  colorCode: number;
  name: string;
  /** LINEAR rgb, 0..1. Do not convert. */
  baseColor: [number, number, number];
  /** LINEAR rgb, 0..1 — the colour's own real `EDGE` value, used by M57's
   * line pass. */
  edgeColor: [number, number, number];
  /** The real `LDConfig.ldr` finish keyword. */
  finish: Finish;
  pbr: PbrParams;
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
  if (bundle?.version !== MESH_FORMAT_VERSION) {
    // Loud rather than silent, and specific: a bundle we can't read is a
    // build problem. Saying "rebuild it" here is the difference between a
    // one-line fix and an afternoon inside minified three.js — which is
    // exactly what happened when M56 added required material fields without
    // bumping this number.
    throw new Error(
      `mesh bundle is version ${bundle?.version}, this viewer reads ${MESH_FORMAT_VERSION}. ` +
      `Rebuild it with \`spex mesh-model\`.`,
    );
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
async function fetchLevel(baseUrl: string, paths: PartBufferPaths): Promise<PartBuffers> {
  const [position, normal, index, hardEdge, condEdge] = await Promise.all([
    fetchTyped(baseUrl, paths.position, Float32Array),
    fetchTyped(baseUrl, paths.normal, Float32Array),
    fetchTyped(baseUrl, paths.index, Uint32Array),
    fetchTyped(baseUrl, paths.hardEdge, Float32Array),
    fetchTyped(baseUrl, paths.condEdge, Float32Array),
  ]);
  return { position, normal, index, hardEdge, condEdge };
}

/** Every LOD of every part, keyed `part` for level 0 and `part:level`
 * otherwise. They are all fetched up front and on purpose: an LOD is a few
 * dozen triangles next to a few hundred, so the whole set costs less than one
 * more brick, and a level that arrives mid-dolly is a visible pop. */
export async function fetchMeshLodBuffers(
  baseUrl: string,
  bundle: MeshBundle,
): Promise<Map<string, PartBuffers>> {
  const jobs: Array<Promise<[string, PartBuffers]>> = [];
  for (const part of bundle.parts) {
    for (const lod of part.lods ?? []) {
      jobs.push(fetchLevel(baseUrl, lod.buffers).then((b) => [`${part.index}:${lod.level}`, b]));
    }
  }
  return new Map(await Promise.all(jobs));
}

export async function fetchMeshBuffers(
  baseUrl: string,
  bundle: MeshBundle,
): Promise<Map<number, PartBuffers>> {
  const entries = await Promise.all(
    bundle.parts.map(async (part): Promise<[number, PartBuffers]> => {
      const { position, normal, index, hardEdge, condEdge } = await fetchLevel(baseUrl, part.buffers);
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
