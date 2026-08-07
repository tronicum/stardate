/** M54 — the mesh render mode.
 *
 * The point path draws millions of splats and gets its shape from density.
 * This path draws real triangles with real normals and gets its shape from
 * light, so almost everything here is about light: a rig scaled to the
 * scene's own bounds, a ground to catch a shadow, and tone mapping so the
 * highlights don't clip.
 *
 * Placements are drawn instanced (M55, `instanced.ts`): one `InstancedMesh`
 * per distinct (part, material) pair, so the draw-call count depends on how
 * many *kinds* of brick a scene has and not on how many bricks.
 */

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { Bounds } from '../tileset';
import {
  fetchMeshBuffers,
  fetchMeshLodBuffers,
  fetchMeshInstances,
  type MeshBundle,
  type MeshInstance,
  type PartBuffers,
} from './bundle';
import { buildSyntheticEnvironment, MaterialLibrary } from './materials';
import { buildInstanceGroups, InstanceWriter, type InstanceGroup } from './instanced';
import { buildEdgeGroups, visibleConditionalEdges, EdgeRenderer } from './edges';
import { attachLodMeshes, LodSelector } from './lod';
import { PostChain, tierFromFps, tierFromUrl, TIERS, type QualityTier } from './post';

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

// `buildPartGeometries` / `buildInstanceGroup` lived here in M54: one
// `THREE.Mesh` per placement, sharing a per-part geometry. M55 replaced both
// with `instanced.ts`, which groups placements by (part, material) into
// `InstancedMesh`es. Nothing else in this file changed — which was the point
// of building the naive version first: it isolated the swap to one call, and
// left a measured before/after (the car: 125 draw calls, now 33).

/** Key and rim. Every position is a multiple of the scene's bounds diagonal
 * rather than an absolute distance, so the same rig lights a single 8 mm
 * brick and a 40-site atlas without being retuned.
 *
 * **Rebalanced in M56.** The M54 rig was a key, a hemisphere fill and a rim,
 * calibrated with no environment at all. M56 adds a real prefiltered
 * environment, and that changes what the rig is for: the environment is now
 * the fill, and a better one, because it has structure — a hemisphere light
 * is two flat colours from above and below. Keeping both double-counted the
 * ambient term and forced the environment to stay dim, which is precisely
 * what left chrome reading as grey plastic: a metal has no diffuse term and
 * lives entirely off the environment, while every dielectric was also being
 * lit by the rig. So the hemisphere is gone and key and rim are lower; they
 * shape, the environment fills. */
export function createLightingRig(bounds: Bounds): THREE.Group {
  const rig = new THREE.Group();
  rig.name = 'lighting';
  const center = boundsCenter(bounds);
  const diag = boundsDiagonal(bounds);

  const key = new THREE.DirectionalLight(0xffffff, 1.6);
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

  const rim = new THREE.DirectionalLight(0xbfd4ff, 0.5);
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
  /** Distinct (part, material) pairs — one `InstancedMesh` each, and the
   * number the draw-call count actually tracks. */
  groups: number;
  instances: number;
  /** Triangles actually submitted per frame — every instance's part, counted
   * once per placement. */
  drawnTriangles: number;
  /** Triangles that exist as *geometry* — each distinct part counted once.
   * The gap between the two is exactly what instancing buys, which is why
   * both numbers are on screen rather than one ambiguous "triangles". */
  uniqueTriangles: number;
  materials: number;
  /** Distinct real LDConfig finishes in this bundle — M56's acceptance
   * criterion, read off the data rather than asserted. */
  finishes: string[];
  /** Screen-space quads submitted by the edge pass: (type-2 + type-5) x
   * brick instances. */
  edgeQuads: number;
  /** How many of those are conditional, i.e. tested per frame. */
  conditionalEdges: number;
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
  const qualityRow = document.getElementById('mesh-quality-row') as HTMLLabelElement;
  const qualitySelect = document.getElementById('quality') as HTMLSelectElement;
  const showEdgesInput = document.getElementById('showEdges') as HTMLInputElement;
  const edgeWidthRow = document.getElementById('mesh-edge-width-row') as HTMLLabelElement;
  const edgeWidthInput = document.getElementById('edgeWidth') as HTMLInputElement;

  // The point-cloud controls have no meaning here; leaving a dead "Point
  // budget" slider on screen is worse than hiding it.
  for (const id of ['pointSize', 'pointBudget', 'showLabels', 'animatePacket']) {
    document.getElementById(id)?.closest('label')?.setAttribute('style', 'display:none');
  }
  exposureRow.style.display = 'flex';

  statusEl.textContent = 'loading mesh bundle…';
  const [buffers, lodBuffers, instances] = await Promise.all([
    fetchMeshBuffers(baseUrl, bundle),
    fetchMeshLodBuffers(baseUrl, bundle),
    fetchMeshInstances(baseUrl, bundle),
  ]);

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x0b0e12);

  // The renderer has to exist before the environment can be prefiltered, and
  // the environment has to exist before chrome or metal is anything but grey
  // — so the renderer is built first and the material library is told about
  // it, rather than the library trying to make one.
  const renderer = new THREE.WebGLRenderer({ antialias: true });
  const materials = new MaterialLibrary(bundle);
  const environment = buildSyntheticEnvironment(renderer);
  materials.setEnvironment(environment);
  scene.environment = environment;
  const groups: InstanceGroup[] = buildInstanceGroups(bundle, buffers, materials, instances);
  const instanceRoot = new THREE.Group();
  instanceRoot.name = 'mesh-instances';
  for (const g of groups) instanceRoot.add(g.mesh);
  scene.add(instanceRoot);
  const writer = new InstanceWriter(groups);

  // M59: coarser levels, chosen per instance. `attachLodMeshes` returns false
  // for a bundle written before M59, and then nothing below it ever runs.
  const hasLods = attachLodMeshes(bundle, lodBuffers, materials, groups);
  const lod = hasLods ? new LodSelector(bundle, groups, writer) : null;
  lod?.addTo(instanceRoot);

  // M57: real LDraw type-2 and type-5 lines, as screen-space quads. This is
  // the difference between a soft plastic blob and the catalogue picture.
  const edges = new EdgeRenderer(buildEdgeGroups(bundle, buffers, materials, groups));
  edges.addTo(instanceRoot);

  scene.add(createLightingRig(bundle.bounds));
  scene.add(createGround(bundle.bounds));

  const center = boundsCenter(bundle.bounds);
  const diag = boundsDiagonal(bundle.bounds);

  // Near and far are set from the scene's own size *and* from how far the
  // screenplay actually pulls the camera. M57's AC3 tests 50x the default
  // distance — with the old `far = diag * 20` the object simply fell out of
  // the frustum there and the test rendered an empty frame, which is not a
  // pass. Ratio is ~30 000:1, which a 24-bit depth buffer handles; the
  // coincident-surface problem is solved by `polygonOffset` on the faces
  // rather than by depth precision, so widening this is safe.
  const camera = new THREE.PerspectiveCamera(
    45,
    window.innerWidth / window.innerHeight,
    Math.max(diag / 200, 0.005),
    diag * 150,
  );
  camera.position.set(center.x + diag * 1.1, center.y + diag * 0.7, center.z + diag * 1.35);
  camera.lookAt(center);

  const pixelRatio = Math.min(window.devicePixelRatio, MAX_PIXEL_RATIO);
  renderer.setPixelRatio(pixelRatio);
  renderer.setSize(window.innerWidth, window.innerHeight);
  // Device pixels, not CSS pixels — the whole point of a screen-space width
  // is that it is the same thickness on every display.
  edges.setResolution(window.innerWidth * pixelRatio, window.innerHeight * pixelRatio, pixelRatio);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  // M58: tone mapping is OFF here on purpose. The scene renders into a
  // HalfFloat target so values above 1.0 reach bloom intact — a bloom
  // threshold is a statement about scene radiance, and squashing everything
  // into 0..1 first makes it a statement about nothing. ACES happens last,
  // in `post.ts`'s grade pass, together with the sRGB encode and the dither.
  renderer.toneMapping = THREE.NoToneMapping;
  renderer.shadowMap.enabled = true;
  // PCFSoftShadowMap is deprecated as of r185 and silently falls back to
  // PCFShadowMap anyway — asking for it only buys a console warning, and
  // M54's AC4 is zero warnings.
  renderer.shadowMap.type = THREE.PCFShadowMap;
  document.getElementById('app')!.prepend(renderer.domElement);

  // Quality tier: an explicit `?quality=` wins; otherwise the scene starts at
  // Medium, is measured for two seconds while already on screen, and settles.
  const forcedTier = tierFromUrl();
  let tier: QualityTier = forcedTier ?? 'medium';
  renderer.shadowMap.enabled = true;
  // The composer issues several `renderer.render` calls per frame, and each
  // one resets the counters — so with autoReset on, the HUD showed the draw
  // calls of the *last* pass, which is one full-screen quad. Reset once per
  // frame instead, and the number becomes the honest per-frame total.
  renderer.info.autoReset = false;
  const applyShadowSize = (t: QualityTier) => {
    const size = TIERS[t].shadowMapSize;
    scene.traverse((o) => {
      const l = o as THREE.DirectionalLight;
      if (l.isDirectionalLight && l.castShadow) {
        l.shadow.mapSize.set(size, size);
        l.shadow.map?.dispose();
        l.shadow.map = null;
      }
    });
  };
  applyShadowSize(tier);

  let post = new PostChain(renderer, scene, camera, tier, window.innerWidth * pixelRatio, window.innerHeight * pixelRatio);
  post.exposure = parseFloat(exposureInput.value);

  exposureInput.addEventListener('input', () => {
    post.exposure = parseFloat(exposureInput.value);
  });
  qualityRow.style.display = 'flex';
  qualitySelect.value = tier;
  const rebuildPost = (next: QualityTier) => {
    if (next === tier) return;
    tier = next;
    applyShadowSize(tier);
    post.dispose();
    post = new PostChain(renderer, scene, camera, tier, window.innerWidth * pixelRatio, window.innerHeight * pixelRatio);
    post.exposure = parseFloat(exposureInput.value);
    qualitySelect.value = tier;
  };
  qualitySelect.addEventListener('change', () => {
    benchmarkDone = true; // a human has spoken; stop second-guessing them
    rebuildPost(qualitySelect.value as QualityTier);
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
    const dpr = Math.min(window.devicePixelRatio, MAX_PIXEL_RATIO);
    edges.setResolution(window.innerWidth * dpr, window.innerHeight * dpr, dpr);
    post.setSize(window.innerWidth * dpr, window.innerHeight * dpr);
  });

  edgeWidthRow.style.display = 'flex';
  edgeWidthInput.addEventListener('input', () => edges.setWidth(parseFloat(edgeWidthInput.value)));
  showEdgesInput.closest('label')?.setAttribute('style', 'display:flex');
  showEdgesInput.addEventListener('input', () => edges.setVisible(showEdgesInput.checked));
  edges.setVisible(showEdgesInput.checked);

  const stats: MeshSceneStats = {
    parts: bundle.parts.length,
    groups: groups.length,
    instances: instances.length,
    drawnTriangles: instances.reduce((sum, i) => sum + (bundle.parts[i.part]?.triangleCount ?? 0), 0),
    uniqueTriangles: bundle.parts.reduce((sum, p) => sum + p.triangleCount, 0),
    materials: bundle.materials.length,
    finishes: materials.finishes(),
    edgeQuads: edges.quadCount,
    conditionalEdges: edges.conditionalCount,
  };
  statusEl.style.display = 'none';

  let lastTime = performance.now();
  let frames = 0;
  let fpsAccumMs = 0;
  let fps = 0;
  const startedAt = performance.now();
  let benchmarkDone = forcedTier !== null;
  let benchFrames = 0;

  function updateHud() {
    hudEl.innerHTML = `
      <div>${stats.instances.toLocaleString()} instances &middot; ${stats.parts} parts &middot; ${stats.groups} groups</div>
      <div>${stats.drawnTriangles.toLocaleString()} triangles drawn &middot; ${stats.uniqueTriangles.toLocaleString()} unique</div>
      <div>${renderer.info.render.calls} draw calls</div>
      <div>${stats.materials} materials &middot; ${stats.finishes.join(', ')}</div>
      <div>${stats.edgeQuads.toLocaleString()} edge quads &middot; ${stats.conditionalEdges.toLocaleString()} conditional &middot; ${edges.visibleGroupCount}/${edges.groups.length} drawn</div>
      <div>${lod ? `LOD ${lod.stats.perLevel.join(' / ')} &middot; ${lod.stats.triangles.toLocaleString()} tris drawn` : 'no LODs in bundle'}</div>
      <div>crease ${bundle.creaseDegrees}&deg;</div>
      <div>${tier} quality${benchmarkDone ? '' : ' (measuring…)'}</div>
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
    lod?.update(camera, window.innerHeight * pixelRatio);
    edges.update(camera, window.innerHeight * pixelRatio);
    renderer.info.reset();
    post.render((now - startedAt) / 1000);

    // The two-second benchmark, run against frames the viewer is already
    // watching rather than behind a black screen. Same number, better first
    // impression.
    if (!benchmarkDone) {
      benchFrames++;
      const elapsed = now - startedAt;
      if (elapsed >= 2000) {
        benchmarkDone = true;
        rebuildPost(tierFromFps((benchFrames * 1000) / elapsed));
      }
    }
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
    groups,
    writer,
    edges,
    // M65: the crossfade needs each group's own material colour, and a
    // harness has no other way to reach the library.
    materials,
    post: () => post,
    lod: () => lod,
    quality: () => tier,
    /** M57 AC2: *which* conditional edges the shader's predicate lets through
     * from where the camera is now, recomputed on the CPU. The set must
     * change as the camera orbits — an unchanging set is proof the test is
     * not running. The set, not the count: a cylinder has two silhouette
     * edges from every angle, so the count is constant even when everything
     * is working. */
    conditionalEdgesDrawn: () => {
      const res = new THREE.Vector2(window.innerWidth, window.innerHeight);
      return edges.groups.flatMap((g, gi) =>
        visibleConditionalEdges(g, camera, res).map((i) => `${gi}:${i}`));
    },
    orbitTo: (radians: number) => {
      const r = Math.hypot(camera.position.x - center.x, camera.position.z - center.z);
      camera.position.set(center.x + Math.cos(radians) * r, camera.position.y, center.z + Math.sin(radians) * r);
      camera.lookAt(center);
      controls.update();
    },
    /** M55 AC3: what it really costs to rewrite every instance's transform
     * and upload it — the per-frame price of choreography, measured rather
     * than asserted, because the show's budget is built on this number.
     *
     * Two paths, because they differ by more than they look like they should:
     *   compose — position/quaternion/scale, the general case
     *   matrix  — a `Matrix4` the caller already has, which is what an
     *             animation curve actually produces
     * Returns the median of `runs` full passes over every instance, in ms. */
    benchTransforms: (runs = 9) => {
      const p = new THREE.Vector3();
      const q = new THREE.Quaternion();
      const scratchScale = new THREE.Vector3();
      const m = new THREE.Matrix4();
      const median = (xs: number[]) => xs.sort((a, b) => a - b)[Math.floor(xs.length / 2)];

      const compose: number[] = [];
      const matrix: number[] = [];
      for (let r = 0; r < runs; r++) {
        let t0 = performance.now();
        for (const g of groups) {
          for (let i = 0; i < g.ids.length; i++) {
            g.mesh.getMatrixAt(i, m);
            m.decompose(p, q, scratchScale);
            p.y += 0.0001; // a real change, so nothing can be optimised away
            writer.setTransform(g.ids[i], p, q, 1);
          }
        }
        writer.flush();
        compose.push(performance.now() - t0);

        t0 = performance.now();
        for (const g of groups) {
          for (let i = 0; i < g.ids.length; i++) {
            g.mesh.getMatrixAt(i, m);
            m.elements[13] += 0.0001;
            writer.setMatrix(g.ids[i], m);
          }
        }
        writer.flush();
        matrix.push(performance.now() - t0);
      }
      return {
        composeMs: median(compose),
        matrixMs: median(matrix),
        instances: writer.size,
        runs,
      };
    },
    camera,
    controls,
  };
}
