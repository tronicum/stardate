/** M58 — the post-processing and lighting pipeline.
 *
 * Act IV is neon, the Kick is a flash, and the whole piece has to hold one
 * filmic look. Doing that once here is cheaper than fighting it per shot.
 *
 * **The order is the milestone.** The rev 3 corrections rewrote it, and the
 * reason is worth keeping in front of whoever edits this next:
 *
 *  1. The scene renders into a **HalfFloat** target with the renderer's tone
 *     mapping OFF, so what lands there is linear HDR — values above 1.0
 *     survive instead of being clipped at the point where they still matter.
 *  2. Bloom reads that. A bloom threshold is a statement about scene
 *     radiance; if tone mapping has already squashed everything into 0..1,
 *     the threshold is a statement about nothing, and every value that could
 *     have bloomed is gone before bloom sees it.
 *  3. Tone mapping (ACES) happens **last**, in the grade pass, together with
 *     the sRGB encode.
 *  4. And then dither, because a wide soft gradient over near-black on an
 *     8-bit backbuffer *bands*, visibly, and no amount of care upstream
 *     fixes that. Triangular +/-0.5/255 noise plus a little fixed grain is
 *     the cheapest thing in this whole document that separates "cheap WebGL"
 *     from "print".
 */

import * as THREE from 'three';
import { EffectComposer } from 'three/addons/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/addons/postprocessing/RenderPass.js';
import { ShaderPass } from 'three/addons/postprocessing/ShaderPass.js';
import { UnrealBloomPass } from 'three/addons/postprocessing/UnrealBloomPass.js';
import { SMAAPass } from 'three/addons/postprocessing/SMAAPass.js';
import { SSAOPass } from 'three/addons/postprocessing/SSAOPass.js';

export type QualityTier = 'low' | 'medium' | 'high';

/** The bloom pass's resting strength. A constant rather than a literal
 * because M71's accents add to it and have to know what to add back down to. */
export const BLOOM_STRENGTH = 0.45;

export interface TierSettings {
  ssao: boolean;
  shadowMapSize: number;
  bloomResolutionScale: number;
  smaa: boolean;
}

/** Low must stay *watchable*, not merely runnable. So it keeps bloom and the
 * grade — the things that carry the look — and gives up the two that cost
 * most per pixel and read least: SSAO, and half the shadow map. */
export const TIERS: Record<QualityTier, TierSettings> = {
  low: { ssao: false, shadowMapSize: 1024, bloomResolutionScale: 0.5, smaa: false },
  medium: { ssao: false, shadowMapSize: 2048, bloomResolutionScale: 0.75, smaa: true },
  high: { ssao: true, shadowMapSize: 2048, bloomResolutionScale: 1.0, smaa: true },
};

export interface GradeParams {
  /** ACES exposure, applied before tone mapping. */
  exposure: number;
  /** 0 = none, 1 = heavy. Darkens the corners. */
  vignette: number;
  /** How far toward the graded look to go. The grade itself is a lift/gamma/
   * gain curve rather than a LUT texture: a LUT is an asset, and this project
   * does not ship assets it did not compute. M62 animates this. */
  gradeStrength: number;
  /** Fixed film grain, as a fraction. ~1.5 % is the documented value. */
  grain: number;
}

export const DEFAULT_GRADE: GradeParams = {
  exposure: 0.7,
  vignette: 0.35,
  gradeStrength: 0.6,
  grain: 0.015,
};

/** Tone map + grade + sRGB encode + dither, in that order, in one pass.
 *
 * This replaces three's `OutputPass` rather than following it: the dither has
 * to be applied in the *encoded* signal, immediately before the 8-bit write,
 * and anything that runs after an OutputPass is already quantised. */
const GRADE_SHADER = {
  uniforms: {
    tDiffuse: { value: null as THREE.Texture | null },
    uExposure: { value: DEFAULT_GRADE.exposure },
    uVignette: { value: DEFAULT_GRADE.vignette },
    uGrade: { value: DEFAULT_GRADE.gradeStrength },
    uGrain: { value: DEFAULT_GRADE.grain },
    uTime: { value: 0 },
  },
  vertexShader: /* glsl */ `
    varying vec2 vUv;
    void main() {
      vUv = uv;
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }`,
  fragmentShader: /* glsl */ `
    uniform sampler2D tDiffuse;
    uniform float uExposure;
    uniform float uVignette;
    uniform float uGrade;
    uniform float uGrain;
    uniform float uTime;
    varying vec2 vUv;

    // Narkowicz's ACES fit — the same curve three.js uses, inlined so this
    // pass owns the whole tail of the pipeline and the order is readable in
    // one place.
    vec3 aces(vec3 x) {
      const float a = 2.51, b = 0.03, c = 2.43, d = 0.59, e = 0.14;
      return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
    }

    vec3 toSRGB(vec3 c) {
      return mix(c * 12.92, 1.055 * pow(max(c, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055,
                 step(0.0031308, c));
    }

    float hash(vec2 p) {
      return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
    }

    void main() {
      vec3 color = texture2D(tDiffuse, vUv).rgb;

      // Vignette, in linear light, before tone mapping — darkening after the
      // curve crushes the corners instead of exposing them down.
      float d = distance(vUv, vec2(0.5)) * 1.4142;
      color *= mix(1.0, smoothstep(1.0, 0.25, d), uVignette);

      color = aces(color * uExposure);

      // Lift / gamma / gain: a cool shadow lift and a slight warm gain, which
      // is what makes a render read as photographed rather than computed.
      vec3 graded = color;
      graded = graded * vec3(1.03, 1.0, 0.97) + vec3(0.004, 0.004, 0.012);
      graded = pow(max(graded, vec3(0.0)), vec3(0.98, 1.0, 1.02));
      color = mix(color, clamp(graded, 0.0, 1.0), uGrade);

      color = toSRGB(color);

      // Triangular dither, +/- half a code value. Two uniform samples make a
      // triangular distribution, which is what decorrelates the quantisation
      // error instead of merely hiding it.
      float n1 = hash(gl_FragCoord.xy + uTime);
      float n2 = hash(gl_FragCoord.yx * 1.7 - uTime);
      color += ((n1 + n2) - 1.0) / 255.0;

      // Fixed grain: does not move with the frame, so it reads as film stock
      // rather than as video noise.
      color += (hash(gl_FragCoord.xy * 0.37) - 0.5) * uGrain;

      gl_FragColor = vec4(color, 1.0);
    }`,
};

/** The whole chain, plus the knobs M62's timeline will drive. */
/** M63 — camera-velocity radial blur.
 *
 * **A stylistic approximation, and named as one.** Real motion blur needs a
 * velocity buffer: a previous-frame matrix per object and a second render
 * target, which at Atlas scale costs more than the effect is worth. This
 * streaks radially outward from the shot's own focus point instead, driven by
 * how fast the camera is moving. For a dolly or the Kick's zoom that is very
 * nearly the truth — the motion really is radial from the focus. For an orbit
 * it is plausible and no more, and nothing here should be read as physics.
 *
 * It sits *before* the grade pass so it blurs linear radiance rather than
 * encoded pixels — a smear of already-tone-mapped values darkens as it
 * spreads, which is the wrong direction for a bright object streaking.
 */
export const RADIAL_BLUR_SHADER = {
  uniforms: {
    tDiffuse: { value: null as THREE.Texture | null },
    uStrength: { value: 0 },
    uFocus: { value: new THREE.Vector2(0.5, 0.5) },
  },
  vertexShader: /* glsl */ `
    varying vec2 vUv;
    void main() {
      vUv = uv;
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }
  `,
  fragmentShader: /* glsl */ `
    uniform sampler2D tDiffuse;
    uniform float uStrength;
    uniform vec2 uFocus;
    varying vec2 vUv;

    const int SAMPLES = 12;

    void main() {
      vec4 base = texture2D(tDiffuse, vUv);
      if (uStrength <= 0.0) { gl_FragColor = base; return; }
      vec2 toward = uFocus - vUv;
      vec4 sum = vec4(0.0);
      float wsum = 0.0;
      for (int i = 0; i < SAMPLES; i++) {
        float f = float(i) / float(SAMPLES - 1);
        // Weighted toward the unblurred sample, so an object keeps a core
        // instead of dissolving evenly into its own trail.
        float w = 1.0 - f * 0.65;
        sum += texture2D(tDiffuse, vUv + toward * f * uStrength * 0.25) * w;
        wsum += w;
      }
      gl_FragColor = sum / wsum;
    }
  `,
};

export class PostChain {
  readonly composer: EffectComposer;
  readonly bloom: UnrealBloomPass;
  readonly radialBlur: ShaderPass;
  readonly grade: ShaderPass;
  readonly ssao: SSAOPass | null;
  readonly smaa: SMAAPass | null;
  readonly tier: QualityTier;
  private readonly params: GradeParams = { ...DEFAULT_GRADE };

  constructor(
    renderer: THREE.WebGLRenderer,
    scene: THREE.Scene,
    camera: THREE.PerspectiveCamera,
    tier: QualityTier,
    width: number,
    height: number,
  ) {
    this.tier = tier;
    const settings = TIERS[tier];

    // HalfFloat, and this is load-bearing: an 8-bit target would clip every
    // value above 1.0 before bloom ever sees it, which makes a bloom
    // threshold meaningless.
    this.composer = new EffectComposer(
      renderer,
      new THREE.WebGLRenderTarget(width, height, {
        type: THREE.HalfFloatType,
        colorSpace: THREE.LinearSRGBColorSpace,
        samples: settings.smaa ? 0 : 0,
      }),
    );
    this.composer.setSize(width, height);

    this.composer.addPass(new RenderPass(scene, camera));

    if (settings.ssao) {
      const ssao = new SSAOPass(scene, camera, width, height);
      // The rev 3 corrections are blunt about this: SSAO at a default radius
      // misses the stud annulus entirely and darkens silhouettes instead,
      // which is the opposite of what makes a brick read. Kept small and
      // subtle; baked per-vertex AO is the real answer and belongs upstream.
      ssao.kernelRadius = 2;
      ssao.minDistance = 0.0005;
      ssao.maxDistance = 0.05;
      this.composer.addPass(ssao);
      this.ssao = ssao;
    } else {
      this.ssao = null;
    }

    this.bloom = new UnrealBloomPass(
      new THREE.Vector2(width * settings.bloomResolutionScale, height * settings.bloomResolutionScale),
      BLOOM_STRENGTH,
      0.5, // radius
      1.0, // threshold, in LINEAR scene radiance — M62 ramps this
    );
    this.composer.addPass(this.bloom);

    // Before the grade pass, so it smears linear radiance: blurring
    // already-tone-mapped pixels darkens the streak as it spreads, which is
    // backwards for a bright object in motion.
    this.radialBlur = new ShaderPass(RADIAL_BLUR_SHADER);
    this.radialBlur.enabled = false;
    this.composer.addPass(this.radialBlur);

    this.grade = new ShaderPass(GRADE_SHADER);
    this.composer.addPass(this.grade);

    // SMAA last, on the encoded signal — it is designed for gamma space, and
    // running it on linear HDR finds edges that are not there.
    if (settings.smaa) {
      this.smaa = new SMAAPass();
      this.composer.addPass(this.smaa);
    } else {
      this.smaa = null;
    }
  }

  setSize(width: number, height: number) {
    this.composer.setSize(width, height);
    const s = TIERS[this.tier].bloomResolutionScale;
    this.bloom.setSize(width * s, height * s);
  }

  render(elapsedSeconds: number) {
    this.grade.uniforms.uTime.value = elapsedSeconds;
    this.composer.render();
  }

  /** M63's camera director drives this. Disabled at zero rather than run with
   * a no-op uniform: a full-screen pass that provably changes nothing is
   * still a full-screen pass, and most shots hold their camera. */
  setMotionBlur(strength: number, focusX: number, focusY: number) {
    const s = strength > 1 ? 1 : strength < 0 ? 0 : strength;
    this.radialBlur.enabled = s > 0.001;
    this.radialBlur.uniforms.uStrength.value = s;
    this.radialBlur.uniforms.uFocus.value.set(focusX, focusY);
  }

  // --- timeline-animatable parameters (M62 drives these) ---

  set exposure(v: number) {
    this.params.exposure = v;
    this.grade.uniforms.uExposure.value = v;
  }
  get exposure(): number {
    return this.params.exposure;
  }

  set vignette(v: number) {
    this.params.vignette = v;
    this.grade.uniforms.uVignette.value = v;
  }
  get vignette(): number {
    return this.params.vignette;
  }

  set gradeStrength(v: number) {
    this.params.gradeStrength = v;
    this.grade.uniforms.uGrade.value = v;
  }
  get gradeStrength(): number {
    return this.params.gradeStrength;
  }

  set grain(v: number) {
    this.params.grain = v;
    this.grade.uniforms.uGrain.value = v;
  }
  get grain(): number {
    return this.params.grain;
  }

  set bloomThreshold(v: number) {
    this.bloom.threshold = v;
  }
  get bloomThreshold(): number {
    return this.bloom.threshold;
  }

  set bloomStrength(v: number) {
    this.bloom.strength = v;
  }
  get bloomStrength(): number {
    return this.bloom.strength;
  }

  set bloomRadius(v: number) {
    this.bloom.radius = v;
  }
  get bloomRadius(): number {
    return this.bloom.radius;
  }

  dispose() {
    this.composer.dispose();
  }
}

/** Picks a tier from what the machine actually manages, not from what it
 * says it is.
 *
 * The benchmark runs *while the scene is already on screen* at Medium rather
 * than behind a two-second black frame: the number is the same, and the first
 * thing a viewer sees is the piece. Overridable by `?quality=` and by the
 * controls dropdown, because an automatic choice is a guess and someone
 * watching on a projector knows better than the guess.
 *
 * **`--disable-gpu` is not a Low-tier proxy** and this is not measured with
 * one — the rev 3 corrections rejected that explicitly. It is SwiftShader,
 * roughly two orders of magnitude slower than the slowest real hardware, and
 * tuning Low against it would make Low far uglier than it needs to be. Real
 * frame rates are asserted on the named hardware in M92.
 */
export function tierFromUrl(): QualityTier | null {
  const q = new URLSearchParams(window.location.search).get('quality');
  return q === 'low' || q === 'medium' || q === 'high' ? q : null;
}

export function tierFromFps(fps: number): QualityTier {
  if (fps >= 55) return 'high';
  if (fps >= 28) return 'medium';
  return 'low';
}
