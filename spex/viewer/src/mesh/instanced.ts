/** M55 — instanced rendering.
 *
 * M54 drew one `THREE.Mesh` per placement. That is fine for a car and fatal
 * for the Atlas: tens of thousands of bricks means tens of thousands of draw
 * calls, and the GPU spends its whole frame being told what to do rather than
 * doing it. Here every placement of the same part in the same colour collapses
 * into one `InstancedMesh`, so the call count stops depending on how many
 * bricks there are and starts depending only on how many *kinds* there are.
 *
 * The grouping key is `(part, material)` and not `part` alone, because a
 * placement's colour is a material binding, not vertex data. That is also why
 * `spex-mesh` keeps geometry colour-neutral (LDraw code 16 stays unresolved):
 * one part's buffers serve every colour it is ever placed in, and the same
 * vertex data is uploaded once no matter how many groups reference it.
 *
 * Sharing works because three.js keys its GPU buffers on the `BufferAttribute`
 * object, not on the `BufferGeometry` that holds it. Several geometries built
 * from the *same* attribute instances therefore share one VBO — which is what
 * lets each group carry its own per-instance attributes without duplicating a
 * single vertex.
 */

import * as THREE from 'three';
import type { MeshBundle, MeshInstance, PartBuffers } from './bundle';
import type { MaterialLibrary } from './materials';

export interface InstanceGroup {
  part: number;
  material: number;
  /** Level 0. Still the mesh everything else refers to. */
  mesh: THREE.InstancedMesh;
  /** One `InstancedMesh` per available level, index = level. Length 1 for a
   * bundle written before M59, in which case nothing below ever runs. */
  lodMeshes: THREE.InstancedMesh[];
  /** **The authoritative transforms**, 16 floats per instance, indexed by
   * instance. The LOD meshes' own `instanceMatrix` buffers are re-packed
   * copies whose row order changes every time an instance switches level —
   * so anything that needs "the matrix of instance i" (M57's edge texture,
   * M60's choreography) reads this, never a LOD mesh. */
  matrices: Float32Array;
  /** Per instance, the level it is currently drawn at. */
  levels: Uint8Array;
  /** Part bounding-sphere radius in world units, for the projected-size test. */
  radius: number;
  /** Instance slot -> the bundle's own stable id, so a show's choreography
   * can resolve its target globs to slots once, at load time. */
  ids: string[];
  /** Per-instance 0..1 scalar, uploaded as `aDissolve`. Nothing reads it
   * until M65's dissolve shader; it exists now so that milestone is a
   * shader change and not a buffer-layout change. */
  dissolve: THREE.InstancedBufferAttribute;
}

/** Where one stable instance id lives after grouping. */
interface Slot {
  group: InstanceGroup;
  index: number;
}

/** Shared vertex data for one part — built once, referenced by every group
 * that draws that part in any colour. */
function baseAttributes(b: PartBuffers) {
  return {
    position: new THREE.BufferAttribute(b.position, 3),
    normal: new THREE.BufferAttribute(b.normal, 3),
    index: new THREE.BufferAttribute(b.index, 1),
  };
}

export function buildInstanceGroups(
  bundle: MeshBundle,
  buffers: Map<number, PartBuffers>,
  materials: MaterialLibrary,
  instances: MeshInstance[],
): InstanceGroup[] {
  // Pass 1: how many instances land in each (part, material) bucket. Sizes
  // have to be known before any InstancedMesh can be allocated, and counting
  // first is cheaper than growing 50 000 times.
  const buckets = new Map<string, MeshInstance[]>();
  for (const inst of instances) {
    const key = `${inst.part}:${inst.material}`;
    const bucket = buckets.get(key);
    if (bucket) bucket.push(inst);
    else buckets.set(key, [inst]);
  }

  const shared = new Map<number, ReturnType<typeof baseAttributes>>();
  const groups: InstanceGroup[] = [];
  const m = new THREE.Matrix4();

  for (const [key, bucket] of buckets) {
    const [partIdx, materialIdx] = key.split(':').map(Number);
    const part = bundle.parts[partIdx];
    const bufs = buffers.get(partIdx);
    if (!part || !bufs) throw new Error(`no geometry for part ${partIdx}`);

    if (!shared.has(partIdx)) shared.set(partIdx, baseAttributes(bufs));
    const attrs = shared.get(partIdx)!;

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', attrs.position);
    geometry.setAttribute('normal', attrs.normal);
    geometry.setIndex(attrs.index);
    part.submeshes.forEach((s, i) => geometry.addGroup(s.indexOffset, s.indexCount, i));
    geometry.boundingBox = new THREE.Box3(
      new THREE.Vector3(...part.bounds.min),
      new THREE.Vector3(...part.bounds.max),
    );
    geometry.boundingSphere = geometry.boundingBox.getBoundingSphere(new THREE.Sphere());
    geometry.name = `${part.partFile}#${materialIdx}`;

    const dissolve = new THREE.InstancedBufferAttribute(new Float32Array(bucket.length), 1);
    dissolve.setUsage(THREE.DynamicDrawUsage);
    geometry.setAttribute('aDissolve', dissolve);

    const mesh = new THREE.InstancedMesh(
      geometry,
      materials.resolve(part, materialIdx),
      bucket.length,
    );
    // Choreography rewrites these every frame from M60 on; telling the driver
    // now avoids it guessing STATIC_DRAW and re-allocating later.
    mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    mesh.name = `${part.partFile}#${materialIdx}`;

    const ids: string[] = [];
    bucket.forEach((inst, i) => {
      const o = bundle.orientations[inst.orientation];
      if (!o) throw new Error(`instance ${inst.id} references unknown orientation ${inst.orientation}`);
      // Row-major in the manifest, row-major in Matrix4.set — one for one.
      m.set(
        o[0], o[1], o[2], inst.translation[0],
        o[3], o[4], o[5], inst.translation[1],
        o[6], o[7], o[8], inst.translation[2],
        0, 0, 0, 1,
      );
      mesh.setMatrixAt(i, m);
      ids.push(inst.id);
    });
    mesh.instanceMatrix.needsUpdate = true;
    // Real per-instance extent, so frustum culling works on where the bricks
    // actually are rather than on one part's local box at the origin.
    mesh.computeBoundingSphere();

    const matrices = new Float32Array(mesh.instanceMatrix.array as Float32Array);
    groups.push({
      part: partIdx,
      material: materialIdx,
      mesh,
      lodMeshes: [mesh],
      matrices,
      levels: new Uint8Array(bucket.length),
      radius: geometry.boundingSphere?.radius ?? 1,
      ids,
      dissolve,
    });
  }

  return groups;
}

/** Batched per-frame writes into the instance buffers.
 *
 * The rule this class exists to enforce: an `InstancedMesh`'s matrix buffer is
 * re-uploaded whole whenever `needsUpdate` is set, so setting it per write
 * would re-upload 50 000 matrices once per brick moved. Callers make as many
 * `set*` calls as they like and call `flush()` once, which uploads each
 * touched buffer at most once per frame.
 */
export class InstanceWriter {
  private readonly slots = new Map<string, Slot>();
  private readonly dirtyMatrix = new Set<InstanceGroup>();
  private readonly dirtyDissolve = new Set<InstanceGroup>();
  private readonly m = new THREE.Matrix4();
  private readonly zero = new THREE.Matrix4().makeScale(0, 0, 0);
  /** Reused, not allocated per call. `setTransform` runs once per instance
   * per frame — at 50 000 instances a fresh `Vector3` each time is 50 000
   * allocations a frame, which the GC then has to walk.
   *
   * Honest note on the measurement: the 50 000-instance pass came out at 14.5
   * ms before and 11.0 / 14.2 ms after, across runs of the *same* code — so
   * this container's run-to-run variance is wider than the effect, and no
   * speedup is claimed. It stays because allocating once per instance per
   * frame in a hot path is wrong regardless of what a noisy machine says, and
   * because it is the difference between a flat cost and one that grows with
   * GC pressure over a 60-minute show. Re-measure on the M92 hardware. */
  private readonly scaleVec = new THREE.Vector3();
  /** Lazily allocated, and only for groups something has actually hidden:
   * hiding is done by writing a zero-scale matrix (three.js has no
   * per-instance visibility flag), so restoring one needs the transform it
   * had. Most scenes never call `setVisible`, and those pay nothing. */
  private readonly parked = new Map<InstanceGroup, Float32Array>();
  private readonly hidden = new Map<InstanceGroup, Uint8Array>();

  constructor(groups: InstanceGroup[]) {
    for (const group of groups) {
      group.ids.forEach((id, index) => this.slots.set(id, { group, index }));
    }
  }

  get size(): number {
    return this.slots.size;
  }

  has(id: string): boolean {
    return this.slots.has(id);
  }

  setTransform(id: string, position: THREE.Vector3, quaternion: THREE.Quaternion, scale: number): void {
    const slot = this.slots.get(id);
    if (!slot) return;
    this.m.compose(position, quaternion, this.scaleVec.set(scale, scale, scale));
    this.write(slot, this.m);
  }

  /** Writes an already-composed matrix — the cheap path for choreography that
   * has one in hand, since it skips a compose and a Vector3 allocation. */
  setMatrix(id: string, matrix: THREE.Matrix4): void {
    const slot = this.slots.get(id);
    if (slot) this.write(slot, matrix);
  }

  private write(slot: Slot, matrix: THREE.Matrix4): void {
    const parked = this.parked.get(slot.group);
    if (parked) {
      matrix.toArray(parked, slot.index * 16);
      if (this.hidden.get(slot.group)?.[slot.index]) return; // stays parked
    }
    // Into the authoritative array, indexed by *instance*. The LOD meshes are
    // re-packed from this, and their row order is not instance order.
    matrix.toArray(slot.group.matrices, slot.index * 16);
    this.dirtyMatrix.add(slot.group);
  }

  setVisible(id: string, visible: boolean): void {
    const slot = this.slots.get(id);
    if (!slot) return;
    const { group, index } = slot;
    let parked = this.parked.get(group);
    if (!parked) {
      parked = new Float32Array(group.matrices);
      this.parked.set(group, parked);
      this.hidden.set(group, new Uint8Array(group.ids.length));
    }
    const hidden = this.hidden.get(group)!;
    if (hidden[index] === (visible ? 0 : 1)) return;
    hidden[index] = visible ? 0 : 1;
    if (visible) {
      this.m.fromArray(parked, index * 16);
      this.m.toArray(group.matrices, index * 16);
    } else {
      this.zero.toArray(group.matrices, index * 16);
    }
    this.dirtyMatrix.add(group);
  }

  setDissolve(id: string, amount: number): void {
    const slot = this.slots.get(id);
    if (!slot) return;
    slot.group.dissolve.setX(slot.index, amount);
    this.dirtyDissolve.add(slot.group);
  }

  /** Groups whose authoritative matrices changed since the last flush —
   * read by the LOD selector, which has to re-pack them. */
  readonly touched = new Set<InstanceGroup>();

  flush(): void {
    for (const group of this.dirtyMatrix) this.touched.add(group);
    for (const group of this.dirtyDissolve) group.dissolve.needsUpdate = true;
    this.dirtyMatrix.clear();
    this.dirtyDissolve.clear();
  }
}
