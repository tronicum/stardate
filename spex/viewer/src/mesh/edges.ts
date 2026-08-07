/** M57 — crisp edges and conditional edges. The *vektorgenau* milestone.
 *
 * A brick without its outline reads as a soft plastic blob. With it, it reads
 * as the catalogue picture everybody already has in their head. That outline
 * is not a post-process trick: it is real geometry that ships inside every
 * LDraw part, as type-2 lines (always drawn) and type-5 conditional lines
 * (drawn only on a silhouette).
 *
 * Three things make this harder than "draw some lines".
 *
 * **`LineBasicMaterial.linewidth` is ignored on every real platform.** The
 * WebGL core profile only guarantees 1 px lines. So each edge is expanded
 * into a screen-space quad in the vertex shader — the standard fat-line
 * technique — which is also what makes the width constant in *device pixels*
 * regardless of distance.
 *
 * **A conditional edge's test is per frame and per camera.** It is drawn when
 * its two control points project to the **same** side of the line (rev 1 of
 * the spec had this inverted, which would have drawn the cylinder's
 * tessellation and hidden the silhouette — review 01, B1). That has to run on
 * the GPU, and both control points are handed to all four corners of the quad
 * so every corner reaches the same verdict; a per-corner decision produces
 * flickering half-quads.
 *
 * **Edges must not z-fight the faces they bound.** They sit exactly on the
 * surface, so they get a small depth bias, in clip space, proportional to `w`
 * — a constant offset would be right at one distance and wrong at every
 * other, and the screenplay uses camera distances two orders of magnitude
 * apart.
 */

import * as THREE from 'three';
import { NOISE_GLSL } from '../show/dissolve';
import type { MeshBundle, PartBuffers } from './bundle';
import type { MaterialLibrary } from './materials';
import type { InstanceGroup } from './instanced';

/** Device pixels, at DPR 1. The rev 3 corrections give 1.25 px at DPR 1 and
 * 1.6 px at DPR >= 2 — a thin line on a high-density screen disappears. */
export const DEFAULT_EDGE_WIDTH = 1.25;
export const HIDPI_EDGE_WIDTH = 1.6;

/** Clip-space depth offset, as a fraction of `w`.
 *
 * **Zero by default, and that is the finding.** The obvious approach — pull
 * the edges toward the viewer until they beat the faces they lie on — cannot
 * work: any offset large enough to win against a coplanar face is also large
 * enough to let a brick's *interior* edges show through its front wall, and
 * on a hollow LDraw brick that is most of them. The real fix is at the other
 * end, on the solid material: `polygonOffset`, which pushes only the surface
 * back and scales with its own depth slope. This uniform stays for tuning at
 * extreme camera distances, where it can be nudged without breaking that. */
export const DEFAULT_DEPTH_BIAS = 0;

/** Below this projected height, in device pixels, a part's outline stops
 * being an outline and becomes a smear. The rev 3 corrections set the
 * threshold at ~40 px: at Atlas distance every brick's outline merges into
 * one black mass anyway, so drawing them costs a fortune and buys a worse
 * picture. */
/** Raised from 40 to 56 by M59 so that this agrees with the LOD1 promote
 * threshold: geometric outlines switch off at exactly the size the studs
 * they outline stop being drawn. An outline of a stud that is not there is
 * worse than no outline. */
export const MIN_PROJECTED_HEIGHT_PX = 56;

/** Hard ceiling on geometric edge quads for a whole scene.
 *
 * This is the number that makes the gate necessary rather than nice. A 1x4
 * brick carries 248 hard and 112 conditional lines, so 50 000 of them is
 * 21.6 million quads — about 1.2 GB of attributes, which is not a slow
 * renderer, it is a browser tab that dies at load. WebGL2 has no
 * instancing-of-instances, so there is no cheaper encoding available here.
 * Above this budget the geometric pass does not get built at all, and the
 * crowd case belongs to a screen-space depth+normal-discontinuity outline
 * whose cost is independent of instance count (not built yet; recorded in
 * the milestone note rather than half-done).
 *
 * 1.5 M quads is roughly 84 MB of attributes, and covers the ~3 000 bricks
 * the corrections name as the hero-shot ceiling. */
export const MAX_EDGE_QUADS = 1_500_000;

/** Instance matrices are handed to the shader as a texture rather than as
 * per-edge attributes. One brick has hundreds of edges, so duplicating its
 * 16-float matrix into every one of them would be the largest buffer in the
 * scene; a texel fetch costs nothing and — more importantly — moving a brick
 * later (M60's choreography) is then one texel write instead of hundreds of
 * attribute writes. */
export const MATRICES_PER_ROW = 64;
export const TEXELS_PER_MATRIX = 4;

export interface EdgeGroup {
  /** The instance group these edges belong to — same part, same colour. */
  source: InstanceGroup;
  mesh: THREE.Mesh;
  material: THREE.ShaderMaterial;
  /** Instance matrices, shared with the InstancedMesh's own buffer. */
  matrixTexture: THREE.DataTexture;
  /** M65: per-instance dissolve, refreshed from `source.dissolve`. */
  dissolveTexture: THREE.DataTexture;
  hardEdgeCount: number;
  conditionalEdgeCount: number;
  /** Quads submitted: (hard + conditional) x brick instances. */
  quadCount: number;
}

const VERTEX_SHADER = /* glsl */ `
precision highp float;

in vec2 aCorner;          // (0|1 along the edge, -1|+1 across it)
in vec3 aStart;
in vec3 aEnd;
in vec3 aCtrl0;
in vec3 aCtrl1;
in float aConditional;    // 1.0 for a type-5 line
in float aBrick;          // row into the instance-matrix texture

uniform sampler2D uMatrices;
uniform sampler2D uDissolve;   // M65: one texel per instance, R = 0 solid .. 1 gone
${NOISE_GLSL}
uniform vec2 uResolution;     // device pixels
uniform float uWidth;         // device pixels
uniform float uDepthBias;

flat out float vDiscard;

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

/** Screen-space position in device pixels, or w <= 0 if behind the camera. */
vec3 toScreen(vec4 clip) {
  return vec3(clip.xy / clip.w * uResolution * 0.5, clip.w);
}

void main() {
  // M65. A brick's outline has to go with its surface, or a fully dissolved
  // object leaves a perfect wireframe of itself hanging in the air — which is
  // a striking picture and entirely the wrong one. Each edge gets its own
  // threshold from a hash of its start point, so the outline erodes rather
  // than switching off, and it erodes at the same rate as the faces because
  // both read the same per-instance value.
  int dissolveRow = int(aBrick) / ${MATRICES_PER_ROW};
  int dissolveCol = int(aBrick) - dissolveRow * ${MATRICES_PER_ROW};
  float dissolve = texelFetch(uDissolve, ivec2(dissolveCol, dissolveRow), 0).r;
  if (dissolve > 0.0) {
    // The SAME noise field the surface uses, sampled at the edge's midpoint,
    // and not a uniform per-edge hash. See NOISE_GLSL: a uniform hash
    // erodes linearly while smoothed value noise erodes around its mean, so
    // the two came apart in the middle of every dissolve and the bricks read
    // as turning into wire cages.
    vec3 mid = (aStart + aEnd) * 0.5;
    float n = dissolveNoise(mid * 0.35);
    n = mix(n, dissolveNoise(mid * 0.35 * 3.7), 0.35);
    if (n < dissolve * 1.06) {
      // Off-screen and degenerate: cheaper than a discard in the fragment
      // stage, and it removes the quad rather than making it invisible.
      gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
      vDiscard = 1.0;
      return;
    }
  }

  mat4 model = brickMatrix(aBrick);
  mat4 mvp = projectionMatrix * viewMatrix * model;

  vec4 clipA = mvp * vec4(aStart, 1.0);
  vec4 clipB = mvp * vec4(aEnd, 1.0);

  // Near-plane clipping. Without it an edge with one endpoint behind the
  // camera projects to a wild screen position and smears a bright streak
  // across the frame — which is exactly what happens at the 0.5x camera
  // distance the screenplay uses.
  if (clipA.w <= 0.0 && clipB.w <= 0.0) {
    vDiscard = 1.0;
    gl_Position = vec4(0.0, 0.0, 2.0, 1.0);
    return;
  }
  if (clipA.w <= 0.0) {
    float t = (1e-5 - clipA.w) / (clipB.w - clipA.w);
    clipA = mix(clipA, clipB, t);
  } else if (clipB.w <= 0.0) {
    float t = (1e-5 - clipB.w) / (clipA.w - clipB.w);
    clipB = mix(clipB, clipA, t);
  }

  vec3 screenA = toScreen(clipA);
  vec3 screenB = toScreen(clipB);

  vDiscard = 0.0;
  if (aConditional > 0.5) {
    vec4 clipC0 = mvp * vec4(aCtrl0, 1.0);
    vec4 clipC1 = mvp * vec4(aCtrl1, 1.0);
    if (clipC0.w <= 0.0 || clipC1.w <= 0.0) {
      // A control point behind the camera has no defined side. Dropping the
      // edge is the conservative answer: a missing silhouette line is a
      // smaller error than a line drawn across a cylinder.
      vDiscard = 1.0;
    } else {
      vec2 dir = screenB.xy - screenA.xy;
      vec2 c0 = clipC0.xy / clipC0.w * uResolution * 0.5 - screenA.xy;
      vec2 c1 = clipC1.xy / clipC1.w * uResolution * 0.5 - screenA.xy;
      float s0 = dir.x * c0.y - dir.y * c0.x;
      float s1 = dir.x * c1.y - dir.y * c1.x;
      // Drawn when both control points fall on the SAME side. Inverting this
      // draws the tessellation and hides the silhouette.
      if (s0 * s1 < 0.0) vDiscard = 1.0;
    }
  }

  if (vDiscard > 0.5) {
    gl_Position = vec4(0.0, 0.0, 2.0, 1.0); // outside the depth range: culled
    return;
  }

  vec2 delta = screenB.xy - screenA.xy;
  float len = length(delta);
  vec2 dir = len > 1e-6 ? delta / len : vec2(1.0, 0.0);
  vec2 normal = vec2(-dir.y, dir.x);

  vec4 clip = mix(clipA, clipB, aCorner.x);
  float half_w = uWidth * 0.5;
  // Across the line, plus half a width past each cap so that two edges
  // meeting at a corner overlap instead of leaving a notch — AC1's "no gaps
  // at the stud's silhouette".
  vec2 offsetPx = normal * (half_w * aCorner.y) + dir * (half_w * (aCorner.x * 2.0 - 1.0));

  clip.xy += offsetPx / uResolution * 2.0 * clip.w;
  // Toward the viewer, proportional to w: a constant offset is correct at
  // exactly one distance.
  clip.z -= uDepthBias * clip.w;
  gl_Position = clip;
}
`;

const FRAGMENT_SHADER = /* glsl */ `
precision highp float;
uniform vec3 uColor;
flat in float vDiscard;
layout(location = 0) out vec4 pc_fragColor;
void main() {
  if (vDiscard > 0.5) discard;
  pc_fragColor = vec4(uColor, 1.0);
}
`;

/** The four corners of one edge quad, and the two triangles over them. */
function quadGeometry(): { corners: Float32Array; index: Uint16Array } {
  return {
    corners: new Float32Array([0, -1, 0, 1, 1, -1, 1, 1]),
    index: new Uint16Array([0, 1, 2, 2, 1, 3]),
  };
}

/** Wraps an `InstancedMesh`'s own matrix array in a float texture, without
 * copying it. `InstanceWriter.flush()` marks it dirty alongside the mesh. */
function matrixTexture(group: InstanceGroup): THREE.DataTexture {
  const count = group.ids.length;
  const rows = Math.ceil(count / MATRICES_PER_ROW);
  const width = MATRICES_PER_ROW * TEXELS_PER_MATRIX;
  const data = new Float32Array(width * rows * 4);
  // `group.matrices`, not `group.mesh.instanceMatrix`: M59 re-packs the LOD
  // meshes every time an instance changes level, so their row order is not
  // instance order any more. An edge indexes an *instance*.
  data.set(group.matrices);
  const tex = new THREE.DataTexture(data, width, rows, THREE.RGBAFormat, THREE.FloatType);
  tex.needsUpdate = true;
  return tex;
}

/** One texel per instance, R = the same 0..1 the solid material reads.
 *
 * A texture rather than a per-quad attribute because a quad is per *edge*,
 * and a 1x4 brick has 360 of them: duplicating one float across 360 quads per
 * instance would cost more upload than the matrices do. Same width as the
 * matrix texture so one index arithmetic serves both. */
function dissolveTexture(group: InstanceGroup): THREE.DataTexture {
  const rows = Math.ceil(group.ids.length / MATRICES_PER_ROW);
  const data = new Float32Array(MATRICES_PER_ROW * rows);
  const tex = new THREE.DataTexture(data, MATRICES_PER_ROW, rows, THREE.RedFormat, THREE.FloatType);
  tex.needsUpdate = true;
  return tex;
}

/** Builds one edge mesh per instance group.
 *
 * Every (edge, brick) pair is one instanced quad. That is a real multiplier —
 * a 1x4 brick has 248 hard and 112 conditional edges, so nine of them is
 * 3 240 quads — which is exactly why the rev 3 corrections cap geometric
 * edges at hero-shot scale and hand crowd distance to a screen-space pass.
 * `shouldDrawEdges` below is that gate.
 */
export function buildEdgeGroups(
  bundle: MeshBundle,
  buffers: Map<number, PartBuffers>,
  materials: MaterialLibrary,
  groups: InstanceGroup[],
): EdgeGroup[] {
  const { corners, index } = quadGeometry();
  const out: EdgeGroup[] = [];

  const wanted = groups.reduce((n, g) => {
    const p = bundle.parts[g.part];
    return n + (p ? (p.hardEdgeCount + p.conditionalEdgeCount) * g.ids.length : 0);
  }, 0);
  if (wanted > MAX_EDGE_QUADS) {
    // Loud, and with the real number: a silently missing outline reads as a
    // rendering bug for as long as it takes someone to find this line.
    console.warn(
      `[spex] geometric edges skipped: this scene wants ${wanted.toLocaleString()} edge quads, ` +
      `over the ${MAX_EDGE_QUADS.toLocaleString()} budget. Crowd-scale outlines need the ` +
      `screen-space pass (see docs/fugen/phase1-renderer.md, M57).`,
    );
    return out;
  }

  for (const group of groups) {
    const part = bundle.parts[group.part];
    const b = buffers.get(group.part);
    if (!part || !b) continue;
    const hard = part.hardEdgeCount;
    const cond = part.conditionalEdgeCount;
    const edges = hard + cond;
    const instances = group.ids.length;
    if (edges === 0 || instances === 0) continue;

    const total = edges * instances;
    const aStart = new Float32Array(total * 3);
    const aEnd = new Float32Array(total * 3);
    const aCtrl0 = new Float32Array(total * 3);
    const aCtrl1 = new Float32Array(total * 3);
    const aConditional = new Float32Array(total);
    const aBrick = new Float32Array(total);

    let w = 0;
    for (let inst = 0; inst < instances; inst++) {
      for (let e = 0; e < hard; e++) {
        const o = e * 6;
        aStart.set([b.hardEdge[o], b.hardEdge[o + 1], b.hardEdge[o + 2]], w * 3);
        aEnd.set([b.hardEdge[o + 3], b.hardEdge[o + 4], b.hardEdge[o + 5]], w * 3);
        aBrick[w] = inst;
        w++;
      }
      for (let e = 0; e < cond; e++) {
        const o = e * 12;
        aStart.set([b.condEdge[o], b.condEdge[o + 1], b.condEdge[o + 2]], w * 3);
        aEnd.set([b.condEdge[o + 3], b.condEdge[o + 4], b.condEdge[o + 5]], w * 3);
        aCtrl0.set([b.condEdge[o + 6], b.condEdge[o + 7], b.condEdge[o + 8]], w * 3);
        aCtrl1.set([b.condEdge[o + 9], b.condEdge[o + 10], b.condEdge[o + 11]], w * 3);
        aConditional[w] = 1;
        aBrick[w] = inst;
        w++;
      }
    }

    const geometry = new THREE.InstancedBufferGeometry();
    geometry.instanceCount = total;
    geometry.setAttribute('aCorner', new THREE.BufferAttribute(corners, 2));
    geometry.setIndex(new THREE.BufferAttribute(index, 1));
    geometry.setAttribute('aStart', new THREE.InstancedBufferAttribute(aStart, 3));
    geometry.setAttribute('aEnd', new THREE.InstancedBufferAttribute(aEnd, 3));
    geometry.setAttribute('aCtrl0', new THREE.InstancedBufferAttribute(aCtrl0, 3));
    geometry.setAttribute('aCtrl1', new THREE.InstancedBufferAttribute(aCtrl1, 3));
    geometry.setAttribute('aConditional', new THREE.InstancedBufferAttribute(aConditional, 1));
    geometry.setAttribute('aBrick', new THREE.InstancedBufferAttribute(aBrick, 1));
    // The quad is built in screen space from attributes the bounding-sphere
    // maths knows nothing about, so let three.js draw it unconditionally
    // (`frustumCulled = false` below) and let the solid pass own culling. The
    // radius is finite rather than Infinity: three.js still reads this in
    // places where an infinity turns into a NaN.
    geometry.boundingSphere = new THREE.Sphere(new THREE.Vector3(), 1e9);

    const tex = matrixTexture(group);
    const dtex = dissolveTexture(group);
    const material = new THREE.ShaderMaterial({
      glslVersion: THREE.GLSL3,
      // A screen-space quad's winding flips with the direction of the line it
      // expands, so half of them are back-facing at any camera angle. With
      // the default `FrontSide` the draw call is issued and rasterises
      // nothing at all — which looks exactly like a broken shader and is not
      // one. Cost is zero: these quads are 1.25 px wide.
      side: THREE.DoubleSide,
      vertexShader: VERTEX_SHADER,
      fragmentShader: FRAGMENT_SHADER,
      uniforms: {
        uMatrices: { value: tex },
        uResolution: { value: new THREE.Vector2(1, 1) },
        uWidth: { value: DEFAULT_EDGE_WIDTH },
        uDepthBias: { value: DEFAULT_DEPTH_BIAS },
        // LDraw's own EDGE value for this colour, linear — not a hardcoded
        // black. A black brick's edge is lighter than the brick.
        uColor: { value: materials.edgeColor(group.material) },
        uDissolve: { value: dtex },
      },
    });

    const mesh = new THREE.Mesh(geometry, material);
    mesh.frustumCulled = false;
    mesh.name = `edges:${part.partFile}#${group.material}`;
    // Edges belong to the object, not to the light: they neither cast nor
    // receive shadows, and a 1.25 px quad in a shadow map is noise.
    mesh.castShadow = false;
    mesh.receiveShadow = false;

    out.push({
      source: group,
      mesh,
      material,
      matrixTexture: tex,
      dissolveTexture: dtex,
      hardEdgeCount: hard,
      conditionalEdgeCount: cond,
      quadCount: total,
    });
  }

  return out;
}

/** Keeps the per-frame uniforms and the instance-matrix texture current. */
export class EdgeRenderer {
  readonly groups: EdgeGroup[];
  private width = DEFAULT_EDGE_WIDTH;
  private bias = DEFAULT_DEPTH_BIAS;
  private enabled = true;

  constructor(groups: EdgeGroup[]) {
    this.groups = groups;
  }

  get quadCount(): number {
    return this.groups.reduce((n, g) => n + g.quadCount, 0);
  }

  get conditionalCount(): number {
    return this.groups.reduce((n, g) => n + g.conditionalEdgeCount * g.source.ids.length, 0);
  }

  /** How many groups are currently drawing, after the projected-height gate. */
  get visibleGroupCount(): number {
    return this.groups.reduce((n, g) => n + (g.mesh.visible ? 1 : 0), 0);
  }

  addTo(scene: THREE.Scene | THREE.Group) {
    for (const g of this.groups) scene.add(g.mesh);
  }

  setVisible(visible: boolean) {
    this.enabled = visible;
    for (const g of this.groups) g.mesh.visible = visible;
  }

  /** Per-frame gate: a part whose outline projects to less than
   * `MIN_PROJECTED_HEIGHT_PX` is not outlined at all. One sphere-vs-camera
   * test per group per frame — the cost does not grow with instance count,
   * which is the whole point. */
  update(camera: THREE.PerspectiveCamera, viewportHeightPx: number) {
    if (!this.enabled) return;
    this.syncDissolve();
    const fovRad = (camera.fov * Math.PI) / 180;
    const k = viewportHeightPx / (2 * Math.tan(fovRad / 2));
    for (const g of this.groups) {
      const sphere = g.source.mesh.geometry.boundingSphere;
      if (!sphere) continue;
      const dist = Math.max(camera.position.distanceTo(g.source.mesh.position) , 1e-6);
      const projected = (2 * sphere.radius * k) / dist;
      g.mesh.visible = projected >= MIN_PROJECTED_HEIGHT_PX;
    }
  }

  /** Device pixels. Called on resize too, because DPR can change when a
   * window moves between screens. */
  setResolution(widthPx: number, heightPx: number, dpr: number) {
    this.width = dpr >= 2 ? HIDPI_EDGE_WIDTH : DEFAULT_EDGE_WIDTH;
    for (const g of this.groups) {
      g.material.uniforms.uResolution.value.set(widthPx, heightPx);
      g.material.uniforms.uWidth.value = this.width;
    }
  }

  setWidth(px: number) {
    this.width = px;
    for (const g of this.groups) g.material.uniforms.uWidth.value = px;
  }

  setDepthBias(bias: number) {
    this.bias = bias;
    for (const g of this.groups) g.material.uniforms.uDepthBias.value = bias;
  }

  get depthBias(): number {
    return this.bias;
  }

  /** Re-uploads instance matrices. Call after `InstanceWriter.flush()` in any
   * frame that moved a brick. */
  syncMatrices() {
    for (const g of this.groups) {
      (g.matrixTexture.image.data as Float32Array).set(g.source.matrices);
      g.matrixTexture.needsUpdate = true;
    }
    this.syncDissolve();
  }

  /** M65: the outline reads the same per-instance dissolve the surface does,
   * refreshed from the one authoritative attribute rather than written twice
   * by the caller.
   *
   * Called from `update()` as well as from `syncMatrices()`: dissolving is
   * not moving, and a shot that erodes a still object would otherwise leave
   * its wireframe hanging in the air — which is exactly what the first run of
   * `dissolve.mjs` photographed. */
  syncDissolve() {
    for (const g of this.groups) {
      (g.dissolveTexture.image.data as Float32Array).set(g.source.dissolve.array as Float32Array);
      g.dissolveTexture.needsUpdate = true;
    }
  }

  dispose() {
    for (const g of this.groups) {
      g.mesh.geometry.dispose();
      g.material.dispose();
      g.matrixTexture.dispose();
    }
  }
}

/** A CPU mirror of the shader's conditional-edge predicate, for verification
 * only. Returns *which* edges pass, not how many.
 *
 * M57's AC2 wants proof the test actually runs. Rev 1 phrased that as "the
 * count changes between angles", and the count is the wrong quantity: a
 * cylinder is a body of revolution, so it has exactly **two** silhouette
 * edges from every direction. Measured on a real `3005.dat` stud: 2 drawn out
 * of 16, at all twelve orbit angles. The identity of those two rotates with
 * the camera; the number never moves. Asserting on the count would have
 * failed a correct renderer.
 *
 * Reading this back off the GPU would need a second pass whose only purpose
 * is counting, so this recomputes the same predicate in JS for one instance
 * of one part. It is a mirror, not the authority: the authority is the
 * screenshots, which show whether the cylinder has tessellation lines across
 * it. If the two ever disagree, believe the picture.
 */
export function visibleConditionalEdges(
  group: EdgeGroup,
  camera: THREE.Camera,
  resolution: THREE.Vector2,
): number[] {
  const geometry = group.mesh.geometry;
  const start = geometry.getAttribute('aStart') as THREE.BufferAttribute;
  const end = geometry.getAttribute('aEnd') as THREE.BufferAttribute;
  const c0 = geometry.getAttribute('aCtrl0') as THREE.BufferAttribute;
  const c1 = geometry.getAttribute('aCtrl1') as THREE.BufferAttribute;
  const cond = geometry.getAttribute('aConditional') as THREE.BufferAttribute;

  camera.updateMatrixWorld();
  const mvp = new THREE.Matrix4()
    .multiplyMatrices(camera.projectionMatrix, camera.matrixWorldInverse);
  const m = new THREE.Matrix4();
  m.fromArray(group.source.mesh.instanceMatrix.array as ArrayLike<number>, 0);
  mvp.multiply(m);

  const v = new THREE.Vector4();
  const screen = (i: number, attr: THREE.BufferAttribute): [number, number] | null => {
    v.set(attr.getX(i), attr.getY(i), attr.getZ(i), 1).applyMatrix4(mvp);
    if (v.w <= 0) return null;
    return [(v.x / v.w) * resolution.x * 0.5, (v.y / v.w) * resolution.y * 0.5];
  };

  const drawn: number[] = [];
  // Only the first brick instance: this is a signal, not a census.
  for (let i = 0; i < group.hardEdgeCount + group.conditionalEdgeCount; i++) {
    if (cond.getX(i) < 0.5) continue;
    const a = screen(i, start);
    const b = screen(i, end);
    const p0 = screen(i, c0);
    const p1 = screen(i, c1);
    if (!a || !b || !p0 || !p1) continue;
    const dx = b[0] - a[0];
    const dy = b[1] - a[1];
    const s0 = dx * (p0[1] - a[1]) - dy * (p0[0] - a[0]);
    const s1 = dx * (p1[1] - a[1]) - dy * (p1[0] - a[0]);
    if (s0 * s1 >= 0) drawn.push(i);
  }
  return drawn;
}
