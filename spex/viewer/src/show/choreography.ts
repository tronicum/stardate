/** M64 — runtime choreography. Gap B, closed.
 *
 * `spex brick-assembly` bakes nine bricks settling out of a scattered start
 * into a sequence of point-cloud frames. A1-S04 is that shot, and from here it
 * is *evaluated*, not baked: the same constants, the same seeding, the same
 * curve, computed per frame through `InstanceWriter`.
 *
 * The baked path is not retired. It stays exactly as it is, and it is still
 * the right tool for a quick demo of a scene nobody is going to seek around
 * in. What changes is that the *show* stops using it.
 *
 * # The generator is declared, not expanded
 *
 * The spec asked for the scatter to be turned into a transform track at
 * `show-build` time. That works for nine bricks and not for an Atlas site: a
 * transform track carries one value per keyframe shared across its whole
 * target, so per-instance scatter needs one track *per instance* — five
 * thousand tracks of two keyframes each, several megabytes of JSON, to
 * express two constants and a seed. So the document declares the generator
 * (A1-S04's `seed` cue carries `floatHeightLdu`, `scatterRadiusLdu`, the
 * stagger mode and the easing) and this evaluates it. The resolved document
 * stays the size of a screenplay.
 *
 * # One PRNG, two languages
 *
 * `crates/spex-show/src/choreography.rs` is the other half of this file, and
 * the two are pinned to `docs/fugen/fixtures/assembly-scatter.json` rather
 * than to each other — a fixture they can both fail against, instead of a
 * comparison they could both drift through.
 *
 * splitmix64 rather than `rand::StdRng`, because `StdRng` is ChaCha12 and
 * `rand` makes no promise its stream is stable across versions. See the Rust
 * module's header.
 *
 * # Coordinates
 *
 * The generator works in **LDraw units, Y-down**, because that is the frame
 * the constants were measured in. `startOffsetMm` converts at the same
 * boundary `spex_mesh::bundle::to_output_position` does, and negating Y is
 * the same mirror it is everywhere else in this project.
 */

import { cubicInOut, easingByName, type EasingFn } from './easing';

export const FLOAT_HEIGHT_LDU = 420;
export const SCATTER_RADIUS_LDU = 260;
/** LDraw's own unit. One LDU is 0.4 mm. */
export const LDU_MM = 0.4;
const TAU = Math.PI * 2;

/** splitmix64 over a pair of 32-bit halves, because JavaScript has no u64.
 *
 * `BigInt` would be the obvious way and is roughly an order of magnitude
 * slower; this runs once per instance per shot, not per frame, but the Atlas
 * has forty thousand instances and "once per instance" is still a number. */
class U64 {
  hi: number;
  lo: number;
  constructor(hi: number, lo: number) {
    this.hi = hi >>> 0;
    this.lo = lo >>> 0;
  }
}

const GAMMA_HI = 0x9e3779b9;
const GAMMA_LO = 0x7f4a7c15;

function add64(a: U64, hi: number, lo: number): void {
  const lo2 = (a.lo >>> 0) + (lo >>> 0);
  a.lo = lo2 >>> 0;
  a.hi = (a.hi + hi + (lo2 > 0xffffffff ? 1 : 0)) >>> 0;
}

function mul64(a: U64, hi: number, lo: number): void {
  // 32x32 -> 64 in 16-bit limbs. The naive `a.lo * lo` loses bits above 2^53.
  const al = a.lo >>> 0;
  const ah = a.hi >>> 0;
  const bl = lo >>> 0;
  const bh = hi >>> 0;
  const a0 = al & 0xffff;
  const a1 = al >>> 16;
  const b0 = bl & 0xffff;
  const b1 = bl >>> 16;

  const p00 = a0 * b0;
  const p01 = a0 * b1;
  const p10 = a1 * b0;
  const p11 = a1 * b1;

  let mid = (p00 >>> 16) + (p01 & 0xffff) + (p10 & 0xffff);
  const resLo = ((mid & 0xffff) << 16) | (p00 & 0xffff);
  let carry = (mid / 0x10000) | 0;
  carry += (p01 >>> 16) + (p10 >>> 16) + p11;
  // The cross terms that only reach the high word.
  const resHi = (carry + Math.imul(al, bh) + Math.imul(ah, bl)) >>> 0;
  a.lo = resLo >>> 0;
  a.hi = resHi >>> 0;
}

function xorShiftRight(a: U64, n: number): void {
  if (n < 32) {
    const lo = ((a.lo >>> n) | (a.hi << (32 - n))) >>> 0;
    const hi = a.hi >>> n;
    a.lo = (a.lo ^ lo) >>> 0;
    a.hi = (a.hi ^ hi) >>> 0;
  } else {
    const lo = a.hi >>> (n - 32);
    a.lo = (a.lo ^ lo) >>> 0;
  }
}

/** One splitmix64 step, returning a double in [0,1) from the top 53 bits —
 * the same construction the Rust side uses, so the two agree exactly. */
export function nextFloat(state: U64): number {
  add64(state, GAMMA_HI, GAMMA_LO);
  const z = new U64(state.hi, state.lo);
  xorShiftRight(z, 30);
  mul64(z, 0xbf58476d, 0x1ce4e5b9);
  xorShiftRight(z, 27);
  mul64(z, 0x94d049bb, 0x133111eb);
  xorShiftRight(z, 31);
  // Top 53 bits: the whole high word plus the top 21 of the low.
  return (z.hi * 0x200000 + (z.lo >>> 11)) / 9007199254740992;
}

/** `SPLITMIX_GAMMA * (index + 1) + editionSeed * 0x2545F4914F6CDD1D`. */
export function placementSeed(index: number, editionSeed: number): U64 {
  const s = new U64(GAMMA_HI, GAMMA_LO);
  mul64(s, 0, (index + 1) >>> 0);
  if (editionSeed) {
    const e = new U64((editionSeed / 0x100000000) >>> 0, editionSeed >>> 0);
    mul64(e, 0x2545f491, 0x4f6cdd1d);
    add64(s, e.hi, e.lo);
  }
  return s;
}

/** Where a placement starts, as an offset from its final position, in LDraw
 * units — Y-down, so the negative Y is upward. */
export function startOffsetLdu(index: number, editionSeed = 0): [number, number, number] {
  const state = placementSeed(index, editionSeed);
  const angle = nextFloat(state) * TAU;
  const radius = SCATTER_RADIUS_LDU * (0.4 + 0.6 * nextFloat(state));
  return [radius * Math.cos(angle), -FLOAT_HEIGHT_LDU, radius * Math.sin(angle)];
}

/** The same offset in the viewer's frame: millimetres, +Y up.
 *
 * The Y negation is the mirror `spex_mesh::bundle::to_output_position`
 * applies to every vertex in the library, and it is applied here for the
 * same reason — the two frames differ by exactly this. */
export function startOffsetMm(index: number, editionSeed = 0): [number, number, number] {
  const [x, y, z] = startOffsetLdu(index, editionSeed);
  return [x * LDU_MM, -y * LDU_MM, z * LDU_MM];
}

/** A placement's own progress, given the shot's. See the Rust twin. */
export function staggeredProgress(t01: number, order: number, count: number, stagger: number): number {
  const t = t01 < 0 ? 0 : t01 > 1 ? 1 : t01;
  if (count <= 1 || stagger <= 0) return t;
  const s = stagger > 1 ? 1 : stagger;
  const span = 1 - s;
  const start = (s * Math.min(order, count - 1)) / (count - 1);
  if (span <= Number.EPSILON) return t >= start ? 1 : 0;
  const u = (t - start) / span;
  return u < 0 ? 0 : u > 1 ? 1 : u;
}

/** Anything that can be told where an instance is. `InstanceWriter` from
 * M55 satisfies this; so does a recording object in a test harness, which is
 * how this module gets verified without a scene. */
export interface TransformTarget {
  setTransform(id: string, position: PositionLike, quaternion: QuaternionLike, scale: number): void;
}
export interface PositionLike {
  set(x: number, y: number, z: number): unknown;
  x: number;
  y: number;
  z: number;
}
export interface QuaternionLike {
  set(x: number, y: number, z: number, w: number): unknown;
}

export interface AssemblySpec {
  /** Instance ids, in bundle order, that this assembly moves. */
  ids: readonly string[];
  /** Each instance's final position in millimetres, +Y up — where the bundle
   * already placed it. */
  finals: Float32Array | readonly number[];
  /** Rank per instance, deciding the stagger order. The real `0 STEP` build
   * step where the scene has one; the index otherwise. */
  order?: readonly number[];
  editionSeed?: number;
  /** Fraction of the shot spent handing over from the first part to the last. */
  stagger?: number;
  easing?: EasingFn;
}

/** How much of a shot the first part spends landing before the last begins.
 *
 * 0.55 rather than a round number: the assembly is four bars, and the
 * screenplay's direction is that each landing is its own accent — too much
 * stagger and the last brick arrives after the music has moved on, too
 * little and nine accents become one. */
export const DEFAULT_STAGGER = 0.55;

/** The scattered-start assembly, evaluated.
 *
 * Holds the start offsets it generated so the per-frame path is a lerp and
 * nothing else: seeding once per instance per *shot* rather than per frame is
 * the difference between this costing nothing and it costing 40 000 splitmix
 * chains a frame at Atlas scale.
 */
export class AssemblyChoreography {
  readonly count: number;
  private readonly ids: readonly string[];
  private readonly finals: Float32Array;
  private readonly starts: Float32Array;
  private readonly order: readonly number[];
  private readonly stagger: number;
  private readonly ease: EasingFn;
  private readonly ranks: number[];
  /** How many *distinct* build steps the scene has — the span the stagger
   * divides, which is not the instance count when steps repeat. */
  private readonly rankCount: number;

  constructor(spec: AssemblySpec) {
    this.ids = spec.ids;
    this.count = spec.ids.length;
    this.finals = spec.finals instanceof Float32Array ? spec.finals : Float32Array.from(spec.finals);
    this.stagger = spec.stagger ?? DEFAULT_STAGGER;
    this.ease = spec.easing ?? cubicInOut;
    this.order = spec.order ?? spec.ids.map((_, i) => i);

    // Build steps are not dense: a scene may have steps 0, 0, 1, 3. Ranking
    // them turns "which step" into "how far through the build", which is what
    // the stagger actually wants — otherwise a scene whose steps happen to be
    // sparse staggers over a shorter span than one whose steps are dense, for
    // no reason a viewer could perceive.
    const distinct = Array.from(new Set(this.order)).sort((a, b) => a - b);
    const rankOf = new Map(distinct.map((v, i) => [v, i]));
    this.ranks = this.order.map((v) => rankOf.get(v)!);
    this.rankCount = distinct.length;

    this.starts = new Float32Array(this.count * 3);
    for (let i = 0; i < this.count; i++) {
      const [dx, dy, dz] = startOffsetMm(i, spec.editionSeed ?? 0);
      this.starts[i * 3] = this.finals[i * 3] + dx;
      this.starts[i * 3 + 1] = this.finals[i * 3 + 1] + dy;
      this.starts[i * 3 + 2] = this.finals[i * 3 + 2] + dz;
    }
  }

  /** Where instance `i` is at shot progress `t01`, written into `out`. */
  positionAt(i: number, t01: number, out: [number, number, number]): [number, number, number] {
    const u = this.ease(staggeredProgress(t01, this.ranks[i], this.rankCount, this.stagger));
    const o = i * 3;
    out[0] = this.starts[o] + (this.finals[o] - this.starts[o]) * u;
    out[1] = this.starts[o + 1] + (this.finals[o + 1] - this.starts[o + 1]) * u;
    out[2] = this.starts[o + 2] + (this.finals[o + 2] - this.starts[o + 2]) * u;
    return out;
  }

  /** Writes every instance's position for this frame.
   *
   * `position` and `quaternion` are the caller's scratch objects, reused —
   * see `InstanceWriter`, which makes the same bargain for the same reason. */
  apply(t01: number, target: TransformTarget, position: PositionLike, quaternion: QuaternionLike): void {
    quaternion.set(0, 0, 0, 1);
    for (let i = 0; i < this.count; i++) {
      const u = this.ease(staggeredProgress(t01, this.ranks[i], this.rankCount, this.stagger));
      const o = i * 3;
      position.set(
        this.starts[o] + (this.finals[o] - this.starts[o]) * u,
        this.starts[o + 1] + (this.finals[o + 1] - this.starts[o + 1]) * u,
        this.starts[o + 2] + (this.finals[o + 2] - this.starts[o + 2]) * u,
      );
      target.setTransform(this.ids[i], position, quaternion, 1);
    }
  }
}

/** Reads an assembly out of a shot's `seed` cue payload.
 *
 * The document says *what* to generate and this decides *how*, which is the
 * split that keeps five thousand transform tracks out of the resolved file.
 * An unknown generator name returns null rather than throwing: a document
 * from a later version of the piece should degrade to "that shot does not
 * animate", not to a blank page. */
export function assemblyFromCue(
  payload: Record<string, unknown> | undefined,
  spec: Omit<AssemblySpec, 'stagger' | 'easing' | 'editionSeed'>,
  editionSeed: number,
): AssemblyChoreography | null {
  if (!payload || payload.generator !== 'assembly') return null;
  const easingName = typeof payload.easing === 'string' ? payload.easing : 'cubicInOut';
  return new AssemblyChoreography({
    ...spec,
    editionSeed,
    stagger: typeof payload.stagger === 'number' ? payload.stagger : DEFAULT_STAGGER,
    easing: easingByName(easingName),
  });
}
