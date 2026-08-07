/** M59 — choosing a level of detail, per instance, every frame.
 *
 * The bundle carries three levels of every part (`spex_mesh::lod`): the real
 * geometry, the same brick with its studs and underside tubes removed, and
 * its box. Which one a *particular placement* should be drawn at depends on
 * how big it is on screen, and in an Atlas shot two instances of the same
 * part are metres apart — so this is a per-instance decision, not a
 * per-part one.
 *
 * **Hysteresis is the whole reason this isn't three lines.** A single
 * threshold makes a brick sitting exactly on it flip level every frame the
 * camera breathes, which is a visible strobe. Each boundary is therefore two
 * numbers, and a brick has to cross the *far* one to be demoted and the
 * *near* one to be promoted.
 *
 * The thresholds also agree with M57's edge gate on purpose: geometric
 * outlines switch off at the same size LOD1 switches on. An outline of a stud
 * that is no longer being drawn is worse than no outline at all.
 */

import * as THREE from 'three';
import { makeDissolveDepthMaterial } from '../show/dissolve';
import type { MeshBundle, PartBuffers } from './bundle';
import type { MaterialLibrary } from './materials';
import type { InstanceGroup, InstanceWriter } from './instanced';

/** Projected height in device pixels. Demote below the first number, promote
 * back above the second — the gap is the hysteresis band. */
export const LOD1_DEMOTE_PX = 44;
export const LOD1_PROMOTE_PX = 56;
export const LOD2_DEMOTE_PX = 12;
export const LOD2_PROMOTE_PX = 18;

export interface LodStats {
  /** Instances currently drawn at each level. */
  perLevel: [number, number, number];
  /** Triangles actually submitted this frame, after LOD. */
  triangles: number;
  /** How many frames have needed a re-pack — a cheap proxy for how much the
   * selector is actually costing. */
  repacks: number;
}

/** Builds the extra `InstancedMesh`es, one per available level per group. */
export function attachLodMeshes(
  bundle: MeshBundle,
  lodBuffers: Map<string, PartBuffers>,
  materials: MaterialLibrary,
  groups: InstanceGroup[],
): boolean {
  let any = false;
  for (const group of groups) {
    const part = bundle.parts[group.part];
    if (!part?.lods?.length) continue;
    for (const lod of part.lods) {
      const b = lodBuffers.get(`${group.part}:${lod.level}`);
      if (!b) continue;
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.BufferAttribute(b.position, 3));
      geometry.setAttribute('normal', new THREE.BufferAttribute(b.normal, 3));
      geometry.setIndex(new THREE.BufferAttribute(b.index, 1));
      lod.submeshes.forEach((s, i) => geometry.addGroup(s.indexOffset, s.indexCount, i));
      // The part's own level-0 bounds: a level is a simplification of the
      // same object and occupies the same space, and the selector's
      // projected-size test must not change just because detail did.
      geometry.boundingBox = new THREE.Box3(
        new THREE.Vector3(...part.bounds.min),
        new THREE.Vector3(...part.bounds.max),
      );
      geometry.boundingSphere = geometry.boundingBox.getBoundingSphere(new THREE.Sphere());
      geometry.name = `${part.partFile}#${group.material}@lod${lod.level}`;

      const mesh = new THREE.InstancedMesh(
        geometry,
        lod.submeshes.map((s) => materials.get(s.material ?? group.material)),
        group.ids.length,
      );
      mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
      mesh.customDepthMaterial = makeDissolveDepthMaterial();
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      mesh.count = 0;
      mesh.name = geometry.name;
      group.lodMeshes[lod.level] = mesh;
      any = true;
    }
  }
  return any;
}

/** Per-frame level assignment and re-packing. */
export class LodSelector {
  readonly stats: LodStats = { perLevel: [0, 0, 0], triangles: 0, repacks: 0 };
  private readonly triangleCounts = new Map<InstanceGroup, [number, number, number]>();
  private readonly centre = new THREE.Vector3();
  private lastCameraKey = '';
  /** Verification only: pin every instance to one level.
   *
   * AC2 asks whether LOD transitions are *visible*, and comparing consecutive
   * frames of a dolly cannot answer that — the shot is changing too, and at
   * the far end a few-pixel object sits inside a frame whose luminance is
   * mostly ground. Pinning to LOD0 and re-shooting the *same* camera gives
   * the LOD error on its own. */
  private override: number | null = null;

  constructor(
    bundle: MeshBundle,
    private readonly groups: InstanceGroup[],
    private readonly writer: InstanceWriter,
  ) {
    // From here the selector owns every level's upload, level 0 included —
    // see `InstanceWriter.lodManaged`.
    writer.lodManaged = true;
    for (const g of groups) {
      const part = bundle.parts[g.part];
      const counts: [number, number, number] = [part?.triangleCount ?? 0, 0, 0];
      for (const l of part?.lods ?? []) counts[l.level] = l.triangleCount;
      this.triangleCounts.set(g, counts);
    }
    for (const g of groups) this.repack(g);
  }

  /** Adds every level's mesh to the scene. Level 0 is already there. */
  addTo(parent: THREE.Object3D) {
    for (const g of this.groups) {
      for (let l = 1; l < g.lodMeshes.length; l++) {
        if (g.lodMeshes[l]) parent.add(g.lodMeshes[l]);
      }
    }
  }

  /** See `override`. Pass `null` to hand control back to the selector. */
  forceLevel(level: number | null) {
    this.override = level;
    this.lastCameraKey = '';
    for (const g of this.groups) this.repack(g);
  }

  update(camera: THREE.PerspectiveCamera, viewportHeightPx: number) {
    // The camera key is a cheap "did anything that matters change" test: with
    // a still camera and a still scene the whole selector costs one string
    // compare per frame instead of a pass over every instance.
    const key = `${camera.position.x.toFixed(3)},${camera.position.y.toFixed(3)},${camera.position.z.toFixed(3)},${viewportHeightPx}`;
    const cameraMoved = key !== this.lastCameraKey;
    const moved = this.writer.touched;
    if (!cameraMoved && moved.size === 0) return;
    this.lastCameraKey = key;

    const k = viewportHeightPx / (2 * Math.tan((camera.fov * Math.PI) / 180 / 2));
    this.stats.perLevel = [0, 0, 0];
    this.stats.triangles = 0;

    for (const g of this.groups) {
      let changed = moved.has(g);
      const levels = g.levels;
      const maxLevel = g.lodMeshes.length - 1;
      if (cameraMoved && maxLevel > 0) {
        for (let i = 0; i < levels.length; i++) {
          // The instance's own translation, straight out of the authoritative
          // matrix — elements 12..14 of a column-major Matrix4.
          const o = i * 16;
          this.centre.set(g.matrices[o + 12], g.matrices[o + 13], g.matrices[o + 14]);
          const dist = Math.max(camera.position.distanceTo(this.centre), 1e-6);
          const px = (2 * g.radius * k) / dist;
          const was = levels[i];
          let now = was;
          if (was === 0 && px < LOD1_DEMOTE_PX) now = 1;
          else if (was === 1 && px >= LOD1_PROMOTE_PX) now = 0;
          else if (was === 1 && px < LOD2_DEMOTE_PX) now = 2;
          else if (was === 2 && px >= LOD2_PROMOTE_PX) now = 1;
          if (now > maxLevel) now = maxLevel;
          if (now !== was) {
            levels[i] = now;
            changed = true;
          }
        }
      }
      if (changed) this.repack(g);

      const counts = this.triangleCounts.get(g)!;
      for (let l = 0; l <= maxLevel; l++) {
        const n = g.lodMeshes[l]?.count ?? 0;
        this.stats.perLevel[l] += n;
        this.stats.triangles += n * counts[l];
      }
    }
    moved.clear();
  }

  /** Copies each instance's authoritative matrix into whichever level's mesh
   * it currently belongs to, and sets that mesh's draw count. */
  private repack(g: InstanceGroup) {
    this.stats.repacks++;
    const maxLevel = g.lodMeshes.length - 1;
    const cursor = [0, 0, 0];
    for (let i = 0; i < g.levels.length; i++) {
      const level = Math.min(this.override ?? g.levels[i], maxLevel);
      const mesh = g.lodMeshes[level];
      if (!mesh) continue;
      const dst = mesh.instanceMatrix.array as Float32Array;
      dst.set(g.matrices.subarray(i * 16, i * 16 + 16), cursor[level] * 16);
      cursor[level]++;
    }
    for (let l = 0; l <= maxLevel; l++) {
      const mesh = g.lodMeshes[l];
      if (!mesh) continue;
      mesh.count = cursor[l];
      mesh.instanceMatrix.needsUpdate = true;
    }
  }
}
