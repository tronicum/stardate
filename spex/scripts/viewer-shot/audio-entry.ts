/** Bundle entry for `audioprobe.mjs` — the only reason it exists.
 *
 * M69's engine is WebAudio, so it cannot be tested in Node without a shim
 * that would be the thing under test. It runs in real Chromium instead, and
 * this is what esbuild bundles to get it there.
 *
 * Nothing in `viewer/src` imports this file, and it must stay that way: a test
 * entry that production code reaches into stops being a test entry.
 */
import { AudioEngine, analyse, measureOutputLatency, saturationCurve, ceilingCurve, CEILING, LIMITER } from '../../viewer/src/audio/engine';
import { makeImpulseResponse, mulberry32, ReverbRack, SPACES } from '../../viewer/src/audio/reverb';
import { midiToFrequency, PARTIALS, VOICE_PAN, VOICE_TIMBRES } from '../../viewer/src/audio/synth';
import { parseSmf, tickToSeconds, secondsToTick } from '../../viewer/src/audio/midi';
import { Scheduler, cuesFromScore, LOOKAHEAD_SEC, TICK_MS } from '../../viewer/src/audio/scheduler';

(globalThis as unknown as { __spexAudio: unknown }).__spexAudio = {
  AudioEngine,
  analyse,
  measureOutputLatency,
  saturationCurve,
  ceilingCurve,
  CEILING,
  LIMITER,
  makeImpulseResponse,
  mulberry32,
  ReverbRack,
  SPACES,
  midiToFrequency,
  PARTIALS,
  VOICE_PAN,
  VOICE_TIMBRES,
  parseSmf,
  tickToSeconds,
  secondsToTick,
  Scheduler,
  cuesFromScore,
  LOOKAHEAD_SEC,
  TICK_MS,
};
