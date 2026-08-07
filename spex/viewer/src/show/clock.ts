/** M62 — one clock.
 *
 * Everything reads the same time value or the piece comes apart: the camera
 * arrives a frame before the cut, the brick lands a frame after its accent,
 * and each is individually invisible while the whole is subtly wrong.
 *
 * # Why an AudioContext, when `performance.now()` exists
 *
 * `performance.now()` measures the same thing the display does, and nothing
 * else. The audio hardware runs on **its own oscillator**, and the two are
 * not the same crystal — over four minutes a browser's audio clock and its
 * high-resolution timer routinely differ by tens of milliseconds. A visual
 * accent driven by `performance.now()` therefore drifts against the sound it
 * is an accent *for*, and by the end of the piece the drift is audible as
 * well as visible.
 *
 * So when an `AudioContext` exists, show time comes from
 * `audioContext.currentTime`: whatever the audio hardware believes, the
 * picture believes too, and the two cannot disagree by construction rather
 * than by measurement. `performance.now()` remains the fallback for muted
 * sessions and headless runs, where there is nothing to drift against.
 *
 * # Time is derived, never accumulated
 *
 * Not `time += delta`. Show time is `(source now − the source reading when
 * playback last started) + the offset it started from`. Accumulating deltas
 * accumulates their rounding too — sixty times a second for an hour is 216
 * 000 additions of a float — and it makes a dropped frame permanent. Derived
 * time self-corrects: after a stall of any length the very next `tick` is
 * already right.
 */

export interface ShowClockOptions {
  endless: boolean;
  audioContext?: { currentTime: number };
}

export type LoopHandler = (cycle: number) => void;

export class ShowClock {
  readonly durationSec: number;
  readonly endless: boolean;

  private readonly audio?: { currentTime: number };
  /** The source's reading when the current play span began. */
  private anchor = 0;
  /** Show time (within one cycle) at that anchor. */
  private offset = 0;
  private cycles = 0;
  private running = false;
  private lastTime = 0;
  private readonly loopHandlers: LoopHandler[] = [];

  constructor(durationSec: number, opts: ShowClockOptions) {
    if (!(durationSec > 0)) {
      throw new Error(`show duration must be positive, got ${durationSec}`);
    }
    this.durationSec = durationSec;
    this.endless = opts.endless;
    this.audio = opts.audioContext;
  }

  /** Which clock this instance is actually reading — recorded in the HUD and
   * in every measurement, because a drift number means nothing without it. */
  get source(): 'audio' | 'performance' {
    return this.audio ? 'audio' : 'performance';
  }

  private now(): number {
    return this.audio ? this.audio.currentTime : performance.now() / 1000;
  }

  get playing(): boolean {
    return this.running;
  }

  /** Completed loops. The endless edition advances its seed on each one. */
  get cycle(): number {
    return this.cycles;
  }

  /** Current show time in seconds, in [0, durationSec]. */
  get time(): number {
    return this.lastTime;
  }

  /** Raw, unwrapped elapsed show time — what `time` would be if the piece
   * never looped. Useful for measuring drift against a monotonic reference. */
  get elapsed(): number {
    if (!this.running) return this.cycles * this.durationSec + this.lastTime;
    return this.cycles * this.durationSec + this.offset + (this.now() - this.anchor);
  }

  play() {
    if (this.running) return;
    this.anchor = this.now();
    this.offset = this.lastTime;
    this.running = true;
  }

  pause() {
    if (!this.running) return;
    this.tick();
    this.running = false;
    this.offset = this.lastTime;
  }

  /** Jump to an absolute show time.
   *
   * Clamped, not wrapped: a seek is a person moving a playhead, and a
   * playhead dragged past the end should sit at the end rather than silently
   * reappear at the start. Looping is what `endless` is for. */
  seek(sec: number) {
    const t = Math.min(Math.max(sec, 0), this.durationSec);
    this.lastTime = t;
    this.offset = t;
    this.anchor = this.now();
  }

  onLoop(cb: LoopHandler) {
    this.loopHandlers.push(cb);
  }

  /** Called once per animation frame. Returns the delta actually applied,
   * which is what a caller needs to know after a stall. */
  tick(): number {
    if (!this.running) return 0;
    const prev = this.lastTime;
    let t = this.offset + (this.now() - this.anchor);

    if (t >= this.durationSec) {
      if (this.endless) {
        // A tab left in the background for ten minutes comes back with a
        // delta of 600 s. Firing 210 loop handlers to catch up would be a
        // stampede for no benefit — the seed advance is a state change, not
        // a sequence of events with individual meaning. So: count the whole
        // jump, notify once with the new cycle number.
        const passed = Math.floor(t / this.durationSec);
        this.cycles += passed;
        t -= passed * this.durationSec;
        this.offset = t;
        this.anchor = this.now();
        for (const cb of this.loopHandlers) cb(this.cycles);
      } else {
        t = this.durationSec;
        this.running = false;
        this.offset = t;
      }
    }

    this.lastTime = t;
    // Across a loop boundary the delta is not `t - prev`; the caller wants
    // how much show time really passed, not a negative number.
    return t >= prev ? t - prev : t + this.durationSec - prev;
  }
}
