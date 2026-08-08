/** M65 — dissolve and materialise.
 *
 * A brick that has to leave the screen can fade, or it can come apart. The
 * piece needs the second: Act IV's *Inkpour* is everything on screen
 * dissolving into points, and A2-S02's bulla is a brick that stops being a
 * brick. A uniform opacity ramp reads as "the renderer is fading something
 * out"; a noise-driven erosion with a lit edge reads as the object itself
 * going.
 *
 * # It is one attribute, not one material per instance
 *
 * `InstanceWriter.setDissolve` has written into a per-instance
 * `aDissolve` attribute since M55 — the hook was put there for this
 * milestone. That matters more than it looks: giving each instance its own
 * material to animate would turn a 29-draw-call scene into a
 * 50 000-draw-call one, which is the entire benefit of instancing spent on a
 * transition.
 *
 * # Injected, not replaced
 *
 * The dissolve is patched into `MeshPhysicalMaterial` through
 * `onBeforeCompile` rather than written as a `ShaderMaterial`. A brick's look
 * is clearcoat, transmission, iridescence, IBL and shadow — M56's whole
 * argument — and a hand-written shader would have to reproduce all of it to
 * dissolve it. Three lines of injected GLSL keep every one of those intact.
 *
 * # Materialise is dissolve backwards, plus one thing
 *
 * Running the ramp from 1 to 0 gives an object assembling itself out of
 * nothing, which is right, and it ends on a completely ordinary frame, which
 * is wrong — the *arrival* is the event. So completion carries a short
 * emissive flash, decaying over `FLASH_DECAY_SEC`. That is the accent, and
 * it is what makes a materialise land on a beat rather than merely finish
 * near one.
 */

import * as THREE from 'three';

/** Width of the lit edge, in noise units. Wide enough to survive the
 * 8-bit quantisation of a dark scene, narrow enough to read as an edge and
 * not a glow. */
export const RIM_WIDTH = 0.06;
/** How long the materialise flash takes to fall away. Just under a beat at
 * 84 bpm (0.714 s), so it is finished before the next one arrives. */
export const FLASH_DECAY_SEC = 0.45;

/** The noise, in one place — shared by the surface shader, the shadow shader
 * and M57's edge shader.
 *
 * All three have to erode at the same *rate*, and that is a stronger
 * requirement than it sounds. The edge pass first used a plain uniform hash
 * per edge, which is a perfectly good random threshold and gave completely
 * the wrong picture: smoothed two-octave value noise is concentrated around
 * 0.5, so at a threshold of 0.56 most of the surface is gone while only about
 * half the edges are — the bricks read as turning into wire cages rather than
 * eroding. Same field, same distribution, same rate.
 *
 * Also: exported rather than copied, because two implementations of a hash
 * function is the most reliable way for "the same fragments" to stop being
 * true. */
export const NOISE_GLSL = /* glsl */ `
  float dissolveHash(vec3 p) {
    p = fract(p * 0.3183099 + vec3(0.1, 0.2, 0.3));
    p *= 17.0;
    return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
  }
  float dissolveNoise(vec3 x) {
    vec3 i = floor(x);
    vec3 f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
      mix(mix(dissolveHash(i + vec3(0,0,0)), dissolveHash(i + vec3(1,0,0)), f.x),
          mix(dissolveHash(i + vec3(0,1,0)), dissolveHash(i + vec3(1,1,0)), f.x), f.y),
      mix(mix(dissolveHash(i + vec3(0,0,1)), dissolveHash(i + vec3(1,0,1)), f.x),
          mix(dissolveHash(i + vec3(0,1,1)), dissolveHash(i + vec3(1,1,1)), f.x), f.y),
      f.z);
  }`;

/** Injected into every solid material once, at construction.
 *
 * The noise is a cheap value-noise on the *object-space* position, so it
 * stays welded to the brick as it moves: world-space noise would make a
 * dissolving object appear to swim through a fixed cloud, which is the tell
 * that gives away every cheap version of this effect.
 */
export function applyDissolveChunks(material: THREE.Material, rimColor: THREE.Color): void {
  const uniforms = {
    uRimColor: { value: rimColor },
    uRimWidth: { value: RIM_WIDTH },
    uNoiseScale: { value: 0.35 },
    uFlash: { value: 0 },
    // The entry lift's colour. Warmer than the dissolve rim on purpose: the
    // rim is the object coming apart and the lift is a line of music arriving,
    // and if the two read as the same event the binding has said nothing.
    uLiftColor: { value: new THREE.Color(1.0, 0.86, 0.62) },
  };
  (material as unknown as { userData: Record<string, unknown> }).userData.dissolve = uniforms;

  material.onBeforeCompile = (shader) => {
    Object.assign(shader.uniforms, uniforms);

    shader.vertexShader = shader.vertexShader
      .replace(
        '#include <common>',
        `#include <common>
        attribute float aDissolve;
        attribute float aLift;
        varying float vDissolve;
        varying float vLift;
        varying vec3 vDissolvePos;`,
      )
      .replace(
        '#include <begin_vertex>',
        `#include <begin_vertex>
        vDissolve = aDissolve;
        vLift = aLift;
        vDissolvePos = position;`,
      );

    shader.fragmentShader = shader.fragmentShader
      .replace(
        '#include <common>',
        `#include <common>
        uniform vec3 uRimColor;
        uniform float uRimWidth;
        uniform float uNoiseScale;
        uniform float uFlash;
        uniform vec3 uLiftColor;
        #define LIFT_SCALE 0.18
        varying float vDissolve;
        varying float vLift;
        varying vec3 vDissolvePos;

        ${NOISE_GLSL}`,
      )
      .replace(
        '#include <clipping_planes_fragment>',
        `#include <clipping_planes_fragment>
        float dNoise = dissolveNoise(vDissolvePos * uNoiseScale);
        // Two octaves, because one produces visible blobs at brick scale —
        // a 1x1 brick is 8 mm across and the base frequency has about two
        // features in it.
        dNoise = mix(dNoise, dissolveNoise(vDissolvePos * uNoiseScale * 3.7), 0.35);
        // The threshold is pushed a hair past 1 at full dissolve so the last
        // fragments actually go: at exactly 1.0 the noise's own maximum
        // survives, and an object that dissolves to three stubborn texels is
        // worse than one that does not dissolve at all.
        float dThreshold = vDissolve * (1.0 + uRimWidth);
        if (dNoise < dThreshold) discard;`,
      )
      .replace(
        '#include <dithering_fragment>',
        `#include <dithering_fragment>
        // The rim: fragments that only just survived. Added after tone
        // mapping is *not* an option here — this is the linear-HDR path, and
        // an emissive rim below 1.0 would never reach bloom.
        float dRim = 1.0 - smoothstep(0.0, uRimWidth, dNoise - dThreshold);
        gl_FragColor.rgb += uRimColor * dRim * step(0.001, vDissolve) * 2.5;
        gl_FragColor.rgb += uRimColor * uFlash;
        // M71's entry lift, per instance. Added in the same place and for the
        // same reason as the rim: this is the linear-HDR path, and an emissive
        // addition below 1.0 would never reach the bloom pass.
        //
        // **The scale is 0.18 and the first version was 1.6.** That number was
        // chosen by analogy with the rim above, which is wrong by a factor of
        // the object: the rim multiplies a dRim that is non-zero on a thin
        // (no backticks in here — this is inside a JS template literal, and
        // one would end the string; the same trap M66 hit in edges.ts)
        // band of fragments that only just survived the erosion, while this
        // multiplies every fragment of the brick. The screenshot pair measured
        // the difference at **89 luma out of 255** on a white monolith — not a
        // voice announcing itself, an object replaced by a light source. An
        // entry is an accent inside a texture, and it has to still be a brick
        // afterwards.
        gl_FragColor.rgb += uLiftColor * vLift * LIFT_SCALE;`,
      );
  };
  material.customProgramCacheKey = () => 'spex-dissolve-lift';
}

/** Anything with a per-instance dissolve channel. `InstanceWriter` satisfies
 * this; so does a recording object in a harness. */
export interface DissolveTarget {
  setDissolve(id: string, amount: number): void;
  flush(): void;
}

/** Drives a dissolve or a materialise across a set of instances, and owns the
 * flash that ends a materialise. */
export class DissolveController {
  /** Flash amount, 0..1, decaying. Read by whatever owns the materials. */
  flash = 0;

  private readonly materials: THREE.Material[];
  private lastAmount = new Map<string, number>();

  constructor(materials: THREE.Material[]) {
    this.materials = materials;
  }

  /** `amount` is 0 solid, 1 gone. A materialise is the same call with the
   * value running the other way — there is no separate mode, because "the
   * object is 30 % gone" does not depend on which direction it is heading,
   * and a mode would be one more thing to get out of sync with the
   * timeline. */
  set(target: DissolveTarget, ids: readonly string[], amount: number): void {
    const a = amount < 0 ? 0 : amount > 1 ? 1 : amount;
    for (const id of ids) {
      if (this.lastAmount.get(id) === a) continue;
      this.lastAmount.set(id, a);
      target.setDissolve(id, a);
    }
    target.flush();
  }

  /** Call when a materialise reaches 0. Idempotent within one flash. */
  triggerFlash(): void {
    this.flash = 1;
  }

  /** Decays the flash and pushes it into the materials. */
  update(dtSec: number): void {
    if (this.flash > 0) {
      this.flash = Math.max(0, this.flash - dtSec / FLASH_DECAY_SEC);
    }
    for (const m of this.materials) {
      const u = (m as unknown as { userData: { dissolve?: { uFlash: { value: number } } } }).userData
        .dissolve;
      if (u) u.uFlash.value = this.flash;
    }
  }
}

/** The same erosion, for the shadow pass.
 *
 * three.js renders shadows with its own depth material, which knows nothing
 * about `aDissolve` — so without this a fully dissolved object still casts a
 * complete shadow. That is not a subtle artefact: the second run of
 * `dissolve.mjs` measured the object making the frame *darker* than the empty
 * scene at dissolve = 1.0, which is a shadow of nothing and could not be
 * anything else.
 *
 * No rim here. A rim is a lit edge, and the shadow pass has no lighting; the
 * only thing that has to agree between the two is *which fragments exist*.
 */
export function makeDissolveDepthMaterial(): THREE.MeshDepthMaterial {
  const material = new THREE.MeshDepthMaterial({ depthPacking: THREE.RGBADepthPacking });
  const uniforms = { uNoiseScale: { value: 0.35 }, uRimWidth: { value: RIM_WIDTH } };
  material.onBeforeCompile = (shader) => {
    Object.assign(shader.uniforms, uniforms);
    shader.vertexShader = shader.vertexShader
      .replace(
        '#include <common>',
        `#include <common>
        attribute float aDissolve;
        attribute float aLift;
        varying float vDissolve;
        varying float vLift;
        varying vec3 vDissolvePos;`,
      )
      .replace(
        '#include <begin_vertex>',
        `#include <begin_vertex>
        vDissolve = aDissolve;
        vLift = aLift;
        vDissolvePos = position;`,
      );
    shader.fragmentShader = shader.fragmentShader
      .replace('#include <common>', `#include <common>\n${NOISE_GLSL}
        uniform float uNoiseScale;
        uniform float uRimWidth;
        varying float vDissolve;
        varying vec3 vDissolvePos;`)
      .replace(
        '#include <clipping_planes_fragment>',
        `#include <clipping_planes_fragment>
        float dNoise = dissolveNoise(vDissolvePos * uNoiseScale);
        dNoise = mix(dNoise, dissolveNoise(vDissolvePos * uNoiseScale * 3.7), 0.35);
        if (dNoise < vDissolve * (1.0 + uRimWidth)) discard;`,
      );
  };
  material.customProgramCacheKey = () => 'spex-dissolve-depth';
  return material;
}
