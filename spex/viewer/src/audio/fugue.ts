/** M70 — the piece's audio, assembled.
 *
 * One function that a show loads and one object it drives. Everything
 * interesting is in the three modules under it; this is the seam where the
 * score, the engine and the clock meet, and it is deliberately thin — a seam
 * with logic in it is a fourth place to look when something is late.
 */

import { AudioEngine, type EngineOptions } from './engine';
import { fetchScore, type Score } from './midi';
import { cuesFromScore, Scheduler, type Cue, type ReadableClock } from './scheduler';

export interface FugueOptions extends EngineOptions {
  /** Show time at which score position zero falls. The piece is silent until
   * the brick is legible: the fugue's first entry is at bar 5. */
  scoreOffsetSec?: number;
  onCue?: (cue: Cue) => void;
}

export interface FugueAudio {
  engine: AudioEngine;
  scheduler: Scheduler;
  score: Score;
  cues: Cue[];
  start(): void;
  stop(): void;
  seek(showTime: number): void;
}

/** Loads `fugue.mid` from a show directory and wires it up.
 *
 * Returns `null` when there is no score — every point-cloud and graph tileset,
 * and any show built before the audio existed. The same absence-is-a-fact
 * pattern the viewer's three render modes already use. */
export async function loadFugueAudio(
  ctx: BaseAudioContext,
  baseUrl: string,
  clock: ReadableClock,
  opts: FugueOptions = {},
): Promise<FugueAudio | null> {
  const score = await fetchScore(baseUrl);
  if (!score) return null;

  const engine = new AudioEngine(ctx, opts);
  const cues = cuesFromScore(score);
  const scheduler = new Scheduler(engine, score, clock, {
    scoreOffsetSec: opts.scoreOffsetSec ?? 0,
    cues,
    onCue: opts.onCue,
  });

  return {
    engine,
    scheduler,
    score,
    cues,
    start: () => scheduler.start(),
    stop: () => scheduler.stop(),
    seek: (showTime: number) => scheduler.seek(showTime),
  };
}
