import * as THREE from 'three';
import { MaxHeap } from './maxheap';
import type { Bounds, NodeMeta, Tileset } from './tileset';

export const ROOT_ID = 'r';

export class NodeIndex {
  private byId = new Map<string, NodeMeta>();

  constructor(tileset: Tileset) {
    for (const n of tileset.nodes) this.byId.set(n.id, n);
  }

  get(id: string): NodeMeta | undefined {
    return this.byId.get(id);
  }

  children(id: string): string[] {
    const out: string[] = [];
    for (let o = 0; o < 8; o++) {
      const cid = id + o;
      if (this.byId.has(cid)) out.push(cid);
    }
    return out;
  }
}

function boxOf(bounds: Bounds): THREE.Box3 {
  return new THREE.Box3(new THREE.Vector3(...bounds.min), new THREE.Vector3(...bounds.max));
}

function boxDiagonal(bounds: Bounds): number {
  const dx = bounds.max[0] - bounds.min[0];
  const dy = bounds.max[1] - bounds.min[1];
  const dz = bounds.max[2] - bounds.min[2];
  return Math.sqrt(dx * dx + dy * dy + dz * dz);
}

function boxCenter(bounds: Bounds): THREE.Vector3 {
  return new THREE.Vector3(
    (bounds.min[0] + bounds.max[0]) / 2,
    (bounds.min[1] + bounds.max[1]) / 2,
    (bounds.min[2] + bounds.max[2]) / 2,
  );
}

/**
 * Selects which octree nodes to render this frame: a greedy, priority-driven
 * traversal (screen-space error proxy = node size / camera distance) that
 * expands the highest-priority, frustum-visible nodes first until the point
 * budget is exhausted. Mirrors the Potree/3D-Tiles refinement strategy.
 */
export function selectNodes(index: NodeIndex, camera: THREE.Camera, pointBudget: number): Set<string> {
  const frustum = new THREE.Frustum();
  const m = new THREE.Matrix4().multiplyMatrices(camera.projectionMatrix, camera.matrixWorldInverse);
  frustum.setFromProjectionMatrix(m);

  const priority = (id: string): number => {
    const node = index.get(id)!;
    const dist = camera.position.distanceTo(boxCenter(node.bounds));
    return boxDiagonal(node.bounds) / Math.max(dist, 1e-3);
  };

  const heap = new MaxHeap<string>(priority);
  if (index.get(ROOT_ID)) heap.push(ROOT_ID);

  const selected = new Set<string>();
  let remaining = pointBudget;

  while (heap.size > 0 && remaining > 0) {
    const id = heap.pop() as string;
    const node = index.get(id)!;
    if (!frustum.intersectsBox(boxOf(node.bounds))) continue;
    selected.add(id);
    remaining -= node.pointCount;
    for (const childId of index.children(id)) heap.push(childId);
  }

  return selected;
}
