/** M71 — what a cue does to the picture.
 *
 * # Why this is a module and not four lines in the player
 *
 * A cue arrives from the scheduler **up to 150 ms before it sounds**. That is
 * the whole point of a lookahead scheduler (M70): WebAudio is handed an
 * absolute start time and plays the note on the right sample whatever the main
 * thread is doing. A visual bound to the *arrival* of the cue would therefore
 * fire a tenth of a second early, every time, and the milestone's own AC2 asks
 * for the Kick and its frame to land within 16.7 ms of each other.
 *
 * So every cue is carried with the `AudioContext` time it will sound at, held
 * until that time has actually come, and only then applied. The frame loop
 * asks "what is due?" with the same clock the sound is on, which makes the
 * binding accurate to one frame by construction rather than by tuning.
 *
 * # The envelopes are here too
 *
 * An entry lift and a bloom pulse are both "go to 1, come back down". Doing
 * that in the player would put four decay counters in a frame loop that
 * already has plenty; doing it here means the decay is a property of the
 * binding and can be tested without a renderer, which is the same reason
 * `timeline.ts` writes to a sinks object instead of to a scene.
 */

import type { Cue } from '../audio/scheduler';

/** How long a subject entry's emissive lift takes to fall back.
 *
 * Nearly a bar at 84 bpm (2.86 s a bar), which sounds long and is not: the
 * entry it marks is a two-bar subject, and a flash that is over before the
 * head of the subject is would mark the attack rather than the entry. */
export const ENTRY_LIFT_DECAY_SEC = 0.9;

/** The bloom pulse an Act IV accent adds, and how fast it goes.
 *
 * Short, because this one *is* the attack: a kick is a transient and a bloom
 * that hangs on past it reads as the exposure drifting rather than as the
 * picture being hit. */
export const ACCENT_BLOOM_GAIN = 0.35;
export const ACCENT_BLOOM_DECAY_SEC = 0.22;

/** Which scene element a voice lights up.
 *
 * Authored, never inferred. The screenplay assigns the four voices to four
 * things in the frame and that assignment is a directorial decision; a rule
 * like "the nth scene in the shot's list" would be a coin flip that happened
 * to land right once. */
export interface VoiceBinding {
  voice: number;
  scene: string;
  /** Optional glob within the scene. Absent means the whole scene. */
  glob?: string;
}

export interface BinderEvents {
  /** A section boundary: the HUD's movement card. */
  onSection?: (label: string, atAudioSec: number) => void;
  /** The one accent the piece ends on. */
  onKick?: (atAudioSec: number) => void;
}

/** Holds cues until they sound, then drives the decaying visual state. */
export class CueBinder {
  /** Per voice, 0..1, decaying. Read by the player each frame. */
  readonly lift = new Map<number, number>();
  /** Extra bloom strength, 0..`ACCENT_BLOOM_GAIN`, decaying. */
  bloom = 0;
  /** The most recent section label, for the HUD. */
  section: string | null = null;
  /** The audio time the Kick was *applied* at — AC2 measures this against the
   * audio time it was *scheduled* for. Null until it fires. */
  kickAppliedAt: number | null = null;
  kickScheduledAt: number | null = null;

  /** The audio↔visual number, measured where it happens.
   *
   * A cue is held until its `AudioContext` time arrives and applied on the
   * next frame after that, so the latency is bounded below by the frame
   * interval and by nothing else. Both are recorded: the milliseconds, which
   * are a property of whatever is rendering, and the count in *frames*, which
   * is a property of this code and is the thing that can be asserted anywhere.
   * On a machine at 60 Hz, one frame is 16.7 ms. */
  worstLatencySec = 0;
  worstLatencyFrames = 0;
  appliedCount = 0;

  /** The interval between updates, taken from the audio clock itself rather
   * than from the frame's `dtSec`.
   *
   * The player clamps `dtSec` to 0.25 s (a tab that was in the background must
   * not produce a quarter-hour of decay in one frame) and zeroes it on a seek.
   * Both are right for an envelope and wrong for a measurement: a frame that
   * really took 460 ms reported 250, and the lateness came out at 1.8 frames
   * when it was one. The audio clock's own delta between two updates *is* the
   * frame interval, exactly, with nothing clamped. */
  private lastNowAudio: number | null = null;
  private frameSec = 0;

  /** Cues handed over but not yet due, oldest first. */
  private readonly pending: { cue: Cue; atAudioSec: number }[] = [];
  private readonly events: BinderEvents;

  constructor(events: BinderEvents = {}) {
    this.events = events;
  }

  /** Take a cue and the absolute `AudioContext` time it will sound at. */
  schedule(cue: Cue, atAudioSec: number): void {
    this.pending.push({ cue, atAudioSec });
    // Insertion sort from the end: cues arrive in time order almost always,
    // so this is one comparison per cue in the normal case, and correct in
    // the case a seek makes it not.
    for (let i = this.pending.length - 1; i > 0; i--) {
      if (this.pending[i - 1].atAudioSec <= this.pending[i].atAudioSec) break;
      const t = this.pending[i - 1];
      this.pending[i - 1] = this.pending[i];
      this.pending[i] = t;
    }
  }

  /** Apply everything due, then decay. `dtSec` is frame time, not show time:
   * these are envelopes on the picture, and a paused show should not hold a
   * flash on screen for ever. */
  update(nowAudioSec: number, dtSec: number): void {
    this.frameSec = this.lastNowAudio === null ? 0 : nowAudioSec - this.lastNowAudio;
    this.lastNowAudio = nowAudioSec;
    while (this.pending.length && this.pending[0].atAudioSec <= nowAudioSec) {
      const { cue, atAudioSec } = this.pending.shift()!;
      this.apply(cue, atAudioSec, nowAudioSec);
    }

    if (this.bloom > 0) {
      this.bloom = Math.max(0, this.bloom - (dtSec / ACCENT_BLOOM_DECAY_SEC) * ACCENT_BLOOM_GAIN);
    }
    for (const [voice, value] of this.lift) {
      const next = value - dtSec / ENTRY_LIFT_DECAY_SEC;
      if (next <= 0) this.lift.delete(voice);
      else this.lift.set(voice, next);
    }
  }

  private apply(cue: Cue, atAudioSec: number, nowAudioSec: number): void {
    const late = nowAudioSec - atAudioSec;
    this.appliedCount++;
    if (late > this.worstLatencySec) this.worstLatencySec = late;
    if (this.frameSec > 0) {
      const frames = late / this.frameSec;
      if (frames > this.worstLatencyFrames) this.worstLatencyFrames = frames;
    }
    switch (cue.kind) {
      case 'entry':
        if (cue.voice !== undefined) this.lift.set(cue.voice, 1);
        return;
      case 'section':
        this.section = cue.label;
        this.events.onSection?.(cue.label, atAudioSec);
        return;
      case 'accent':
        this.bloom = ACCENT_BLOOM_GAIN;
        if (cue.kick) {
          this.kickScheduledAt = atAudioSec;
          this.kickAppliedAt = nowAudioSec;
          this.events.onKick?.(atAudioSec);
        }
    }
  }

  /** A seek drops everything queued. A cue that was going to sound in 120 ms
   * on a passage nobody is at any more is not an accent, it is a leftover. */
  /** Zero the latency counters — the harness samples them per interval. */
  resetMeasurements(): void {
    this.lastNowAudio = null;
    this.worstLatencySec = 0;
    this.worstLatencyFrames = 0;
    this.appliedCount = 0;
  }

  reset(): void {
    this.pending.length = 0;
    this.lift.clear();
    this.bloom = 0;
  }

  get pendingCount(): number {
    return this.pending.length;
  }

  /** The audio time of the next cue waiting, or `null`. The harness reads it
   * to measure how late the frame that applies it actually was — which is the
   * audio↔visual number, and cannot be derived from anything else. */
  pendingPeek(): number | null {
    return this.pending.length ? this.pending[0].atAudioSec : null;
  }
}
