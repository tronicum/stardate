/** M65 part 2 — the point↔mesh crossfade.
 *
 * The piece opens on a point that becomes a brick and ends with every brick
 * becoming points. This is the module that lets the show cross between its
 * own two render modes on screen, which is also the moment the *existing*
 * point pipeline earns its keep inside the mesh show rather than being
 * superseded by it.
 *
 * # The two halves of the crossfade do different things
 *
 * A `pointCloud` track's value runs 0 (mesh) to 1 (swarm), and the two halves
 * are not the same transition:
 *
 * - **0 → 0.5** the *representation* changes. The mesh erodes away through
 *   M65's dissolve while the cloud fades up, and the points sit exactly on
 *   the surface they were sampled from. At 0.5 both are on screen and
 *   spatially identical — which is what makes the swap legible rather than a
 *   dip to nothing and back.
 * - **0.5 → 1** the *object* comes apart. Only now do the points drift
 *   outward along their own normals. This is Act IV's Inkpour.
 *
 * Spreading from 0 instead would have looked the same in a still and been
 * wrong in motion: the object would start dissolving and expanding at once,
 * so nothing is ever *both* representations of the same shape, and the
 * moment the piece is about — a statistical cloud and a countable thing being
 * the same object — never happens.
 *
 * # One draw call per group, not per instance
 *
 * The cloud is an `InstancedBufferGeometry`: the part's points as ordinary
 * attributes, the brick index as an instanced one, drawn as `POINTS`. So a
 * 5 000-brick site is one draw call of 5 000 × N points, not 5 000 draw
 * calls — the same bargain `instanced.ts` and `edges.ts` already make, and
 * for the same reason.
 *
 * The instance matrices come from **M57's own texture**, shared rather than
 * duplicated: a second copy would be a second upload every frame and one more
 * thing that can be a frame out of date.
 */

import * as THREE from 'three';
import { MATRICES_PER_ROW, TEXELS_PER_MATRIX, type EdgeGroup } from '../mesh/edges';
import type { InstanceGroup } from '../mesh/instanced';
import type { MaterialLibrary } from '../mesh/materials';
import type { MeshBundle } from '../mesh/bundle';

/** How far a point drifts at value 1, as a multiple of its own part's radius.
 *
 * Relative and not absolute: the first version used a flat 26 mm, which on a
 * 200 mm monolith is a gentle loosening and on an 8 mm brick throws the swarm
 * clean off the frame. 1.6 radii means the object roughly triples in extent —
 * visibly no longer an object, still occupying the space it did. The Inkpour
 * is a dissolution, not an explosion. */
export const SPREAD_RADII = 1.6;
/** A point's physical radius, in millimetres.
 *
 * Physical and not "N device pixels", because the scene is in millimetres and
 * a fixed pixel size means a swarm that looks like dust close up and like a
 * solid mass from far away. 0.35 mm is roughly a stud's fillet — small enough
 * that a 1x1 brick's 1261 points read as points rather than as paint.
 *
 * The first version wrote `uSize / -mv.z * 1000.0`, which assumed the scene
 * was in metres. Every point came out clamped at the 24 px ceiling and the
 * brick rendered as one solid red blob. */
export const POINT_RADIUS_MM = 0.08;
/** Ceiling, in device pixels. A point the camera is almost inside would
 * otherwise become a full-screen quad. */
export const MAX_POINT_PX = 14;
/** Floor, in device pixels.
 *
 * Two, not one, and M66's A1-S02 is why. The physical size is right and at a
 * viewing distance of 200 mm it works out to 0.95 px — so every point clamped
 * to the old floor of 1, and a swarm of 1 261 single pixels at the far end of
 * a two-bar dolly is a shot in which nothing visibly happens. A point that
 * exists has to be seen to exist; below about two pixels it is indistinguishable
 * from sensor noise, and the piece's whole opening gesture is *one point*
 * becoming *many*. Above this floor the size is physical again. */
export const MIN_POINT_PX = 2;

const VERTEX_SHADER = /* glsl */ `
precision highp float;

in vec3 aNormal;
in float aBrick;

uniform sampler2D uMatrices;
uniform float uSpread;      // millimetres along the normal
uniform float uRadius;      // millimetres
uniform float uProjScale;   // viewportHeightPx / (2 tan(fov/2)) — the same
                            // projection constant M57 and M59 gate on

out float vSize;            // device pixels, for the round mask below

mat4 brickMatrix(float index) {
  int i = int(index) * ${TEXELS_PER_MATRIX};
  int w = ${MATRICES_PER_ROW * TEXELS_PER_MATRIX};
  ivec2 p = ivec2(i % w, i / w);
  return mat4(
    texelFetch(uMatrices, p,               0),
    texelFetch(uMatrices, p + ivec2(1, 0), 0),
    texelFetch(uMatrices, p + ivec2(2, 0), 0),
    texelFetch(uMatrices, p + ivec2(3, 0), 0)
  );
}

void main() {
  mat4 model = brickMatrix(aBrick);
  // The normal is rotated by the instance's own matrix, not left in object
  // space: a brick placed upside down has to pour downward.
  vec3 n = normalize(mat3(model) * aNormal);
  vec4 world = model * vec4(position, 1.0);
  world.xyz += n * uSpread;
  vec4 mv = viewMatrix * world;
  gl_Position = projectionMatrix * mv;
  // Real projected size: a sphere of radius uRadius at distance -mv.z
  // subtends 2*r*k/d device pixels. Same arithmetic as the edge gate and the
  // LOD selector, so "how big is this on screen" means one thing everywhere.
  vSize = clamp(2.0 * uRadius * uProjScale / max(-mv.z, 0.001), ${MIN_POINT_PX}.0, ${MAX_POINT_PX}.0);
  gl_PointSize = vSize;
}
`;

const FRAGMENT_SHADER = /* glsl */ `
precision highp float;

uniform vec3 uColor;
uniform float uOpacity;

in float vSize;

layout(location = 0) out vec4 pc_fragColor;

void main() {
  // Round, and soft at the edge. A square point reads as a pixel artefact;
  // this reads as a particle, which is what it is standing in for.
  //
  // But only above a few pixels. A two-pixel sprite has no shape to round off,
  // and gl_PointCoord for a point that small is one or two samples whose exact
  // values are not something the spec pins down — round it and a driver that
  // reports the corner rather than the centre discards the whole swarm. Below
  // the threshold the point is its own antialiasing.
  // (No backticks in here: this comment lives inside a JS template literal.)
  float a = uOpacity;
  if (vSize > 3.0) {
    vec2 d = gl_PointCoord - vec2(0.5);
    float r = dot(d, d);
    if (r > 0.25) discard;
    a *= 1.0 - smoothstep(0.10, 0.25, r);
  }
  pc_fragColor = vec4(uColor, a);
}
`;

export interface PointCloudGroup {
  source: InstanceGroup;
  points: THREE.Points;
  material: THREE.ShaderMaterial;
  /** Points in one instance's cloud. */
  pointsPerInstance: number;
  /** The part's own bounding radius, in millimetres — what the spread scales
   * against. */
  radiusMm: number;
}

/** Reads one part's `p<N>.pts.bin` — 24 bytes a point, position then normal,
 * already in the output frame. */
export async function fetchPartPoints(baseUrl: string, path: string): Promise<Float32Array> {
  const res = await fetch(`${baseUrl.replace(/\/$/, '')}/${path}`);
  if (!res.ok) throw new Error(`point buffer ${path}: ${res.status}`);
  return new Float32Array(await res.arrayBuffer());
}

/** Builds one point cloud per instance group that has a point buffer.
 *
 * `edgeGroups` is used only for its matrix textures — see the header. A group
 * with no matching edge group gets no cloud rather than a private texture:
 * every bundle the CLI writes has edges, and silently paying for a second
 * per-frame upload would be worse than not drawing.
 */
export function buildPointClouds(
  bundle: MeshBundle,
  buffers: Map<number, Float32Array>,
  materials: MaterialLibrary,
  groups: InstanceGroup[],
  edgeGroups: readonly EdgeGroup[],
): PointCloudGroup[] {
  const textureFor = new Map<InstanceGroup, THREE.DataTexture>();
  for (const e of edgeGroups) textureFor.set(e.source, e.matrixTexture);

  const out: PointCloudGroup[] = [];
  for (const group of groups) {
    const data = buffers.get(group.part);
    const tex = textureFor.get(group);
    if (!data || !tex || data.length === 0) continue;

    const n = data.length / 6;
    const position = new Float32Array(n * 3);
    const normal = new Float32Array(n * 3);
    for (let i = 0; i < n; i++) {
      position[i * 3] = data[i * 6];
      position[i * 3 + 1] = data[i * 6 + 1];
      position[i * 3 + 2] = data[i * 6 + 2];
      normal[i * 3] = data[i * 6 + 3];
      normal[i * 3 + 1] = data[i * 6 + 4];
      normal[i * 3 + 2] = data[i * 6 + 5];
    }

    const geometry = new THREE.InstancedBufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(position, 3));
    geometry.setAttribute('aNormal', new THREE.BufferAttribute(normal, 3));
    const brick = new Float32Array(group.ids.length);
    for (let i = 0; i < brick.length; i++) brick[i] = i;
    geometry.setAttribute('aBrick', new THREE.InstancedBufferAttribute(brick, 1));
    geometry.instanceCount = group.ids.length;
    // The cloud occupies the part's own bounds plus however far it may drift.
    const part = bundle.parts[group.part];
    geometry.boundingSphere = new THREE.Box3(
      new THREE.Vector3(...part.bounds.min),
      new THREE.Vector3(...part.bounds.max),
    ).getBoundingSphere(new THREE.Sphere());

    const material = new THREE.ShaderMaterial({
      glslVersion: THREE.GLSL3,
      vertexShader: VERTEX_SHADER,
      fragmentShader: FRAGMENT_SHADER,
      transparent: true,
      // Points are a swarm: they overlap constantly, and depth-writing would
      // make whichever happened to draw first hide the rest.
      depthWrite: false,
      uniforms: {
        uMatrices: { value: tex },
        uSpread: { value: 0 },
        uRadius: { value: POINT_RADIUS_MM },
        uProjScale: { value: 500 },
        uOpacity: { value: 0 },
        // The instance's own **edge** colour, so a Terrakotta brick pours
        // Terrakotta. This is why the buffer is colour-neutral: one cloud,
        // every colour.
        //
        // The edge value and not the base colour, and that took a black brick
        // to notice. A point is not lit — it has no normal that matters at one
        // pixel and no shading model behind it — so it draws at its own
        // colour, and LDraw Black is linear 0.011, which against this piece's
        // background is nothing at all. M66's A1-S02 is two bars of a swarm
        // that was rendering perfectly and could not be seen.
        //
        // `EDGE` is the value LDConfig already publishes for exactly this
        // question: how a colour should read when it is a line rather than a
        // surface. Black's is a mid grey. It is a real number from the
        // library rather than a fudge factor, it is the same one M57's outline
        // pass uses, and it keeps the cloud recognisably the object's own
        // colour for every colour that has one.
        uColor: { value: new THREE.Color().copy(materials.edgeColor(group.material)) },
      },
    });

    const points = new THREE.Points(geometry, material);
    points.frustumCulled = false;
    points.visible = false;
    points.name = `points:${part.partFile}#${group.material}`;
    out.push({
      source: group,
      points,
      material,
      pointsPerInstance: n,
      radiusMm: geometry.boundingSphere.radius,
    });
  }
  return out;
}

/** Drives the crossfade. */
export class PointCloudRenderer {
  readonly groups: PointCloudGroup[];
  /** Last value applied, 0 mesh .. 1 swarm. */
  value = 0;

  constructor(groups: PointCloudGroup[]) {
    this.groups = groups;
  }

  get pointCount(): number {
    return this.groups.reduce((n, g) => n + g.pointsPerInstance * g.source.ids.length, 0);
  }

  addTo(parent: THREE.Object3D) {
    for (const g of this.groups) parent.add(g.points);
  }

  /** Call on resize and whenever the field of view changes — M63's camera
   * director changes it per shot. */
  setViewport(camera: THREE.PerspectiveCamera, viewportHeightPx: number) {
    const k = viewportHeightPx / (2 * Math.tan((camera.fov * Math.PI) / 180 / 2));
    for (const g of this.groups) g.material.uniforms.uProjScale.value = k;
  }

  /** `value` 0..1. See the header for why the spread only starts at 0.5. */
  set(value: number) {
    const v = value < 0 ? 0 : value > 1 ? 1 : value;
    this.value = v;
    // Fades in over the first half and holds: the cloud is at full strength
    // for the whole of the coming-apart, because a swarm that is still
    // fading in while it disperses reads as an error rather than an event.
    const opacity = Math.min(1, v * 2);
    const t = Math.max(0, v - 0.5) * 2;
    for (const g of this.groups) {
      g.material.uniforms.uOpacity.value = opacity;
      g.material.uniforms.uSpread.value = t * SPREAD_RADII * g.radiusMm;
      g.points.visible = opacity > 0.001;
    }
  }

  /** What the mesh's dissolve should be for the same value: the inverse ramp,
   * finished by the halfway point.
   *
   * Exposed rather than hidden inside `set` because the dissolve is written
   * per *instance* through `InstanceWriter`, and only the caller knows which
   * instances this track addresses. */
  meshDissolveFor(value: number): number {
    const v = value < 0 ? 0 : value > 1 ? 1 : value;
    return Math.min(1, v * 2);
  }
}
