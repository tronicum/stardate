/** M56 — LDraw colours become real three.js materials.
 *
 * The manifest's colours are already **linear** (see `bundle.ts`), so they go
 * into `Color.setRGB(..., LinearSRGBColorSpace)` verbatim. Passing them
 * through any sRGB conversion here would brighten every material by roughly
 * 2.2x — the single easiest way to make real LDraw colours look wrong.
 *
 * The PBR numbers are **not** decided here. `spex_mesh::material` resolves
 * every finish to metalness/roughness/clearcoat/transmission and writes them
 * into `mesh.json`, with the reasoning attached to each constant. This file
 * only binds them. That split is deliberate: the bundle should say what a
 * material *is*, so a second renderer resolves the same brick to the same
 * look without re-deriving anyone's table.
 */

import * as THREE from 'three';
import type { MeshBundle, MeshMaterial, MeshPart } from './bundle';

function linearColor(rgb: [number, number, number]): THREE.Color {
  return new THREE.Color().setRGB(rgb[0], rgb[1], rgb[2], THREE.LinearSRGBColorSpace);
}

/** A synthetic environment, built from geometry and gradients rather than
 * loaded from an HDRI.
 *
 * Chrome and metal are *only* their reflections — a chrome brick with nothing
 * to reflect is a flat grey brick, which is what M54 shipped. But shipping an
 * HDRI would break two of this project's own rules at once: no asset that
 * isn't real data, and nothing that needs the network at show time. So the
 * environment is a room: a vertical gradient for sky and floor, plus three
 * emissive cards standing in for practical lights. Deterministic, about 4 KB
 * of code, and reflections in it read as a room because it is one.
 */
export function buildSyntheticEnvironment(renderer: THREE.WebGLRenderer): THREE.Texture {
  const scene = new THREE.Scene();

  // A studio, not a landscape. The floor is a light sweep rather than a void
  // — a mirrored 1x1 brick spends most of its reflection looking downwards,
  // and against a near-black floor chrome came out at linear 0.22 and read as
  // grey plastic. Measured by rendering the environment as the background and
  // sampling it at the height a brick's flank actually reflects.
  const sky = new THREE.Mesh(
    new THREE.SphereGeometry(10, 32, 16),
    new THREE.ShaderMaterial({
      side: THREE.BackSide,
      depthWrite: false,
      uniforms: {
        // Dynamic range is the whole point, and it is what a three-stop
        // gradient cannot express on its own.
        //
        // A dielectric's diffuse term integrates the environment over the
        // hemisphere, so it responds to *irradiance* — broad and dim wins.
        // A metal's specular term samples one direction, so it responds to
        // *radiance* — narrow and bright wins. Every attempt to serve both
        // with one brightness failed in a measurable way: dim, and chrome
        // came out at linear 0.22 and read as grey plastic; bright, and real
        // LDraw Red clipped its own channel and rendered orange.
        //
        // What actually worked, measured at each step: a moderately bright
        // gradient with a **light floor sweep** rather than a void — that is
        // what a mirrored 1x1 brick spends most of its reflection looking at
        // — plus a modest band at the horizon and three small, very bright
        // cards. Chrome went from linear 0.22 (grey plastic) to sRGB 206
        // (mirror) without the dielectrics blowing out.
        top: { value: new THREE.Color().setRGB(1.0, 1.06, 1.2, THREE.LinearSRGBColorSpace) },
        horizon: { value: new THREE.Color().setRGB(1.5, 1.58, 1.75, THREE.LinearSRGBColorSpace) },
        bottom: { value: new THREE.Color().setRGB(0.55, 0.58, 0.66, THREE.LinearSRGBColorSpace) },
        band: { value: new THREE.Color().setRGB(2.5, 2.6, 2.9, THREE.LinearSRGBColorSpace) },
      },
      vertexShader: `
        varying float vH;
        void main() {
          vH = normalize(position).y * 0.5 + 0.5;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }`,
      fragmentShader: `
        uniform vec3 top; uniform vec3 horizon; uniform vec3 bottom; uniform vec3 band;
        varying float vH;
        void main() {
          // Wide, soft transitions. A narrow bright ring is a hard edge in
          // every reflection, and a mirror finds it — which is exactly why
          // the band below is deliberate and the gradient's steps are not.
          vec3 c = vH < 0.5
            ? mix(bottom, horizon, smoothstep(0.05, 0.50, vH))
            : mix(horizon, top, smoothstep(0.50, 0.95, vH));
          c += band * (1.0 - smoothstep(0.0, 0.045, abs(vH - 0.52)));
          gl_FragColor = vec4(c, 1.0);
        }`,
    }),
  );
  scene.add(sky);

  // Three practicals — small and bright, which is what a real softbox is:
  // high luminance over a small solid angle. A metal reflects the environment
  // *sharply*, so it sees a card's luminance; a dielectric integrates the
  // environment over the hemisphere, so it sees a card's solid angle.
  //
  // "Almost nothing to the diffuse" turned out to be wrong at the first
  // numbers tried, and only measurement caught it. A 1.3x1.8 card at 4.7
  // units subtends about 0.85 % of the sphere — at intensity 260 that is
  // still ~2.2 of irradiance each, and switching the direct lights off proved
  // it: real LDraw Red rendered rgb(255,120,88) from the environment *alone*,
  // while the whole direct rig contributed rgb(40,0,0). The cards were the
  // scene's lighting and nobody had said so. At these intensities they light
  // the highlights and the rig lights the scene, which is what was intended.
  //
  // Three of them, at different sizes and heights, because a single card
  // gives chrome one dot and reads as shiny plastic. Two sit near eye level
  // on purpose — that is what a standing brick's flanks actually see.
  const cards: Array<[number, number, number, number, number, number]> = [
    // x, y, z, width, height, intensity
    [3.5, 1.8, 3.0, 1.3, 1.8, 40.0],
    [-4.5, 1.0, -1.5, 1.6, 1.1, 20.0],
    [0.0, 5.5, 2.0, 2.0, 1.0, 14.0],
  ];
  for (const [x, y, z, w, h, i] of cards) {
    const card = new THREE.Mesh(
      new THREE.PlaneGeometry(w, h),
      new THREE.MeshBasicMaterial({
        color: new THREE.Color().setRGB(i, i, i * 1.02, THREE.LinearSRGBColorSpace),
        side: THREE.DoubleSide,
      }),
    );
    card.position.set(x, y, z);
    card.lookAt(0, 0, 0);
    scene.add(card);
  }

  const pmrem = new THREE.PMREMGenerator(renderer);
  const target = pmrem.fromScene(scene, 0.04);
  pmrem.dispose();
  sky.geometry.dispose();
  (sky.material as THREE.Material).dispose();
  return target.texture;
}

/** Builds and caches one three.js material per LDraw colour, and resolves a
 * part's submeshes against a given instance colour.
 *
 * A part's geometry carries no colour of its own (M51 keeps LDraw code 16
 * unresolved), which is exactly what lets one mesh serve every colour it is
 * placed in — the basis of M55's instancing. A submesh whose `material` is
 * `null` means "inherit"; anything else is a fixed accent colour molded into
 * the part itself, and stays that colour no matter how the instance is
 * coloured. */
export class MaterialLibrary {
  private readonly materials: MeshMaterial[];
  private readonly cache = new Map<number, THREE.MeshPhysicalMaterial>();
  private environment: THREE.Texture | null = null;

  constructor(bundle: MeshBundle) {
    this.materials = bundle.materials;
  }

  /** Must be called before the first `get()` for chrome and metal to have
   * anything to reflect. Separate from the constructor because it needs a
   * live renderer, and the library is built before one exists. */
  setEnvironment(texture: THREE.Texture) {
    this.environment = texture;
    for (const m of this.cache.values()) {
      m.envMap = texture;
      m.needsUpdate = true;
    }
  }

  /** The three.js material for one LDraw colour index, created once. */
  get(index: number): THREE.MeshPhysicalMaterial {
    const cached = this.cache.get(index);
    if (cached) return cached;
    const entry = this.materials[index];
    if (!entry) throw new Error(`mesh bundle references material ${index}, which it does not define`);
    const p = entry.pbr;
    const transparent = p.opacity < 1;

    const material = new THREE.MeshPhysicalMaterial({
      color: linearColor(entry.baseColor),
      metalness: p.metalness,
      roughness: p.roughness,
      clearcoat: p.clearcoat,
      clearcoatRoughness: p.clearcoatRoughness,
      transmission: p.transmission,
      ior: p.ior,
      iridescence: p.iridescence,
      iridescenceIOR: p.iridescenceIOR,
      opacity: p.opacity,
      transparent,
      // A transparent brick reads *because* you can see its own tubes and the
      // brick behind it. Front-face culling deletes exactly that, and
      // depth-writing makes it hide whatever is drawn after it. Both are
      // wrong here and right everywhere else — which is why this is the one
      // place backface culling is off.
      side: transparent ? THREE.DoubleSide : THREE.FrontSide,
      depthWrite: !transparent,
      // M57: push faces *away* from the viewer by a slope-dependent amount so
      // the edge lines that sit exactly on them win the depth test — without
      // biasing the edges themselves toward the viewer. Biasing the edges was
      // tried first and is wrong in a way that is obvious once seen: a
      // constant pull forward large enough to beat the face it lies on is
      // also large enough to let the brick's *interior* edges show through
      // its front wall. polygonOffset moves only the coincident surface, and
      // scales with the polygon's own depth slope, so it holds at every
      // camera distance.
      polygonOffset: true,
      polygonOffsetFactor: 1,
      polygonOffsetUnits: 1,
    });
    if (p.emissiveIntensity > 0) {
      // Glow-in-the-dark: LDraw's real LUMINANCE, emitting its own colour.
      material.emissive = linearColor(entry.baseColor);
      material.emissiveIntensity = p.emissiveIntensity;
    }
    if (this.environment) material.envMap = this.environment;
    material.name = `${entry.name} (${entry.finish})`;
    this.cache.set(index, material);
    return material;
  }

  /** LDraw's own edge colour for a colour index, linear. Used by M57's line
   * pass — it belongs with the material it describes. */
  edgeColor(index: number): THREE.Color {
    const entry = this.materials[index];
    if (!entry) throw new Error(`mesh bundle references material ${index}, which it does not define`);
    return linearColor(entry.edgeColor);
  }

  /** Materials for one part's geometry groups, in submesh order, with
   * `null` submeshes resolved to the instance's own colour. */
  resolve(part: MeshPart, instanceMaterial: number): THREE.MeshPhysicalMaterial[] {
    return part.submeshes.map((s) => this.get(s.material ?? instanceMaterial));
  }

  /** How many distinct finishes the bundle actually contains — the number
   * M56's acceptance criterion is about, read off the real data rather than
   * asserted. */
  finishes(): string[] {
    return [...new Set(this.materials.map((m) => m.finish))].sort();
  }

  dispose() {
    for (const m of this.cache.values()) m.dispose();
    this.cache.clear();
  }

  get count(): number {
    return this.cache.size;
  }
}
