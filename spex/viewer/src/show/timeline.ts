/** M62 — the timeline evaluator.
 *
 * Given a time, produce the state of the world. Nothing is baked: every
 * transform, dissolve, material and post value the piece will ever show is
 * computed here from the resolved document, sixty times a second.
 *
 * # Sinks, and why the evaluator does not touch three.js
 *
 * `evaluate` pushes results into a `TrackSinks` object rather than writing to
 * a scene. Three reasons, in order of how much they cost when ignored: the
 * evaluator is testable without a renderer (this milestone's own
 * verification is a recording sink and a hash); the same evaluator can drive
 * a director HUD, an export, or a second view of the same show; and the real
 * sinks arrive in M63 (camera), M64 (choreography) and M65 (effects), which
 * would otherwise all have to exist before any of this could run once.
 *
 * # Allocation-free after warm-up
 *
 * AC3 asks for zero allocations per frame, and the reason is not tidiness. A
 * generative piece runs for an hour, or forever. A single small object per
 * track per frame is a few hundred kilobytes a minute of garbage, and the
 * collector that eventually walks it does so *during* a frame — which is a
 * hitch, at an unpredictable moment, in a work whose whole subject is things
 * landing exactly on the beat.
 *
 * So: every scratch value is a preallocated member, `evaluate` allocates
 * nothing and closes over nothing, and **the value handed to a sink is
 * reused on the next call**. A sink that wants to keep one must copy it. That
 * is stated here because it is the one way this design can bite a caller.
 */

import { easingByName, type EasingFn } from './easing';
import type {
  MaterialProperty,
  PostProperty,
  ResolvedCameraTrack,
  ResolvedCue,
  ResolvedKey,
  ResolvedShot,
  ResolvedShow,
  ResolvedTrack,
  DissolveOrder,
  TargetBinding,
  TransformValue,
  Vec3,
} from './resolved';

/** The transform state of one target at one instant. Reused between calls. */
export interface TransformOut {
  position: [number, number, number];
  hasPosition: boolean;
  /** Euler XYZ in degrees — carried rather than converted, because a
   * quaternion pair cannot express more than a half-turn and A1-S03 turns a
   * full one. */
  rotation: [number, number, number];
  hasRotation: boolean;
  quaternion: [number, number, number, number];
  hasQuaternion: boolean;
  scale: number;
  hasScale: boolean;
}

export interface CameraOut {
  position: [number, number, number];
  hasPosition: boolean;
  lookAt: [number, number, number];
  hasLookAt: boolean;
  fovDeg: number;
  hasFov: boolean;
}

export interface TrackSinks {
  transform?(target: TargetBinding, value: Readonly<TransformOut>, shot: ResolvedShot): void;
  dissolve?(
    target: TargetBinding,
    value: number,
    shot: ResolvedShot,
    stagger?: number,
    order?: DissolveOrder,
  ): void;
  material?(target: TargetBinding, property: MaterialProperty, value: number, shot: ResolvedShot): void;
  /** Linear RGB. The array is scratch and is reused every frame — a sink that
   * keeps it must copy it, exactly like the camera's. */
  color?(target: TargetBinding, value: Readonly<Vec3>, shot: ResolvedShot): void;
  post?(property: PostProperty, value: number, shot: ResolvedShot): void;
  hud?(element: string, value: number, shot: ResolvedShot): void;
  pointCloud?(target: TargetBinding, value: number, shot: ResolvedShot): void;
  /** M63 owns the four camera modes; the evaluator only says which shot's
   * camera is live and how far through it we are. */
  camera?(track: ResolvedCameraTrack, t01: number, sampled: Readonly<CameraOut>, shot: ResolvedShot): void;
}

export interface ActiveShot {
  shot: ResolvedShot;
  /** Normalised progress through this shot, 0..1. */
  t01: number;
}

export class Timeline {
  readonly show: ResolvedShow;
  readonly durationSec: number;

  /** Every cue in the show, flattened and sorted once. */
  private readonly cues: ResolvedCue[];
  private cueCursor = 0;

  // Scratch. See the header: reused, never reallocated.
  private readonly transformOut: TransformOut = {
    position: [0, 0, 0],
    hasPosition: false,
    rotation: [0, 0, 0],
    hasRotation: false,
    quaternion: [0, 0, 0, 1],
    hasQuaternion: false,
    scale: 1,
    hasScale: false,
  };
  private readonly colorOut: Vec3 = [0, 0, 0];
  private readonly cameraOut: CameraOut = {
    position: [0, 0, 0],
    hasPosition: false,
    lookAt: [0, 0, 0],
    hasLookAt: false,
    fovDeg: 0,
    hasFov: false,
  };
  /** Preallocated to the largest number of shots that can be live at once,
   * then filled in place. `activeShots` returns a view into it. */
  private readonly activeBuf: ActiveShot[] = [];
  private activeCount = 0;
  /** Easing functions resolved once at construction, so `evaluate` never does
   * a string lookup — and so an unknown easing name fails at load rather than
   * 90 seconds into a screening. */
  private readonly easings = new Map<string, EasingFn>();

  constructor(show: ResolvedShow) {
    this.show = show;
    this.durationSec = show.durationSec;

    this.cues = [];
    for (const shot of show.shots) {
      for (const cue of shot.cues ?? []) this.cues.push(cue);
      for (const key of shot.camera.keys ?? []) this.cacheEasing(key.easing);
      for (const track of shot.tracks ?? []) {
        for (const key of track.keys as ResolvedKey<unknown>[]) this.cacheEasing(key.easing);
      }
      this.activeBuf.push({ shot, t01: 0 });
    }
    this.cues.sort((a, b) => a.timeSec - b.timeSec);
  }

  private cacheEasing(name: string) {
    if (!this.easings.has(name)) this.easings.set(name, easingByName(name));
  }

  /** Which shots are live at `t`.
   *
   * Resolved shots are contiguous and non-overlapping today, so this is
   * normally exactly one — but the loop is written for the general case,
   * because a cross-dissolve is two shots on screen at once and the format
   * does not forbid it.
   *
   * The returned array is **owned by this Timeline** and refilled on the next
   * call. */
  activeShots(t: number): readonly ActiveShot[] {
    this.activeCount = 0;
    const shots = this.show.shots;
    for (let i = 0; i < shots.length; i++) {
      const s = shots[i];
      // The last shot includes its own end, or the final frame of the piece
      // belongs to nothing — and the final frame of this piece is the whole
      // point of it (it is byte-identical to the first).
      const last = i === shots.length - 1;
      if (t >= s.startSec && (t < s.endSec || (last && t <= s.endSec))) {
        const slot = this.activeBuf[this.activeCount++];
        slot.shot = s;
        slot.t01 = s.durationSec > 0 ? (t - s.startSec) / s.durationSec : 0;
      }
    }
    return this.activeBuf.slice(0, this.activeCount) as readonly ActiveShot[];
  }

  /** Evaluates every track of every live shot into `sinks`.
   *
   * Allocation-free: this walks `activeBuf` directly rather than through
   * `activeShots`, whose `slice` would allocate once a frame. */
  evaluate(t: number, sinks: TrackSinks): void {
    this.fillActive(t);
    for (let a = 0; a < this.activeCount; a++) {
      const { shot, t01 } = this.activeBuf[a];
      const tracks = shot.tracks;
      if (tracks) {
        for (let i = 0; i < tracks.length; i++) {
          this.evaluateTrack(tracks[i], t, shot, sinks);
        }
      }
      if (sinks.camera) {
        this.sampleCamera(shot.camera, t);
        sinks.camera(shot.camera, t01, this.cameraOut, shot);
      }
    }
  }

  private fillActive(t: number) {
    this.activeCount = 0;
    const shots = this.show.shots;
    for (let i = 0; i < shots.length; i++) {
      const s = shots[i];
      const last = i === shots.length - 1;
      if (t >= s.startSec && (t < s.endSec || (last && t <= s.endSec))) {
        const slot = this.activeBuf[this.activeCount++];
        slot.shot = s;
        slot.t01 = s.durationSec > 0 ? (t - s.startSec) / s.durationSec : 0;
      }
    }
  }

  private evaluateTrack(track: ResolvedTrack, t: number, shot: ResolvedShot, sinks: TrackSinks) {
    switch (track.kind) {
      case 'transform': {
        if (!sinks.transform) return;
        this.sampleTransform(track.keys, t);
        sinks.transform(track.target, this.transformOut, shot);
        return;
      }
      case 'dissolve': {
        if (!sinks.dissolve) return;
        sinks.dissolve(track.target, this.sampleNumber(track.keys, t), shot, track.stagger, track.order);
        return;
      }
      case 'material': {
        if (!sinks.material) return;
        sinks.material(track.target, track.property, this.sampleNumber(track.keys, t), shot);
        return;
      }
      case 'color': {
        if (!sinks.color) return;
        sinks.color(track.target, this.sampleColor(track.keys, t), shot);
        return;
      }
      case 'post': {
        if (!sinks.post) return;
        sinks.post(track.property, this.sampleNumber(track.keys, t), shot);
        return;
      }
      case 'hud': {
        if (!sinks.hud) return;
        sinks.hud(track.element, this.sampleNumber(track.keys, t), shot);
        return;
      }
      case 'pointCloud': {
        if (!sinks.pointCloud) return;
        sinks.pointCloud(track.target, this.sampleNumber(track.keys, t), shot);
        return;
      }
    }
  }

  /** Index of the last key at or before `t`, by linear scan.
   *
   * Linear and not binary on purpose: a track has a handful of keys, and for
   * the forward playback that is the normal case the answer is the same index
   * as last frame or the next one. A binary search would win only on tracks
   * this format does not produce. */
  private leftIndex(keys: readonly { timeSec: number }[], t: number): number {
    let i = 0;
    while (i + 1 < keys.length && keys[i + 1].timeSec <= t) i++;
    return i;
  }

  /** Eased 0..1 between key `i` and `i+1`. The easing of the **left** key
   * governs the segment leaving it, which is the convention every animation
   * tool uses and the one the screenplay was written against. */
  private segment(keys: readonly { timeSec: number; easing: string }[], i: number, t: number): number {
    if (i + 1 >= keys.length) return 1;
    const a = keys[i].timeSec;
    const b = keys[i + 1].timeSec;
    if (!(b > a)) return 1; // coincident keys are a step, not a division by zero
    const raw = (t - a) / (b - a);
    const ease = this.easings.get(keys[i].easing)!;
    return ease(raw < 0 ? 0 : raw > 1 ? 1 : raw);
  }

  sampleNumber(keys: readonly ResolvedKey<number>[], t: number): number {
    if (keys.length === 0) return 0;
    if (t <= keys[0].timeSec) return keys[0].value;
    const last = keys[keys.length - 1];
    if (t >= last.timeSec) return last.value;
    const i = this.leftIndex(keys, t);
    const u = this.segment(keys, i, t);
    return keys[i].value + (keys[i + 1].value - keys[i].value) * u;
  }

  /** Linear-RGB lerp between the surrounding keys, into reused scratch.
   *
   * Component-wise, which is a decision and not the only one available: a lerp
   * in linear RGB is a lerp in radiance, and draining a colour IS the light
   * going away. Interpolating in sRGB or through a perceptual space would
   * take the same two endpoints along a different set of intermediate
   * colours, for no reason this piece can state. */
  sampleColor(keys: readonly ResolvedKey<Vec3>[], t: number): Readonly<Vec3> {
    const out = this.colorOut;
    if (keys.length === 0) {
      out[0] = out[1] = out[2] = 0;
      return out;
    }
    const first = keys[0];
    const last = keys[keys.length - 1];
    const pick = t <= first.timeSec ? first.value : t >= last.timeSec ? last.value : null;
    if (pick) {
      out[0] = pick[0];
      out[1] = pick[1];
      out[2] = pick[2];
      return out;
    }
    const i = this.leftIndex(keys, t);
    const u = this.segment(keys, i, t);
    const a = keys[i].value;
    const b = keys[i + 1].value;
    out[0] = a[0] + (b[0] - a[0]) * u;
    out[1] = a[1] + (b[1] - a[1]) * u;
    out[2] = a[2] + (b[2] - a[2]) * u;
    return out;
  }

  /** Fills `transformOut`. Each component is independent: a track may key
   * position without ever mentioning rotation, and the `has*` flags are how a
   * sink knows the difference between "0" and "not authored". */
  sampleTransform(keys: readonly ResolvedKey<TransformValue>[], t: number): Readonly<TransformOut> {
    const out = this.transformOut;
    out.hasPosition = out.hasRotation = out.hasQuaternion = out.hasScale = false;
    if (keys.length === 0) return out;

    let i: number;
    let u: number;
    if (t <= keys[0].timeSec) {
      i = 0;
      u = 0;
    } else if (t >= keys[keys.length - 1].timeSec) {
      i = keys.length - 1;
      u = 0;
    } else {
      i = this.leftIndex(keys, t);
      u = this.segment(keys, i, t);
    }
    const a = keys[i].value;
    const b = i + 1 < keys.length ? keys[i + 1].value : a;

    if (a.position) {
      out.hasPosition = true;
      const bp = b.position ?? a.position;
      out.position[0] = a.position[0] + (bp[0] - a.position[0]) * u;
      out.position[1] = a.position[1] + (bp[1] - a.position[1]) * u;
      out.position[2] = a.position[2] + (bp[2] - a.position[2]) * u;
    }
    if (a.rotation) {
      out.hasRotation = true;
      const br = b.rotation ?? a.rotation;
      // Componentwise in degrees, deliberately NOT taking a shortest path:
      // 0 -> 360 has to be a full revolution, and any "wrap to ±180" step
      // here would silently turn A1-S03's single turn into no turn at all.
      out.rotation[0] = a.rotation[0] + (br[0] - a.rotation[0]) * u;
      out.rotation[1] = a.rotation[1] + (br[1] - a.rotation[1]) * u;
      out.rotation[2] = a.rotation[2] + (br[2] - a.rotation[2]) * u;
    }
    if (a.quaternion) {
      out.hasQuaternion = true;
      slerpInto(out.quaternion, a.quaternion, b.quaternion ?? a.quaternion, u);
    }
    if (a.scale !== undefined) {
      out.hasScale = true;
      const bs = b.scale ?? a.scale;
      out.scale = a.scale + (bs - a.scale) * u;
    }
    return out;
  }

  /** Fills `cameraOut` from a keyed camera track. The other three modes are
   * parametric and M63 evaluates them from `t01`; there is nothing to sample. */
  sampleCamera(track: ResolvedCameraTrack, t: number): Readonly<CameraOut> {
    const out = this.cameraOut;
    out.hasPosition = out.hasLookAt = out.hasFov = false;
    if (track.fovDeg !== undefined) {
      out.hasFov = true;
      out.fovDeg = track.fovDeg;
    }
    const keys = track.keys;
    if (!keys || keys.length === 0) return out;

    let i: number;
    let u: number;
    if (t <= keys[0].timeSec) {
      i = 0;
      u = 0;
    } else if (t >= keys[keys.length - 1].timeSec) {
      i = keys.length - 1;
      u = 0;
    } else {
      i = this.leftIndex(keys, t);
      u = this.segment(keys, i, t);
    }
    const a = keys[i].value;
    const b = i + 1 < keys.length ? keys[i + 1].value : a;

    if (a.position) {
      out.hasPosition = true;
      const bp = b.position ?? a.position;
      out.position[0] = a.position[0] + (bp[0] - a.position[0]) * u;
      out.position[1] = a.position[1] + (bp[1] - a.position[1]) * u;
      out.position[2] = a.position[2] + (bp[2] - a.position[2]) * u;
    }
    if (a.lookAt) {
      out.hasLookAt = true;
      const bl = b.lookAt ?? a.lookAt;
      out.lookAt[0] = a.lookAt[0] + (bl[0] - a.lookAt[0]) * u;
      out.lookAt[1] = a.lookAt[1] + (bl[1] - a.lookAt[1]) * u;
      out.lookAt[2] = a.lookAt[2] + (bl[2] - a.lookAt[2]) * u;
    }
    if (a.fovDeg !== undefined) {
      out.hasFov = true;
      const bf = b.fovDeg ?? a.fovDeg;
      out.fovDeg = a.fovDeg + (bf - a.fovDeg) * u;
    }
    return out;
  }

  /** Fires cues whose time was crossed since `prevT`.
   *
   * Monotonic playback advances a cursor. A **seek** — including the wrap at
   * the end of an endless cycle — moves the cursor instead of firing
   * everything in between: arriving at 3:00 should not replay every accent of
   * the first three minutes, which would be both a stampede and, for an audio
   * cue, actually audible. */
  fireCues(prevT: number, t: number, handler: (cue: ResolvedCue) => void): void {
    const cues = this.cues;
    if (t < prevT || t - prevT > SEEK_THRESHOLD_SEC) {
      this.cueCursor = 0;
      while (this.cueCursor < cues.length && cues[this.cueCursor].timeSec <= t) this.cueCursor++;
      return;
    }
    while (this.cueCursor < cues.length && cues[this.cueCursor].timeSec <= t) {
      handler(cues[this.cueCursor]);
      this.cueCursor++;
    }
  }

  /** Call after `ShowClock.seek`. Equivalent to what `fireCues` does when it
   * detects a jump, but explicit — a caller that knows it seeked should not
   * have to rely on a heuristic. */
  resetCueCursor(t: number): void {
    this.cueCursor = 0;
    while (this.cueCursor < this.cues.length && this.cues[this.cueCursor].timeSec <= t) {
      this.cueCursor++;
    }
  }

  get cueCount(): number {
    return this.cues.length;
  }
}

/** A forward jump larger than this is a seek, not a slow frame.
 *
 * **Raised from 0.5 to 2.0 in M66, because 0.5 was measured and wrong.** The
 * original reasoning was that half a second is "well past any real frame (even
 * this container's software rasteriser manages ~4 fps)". The first end-to-end
 * screening ran at **2.2 fps** — 0.45 s a frame — and the result was not a
 * crash or a stutter: it was a piece that fired *no cue at all*. Every frame
 * looked like a seek, so the cursor advanced past every accent without firing
 * it, and the director HUD reported four voices as none. Nothing errored.
 *
 * The number is not really what makes seeks safe, and that is the deeper
 * point. A player that seeks *knows* it seeked and calls `resetCueCursor`
 * explicitly; this heuristic exists only for the case nobody announces — a tab
 * restored from the background, which produces jumps of minutes, not seconds.
 * Two seconds is far above any frame a renderer produces and far below that. */
export const SEEK_THRESHOLD_SEC = 2.0;

/** Shortest-path slerp, writing into `out`. No allocation, no THREE import —
 * the evaluator stays renderer-free. */
function slerpInto(
  out: [number, number, number, number],
  a: readonly [number, number, number, number],
  b: readonly [number, number, number, number],
  u: number,
): void {
  let [bx, by, bz, bw] = b;
  let dot = a[0] * bx + a[1] * by + a[2] * bz + a[3] * bw;
  if (dot < 0) {
    bx = -bx;
    by = -by;
    bz = -bz;
    bw = -bw;
    dot = -dot;
  }
  if (dot > 0.9995) {
    // Nearly parallel: slerp degenerates, lerp-and-normalise does not.
    out[0] = a[0] + (bx - a[0]) * u;
    out[1] = a[1] + (by - a[1]) * u;
    out[2] = a[2] + (bz - a[2]) * u;
    out[3] = a[3] + (bw - a[3]) * u;
    const n = Math.hypot(out[0], out[1], out[2], out[3]) || 1;
    out[0] /= n;
    out[1] /= n;
    out[2] /= n;
    out[3] /= n;
    return;
  }
  const theta = Math.acos(dot);
  const sin = Math.sin(theta);
  const wa = Math.sin((1 - u) * theta) / sin;
  const wb = Math.sin(u * theta) / sin;
  out[0] = a[0] * wa + bx * wb;
  out[1] = a[1] * wa + by * wb;
  out[2] = a[2] * wa + bz * wb;
  out[3] = a[3] * wa + bw * wb;
}
