/** M70 — the lookahead scheduler.
 *
 * # Never from `requestAnimationFrame`
 *
 * The spec says so and it is the single most important line in this file. A
 * rAF callback fires when the compositor is ready, which is *not* a clock: it
 * jitters by whole frames, stops entirely in a background tab, and on this
 * project's own software rasteriser arrives about three times a second.
 * Scheduling a note when the callback happens to run would put every accent
 * wherever the GPU left it.
 *
 * So: a **25 ms `setInterval`** wakes up, looks **150 ms ahead**, and hands
 * WebAudio the *absolute* `AudioContext` time each note should begin at.
 * WebAudio then starts it on the right sample, whatever the main thread is
 * doing. The interval only has to be reliable enough to arrive at least once
 * per lookahead window — six chances per window here — and nothing about the
 * sound depends on when within that window it woke up.
 *
 * That is the standard construction (Chris Wilson's "A Tale of Two Clocks");
 * it is written out here rather than cited because the *reason* is the thing
 * that matters and it is easy to lose in a refactor.
 *
 * # The scheduler reads the show clock, and only the show clock
 *
 * There is one clock in this piece (`show/clock.ts`, M62), it derives time
 * from an anchor rather than accumulating deltas, and it prefers the audio
 * hardware's own oscillator when there is one. The scheduler asks it for the
 * time and maps score seconds onto `AudioContext` seconds through a single
 * offset. Two clocks would be two answers to "when is bar 37".
 *
 * # Seeking
 *
 * A seek flushes every sounding voice with a **20 ms release ramp** and
 * re-primes the cursor. Never an abrupt stop: that is a click, and a piece
 * that clicks whenever anyone scrubs it is a piece nobody scrubs.
 */

import type { AudioEngine } from './engine';
import type { ScoredNote, Score } from './midi';

/** How often the scheduler wakes. */
export const TICK_MS = 25;
/** How far ahead it schedules. Six wake-ups fit in this window, so five of
 * them can be late or missed entirely and every note still lands. */
export const LOOKAHEAD_SEC = 0.15;
/** The release ramp a seek uses. */
export const SEEK_RAMP_SEC = 0.02;

/** What the scheduler tells the rest of the piece about. M71 binds these. */
export interface Cue {
  kind: 'entry' | 'section' | 'accent';
  atSec: number;
  label: string;
  voice?: number;
}

export interface SchedulerOptions {
  /** Show time of score position zero. Non-zero because the piece is silent
   * until the brick is legible — the fugue starts at bar 5, not at 0. */
  scoreOffsetSec?: number;
  cues?: Cue[];
  onCue?: (cue: Cue) => void;
}

/** A clock the scheduler can read. Structural rather than a concrete import,
 * so a test can drive it without building a `ShowClock` — and so `show/` and
 * `audio/` stay independent of each other's types. */
export interface ReadableClock {
  readonly time: number;
  readonly playing: boolean;
}

export class Scheduler {
  readonly engine: AudioEngine;
  readonly score: Score;
  private readonly clock: ReadableClock;
  private readonly notes: ScoredNote[];
  private readonly cues: Cue[];
  private readonly onCue?: (cue: Cue) => void;
  private readonly scoreOffsetSec: number;

  /** Index of the first note not yet scheduled. Monotonic during playback;
   * a seek moves it. */
  private cursor = 0;
  private cueCursor = 0;
  private timer: ReturnType<typeof setInterval> | null = null;
  /** Every note handed to the engine, so a flush knows what to release and
   * `pendingCount` can be asserted on. */
  private readonly inFlight: { voice: number; midi: number; offAt: number }[] = [];

  /** Notes scheduled, for the harness. */
  scheduled = 0;
  /** Notes picked up mid-flight by a seek. */
  resumed = 0;
  /** The longest note in the score — how far back a seek has to look. */
  private readonly maxDurationSec: number;

  constructor(engine: AudioEngine, score: Score, clock: ReadableClock, opts: SchedulerOptions = {}) {
    this.engine = engine;
    this.score = score;
    this.clock = clock;
    this.notes = score.notes;
    this.cues = (opts.cues ?? []).slice().sort((a, b) => a.atSec - b.atSec);
    this.onCue = opts.onCue;
    this.scoreOffsetSec = opts.scoreOffsetSec ?? 0;
    this.maxDurationSec = this.notes.reduce((m, n) => Math.max(m, n.durationSec), 0);
  }

  /** Score position for a show time. */
  scoreTime(showTime: number): number {
    return showTime - this.scoreOffsetSec;
  }

  start() {
    if (this.timer !== null) return;
    this.timer = setInterval(() => this.pump(), TICK_MS);
    this.pump();
  }

  stop() {
    if (this.timer === null) return;
    clearInterval(this.timer);
    this.timer = null;
  }

  /** One scheduling pass.
   *
   * Separate from `start` so a test can drive it with an explicit time — an
   * `OfflineAudioContext` has no wall clock and `setInterval` would never
   * produce a single note in one. The production path calls it from the
   * interval; the harness calls it directly. Same code either way, which is
   * the only reason the harness's numbers mean anything about production.
   */
  pump(nowAudio = this.engine.ctx.currentTime, nowShow = this.clock.time) {
    this.retire(nowAudio);
    if (!this.clock.playing) return;

    const from = this.scoreTime(nowShow);
    const until = from + LOOKAHEAD_SEC;
    // Show time and audio time run at the same rate; the difference between
    // them is the offset this frame. Computing it once per pump rather than
    // per note is what keeps every note in a window relative to the same
    // reference — deriving it per note would let the two clocks' jitter into
    // the intervals *between* notes.
    const audioForScore = (scoreSec: number) => nowAudio + (scoreSec - from);

    while (this.cursor < this.notes.length && this.notes[this.cursor].atSec < until) {
      const n = this.notes[this.cursor++];
      if (n.atSec < from - 1e-6) continue; // already gone by
      const at = audioForScore(n.atSec);
      this.engine.noteOn(n.voice, n.midi, at, 0.35 + n.velocity * 0.55);
      const offAt = at + Math.max(0.02, n.durationSec);
      this.engine.noteOff(n.voice, n.midi, offAt);
      this.inFlight.push({ voice: n.voice, midi: n.midi, offAt });
      this.scheduled++;
    }

    while (this.cueCursor < this.cues.length && this.cues[this.cueCursor].atSec < until) {
      const c = this.cues[this.cueCursor++];
      if (c.atSec < from - 1e-6) continue;
      this.onCue?.(c);
    }
  }

  /** Drop the bookkeeping for notes whose release has passed. Without this
   * the list grows for the length of the piece and `pendingCount` — which
   * AC3 is about — becomes meaningless. */
  private retire(nowAudio: number) {
    for (let i = this.inFlight.length - 1; i >= 0; i--) {
      if (this.inFlight[i].offAt <= nowAudio) this.inFlight.splice(i, 1);
    }
  }

  /** Move to a show time: release everything, re-prime, and **resume the
   * notes that span the seek point**.
   *
   * That last part is not a nicety. The first version placed the cursor at
   * the next note and stopped there, on the reasoning that a note has an
   * attack and starting one in the middle is a click with a pitch — which is
   * true, and which produced **silence** when the harness seeked into bar 81.
   * The piece has a three-and-a-half-bar pedal point there: one held note
   * under everything else, started long before. Landing inside it and playing
   * nothing is not "musically correct material for that position", it is the
   * absence of the material.
   *
   * So a spanning note is resumed for its *remaining* duration, and the click
   * problem is solved where it belongs — in the envelope. The voice is given
   * the same attack it would have had; at a few tens of milliseconds nobody
   * hears an attack that was not in the score, and everybody hears a missing
   * pedal.
   *
   * **Only while the clock is running.** Scrubbing a paused show must not
   * start a note: the show is stopped, and a held pedal that begins sounding
   * because someone dragged the scrubber is a stuck note by any definition —
   * nothing will ever come along to stop it, because nothing is playing. This
   * is the whole of AC3's residue: the randomised harness ended on a seek
   * while paused and found four notes pending, and they were pending because
   * the seek had started them. `pump` resumes normal scheduling on the next
   * wake-up after play, and a play from a paused position re-seeks.
   */
  seek(showTime: number, nowAudio = this.engine.ctx.currentTime) {
    this.engine.flushAll(nowAudio + SEEK_RAMP_SEC);
    this.inFlight.length = 0;
    const target = this.scoreTime(showTime);
    this.cursor = lowerBound(this.notes, target, (n) => n.atSec);
    this.cueCursor = lowerBound(this.cues, target, (c) => c.atSec);
    if (!this.clock.playing) return;

    // Walk back over anything long enough to still be sounding. The longest
    // note in this score is the pedal; `maxDurationSec` bounds the walk so a
    // seek stays O(few) rather than O(score).
    const at = nowAudio + SEEK_RAMP_SEC;
    for (let i = this.cursor - 1; i >= 0; i--) {
      const n = this.notes[i];
      if (target - n.atSec > this.maxDurationSec) break;
      const remaining = n.atSec + n.durationSec - target;
      if (remaining <= 0.05) continue;
      this.engine.noteOn(n.voice, n.midi, at, 0.35 + n.velocity * 0.55);
      const offAt = at + remaining;
      this.engine.noteOff(n.voice, n.midi, offAt);
      this.inFlight.push({ voice: n.voice, midi: n.midi, offAt });
      this.resumed++;
    }
  }

  /** Notes handed to the engine whose release has not yet passed. After a
   * flush this must be zero — that is AC3, as a number. */
  get pendingCount(): number {
    return this.inFlight.length;
  }

  get running(): boolean {
    return this.timer !== null;
  }
}

/** First index whose key is >= target. */
function lowerBound<T>(items: readonly T[], target: number, key: (t: T) => number): number {
  let lo = 0;
  let hi = items.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (key(items[mid]) < target) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

/** The cues a score implies, derived rather than authored.
 *
 * A subject entry is where a voice starts a phrase after a rest — which is
 * exactly what a fugue entry *is*, and deriving it from the notes means the
 * cue list cannot disagree with the music. The alternative, a hand-written
 * list of bar numbers beside the score, is two things to keep in step.
 */
export function cuesFromScore(score: Score, minRestSec = 1.0): Cue[] {
  const out: Cue[] = [];
  const lastEnd = new Map<number, number>();
  for (const n of score.notes) {
    const prev = lastEnd.get(n.voice);
    if (prev === undefined || n.atSec - prev >= minRestSec) {
      out.push({
        kind: 'entry',
        atSec: n.atSec,
        voice: n.voice,
        label: `${score.trackNames[n.voice + 1] ?? `voice ${n.voice}`} enters`,
      });
    }
    lastEnd.set(n.voice, Math.max(prev ?? 0, n.atSec + n.durationSec));
  }
  return out.sort((a, b) => a.atSec - b.atSec);
}
