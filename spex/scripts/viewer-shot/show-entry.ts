/** Bundle entry for `showprobe.mjs` — the only reason it exists.
 *
 * M62's engine is deliberately renderer-free, so it can be verified without a
 * scene, a canvas or a GPU. But it is still *browser* code: it reads
 * `performance.now()`, it is meant to run against a real `AudioContext`, and
 * the allocation criterion is a statement about a real JavaScript heap. So
 * the probe runs it in real Chromium rather than in Node, and this file is
 * what esbuild bundles to get it there.
 *
 * It exports nothing the viewer imports. Nothing in `viewer/src` depends on
 * this file, and it must stay that way: a test entry that production code
 * reaches into stops being a test entry.
 */

import { ShowClock } from '../../viewer/src/show/clock';
import { Timeline } from '../../viewer/src/show/timeline';
import { EASINGS, easingByName, cubicBezier } from '../../viewer/src/show/easing';

(globalThis as unknown as { __spexShow: unknown }).__spexShow = {
  ShowClock,
  Timeline,
  EASINGS,
  easingByName,
  cubicBezier,
};
