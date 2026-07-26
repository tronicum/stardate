/** M54 — the mesh render mode.
 *
 * The point path draws millions of splats and gets its shape from density.
 * This path draws real triangles with real normals and gets its shape from
 * light, so almost everything here is about light: a rig scaled to the
 * scene's own bounds, a ground to catch a shadow, and tone mapping so the
 * highlights don't clip.
 *
 * One instance is one `THREE.Mesh` sharing a per-part `BufferGeometry`.
 * That is deliberately naive — M55 replaces it with `InstancedMesh` and one
 * draw call per part. Doing it in this order means M55 has something to be
 * measured against, and means this milestone can be wrong about exactly one
 * thing at a time.
 */

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { Bounds } from '../tileset';
import {
  fetchMeshBuffers,
  fetchMeshInstances,
  type MeshBundle,
  type MeshInstance,
  type PartBuffers,
} from './bundle';
import { MaterialLibrary } from './materials';

/** `devicePixelRatio` unclamped is 9x the fragments on a 3x-DPR tablet, for a
 * difference nobody can see at arm's length. See `docs/fugen/budgets.md`. */
const MAX_PIXEL_RATIO = 1.5;

function boundsCenter(b: Bounds): THREE.Vector3 {
  return new THREE.Vector3(
    (b.min[0] + b.max[0]) / 2,
    (b.min[1] + b.max[1]) / 2,
    (b.min[2] + b.max[2]) / 2,
  );
}

function boundsDiagonal(b: Bounds): number {
  return (
    Math.hypot(b.max[0] - b.min[0], b.max[1] - b.min[1], b.max[2] - b.min[2]) || 1
  );
}

/** One `BufferGeometry` per *part*, with one draw group per submesh so a
 * renderer binds a material once per contiguous index range and never
 * mid-buffer. The buffers arrive already interleaved-free and in the output
 * frame; nothing here transforms them. */
export function buildPartGeometries(
  bundle: MeshBundle,
  buffers: Map<number, PartBuffers>,
): Map<number, THREE.BufferGeometry> {
  const out = new Map<number, THREE.BufferGeometry>();
  for (const part of bundle.parts) {
    const b = buffers.get(part.index);
    if (!b) throw new Error(`no buffers loaded for part ${part.partFile}`);
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(b.position, 3));
    geometry.setAttribute('normal', new THREE.BufferAttribute(b.normal, 3));
    geometry.setIndex(new THREE.BufferAttribute(b.index, 1));
    part.submeshes.forEach((s, i) => geometry.addGroup(s.indexOffset, s.indexCount, i));
    geometry.boundingBox = new THREE.Box3(
      new THREE.Vector3(...part.bounds.min),
      new THREE.Vector3(...part.bounds.max),
    );
    geometry.boundingSphere = geometry.boundingBox.getBoundingSphere(new THREE.Sphere());
    geometry.name = part.partFile;
    out.set(part.index, geometry);
  }
  return out;
}

/** Places every instance. Orientations are row-major 3x3, already conjugated
 * into the output frame by the writer, so they compose straight into a
 * `Matrix4` — no basis change, no axis swap, no second mirror. */
export function buildInstanceGroup(
  bundle: MeshBundle,
  geometries: Map<number, THREE.BufferGeometry>,
  materials: MaterialLibrary,
  instances: MeshInstance[],
): THREE.Group {
  const group = new THREE.Group();
  group.name = 'mesh-instances';
  const m = new THREE.Matrix4();
  for (const inst of instances) {
    const geometry = geometries.get(inst.part);
    const part = bundle.parts[inst.part];
    if (!geometry || !part) throw new Error(`instance ${inst.id} references unknown part ${inst.part}`);
    const o = bundle.orientations[inst.orientation];
    if (!o) throw new Error(`instance ${inst.id} references unknown orientation ${inst.orientation}`);
    const mesh = new THREE.Mesh(geometry, materials.resolve(part, inst.material));
    // Matrix4.set takes arguments in row-major order, which is the order the
    // manifest stores them in — they line up one for one.
    m.set(
      o[0], o[1], o[2], inst.translation[0],
      o[3], o[4], o[5], inst.translation[1],
      o[6], o[7], o[8], inst.translation[2],
      0, 0, 0, 1,
    );
    mesh.matrixAutoUpdate = false;
    mesh.matrix.copy(m);
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    mesh.name = inst.id;
    group.add(mesh);
  }
  return group;
}

/** Key light, hemisphere fill, rim. Every position is a multiple of the
 * scene's bounds diagonal rather than an absolute distance, so the same rig
 * lights a single 8 mm brick and a 40-site atlas without being retuned. */
export function createLightingRig(bounds: Bounds): THREE.Group {
  const rig = new THREE.Group();
  rig.name = 'lighting';
  const center = boundsCenter(bounds);
  const diag = boundsDiagonal(bounds);

  const key = new THREE.DirectionalLight(0xffffff, 2.1);
  key.position.set(center.x + diag * 0.8, center.y + diag * 1.2, center.z + diag * 0.7);
  key.target.position.copy(center);
  key.castShadow = true;
  key.shadow.mapSize.set(2048, 2048);
  const extent = diag * 0.75;
  key.shadow.camera.left = -extent;
  key.shadow.camera.right = extent;
  key.shadow.camera.top = extent;
  key.shadow.camera.bottom = -extent;
  key.shadow.camera.near = diag * 0.05;
  key.shadow.camera.far = diag * 4;
  // Two different acne fixes doing two different jobs: `bias` handles the
  // depth-comparison slop, `normalBias` the self-shadowing on the studs'
  // curved facets. normalBias is in world units, hence scaled by the scene —
  // and it has to be generous here, because a real stacked model has walls
  // that abut exactly at a brick boundary, which is the worst case a shadow
  // map can be handed.
  key.shadow.bias = -0.0008;
  key.shadow.normalBias = Math.max(diag * 0.006, 0.05);
  rig.add(key);
  rig.add(key.target);

  // Sky/ground fill: keeps the shadowed side from going to pure black without
  // flattening the form the way an ambient light would.
  rig.add(new THREE.HemisphereLight(0x9db4d0, 0x2a2418, 0.55));

  const rim = new THREE.DirectionalLight(0xbfd4ff, 0.9);
  rim.position.set(center.x - diag * 1.0, center.y + diag * 0.35, center.z - diag * 0.9);
  rim.target.position.copy(center);
  rig.add(rim);
  rig.add(rim.target);

  return rig;
}

/** A large plate just under the scene, solely so the key light has something
 * to cast onto — a brick floating in void reads as a render, a brick with a
 * contact shadow reads as an object. */
export function createGround(bounds: Bounds): THREE.Mesh {
  const diag = boundsDiagonal(bounds);
  const center = boundsCenter(bounds);
  const ground = new THREE.Mesh(
    new THREE.PlaneGeometry(diag * 8, diag * 8),
    new THREE.MeshStandardMaterial({ color: 0x141a22, roughness: 0.95, metalness: 0.0 }),
  );
  ground.name = 'ground';
  ground.rotation.x = -Math.PI / 2;
  ground.position.set(center.x, bounds.min[1] - diag * 0.001, center.z);
  ground.receiveShadow = true;
  return ground;
}

export interface MeshSceneStats {
  parts: number;
  instances: number;
  /** Triangles actually submitted per frame — every instance's part, counted
   * once per placement. */
  drawnTriangles: number;
  /** Triangles that exist as *geometry* — each distinct part counted once.
   * The gap between the two is exactly what instancing buys, which is why
   * both numbers are on screen rather than one ambiguous "triangles". */
  uniqueTriangles: number;
  materials: number;
}

/** The whole mesh-mode viewer: its own renderer, camera, controls and loop.
 *
 * It is a separate entry point rather than a set of branches inside the point
 * viewer on purpose. The two paths share almost no state — no LOD selector,
 * no octree, no point budget, no node labels — and interleaving them would
 * put a conditional on every line of the hot loop of a pipeline that already
 * works. `main.ts` picks one, once. */
export async function runMeshViewer(baseUrl: string, bundle: MeshBundle): Promise<void> {
  const statusEl = document.getElementById('status') as HTMLDivElement;
  const hudEl = document.getElementById('hud') as HTMLDivElement;
  const autoRotateInput = document.getElementById('autoRotate') as HTMLInputElement;
  const exposureRow = document.getElementById('mesh-exposure-row') as HTMLLabelElement;
  const exposureInput = document.getElementById('exposure') as HTMLInputElement;

  // The point-cloud controls have no meaning here; leaving a dead "Point
  // budget" slider on screen is worse than hiding it.
  for (const id of ['pointSize', 'pointBudget', 'showLabels', 'animatePacket', 'showEdges']) {
    document.getElementById(id)?.closest('label')?.setAttribute('style', 'display:none');
  }
  exposureRow.style.display = 'flex';

  statusEl.textContent = 'loading mesh bundle…';
  const [buffers, instances] = await Promise.all([
    fetchMeshBuffers(baseUrl, bundle),
    fetchMeshInstances(baseUrl, bundle),
  ]);

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x0b0e12);

  const materials = new MaterialLibrary(bundle);
  const geometries = buildPartGeometries(bundle, buffers);
  const group = buildInstanceGroup(bundle, geometries, materials, instances);
  scene.add(group);
  scene.add(createLightingRig(bundle.bounds));
  scene.add(createGround(bundle.bounds));

  const center = boundsCenter(bundle.bounds);
  const diag = boundsDiagonal(bundle.bounds);

  const camera = new THREE.PerspectiveCamera(
    45,
    window.innerWidth / window.innerHeight,
    Math.max(diag / 1000, 0.01),
    diag * 20,
  );
  camera.position.set(center.x + diag * 1.1, center.y + diag * 0.7, center.z + diag * 1.35);
  camera.lookAt(center);

  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, MAX_PIXEL_RATIO));
  renderer.setSize(window.innerWidth, window.innerHeight);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  // ACES on the renderer is correct *only while there is no post chain*. M58
  // adds bloom, which must run in linear HDR — tone mapping before it makes
  // the bloom threshold meaningless. At that point this becomes
  // `NoToneMapping` and ACES moves into the final `OutputPass`. One line,
  // one milestone, written down here so it is not rediscovered by eye.
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = parseFloat(exposureInput.value);
  renderer.shadowMap.enabled = true;
  // PCFSoftShadowMap is deprecated as of r185 and silently falls back to
  // PCFShadowMap anyway — asking for it only buys a console warning, and
  // M54's AC4 is zero warnings.
  renderer.shadowMap.type = THREE.PCFShadowMap;
  document.getElementById('app')!.prepend(renderer.domElement);

  exposureInput.addEventListener('input', () => {
    renderer.toneMappingExposure = parseFloat(exposureInput.value);
  });

  const controls = new OrbitControls(camera, renderer.domElement);
  controls.target.copy(center);
  controls.enableDamping = true;
  controls.dampingFactor = 0.08;
  controls.autoRotateSpeed = 0.6;
  controls.autoRotate = autoRotateInput.checked;
  autoRotateInput.addEventListener('input', () => {
    controls.autoRotate = autoRotateInput.checked;
  });
  controls.update();

  window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
  });

  const stats: MeshSceneStats = {
    parts: bundle.parts.length,
    instances: instances.length,
    drawnTriangles: instances.reduce((sum, i) => sum + (bundle.parts[i.part]?.triangleCount ?? 0), 0),
    uniqueTriangles: bundle.parts.reduce((sum, p) => sum + p.triangleCount, 0),
    materials: bundle.materials.length,
  };
  statusEl.style.display = 'none';

  let lastTime = performance.now();
  let frames = 0;
  let fpsAccumMs = 0;
  let fps = 0;

  function updateHud() {
    hudEl.innerHTML = `
      <div>${stats.instances.toLocaleString()} instances &middot; ${stats.parts} parts</div>
      <div>${stats.drawnTriangles.toLocaleString()} triangles drawn &middot; ${stats.uniqueTriangles.toLocaleString()} unique</div>
      <div>${renderer.info.render.calls} draw calls</div>
      <div>${stats.materials} materials &middot; crease ${bundle.creaseDegrees}&deg;</div>
      <div id="hud-fps">${fps.toFixed(0)} fps</div>
    `;
  }

  function animate() {
    requestAnimationFrame(animate);
    controls.update();
    const now = performance.now();
    frames++;
    fpsAccumMs += now - lastTime;
    lastTime = now;
    if (fpsAccumMs >= 250) {
      fps = (frames * 1000) / fpsAccumMs;
      frames = 0;
      fpsAccumMs = 0;
    }
    renderer.render(scene, camera);
    updateHud();
  }
  animate();

  // Handed to the headless screenshot harness (M54 AC2) so it asserts on the
  // renderer's own counters rather than on pixels it guessed the meaning of.
  (window as unknown as Record<string, unknown>).__spexMesh = {
    stats,
    fps: () => fps,
    drawCalls: () => renderer.info.render.calls,
    // Exposed so a diagnostic run can isolate one contribution at a time —
    // "are those seam lines shadow acne or depth precision?" is a two-minute
    // question with this and an unanswerable one without it.
    renderer,
    scene,
    camera,
    controls,
  };
}
