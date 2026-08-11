/** M66 — the player: the first thing that runs an act end to end.
 *
 * M60 made the screenplay data, M61 resolved it to one duration, M62 evaluated
 * it into sinks, M63/M64/M65 built the three sinks that move a picture. This
 * file is where a resolved document, a set of mesh bundles and a renderer meet
 * and become a screening.
 *
 * # A third mode, not a branch inside the second
 *
 * `main.ts` now tests three absences in order: `show-resolved.json`, then
 * `mesh.json`, then neither. Each test is a 404 that means something, which is
 * the pattern the mesh mode already established — and a show and a single
 * bundle share so little state (one has a clock, a timeline, N bundles and no
 * OrbitControls; the other has one bundle and nothing else) that interleaving
 * them would put a conditional on every line of a hot loop.
 *
 * # N bundles, one scene graph
 *
 * A show's scenes are separate mesh bundles under `bundles/<id>`, each with its
 * own materials, LODs, edges and point clouds. They are loaded in parallel and
 * added to one three.js scene under one `THREE.Group` each, so a shot's
 * `scenes` list is N `visible` flags and nothing more expensive.
 *
 * # What a transform track means for a target with many instances
 *
 * This is the one semantic decision in the file, and it is not obvious from
 * the format. `{"rotation": [0, 360, 0]}` on `brick/**` cannot be an absolute
 * world transform: that would collapse every brick of the scene onto the same
 * point. So an authored transform is read **relative to the placement the
 * bundle already gave each instance** — position adds, rotation composes onto
 * the instance's own orientation, scale multiplies. A single-instance target
 * behaves the way an author expects either way; a multi-instance one only
 * behaves at all under this reading.
 *
 * # What is shared, and therefore whole-scene
 *
 * Three effects are per *material* or per *renderer* rather than per instance,
 * and the player says so rather than pretending otherwise:
 * `material` properties other than `edgeOpacity` change the shared
 * `MeshPhysicalMaterial` of every group the target touches; `edgeOpacity`
 * changes the whole scene's outline pass; a `pointCloud` track crossfades the
 * whole scene's cloud. Every use in the screenplay addresses a whole scene, so
 * nothing is lost today — but a future track that addresses half a scene would
 * silently affect all of it, and that warning is emitted, once, at load.
 */

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { Bounds } from '../tileset';
import {
  fetchMeshBundle,
  fetchMeshBuffers,
  fetchMeshInstances,
  fetchMeshLodBuffers,
  type MeshBundle,
} from '../mesh/bundle';
import { buildSyntheticEnvironment, MaterialLibrary } from '../mesh/materials';
import { buildInstanceGroups, InstanceWriter, type InstanceGroup } from '../mesh/instanced';
import { buildEdgeGroups, EdgeRenderer } from '../mesh/edges';
import { attachLodMeshes, LodSelector } from '../mesh/lod';
import { PostChain, tierFromFps, TIERS, type QualityTier } from '../mesh/post';
import { createLightingRig, createGround } from '../mesh/render';
import { ShowClock } from './clock';
import { Timeline, type TrackSinks } from './timeline';
import { CameraDirector } from './camera';
import { AssemblyChoreography, assemblyFromCue } from './choreography';
import { tokenFlowFromCue, type TokenFlow } from './tokens';
import { DissolveController } from './dissolve';
import { buildPointClouds, fetchPartPoints, PointCloudRenderer } from './points';
import { ShowHud, linearToCss } from './hud';
import { CueBinder, type VoiceBinding } from './binding';
import { loadFugueAudio, type FugueAudio } from '../audio/fugue';
import { BLOOM_STRENGTH } from '../mesh/post';
import type { Monitor } from '../audio/engine';
import type { ShowParams } from './params';
import type {
  MaterialProperty,
  PostProperty,
  ResolvedCue,
  ResolvedShot,
  ResolvedShow,
  TargetBinding,
} from './resolved';

const MAX_PIXEL_RATIO = 1.5;

/** How dark a cast shadow makes the ground.
 *
 * Not 1.0. The piece's ground is already near-black, and a fully opaque shadow
 * on it is a black shape on a black field — which is to say, nothing. 0.55
 * leaves a shadow that is visible against the background without ever becoming
 * a second object in the frame. */
export const GROUND_SHADOW_OPACITY = 0.55;

/** One loaded scene: a bundle, everything built from it, and the placement it
 * came with. */
interface SceneRuntime {
  id: string;
  bundle: MeshBundle;
  root: THREE.Group;
  materials: MaterialLibrary;
  groups: InstanceGroup[];
  writer: InstanceWriter;
  edges: EdgeRenderer;
  lod: LodSelector | null;
  points: PointCloudRenderer | null;
  dissolve: DissolveController;
  /** Bundle order — index *is* the instance index a binding refers to. */
  instanceIds: string[];
  /** The placement each instance was authored at, decomposed once. Authored
   * transforms are read relative to this; see the header. */
  homePos: Float32Array;
  homeQuat: Float32Array;
  homeScale: Float32Array;
  /** Instance index -> which material index it draws with, so a `material`
   * track can find the shared materials its target actually touches. */
  materialOf: Uint16Array;
  /** The shot's generator, whichever kind it is. Both satisfy the same
   * `positionAt(i, t01, out)` contract, which is the whole reason a second
   * generator needed no change to the frame loop. */
  assembly: AssemblyChoreography | TokenFlow | null;
  /** Which shot's `seed` cue built `assembly`, so it is rebuilt on a seek
   * rather than surviving into a shot it does not belong to. */
  assemblyShot: string | null;
}

/** A binding, resolved once into the strings `InstanceWriter` is keyed by.
 *
 * Doing this per frame would be an array map per track per frame — cheap for
 * nine bricks and not for an Atlas site, and pointlessly so: the answer cannot
 * change, which is the same argument `compile.rs` makes for doing the glob
 * once at build time. */
interface BoundTarget {
  scene: SceneRuntime | null;
  ids: string[];
  /** The same instances as indices, parallel to `ids`. Both, because the
   * writer is keyed by id and the home-placement arrays are keyed by index,
   * and looking either one up from the other per instance per frame is a
   * linear scan in a hot loop. */
  indices: number[];
  /** Distinct material indices the target's instances draw with. */
  materials: number[];
  /** True when the target covers every instance of its scene — the condition
   * under which the shared-state effects below are exact rather than
   * approximate. */
  wholeScene: boolean;
}

function boundsCenter(b: Bounds): THREE.Vector3 {
  return new THREE.Vector3(
    (b.min[0] + b.max[0]) / 2,
    (b.min[1] + b.max[1]) / 2,
    (b.min[2] + b.max[2]) / 2,
  );
}

function boundsDiagonal(b: Bounds): number {
  return Math.hypot(b.max[0] - b.min[0], b.max[1] - b.min[1], b.max[2] - b.min[2]) || 1;
}

function mergeSceneBounds(list: Bounds[]): Bounds {
  const min: [number, number, number] = [Infinity, Infinity, Infinity];
  const max: [number, number, number] = [-Infinity, -Infinity, -Infinity];
  for (const b of list) {
    for (let i = 0; i < 3; i++) {
      min[i] = Math.min(min[i], b.min[i]);
      max[i] = Math.max(max[i], b.max[i]);
    }
  }
  if (!Number.isFinite(min[0])) return { min: [-1, -1, -1], max: [1, 1, 1] };
  return { min, max };
}

/** Loads one scene's bundle and builds everything the runtime needs from it.
 *
 * `wantPoints` is false for a scene no `pointCloud` track addresses: point
 * buffers are 24 bytes a point and a scene that never crossfades has no reason
 * to download or upload them. */
async function loadScene(
  showBase: string,
  id: string,
  bundlePath: string,
  renderer: THREE.WebGLRenderer,
  environment: THREE.Texture,
  wantPoints: boolean,
): Promise<SceneRuntime> {
  const base = `${showBase.replace(/\/$/, '')}/${bundlePath}`;
  const bundle = await fetchMeshBundle(base);
  if (!bundle) throw new Error(`scene ${id}: no mesh.json under ${bundlePath}`);

  const [buffers, lodBuffers, instances] = await Promise.all([
    fetchMeshBuffers(base, bundle),
    fetchMeshLodBuffers(base, bundle),
    fetchMeshInstances(base, bundle),
  ]);

  const materials = new MaterialLibrary(bundle);
  materials.setEnvironment(environment);
  const groups = buildInstanceGroups(bundle, buffers, materials, instances);

  const root = new THREE.Group();
  root.name = `scene:${id}`;
  for (const g of groups) root.add(g.mesh);

  const writer = new InstanceWriter(groups);
  const hasLods = attachLodMeshes(bundle, lodBuffers, materials, groups);
  const lod = hasLods ? new LodSelector(bundle, groups, writer) : null;
  lod?.addTo(root);

  const edgeGroups = buildEdgeGroups(bundle, buffers, materials, groups);
  const edges = new EdgeRenderer(edgeGroups);
  edges.addTo(root);

  let points: PointCloudRenderer | null = null;
  if (wantPoints) {
    const entries = await Promise.all(
      bundle.parts
        .filter((p) => p.buffers.points)
        .map(async (p): Promise<[number, Float32Array]> => [
          p.index,
          await fetchPartPoints(base, p.buffers.points!),
        ]),
    );
    const clouds = buildPointClouds(bundle, new Map(entries), materials, groups, edgeGroups);
    if (clouds.length > 0) {
      points = new PointCloudRenderer(clouds);
      points.addTo(root);
    }
  }

  // The placement each instance arrived with, decomposed once. `group.matrices`
  // is the authoritative array (see `InstanceWriter`), and it is still exactly
  // what the bundle wrote at this moment — nothing has run yet.
  const n = bundle.instanceIds.length;
  const homePos = new Float32Array(n * 3);
  const homeQuat = new Float32Array(n * 4);
  const homeScale = new Float32Array(n);
  const materialOf = new Uint16Array(n);
  const indexOf = new Map(bundle.instanceIds.map((id2, i) => [id2, i]));
  const m = new THREE.Matrix4();
  const p = new THREE.Vector3();
  const q = new THREE.Quaternion();
  const s = new THREE.Vector3();
  for (const g of groups) {
    for (let i = 0; i < g.ids.length; i++) {
      const idx = indexOf.get(g.ids[i]);
      if (idx === undefined) continue;
      m.fromArray(g.matrices, i * 16).decompose(p, q, s);
      homePos.set([p.x, p.y, p.z], idx * 3);
      homeQuat.set([q.x, q.y, q.z, q.w], idx * 4);
      homeScale[idx] = s.x;
      materialOf[idx] = g.material;
    }
  }

  // `mesh.material` is an **array** — one entry per submesh, because a part
  // can carry a moulded accent colour alongside the instance's own (see
  // `MaterialLibrary.resolve`). Handing the array itself to a controller that
  // expects a material gives `undefined.dissolve` on the first frame in which
  // any scene is visible, and none at all before that: at t=0 of this act no
  // scene is on screen, so the opening frame renders perfectly.
  const dissolve = new DissolveController(
    groups.flatMap((g) => (Array.isArray(g.mesh.material) ? g.mesh.material : [g.mesh.material])),
  );

  return {
    id,
    bundle,
    root,
    materials,
    groups,
    writer,
    edges,
    lod,
    points,
    dissolve,
    instanceIds: bundle.instanceIds,
    homePos,
    homeQuat,
    homeScale,
    materialOf,
    assembly: null,
    assemblyShot: null,
  };
}

export async function runShowViewer(
  baseUrl: string,
  show: ResolvedShow,
  params: ShowParams,
): Promise<void> {
  const statusEl = document.getElementById('status') as HTMLDivElement;
  const warnings = [...params.warnings];

  // A screening has no sliders. Everything the two other modes put on screen
  // is hidden rather than restyled, so nothing here can drift out of sync with
  // what those modes do with the same elements.
  for (const id of ['controls', 'hud', 'graph-meta', 'cycle-indicator', 'debug-panel']) {
    const node = document.getElementById(id);
    if (node) node.style.display = 'none';
  }
  statusEl.textContent = `loading ${show.title}…`;

  const seed = params.seed ?? show.seed;
  // Which scenes ever crossfade to points. See `loadScene`.
  const pointScenes = new Set<string>();
  for (const shot of show.shots) {
    for (const track of shot.tracks ?? []) {
      if (track.kind === 'pointCloud' && track.target.scene) pointScenes.add(track.target.scene);
    }
  }

  const scene = new THREE.Scene();
  const palette = show.palette ?? {};
  scene.background = new THREE.Color(
    ...(palette.schwarz ?? [0.043, 0.055, 0.071]),
  );

  const renderer = new THREE.WebGLRenderer({ antialias: true });
  const environment = buildSyntheticEnvironment(renderer);
  scene.environment = environment;

  const loaded = await Promise.all(
    show.scenes.map((s) =>
      loadScene(baseUrl, s.id, s.bundle, renderer, environment, pointScenes.has(s.id)),
    ),
  );
  const scenes = new Map(loaded.map((s) => [s.id, s]));
  for (const s of loaded) {
    s.root.visible = false;
    scene.add(s.root);
  }

  const bounds = mergeSceneBounds(loaded.map((s) => s.bundle.bounds));
  const center = boundsCenter(bounds);
  const diag = boundsDiagonal(bounds);

  scene.add(createLightingRig(bounds));
  const ground = createGround(bounds);
  {
    // The screenplay's opening direction is the word "Black", and the first
    // frame this player ever rendered was a mid-grey slab filling half the
    // screen. The albedo was not the reason — `grundUnten` is linear 0.0015,
    // which is black by any measure. **It was the environment**, and the
    // measurement is in the M66 diary entry: hiding the ground dropped that
    // region from sRGB 70 to 15, and removing `scene.environment` with the
    // ground still there did the same. `MeshStandardMaterial` gives every
    // dielectric an F0 of 0.04, and Fresnel takes that to nearly 1.0 at
    // grazing incidence — A1-S01 looks along a plane eight scene-diagonals
    // wide from 4.8 mm above it, so most of those pixels are a near-total
    // mirror of M56's synthetic environment and the albedo never enters the
    // arithmetic at all.
    //
    // `envMapIntensity = 0` did not fix it, which is worth recording: with
    // `scene.environment` rather than a per-material `envMap`, that uniform
    // did not switch the contribution off in this three.js version.
    //
    // So the ground stops being a surface and becomes what it was always for:
    // a **shadow catcher**. `ShadowMaterial` draws the shadow and nothing
    // else, so the plate contributes exactly zero to a frame nothing is
    // standing on. M54's argument for a lit ground — "a brick with a contact
    // shadow reads as an object" — is an argument about previewing a model,
    // and a screening is not a preview.
    (ground.material as THREE.Material).dispose();
    const shadow = new THREE.ShadowMaterial({ opacity: GROUND_SHADOW_OPACITY });
    if (palette.grundUnten) shadow.color.setRGB(...palette.grundUnten);
    ground.material = shadow;
  }
  scene.add(ground);

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
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.NoToneMapping;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFShadowMap;
  renderer.info.autoReset = false;
  document.getElementById('app')!.prepend(renderer.domElement);

  let tier: QualityTier = params.quality ?? 'medium';
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

  let post = new PostChain(
    renderer, scene, camera, tier,
    window.innerWidth * pixelRatio, window.innerHeight * pixelRatio,
  );
  const rebuildPost = (next: QualityTier) => {
    if (next === tier) return;
    tier = next;
    applyShadowSize(tier);
    post.dispose();
    post = new PostChain(
      renderer, scene, camera, tier,
      window.innerWidth * pixelRatio, window.innerHeight * pixelRatio,
    );
  };

  for (const s of loaded) {
    s.edges.setResolution(window.innerWidth * pixelRatio, window.innerHeight * pixelRatio, pixelRatio);
    s.points?.setViewport(camera, window.innerHeight * pixelRatio);
  }

  // OrbitControls exist even during a timed screening, because `?free=1` and
  // pausing both hand the camera back — and a control that is created on
  // demand is a control that has never been tested on the path that creates it.
  const controls = new OrbitControls(camera, renderer.domElement);
  controls.target.copy(center);
  controls.enableDamping = true;
  controls.dampingFactor = 0.08;
  const director = new CameraDirector(camera, controls, params.freeCamera);

  window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
    const dpr = Math.min(window.devicePixelRatio, MAX_PIXEL_RATIO);
    post.setSize(window.innerWidth * dpr, window.innerHeight * dpr);
    for (const s of loaded) {
      s.edges.setResolution(window.innerWidth * dpr, window.innerHeight * dpr, dpr);
      s.points?.setViewport(camera, window.innerHeight * dpr);
    }
    hud.setPixelRatio(dpr);
  });

  // ---------------------------------------------------------------- bindings

  const bound = new Map<TargetBinding, BoundTarget>();
  const resolveTarget = (t: TargetBinding): BoundTarget => {
    let hit = bound.get(t);
    if (hit) return hit;
    const s = t.scene ? scenes.get(t.scene) ?? null : null;
    const ids: string[] = [];
    const indices: number[] = [];
    const mats = new Set<number>();
    if (s && t.instances) {
      for (const i of t.instances) {
        const id = s.instanceIds[i];
        if (id === undefined) continue;
        ids.push(id);
        indices.push(i);
        mats.add(s.materialOf[i]);
      }
    } else if (s && !t.instances) {
      // `--no-bundles` writes a glob and no index list. The document says so
      // and the schema permits it; what it must not do is animate the wrong
      // thing, so it animates nothing and says which glob went unbound.
      warnings.push(`target ${t.glob} has no instance list (built with --no-bundles?); not animated`);
    }
    hit = {
      scene: s,
      ids,
      indices,
      materials: [...mats],
      wholeScene: s !== null && ids.length === s.instanceIds.length,
    };
    bound.set(t, hit);
    return hit;
  };

  // Resolve every target up front, so an unbound glob is reported at load
  // rather than the first time its shot happens to come round.
  for (const shot of show.shots) {
    for (const track of shot.tracks ?? []) {
      if ('target' in track) {
        const b = resolveTarget(track.target);
        if (b.scene && b.ids.length === 0) {
          warnings.push(`${shot.id}: target ${track.target.glob} matched no instance`);
        }
        if (b.scene && !b.wholeScene && (track.kind === 'pointCloud' || track.kind === 'material')) {
          warnings.push(
            `${shot.id}: ${track.kind} on ${track.target.glob} covers ${b.ids.length} of ` +
              `${b.scene.instanceIds.length} instances, but the effect is shared across the scene`,
          );
        }
      }
    }
  }

  // ------------------------------------------------------------------- sinks

  const hud = new ShowHud(show, { director: params.director });
  // `?mute=1` means "no AudioContext", which means the clock stays on
  // `performance.now()`. See `params.ts` for why that is the honest reading of
  // a parameter whose sound does not exist until M71.
  const audio = params.muted ? undefined : makeAudioContext();
  const clock = new ShowClock(show.durationSec, {
    endless: params.loop ?? show.endless,
    audioContext: audio,
  });
  // A context that is not running yet is refused — see `useAudioContext`. It
  // is offered again whenever the browser lets it start, which is on a user
  // gesture in every browser with an autoplay policy, and never at all in a
  // headless session with no audio device. The show plays either way; only
  // which oscillator it reads changes, and `ShowClock.source` reports that.
  if (audio && !clock.useAudioContext(audio)) {
    const adopt = () => {
      void audio.resume?.().catch(() => {});
      clock.useAudioContext(audio);
    };
    audio.addEventListener?.('statechange', adopt);
    for (const ev of ['pointerdown', 'keydown', 'touchstart']) {
      window.addEventListener(ev, adopt, { once: false, passive: true });
    }
  }
  const timeline = new Timeline(show);

  // ------------------------------------------------------------- M71 bindings
  //
  // Which voice lights which scene element is **authored** — the show
  // document's `voiceEntry` cues carry a `target`, and the screenplay's whole
  // structure is four voices entering against four things in the frame. A rule
  // like "the nth scene of the shot" would be a coin flip that landed right
  // once. Voices are 1-based in the document and 0-based on a MIDI channel,
  // which is one subtraction and exactly the sort of thing to do in one place.
  const voiceBindings: VoiceBinding[] = [];
  for (const shot of show.shots ?? []) {
    for (const cue of shot.cues ?? []) {
      if (cue.kind !== 'audio') continue;
      const p = cue.payload ?? {};
      if (p.event !== 'voiceEntry' || typeof p.voice !== 'number') continue;
      const target = (p.target ?? {}) as { scene?: string; glob?: string };
      const scene = typeof target.scene === 'string' ? target.scene : (shot.scenes ?? [])[0];
      if (!scene) continue;
      const v = (p.voice as number) - 1;
      if (voiceBindings.some((b) => b.voice === v)) continue;
      voiceBindings.push({ voice: v, scene, glob: target.glob });
    }
  }
  /** Resolved once at load: which instance ids each voice lifts. */
  const liftTargets = new Map<number, { scene: SceneRuntime; ids: readonly string[] }>();

  /** Set on the frame the Kick is applied, for the harness. */
  let kickFrameAudioSec: number | null = null;
  const binder = new CueBinder({
    onSection: (label) => hud.showSection(label),
    onKick: () => {
      // DER KICK. §7 of the screenplay is explicit that the drum and the
      // camera are one event — and the camera half of it is an
      // `exponentialZoom` shot, which is *authored in the document* and
      // therefore already on show time. There is nothing to trigger here: what
      // this records is when the binding fired, which is the part M71 adds and
      // the part AC2 can actually measure.
      kickFrameAudioSec = fugue?.engine.ctx.currentTime ?? null;
    },
  });
  let fugue: FugueAudio | null = null;
  /** Voices whose lift is non-zero, so the frame loop can write a final 0 once
   * and then stop touching an untouched scene every frame. */
  const liftedVoices = new Set<number>();

  for (const b of voiceBindings) {
    const s = scenes.get(b.scene);
    if (!s) {
      warnings.push(`voice ${b.voice + 1} is bound to scene ${b.scene}, which this show has not loaded`);
      continue;
    }
    if (b.glob) {
      // A cue payload carries a glob, not an index list — `show-build` resolves
      // globs on *tracks* and has no reason to resolve one here. Rather than
      // lift the wrong bricks, lift the whole scene and say so.
      warnings.push(`voice ${b.voice + 1}: glob "${b.glob}" is not resolved; lifting all of ${b.scene}`);
    }
    liftTargets.set(b.voice, { scene: s, ids: s.instanceIds });
  }

  // The score, if there is one. `null` for every point-cloud tileset, every
  // bundle built before the audio existed, and every `?mute=1` session — the
  // same absence-is-a-fact pattern the three render modes already use.
  if (audio) {
    fugue = await loadFugueAudio(audio, baseUrl, clock, {
      onCue: (cue, atAudioSec) => binder.schedule(cue, atAudioSec),
    });
    if (fugue) {
      hud.buildMixer({
        master: fugue.engine.masterLevelValue,
        muted: fugue.engine.isMuted,
        monitor: fugue.engine.monitorValue,
        onMaster: (v) => fugue?.engine.setMasterLevel(v),
        onMuted: (v) => fugue?.engine.setMuted(v),
        onMonitor: (v) => fugue?.engine.setMonitor(v as Monitor),
      });
    }
  }

  const scratchPos = new THREE.Vector3();
  const scratchQuat = new THREE.Quaternion();
  const scratchEuler = new THREE.Euler();
  const homeQ = new THREE.Quaternion();
  const assemblyOut: [number, number, number] = [0, 0, 0];
  const touched = new Set<SceneRuntime>();
  const voices: string[] = [];
  let dtSec = 0;

  const sinks: TrackSinks = {
    transform(target, value) {
      const b = resolveTarget(target);
      const s = b.scene;
      if (!s) return;
      // Rotation is composed onto the placement's own orientation and position
      // is added to it — see the header. A quaternion, when authored, replaces
      // the Euler rather than compounding with it, exactly as the format says.
      if (value.hasRotation || value.hasQuaternion) {
        if (value.hasQuaternion) {
          scratchQuat.set(value.quaternion[0], value.quaternion[1], value.quaternion[2], value.quaternion[3]);
        } else {
          scratchEuler.set(
            (value.rotation[0] * Math.PI) / 180,
            (value.rotation[1] * Math.PI) / 180,
            (value.rotation[2] * Math.PI) / 180,
            'XYZ',
          );
          scratchQuat.setFromEuler(scratchEuler);
        }
      }
      for (let k = 0; k < b.ids.length; k++) {
        const id = b.ids[k];
        const i = b.indices[k];
        scratchPos.set(s.homePos[i * 3], s.homePos[i * 3 + 1], s.homePos[i * 3 + 2]);
        if (value.hasPosition) {
          scratchPos.x += value.position[0];
          scratchPos.y += value.position[1];
          scratchPos.z += value.position[2];
        }
        homeQ.set(s.homeQuat[i * 4], s.homeQuat[i * 4 + 1], s.homeQuat[i * 4 + 2], s.homeQuat[i * 4 + 3]);
        if (value.hasRotation || value.hasQuaternion) homeQ.premultiply(scratchQuat);
        const sc = s.homeScale[i] * (value.hasScale ? value.scale : 1);
        s.writer.setTransform(id, scratchPos, homeQ, sc);
      }
      touched.add(s);
    },

    dissolve(target, value) {
      const b = resolveTarget(target);
      if (!b.scene) return;
      b.scene.dissolve.set(b.scene.writer, b.ids, value);
      touched.add(b.scene);
    },

    material(target, property, value) {
      const b = resolveTarget(target);
      if (!b.scene) return;
      applyMaterial(b, property, value);
    },

    color(target, value) {
      const b = resolveTarget(target);
      if (!b.scene) return;
      for (const mi of b.materials) {
        // `LinearSRGBColorSpace` spelled out, exactly as `materials.ts` binds
        // `mesh.json`'s own colours — the document's numbers are already
        // linear. Letting this default, or going through `set()`/`setStyle()`,
        // would decode numbers that were never encoded and darken every brick
        // by about 2.2.
        (b.scene.materials.get(mi) as THREE.MeshPhysicalMaterial).color.setRGB(
          value[0],
          value[1],
          value[2],
          THREE.LinearSRGBColorSpace,
        );
      }
    },

    post(property, value) {
      applyPost(post, property, value);
    },

    hud(element, value, shot) {
      hudOwner.set(element, shot.id);
      hud.setValue(element, value);
    },

    pointCloud(target, value) {
      const b = resolveTarget(target);
      const s = b.scene;
      if (!s) return;
      s.points?.set(value);
      // The mesh half of the crossfade. `meshDissolveFor` is the inverse ramp,
      // finished by the halfway point — M65's own reasoning for why the two
      // halves of a `pointCloud` value do different things.
      const d = s.points ? s.points.meshDissolveFor(value) : value;
      s.dissolve.set(s.writer, b.ids, d);
      touched.add(s);
    },

    camera(track, t01, sampled) {
      director.apply(track, t01, sampled, dtSec);
    },
  };

  function applyMaterial(b: BoundTarget, property: MaterialProperty, value: number) {
    const s = b.scene!;
    if (property === 'edgeOpacity') {
      s.edges.setOpacity(value);
      return;
    }
    for (const mi of b.materials) {
      const mat = s.materials.get(mi) as THREE.MeshPhysicalMaterial;
      switch (property) {
        case 'opacity':
          mat.opacity = value;
          mat.transparent = value < 1;
          break;
        case 'roughness':
          mat.roughness = value;
          break;
        case 'metalness':
          mat.metalness = value;
          break;
        case 'emissiveIntensity':
          mat.emissiveIntensity = value;
          break;
        case 'transmission':
          mat.transmission = value;
          break;
      }
    }
  }

  function applyPost(chain: PostChain, property: PostProperty, value: number) {
    switch (property) {
      case 'exposure': chain.exposure = value; return;
      case 'bloomThreshold': chain.bloomThreshold = value; return;
      case 'bloomStrength': chain.bloomStrength = value; return;
      case 'bloomRadius': chain.bloomRadius = value; return;
      case 'vignette': chain.vignette = value; return;
      case 'gradeStrength': chain.gradeStrength = value; return;
      case 'grain': chain.grain = value; return;
    }
  }

  // ---------------------------------------------------- shared state, reset

  /** Everything a track can change that is *not* per instance, as it was
   * before any track ran.
   *
   * This exists because of a real defect, and the defect is structural rather
   * than a slip. A track only writes while its own shot is live: A1-S06 leaves
   * the vignette at 0.55, and at t=0 of the next cycle nothing writes it back,
   * because the opening shot has no post track — it has no reason to. So the
   * second time round the piece opens on a frame that is not the frame it
   * opened on the first time, with no error anywhere and no single line of
   * code that is wrong.
   *
   * The same reasoning covers a seek: jumping to 0 from the middle is the same
   * situation as a loop, and a playhead that lands somewhere different
   * depending on where it came from is not a playhead.
   *
   * Per-instance state (dissolve) is included for the same reason and by the
   * same argument. */
  const defaults = {
    exposure: post.exposure,
    vignette: post.vignette,
    gradeStrength: post.gradeStrength,
    grain: post.grain,
    bloomThreshold: post.bloomThreshold,
    bloomStrength: post.bloomStrength,
    bloomRadius: post.bloomRadius,
    materials: loaded.map((s) =>
      s.bundle.materials.map((m, i) => {
        const mat = s.materials.get(i);
        return {
          opacity: m.pbr.opacity,
          roughness: m.pbr.roughness,
          metalness: m.pbr.metalness,
          emissiveIntensity: m.pbr.emissiveIntensity,
          transmission: m.pbr.transmission,
          transparent: mat.transparent,
          // Read off the bound material rather than off `m.color`, so this is
          // whatever `materials.ts` decided the brick's colour is — including
          // any future adjustment there — and not a second opinion about it.
          color: (mat as THREE.MeshPhysicalMaterial).color.clone(),
        };
      }),
    ),
  };

  function resetSharedState() {
    // The HUD goes with it. Same argument as the post chain's: A1-S05 raises a
    // line and nothing at t=0 has any reason to mention it.
    for (const element of hudOwner.keys()) {
      hud.setValue(element, 0);
      hud.setText(element, '');
    }
    hudOwner.clear();
    post.exposure = defaults.exposure;
    post.vignette = defaults.vignette;
    post.gradeStrength = defaults.gradeStrength;
    post.grain = defaults.grain;
    post.bloomThreshold = defaults.bloomThreshold;
    post.bloomStrength = defaults.bloomStrength;
    post.bloomRadius = defaults.bloomRadius;
    loaded.forEach((s, si) => {
      s.edges.setOpacity(1);
      s.points?.set(0);
      defaults.materials[si].forEach((d, mi) => {
        const mat = s.materials.get(mi);
        mat.opacity = d.opacity;
        mat.transparent = d.transparent;
        mat.roughness = d.roughness;
        mat.metalness = d.metalness;
        mat.emissiveIntensity = d.emissiveIntensity;
        mat.transmission = d.transmission;
        (mat as THREE.MeshPhysicalMaterial).color.copy(d.color);
      });
      s.dissolve.set(s.writer, s.instanceIds, 0);
      // The generator is shot-scoped too: A1-S04's assembly must be rebuilt
      // from its own cue rather than survive into the shot the seek landed in.
      s.assembly = null;
      s.assemblyShot = null;
      // And the placements it moved go back where the bundle put them.
      for (let i = 0; i < s.instanceIds.length; i++) {
        scratchPos.set(s.homePos[i * 3], s.homePos[i * 3 + 1], s.homePos[i * 3 + 2]);
        homeQ.set(s.homeQuat[i * 4], s.homeQuat[i * 4 + 1], s.homeQuat[i * 4 + 2], s.homeQuat[i * 4 + 3]);
        s.writer.setTransform(s.instanceIds[i], scratchPos, homeQ, s.homeScale[i]);
      }
      touched.add(s);
    });
  }

  clock.onLoop(() => {
    resetSharedState();
    voices.length = 0;
    // The score has to come round with the picture. Without this the endless
    // edition plays the fugue once and then loops in silence: the scheduler's
    // cursor is monotonic by design (M70), so a show time that jumps back to
    // zero leaves it past every note in the file, for ever, with no error
    // anywhere. The seam is the same one M62's own header is about.
    fugue?.seek(0);
    binder.reset();
    suppressBlurNextFrame = true;
  });
  let suppressBlurNextFrame = false;

  // -------------------------------------------------------------------- cues

  function fireCue(cue: ResolvedCue) {
    const payload = cue.payload ?? {};
    switch (cue.kind) {
      case 'hud': {
        const element = typeof payload.element === 'string' ? payload.element : null;
        if (element && typeof payload.text === 'string') {
          hudOwner.set(element, cue.shotId);
          hud.setText(element, payload.text);
        }
        return;
      }
      case 'audio': {
        // M71 owns the fugue. What the player does today is *record* the voice
        // entries so `?director=1` can show them — the screenplay's whole
        // structure is four voices entering in order, so "which have entered
        // by now" is the thing worth being able to check.
        if (payload.event === 'voiceEntry') {
          const label = `${payload.voice ?? '?'} ${payload.range ?? ''}`.trim();
          if (!voices.includes(label)) voices.push(label);
        }
        return;
      }
      case 'seed': {
        startAssembly(cue, payload);
        return;
      }
      case 'marker':
        return;
    }
  }

  /** Re-applies the cues that carry *state* to a time we jumped to.
   *
   * `Timeline.fireCues` deliberately does not replay everything a seek skipped
   * — arriving at 3:00 must not sound every accent of the first three minutes,
   * which would be a stampede and, once M71 lands, an audible one. That is
   * right for an accent and wrong for a declaration: A1-S04's `seed` cue is
   * not an event that happens at its shot's first frame, it is the statement
   * that this shot has an assembly in it. Seeking to 0.2 s into that shot
   * moved the cursor past it and the nine parts simply never flew.
   *
   * So the rule is by *kind*, not by time: `seed` and `hud` are state and are
   * re-applied for whatever shot we landed in; `audio` and `marker` are events
   * and are not. The distinction is the one M62's own header already draws
   * between "a state change" and "a sequence of events with individual
   * meaning" — this is where it becomes code. */
  function reapplyStateCues(t: number) {
    for (const { shot } of timeline.activeShots(t)) {
      for (const cue of shot.cues ?? []) {
        if (cue.timeSec > t) continue;
        if (cue.kind === 'seed' || cue.kind === 'hud') fireCue(cue);
      }
    }
  }

  /** A1-S04's generator. The document declares it; this evaluates it.
   *
   * `scene` is required in the payload rather than inferred from the shot's
   * scene list: A1-S04 lists two scenes and only one of them assembles, and a
   * rule like "the last one" would be a coin flip that happened to land right
   * once. */
  function startAssembly(cue: ResolvedCue, payload: Record<string, unknown>) {
    const sceneId = typeof payload.scene === 'string' ? payload.scene : null;
    if (!sceneId) {
      warnings.push(`${cue.shotId}: seed cue has no "scene"; nothing assembles`);
      return;
    }
    const s = scenes.get(sceneId);
    if (!s) {
      warnings.push(`${cue.shotId}: seed cue names scene ${sceneId}, which this show has not loaded`);
      return;
    }
    if (s.assemblyShot === cue.shotId && s.assembly) return;
    const steps = (s.bundle as unknown as { instanceBuildSteps?: number[] }).instanceBuildSteps;
    s.assembly =
      assemblyFromCue(payload, { ids: s.instanceIds, finals: s.homePos, order: steps }, seed) ??
      tokenFlowFromCue(payload, s.instanceIds, seed);
    s.assemblyShot = s.assembly ? cue.shotId : null;
    if (!s.assembly) warnings.push(`${cue.shotId}: unknown generator ${String(payload.generator)}`);
  }

  // ------------------------------------------------------------------- frame

  let lastFrameMs = performance.now();
  let frames = 0;
  let fpsAccumMs = 0;
  let fps = 0;
  let benchmarkDone = params.quality !== null;
  let benchFrames = 0;
  const startedAt = performance.now();
  let prevShowTime = 0;

  const activeSceneIds = new Set<string>();
  /** Scratch for the HUD ownership sweep; reused, never reallocated. */
  const activeShotIds = new Set<string>();
  /** Which shot last addressed each HUD element, so the element can be taken
   * down when that shot ends.
   *
   * A1-S05's `monolith-metrics` is why this exists. Its own track fades the
   * line back out — 0 -> 1 -> 1 -> 0 across the shot — and the line was
   * nevertheless on screen over Uruk, over the coin, over Rome and over both
   * patents, right through to the last act. Two separate reasons, and each one
   * alone is enough:
   *
   *   - A TRACK ONLY WRITES WHILE ITS OWN SHOT IS LIVE. The closing key is at
   *     t=1 and the last frame inside the shot lands short of it, so playback
   *     leaves the element at whatever that frame sampled — a fifth of the way
   *     down the ramp, not zero — and nothing ever writes it again.
   *   - A SEEK RE-APPLIES `hud` CUES BY KIND, because they are state. Seeking
   *     to 1:43 re-runs A1-S05's text cue and does not re-run its track, since
   *     that shot is not live at 1:43. The text comes back at full strength.
   *
   * This is the same defect `resetSharedState` was built for, one layer up, and
   * it is invisible in exactly the way that argument predicts: nothing errors,
   * every number is right, and a hairline of type sits over two thirds of the
   * piece. Found by measuring the lower-right corner of all 27 documentation
   * frames rather than by looking at them.
   *
   * Ownership rather than a blanket clear, because `seed-point` is legitimately
   * addressed by three shots an act and a half apart and must keep what the
   * last of them left. */
  const hudOwner = new Map<string, string>();

  statusEl.style.display = 'none';
  const seekTo = (sec: number) => {
    resetSharedState();
    voices.length = 0;
    binder.reset();
    fugue?.seek(sec);
    clock.seek(sec);
    timeline.resetCueCursor(clock.time);
    prevShowTime = clock.time;
    reapplyStateCues(clock.time);
    // A seek is not motion. `CameraDirector` already documents that "a seek
    // passes 0 so a jump does not smear" — and until this line existed, the
    // player never passed it: jumping from the end of the act back to the
    // opening frame produced a camera velocity of three metres in one frame
    // and rendered the first shot of the piece under a full radial smear. The
    // clean-loop pixel test is what found it; nothing else could have.
    suppressBlurNextFrame = true;
  };
  if (params.seekSec !== null) seekTo(params.seekSec);

  /** Start the piece.
   *
   * # The gate is the correct behaviour, not a workaround
   *
   * No browser will start an `AudioContext` without a user gesture, so a
   * screening has to begin on one. That constraint and the piece agree: an
   * installation that begins when someone chooses to begin it is what an
   * installation *is*, and the title card was already in the screenplay. What
   * this adds is that the same gesture is the thing that starts the clock —
   * so the picture cannot run ahead of a fugue that has not been allowed to
   * start.
   *
   * `?mute=1` skips it entirely, which is the parameter's whole meaning
   * (M66): no `AudioContext`, show time from `performance.now()`, visuals
   * immediately, for embedding.
   */
  let begun = false;
  const begin = () => {
    if (begun) return;
    begun = true;
    hud.hideGate();
    void audio?.resume?.().catch(() => {});
    clock.useAudioContext(audio);
    clock.play();
    fugue?.start();
  };

  if (params.muted || !audio) {
    begin();
  } else if (audio.state === 'running') {
    // Already allowed — a kiosk with the autoplay policy relaxed, or a page
    // the user has already interacted with. Waiting for a click there would
    // be ceremony rather than consent.
    begin();
  } else {
    hud.showGate(
      begin,
      'The browser will not start audio without a gesture, and this piece is ' +
        'four voices. Beginning also starts the clock.',
    );
  }


  function frame() {
    requestAnimationFrame(frame);
    const nowMs = performance.now();
    const seeked = suppressBlurNextFrame;
    dtSec = seeked ? 0 : Math.min((nowMs - lastFrameMs) / 1000, 0.25);
    suppressBlurNextFrame = false;
    frames++;
    fpsAccumMs += nowMs - lastFrameMs;
    lastFrameMs = nowMs;
    if (fpsAccumMs >= 250) {
      fps = (frames * 1000) / fpsAccumMs;
      frames = 0;
      fpsAccumMs = 0;
    }

    clock.tick();
    const t = clock.time;
    // The timeline owns the camera whenever show time moved — playback or a
    // seek. It yields to the mouse only when time is standing still. See
    // `CameraDirector.follow`.
    director.follow = clock.playing || t !== prevShowTime || seeked;

    // Scene visibility, from the live shots' own `scenes` lists. A scene no
    // live shot names is not drawn at all — which is what keeps the monolith
    // out of the opening shot without anything having to dissolve it.
    activeSceneIds.clear();
    const active = timeline.activeShots(t);
    for (const a of active) for (const id of a.shot.scenes ?? []) activeSceneIds.add(id);
    for (const s of loaded) s.root.visible = activeSceneIds.has(s.id);

    timeline.fireCues(prevShowTime, t, fireCue);
    prevShowTime = t;
    timeline.evaluate(t, sinks);

    // Take down any HUD element whose shot has ended. After `evaluate`, so a
    // shot that is still live has already rewritten its own element this frame.
    if (hudOwner.size) {
      activeShotIds.clear();
      for (const a of active) activeShotIds.add(a.shot.id);
      for (const [element, shotId] of hudOwner) {
        if (activeShotIds.has(shotId)) continue;
        hud.setValue(element, 0);
        hud.setText(element, '');
        hudOwner.delete(element);
      }
    }

    // Generators run after the tracks, because a generator writes positions
    // and a transform track writes positions, and the shot that has both means
    // the generator.
    for (const a of active) {
      for (const s of loaded) {
        if (!s.assembly || s.assemblyShot !== a.shot.id || !activeSceneIds.has(s.id)) continue;
        // Not `AssemblyChoreography.apply`, which writes an identity
        // quaternion — right for M64's baked demo, where every part is
        // axis-aligned, and wrong for any scene with a rotated placement,
        // which would silently snap upright the moment its shot began.
        // `positionAt` is the part of that class that is actually general.
        for (let i = 0; i < s.instanceIds.length; i++) {
          s.assembly.positionAt(i, a.t01, assemblyOut);
          scratchPos.set(assemblyOut[0], assemblyOut[1], assemblyOut[2]);
          homeQ.set(s.homeQuat[i * 4], s.homeQuat[i * 4 + 1], s.homeQuat[i * 4 + 2], s.homeQuat[i * 4 + 3]);
          s.writer.setTransform(s.instanceIds[i], scratchPos, homeQ, s.homeScale[i]);
        }
        touched.add(s);
      }
    }

    for (const s of touched) {
      s.writer.flush();
      s.dissolve.update(dtSec);
      s.edges.syncMatrices();
    }
    touched.clear();

    post.setMotionBlur(director.blur, director.focus.x, director.focus.y);

    // M71 — the bindings, on audio time.
    //
    // `binder.update` is handed the *audio* clock, not the show clock and not
    // frame time, because the thing it is deciding is "has this note sounded
    // yet". Frame time drives the decays, which are properties of the picture.
    if (fugue) {
      binder.update(fugue.engine.ctx.currentTime, dtSec);
      for (const [voice, target] of liftTargets) {
        const amount = binder.lift.get(voice) ?? 0;
        if (amount === 0 && !liftedVoices.has(voice)) continue;
        if (amount === 0) liftedVoices.delete(voice);
        else liftedVoices.add(voice);
        for (const id of target.ids) target.scene.writer.setLift(id, amount);
        target.scene.writer.flush();
        touched.add(target.scene);
      }
      post.bloom.strength = BLOOM_STRENGTH + binder.bloom;
    }
    hud.updateSection(dtSec);

    const viewportPx = window.innerHeight * Math.min(window.devicePixelRatio, MAX_PIXEL_RATIO);
    for (const s of loaded) {
      if (!s.root.visible) continue;
      s.lod?.update(camera, viewportPx);
      s.edges.update(camera, viewportPx);
      s.points?.setViewport(camera, viewportPx);
    }
    if (director.isFree || !clock.playing) controls.update();

    renderer.info.reset();
    post.render((nowMs - startedAt) / 1000);

    hud.updateTitleCard(t);
    hud.setDirector({
      shot: active.length ? active[0].shot : null,
      timeSec: t,
      durationSec: show.durationSec,
      cycle: clock.cycle,
      fps,
      drawCalls: renderer.info.render.calls,
      instances: loaded.reduce((n, s) => n + s.instanceIds.length, 0),
      voices,
      clockSource: clock.source,
      cut: `${show.durationSec.toFixed(0)}s${show.endless ? ' endless' : ''}`,
      seed,
      warnings,
    });

    if (!benchmarkDone) {
      benchFrames++;
      const elapsed = nowMs - startedAt;
      if (elapsed >= 2000) {
        benchmarkDone = true;
        rebuildPost(tierFromFps((benchFrames * 1000) / elapsed));
      }
    }
  }
  frame();

  if (warnings.length) {
    for (const w of warnings) console.warn(`[spex show] ${w}`);
  }

  // The harness's handle on a running show. Same contract as `__spexMesh`:
  // measured values, not asserted ones.
  (window as unknown as Record<string, unknown>).__spexShow = {
    show,
    params,
    warnings,
    clock,
    timeline,
    director,
    hud,
    scenes: loaded,
    renderer,
    scene,
    camera,
    controls,
    post: () => post,
    quality: () => tier,
    fps: () => fps,
    drawCalls: () => renderer.info.render.calls,
    voices: () => [...voices],
    activeShotId: () => timeline.activeShots(clock.time)[0]?.shot.id ?? null,
    visibleScenes: () => loaded.filter((s) => s.root.visible).map((s) => s.id),
    seek: seekTo,
    begin,
    fugue: () => fugue,
    binder,
    kickFrameAudioSec: () => kickFrameAudioSec,
    liftOf: (voice: number) => binder.lift.get(voice) ?? 0,
    section: () => binder.section,
    bloom: () => binder.bloom,
    resetSharedState,
    setPlaying: (playing: boolean) => {
      if (playing) clock.play();
      else clock.pause();
      director.setPlaying(playing);
    },
    colorOf: (linear: [number, number, number]) => linearToCss(linear),
  };
}

/** An `AudioContext`, or nothing.
 *
 * Nothing is a legitimate answer twice over: `?mute=1` asks for it, and a
 * headless Chromium without an audio device throws on construction. Both end
 * up on `performance.now()`, and `ShowClock.source` reports which — because a
 * drift measurement without that label means nothing. */
function makeAudioContext(): AudioContext | undefined {
  try {
    const Ctor =
      (window as unknown as { AudioContext?: typeof AudioContext }).AudioContext ??
      (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    return Ctor ? new Ctor() : undefined;
  } catch {
    return undefined;
  }
}
