/** M63 — the camera director.
 *
 * Four modes, one camera. `keyed` reads the interpolated keyframes M62 already
 * sampled; `orbit`, `dolly` and `exponentialZoom` are parametric and are
 * evaluated here from the shot's own normalised progress.
 *
 * # The Kick, and the near/far plane
 *
 * `exponentialZoom` is the last two beats of the piece: the camera pulls back
 * by a factor of 10 000 while the network collapses to the single pixel the
 * work opened on. Distance is geometric, not linear —
 * `d(t) = from · (to/from)^expoIn(t)` — because a linear pull-back over four
 * orders of magnitude spends 99 % of its time in the last order and reads as
 * a jump followed by nothing.
 *
 * A fixed near/far cannot survive that. At the near end the object is 300 mm
 * away; at the far end it is 3 000 000 mm away, and no single depth range
 * holds both with usable precision. So near and far track `d`.
 *
 * **The spec asked for `[d/1e4, d·1e4]` and that is a mistake this file does
 * not implement.** That range has a far:near ratio of 10^8 — *worse* than the
 * static 1:20 000 the viewer already had, and depth precision falls off with
 * exactly that ratio. It would guarantee the artefacts it was written to
 * prevent. What actually matters is that the range brackets the scene at the
 * current distance, and the scene is small compared to `d` for all but the
 * first instant: `near = d/100`, `far = d·10` is a ratio of 1 000, four
 * orders of magnitude tighter, and still clears an object of radius up to
 * 0.99·d. Both numbers are named constants below so a future scene with a
 * different aspect can move them knowingly.
 *
 * # Motion blur is an approximation, and says so
 *
 * A real velocity buffer needs a per-object previous-frame matrix and a
 * second render target, which at Atlas scale costs more than the effect is
 * worth. What this does instead is drive a **radial blur centred on the
 * screen-space focus point** from the camera's own angular and radial speed —
 * correct for a dolly or a zoom, where all the motion really is radial from
 * the focus, and merely plausible for anything else. It is a stylistic
 * approximation and it is documented as one rather than named `motionBlur`
 * and left to look like physics.
 *
 * # OrbitControls during playback
 *
 * Disabled while the show plays and re-enabled on pause, because a camera
 * that both follows a timeline and follows a mouse follows neither. `?free=1`
 * hands control back permanently for inspection, and — this is the part worth
 * testing — the timeline keeps running underneath: the show is still
 * evaluating, still firing cues, still advancing; only the camera is yours.
 */

import * as THREE from 'three';
import { expoIn } from './easing';
import type { CameraOut } from './timeline';
import type { ResolvedCameraTrack, Vec3 } from './resolved';

/** See the header. `near = d * NEAR_FACTOR`, `far = d * FAR_FACTOR`. */
export const NEAR_FACTOR = 0.01;
export const FAR_FACTOR = 10;

/** Where the camera sits relative to its focus when a zoom does not say.
 *
 * `exponentialZoom` carries a distance and a look-at point but no direction,
 * so one has to be chosen. This is the piece's own default framing axis:
 * straight back along +Z with a slight rise, the same view every mesh
 * screenshot in `docs/fugen/screenshots/` was taken from. A document can
 * override it per shot with `direction`. */
export const DEFAULT_ZOOM_DIRECTION: Vec3 = [0, 0.15, 1];

export interface DirectorControls {
  enabled: boolean;
  target: THREE.Vector3;
  update(): void;
}

export class CameraDirector {
  /** Screen-space focus of the current shot, 0..1, for the radial blur. */
  readonly focus = new THREE.Vector2(0.5, 0.5);
  /** Blur strength this frame, 0..1 — the shot's authored `motionBlur`
   * scaled by how fast the camera is actually moving. A held camera on a
   * shot that allows blur produces none. */
  blur = 0;

  /** Does the timeline own the camera this frame?
   *
   * **Not the same as "is the show playing".** M63 gated on `playing` and
   * M66's first screening found what that costs: pausing and then *seeking* —
   * which is what scrubbing a show is — left the camera wherever it happened
   * to be standing, so a paused player at t=0 showed the second shot's
   * framing while the HUD read "A1-S01, 0.00 s". Every number agreed and the
   * picture was of a different shot.
   *
   * The rule that works: the timeline owns the camera whenever *show time is
   * moving*, whether that is playback or a seek; the mouse owns it when time
   * is standing still. `?free=1` overrides both, permanently. */
  follow = true;

  private readonly camera: THREE.PerspectiveCamera;
  private readonly controls?: DirectorControls;
  private readonly free: boolean;
  private playing = true;

  /** The viewer's own near/far, restored whenever a shot is not zooming. */
  private readonly baseNear: number;
  private readonly baseFar: number;
  private overrodeRange = false;

  // Scratch — this runs every frame.
  private readonly pos = new THREE.Vector3();
  private readonly look = new THREE.Vector3();
  private readonly dir = new THREE.Vector3();
  private readonly prevPos = new THREE.Vector3();
  private readonly prevLook = new THREE.Vector3();
  private havePrev = false;

  constructor(camera: THREE.PerspectiveCamera, controls?: DirectorControls, free = false) {
    this.camera = camera;
    this.controls = controls;
    this.free = free;
    this.baseNear = camera.near;
    this.baseFar = camera.far;
    if (controls) controls.enabled = free;
  }

  get isFree(): boolean {
    return this.free;
  }

  /** Playback state only gates *the camera*. The timeline keeps running.
   *
   * Pausing enables the controls immediately — the mouse should work the
   * instant someone pauses — but `follow` is what actually decides who wins
   * on any given frame, and a seek turns it back on for that frame. */
  setPlaying(playing: boolean) {
    this.playing = playing;
    this.follow = playing;
    if (this.controls) this.controls.enabled = this.free || !playing;
  }

  /** Position the camera for one frame.
   *
   * `sampled` is M62's interpolated keyed value; the other three modes ignore
   * it and are computed from `t01`. `dtSec` is real elapsed time, used only
   * for the velocity that drives the blur — a seek passes 0 so a jump does
   * not smear. */
  apply(track: ResolvedCameraTrack, t01: number, sampled: Readonly<CameraOut>, dtSec: number): void {
    const t = t01 < 0 ? 0 : t01 > 1 ? 1 : t01;
    let haveLook = false;
    let distance = 0;

    switch (track.mode) {
      case 'keyed': {
        if (sampled.hasPosition) this.pos.set(sampled.position[0], sampled.position[1], sampled.position[2]);
        else this.pos.copy(this.camera.position);
        if (sampled.hasLookAt) {
          this.look.set(sampled.lookAt[0], sampled.lookAt[1], sampled.lookAt[2]);
          haveLook = true;
        }
        if (sampled.hasFov) this.setFov(sampled.fovDeg);
        break;
      }
      case 'orbit': {
        const o = track.orbit;
        if (!o) return;
        const deg = o.startDeg + (o.endDeg - o.startDeg) * t;
        const rad = (deg * Math.PI) / 180;
        this.look.set(o.center[0], o.center[1], o.center[2]);
        haveLook = true;
        this.pos.set(
          o.center[0] + Math.cos(rad) * o.radius,
          o.center[1] + o.height,
          o.center[2] + Math.sin(rad) * o.radius,
        );
        if (track.fovDeg !== undefined) this.setFov(track.fovDeg);
        break;
      }
      case 'dolly': {
        const d = track.dolly;
        if (!d) return;
        this.pos.set(
          d.from[0] + (d.to[0] - d.from[0]) * t,
          d.from[1] + (d.to[1] - d.from[1]) * t,
          d.from[2] + (d.to[2] - d.from[2]) * t,
        );
        this.look.set(d.lookAt[0], d.lookAt[1], d.lookAt[2]);
        haveLook = true;
        if (track.fovDeg !== undefined) this.setFov(track.fovDeg);
        break;
      }
      case 'exponentialZoom': {
        const z = track.exponentialZoom;
        if (!z) return;
        // Geometric, not linear. A linear pull-back over 10^4 spends its
        // whole second act in the last order of magnitude.
        distance = z.from * Math.pow(z.to / z.from, expoIn(t));
        const raw = (z as { direction?: Vec3 }).direction ?? DEFAULT_ZOOM_DIRECTION;
        this.dir.set(raw[0], raw[1], raw[2]);
        if (this.dir.lengthSq() === 0) this.dir.set(0, 0, 1);
        this.dir.normalize();
        this.look.set(z.lookAt[0], z.lookAt[1], z.lookAt[2]);
        haveLook = true;
        this.pos.copy(this.dir).multiplyScalar(distance).add(this.look);
        if (track.fovDeg !== undefined) this.setFov(track.fovDeg);
        break;
      }
    }

    this.applyRange(track.mode === 'exponentialZoom' ? distance : 0);
    this.measureBlur(track, haveLook, dtSec);

    // `free` means the person is driving. Everything above still ran — the
    // blur, the depth range, the velocity — because the show is still
    // playing; only the transform is withheld.
    if (this.free || (!this.follow && this.controls)) {
      if (this.controls && haveLook && !this.free) this.controls.target.copy(this.look);
      return;
    }

    this.camera.position.copy(this.pos);
    if (haveLook) {
      this.camera.lookAt(this.look);
      if (this.controls) this.controls.target.copy(this.look);
    }
  }

  private setFov(deg: number) {
    if (Math.abs(this.camera.fov - deg) < 1e-6) return;
    this.camera.fov = deg;
    this.camera.updateProjectionMatrix();
  }

  /** Track near/far to the zoom distance, and put them back afterwards. */
  private applyRange(distance: number) {
    if (distance > 0) {
      const near = distance * NEAR_FACTOR;
      const far = distance * FAR_FACTOR;
      if (this.camera.near !== near || this.camera.far !== far) {
        this.camera.near = near;
        this.camera.far = far;
        this.camera.updateProjectionMatrix();
      }
      this.overrodeRange = true;
    } else if (this.overrodeRange) {
      this.camera.near = this.baseNear;
      this.camera.far = this.baseFar;
      this.camera.updateProjectionMatrix();
      this.overrodeRange = false;
    }
  }

  /** Blur strength from how far the camera actually moved, relative to how
   * far away it is looking — a metre of travel means something different at
   * 300 mm and at 3 000 000 mm. */
  private measureBlur(track: ResolvedCameraTrack, haveLook: boolean, dtSec: number) {
    const authored = track.motionBlur ?? 0;
    if (authored <= 0 || dtSec <= 0 || !this.havePrev) {
      this.blur = 0;
      this.prevPos.copy(this.pos);
      if (haveLook) this.prevLook.copy(this.look);
      this.havePrev = true;
      return;
    }
    const travelled = this.pos.distanceTo(this.prevPos);
    const range = Math.max(this.pos.distanceTo(haveLook ? this.look : this.prevLook), 1e-6);
    // Fraction of the viewing distance covered in one second.
    const rate = travelled / range / dtSec;
    // Saturating rather than linear: past about one viewing-distance a second
    // the picture is already a smear and more blur only costs fill rate.
    this.blur = authored * (1 - Math.exp(-rate * 2));
    this.prevPos.copy(this.pos);
    if (haveLook) this.prevLook.copy(this.look);

    // Where the blur streaks *from*, in screen space.
    if (haveLook) {
      this.dir.copy(this.look).project(this.camera);
      this.focus.set((this.dir.x + 1) / 2, (this.dir.y + 1) / 2);
    } else {
      this.focus.set(0.5, 0.5);
    }
  }
}

/** `?free=1` — hand the camera to the mouse and leave the show running. */
export function freeCameraFromUrl(search = typeof location === 'undefined' ? '' : location.search): boolean {
  return new URLSearchParams(search).get('free') === '1';
}
