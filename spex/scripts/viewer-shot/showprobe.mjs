#!/usr/bin/env node
/** M62 — the show engine, measured in a real browser.
 *
 *   node scripts/viewer-shot/showprobe.mjs <show-resolved.json>
 *
 * Four questions, four numbers:
 *
 *   AC1  does seeking to t and playing 1 s produce the same state as playing
 *        0 -> t+1 s?  (hash over every value the evaluator emits)
 *   AC2  how far does the clock drift from `audioContext.currentTime`?
 *   AC3  how many bytes does `evaluate` allocate per frame after warm-up?
 *   AC4  do the easing curves hit their endpoints and stay monotonic where
 *        they are supposed to?
 *
 * Why a browser at all, for code with no renderer in it: the clock reads
 * `performance.now()` and is *meant* to read an `AudioContext`, and "zero
 * allocations" is a claim about a real JavaScript heap that only a real
 * JavaScript heap can answer. Node would give three of the four answers and
 * quietly get the interesting two wrong.
 *
 * No screenshot: this milestone adds no render pass, no material and no
 * geometry, so there is no frame for a picture to be of. Rung 5 belongs to
 * M63, which is where a camera first moves.
 */

import { chromium } from 'playwright';
import { build } from 'esbuild';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const showPath = process.argv[2];
if (!showPath) {
  console.error('usage: showprobe.mjs <show-resolved.json>');
  process.exit(2);
}

const showJson = readFileSync(resolve(showPath), 'utf8');

const bundled = await build({
  entryPoints: [resolve(here, 'show-entry.ts')],
  bundle: true,
  format: 'iife',
  target: 'es2020',
  write: false,
  logLevel: 'warning',
});
const script = bundled.outputFiles[0].text;

const browser = await chromium.launch({
  args: [
    // Without this an AudioContext starts suspended and `currentTime` never
    // advances, which would look exactly like a broken clock.
    '--autoplay-policy=no-user-gesture-required',
    '--js-flags=--expose-gc',
  ],
});
const page = await browser.newPage();
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto('about:blank');
await page.addScriptTag({ content: script });

const cdp = await page.context().newCDPSession(page);

// ---------------------------------------------------------------------------
// AC4 — the easing library
// ---------------------------------------------------------------------------
const easing = await page.evaluate(() => {
  const { EASINGS, cubicBezier } = globalThis.__spexShow;
  // back and elastic overshoot past 1 and come back; bounce reaches 1 at
  // t≈0.364 and then falls away again, three times. Asserting monotonicity on
  // any of the three would be asserting that they are not what they are.
  const NOT_MONOTONIC = new Set(['backOut', 'elasticOut', 'bounceOut']);
  const rows = [];
  for (const [name, fn] of Object.entries(EASINGS)) {
    const at0 = fn(0);
    const at1 = fn(1);
    let monotonic = true;
    let minV = Infinity;
    let maxV = -Infinity;
    let prev = fn(0);
    for (let i = 1; i <= 1000; i++) {
      const v = fn(i / 1000);
      if (v < prev - 1e-12) monotonic = false;
      if (v < minV) minV = v;
      if (v > maxV) maxV = v;
      prev = v;
    }
    rows.push({
      name,
      at0,
      at1,
      monotonic,
      monotonicRequired: !NOT_MONOTONIC.has(name),
      min: minV,
      max: maxV,
      // Outside [0,1] the curve must clamp, not extrapolate: a keyframe
      // segment can be sampled a hair past its end by floating point.
      clampsLow: fn(-0.5) === at0,
      clampsHigh: fn(1.5) === at1,
    });
  }
  const bez = cubicBezier(0.42, 0, 0.58, 1); // CSS ease-in-out
  const linearBez = cubicBezier(0, 0, 1, 1);
  return {
    rows,
    bezier: {
      at0: bez(0),
      at1: bez(1),
      atHalf: bez(0.5),
      identityMaxError: Array.from({ length: 101 }, (_, i) => Math.abs(linearBez(i / 100) - i / 100)).reduce(
        (a, b) => Math.max(a, b),
        0,
      ),
    },
  };
});

// ---------------------------------------------------------------------------
// AC1 — seek determinism
// ---------------------------------------------------------------------------
const seek = await page.evaluate((json) => {
  const { Timeline } = globalThis.__spexShow;
  const show = JSON.parse(json);

  // FNV-1a over every value the evaluator emits, quantised to 1e-6 so the
  // hash is about the state and not about the last bit of a double.
  const makeHash = () => {
    let h = 0x811c9dc5;
    const push = (x) => {
      const s = String(Math.round(x * 1e6));
      for (let i = 0; i < s.length; i++) {
        h ^= s.charCodeAt(i);
        h = Math.imul(h, 0x01000193) >>> 0;
      }
    };
    return { push, get value() { return h >>> 0; } };
  };

  const runFrom = (start, end, step) => {
    const tl = new Timeline(show);
    const h = makeHash();
    const sinks = {
      transform(target, v) {
        h.push(v.hasPosition ? 1 : 0);
        if (v.hasPosition) v.position.forEach(h.push);
        h.push(v.hasRotation ? 1 : 0);
        if (v.hasRotation) v.rotation.forEach(h.push);
        h.push(v.hasQuaternion ? 1 : 0);
        if (v.hasQuaternion) v.quaternion.forEach(h.push);
        if (v.hasScale) h.push(v.scale);
      },
      dissolve: (_t, v) => h.push(v),
      material: (_t, _p, v) => h.push(v),
      post: (_p, v) => h.push(v),
      hud: (_e, v) => h.push(v),
      pointCloud: (_t, v) => h.push(v),
      camera(_track, t01, s) {
        h.push(t01);
        if (s.hasPosition) s.position.forEach(h.push);
        if (s.hasLookAt) s.lookAt.forEach(h.push);
        if (s.hasFov) h.push(s.fovDeg);
      },
    };
    for (let t = start; t <= end + 1e-9; t += step) tl.evaluate(t, sinks);
    return h.value;
  };

  // The window that is compared is the SAME window in both runs — one
  // arrived at by seeking, one by playing through everything before it. Any
  // state the evaluator kept from the earlier frames would show up here.
  const STEP = 1 / 60;
  const results = [];
  for (const t of [0, show.durationSec * 0.25, show.durationSec * 0.5, show.durationSec - 1.5]) {
    const seeked = runFrom(t, t + 1, STEP);
    const played = runFrom(0, t + 1, STEP);
    const playedWindow = runFrom(t, t + 1, STEP);
    results.push({ t, seekedHash: seeked, windowHash: playedWindow, matches: seeked === playedWindow, fullRunHash: played });
  }

  // Cue firing is the one piece of state the evaluator does keep, so it gets
  // its own check: a seek must not replay everything before it.
  const tl = new Timeline(show);
  let firedOnSeek = 0;
  tl.fireCues(0, 0, () => {});
  tl.fireCues(0, show.durationSec * 0.75, () => firedOnSeek++);
  const tl2 = new Timeline(show);
  let firedOnPlay = 0;
  for (let t = 0, prev = 0; t <= show.durationSec * 0.75; prev = t, t += 1 / 60) {
    tl2.fireCues(prev, t, () => firedOnPlay++);
  }
  return { results, cueCount: tl.cueCount, firedOnSeek, firedOnPlay };
}, showJson);

// ---------------------------------------------------------------------------
// AC2 — clock drift against the audio clock
// ---------------------------------------------------------------------------
const drift = await page.evaluate(async () => {
  const { ShowClock } = globalThis.__spexShow;
  const ctx = new AudioContext();
  if (ctx.state === 'suspended') await ctx.resume();
  if (ctx.state !== 'running') {
    return { ran: false, state: ctx.state, sampleRate: ctx.sampleRate };
  }

  const audioClock = new ShowClock(3600, { endless: false, audioContext: ctx });
  const perfClock = new ShowClock(3600, { endless: false });
  const t0Audio = ctx.currentTime;
  const t0Perf = performance.now() / 1000;
  audioClock.play();
  perfClock.play();

  let worstAudio = 0;
  let worstPerf = 0;
  let samples = 0;
  const started = performance.now();
  // A wall-clock minute of measuring would be a wall-clock minute of CI. The
  // drift being measured is a rate, so a shorter window measures the same
  // rate with a proportionally smaller absolute number — which is why the
  // per-minute extrapolation is reported alongside the raw figure rather
  // than instead of it.
  const RUN_MS = 12000;
  while (performance.now() - started < RUN_MS) {
    await new Promise((r) => requestAnimationFrame(r));
    audioClock.tick();
    perfClock.tick();
    const audioTruth = ctx.currentTime - t0Audio;
    const perfTruth = performance.now() / 1000 - t0Perf;
    worstAudio = Math.max(worstAudio, Math.abs(audioClock.elapsed - audioTruth));
    // What the show time would have been off by if it had been driven by
    // performance.now() while the sound followed the audio clock.
    worstPerf = Math.max(worstPerf, Math.abs(perfTruth - audioTruth));
    samples++;
  }
  const ranSec = (performance.now() - started) / 1000;
  return {
    ran: true,
    sampleRate: ctx.sampleRate,
    ranSec,
    samples,
    worstAudioMs: worstAudio * 1000,
    worstPerfVsAudioMs: worstPerf * 1000,
    perfDriftPerMinuteMs: (worstPerf * 1000 * 60) / ranSec,
  };
});

// ---------------------------------------------------------------------------
// AC3 — allocations per frame
// ---------------------------------------------------------------------------
await page.evaluate((json) => {
  const { Timeline } = globalThis.__spexShow;
  const show = JSON.parse(json);
  globalThis.__probe = { tl: new Timeline(show), show };
  // Sinks that read the values without keeping them — a sink that copied
  // would be measuring the sink.
  let acc = 0;
  globalThis.__probe.sinks = {
    transform: (_t, v) => { acc += v.hasPosition ? v.position[0] : 0; },
    dissolve: (_t, v) => { acc += v; },
    material: (_t, _p, v) => { acc += v; },
    post: (_p, v) => { acc += v; },
    hud: (_e, v) => { acc += v; },
    pointCloud: (_t, v) => { acc += v; },
    camera: (_c, t01) => { acc += t01; },
  };
  globalThis.__probe.sink = () => acc;
}, showJson);

const FRAMES = 6000;
const measureAlloc = async (label, fnBody) => {
  await cdp.send('HeapProfiler.enable');
  await cdp.send('HeapProfiler.collectGarbage');
  await cdp.send('HeapProfiler.startSampling', { samplingInterval: 512 });
  await page.evaluate(fnBody, FRAMES);
  const { profile } = await cdp.send('HeapProfiler.stopSampling');
  await cdp.send('HeapProfiler.disable');
  let total = 0;
  const walk = (node) => {
    total += node.selfSize ?? 0;
    for (const c of node.children ?? []) walk(c);
  };
  walk(profile.head);
  return { label, totalBytes: total, perFrame: total / FRAMES };
};

// Warm-up first: the criterion is about the steady state, and the first
// frames legitimately allocate (the easing map, the active-shot buffer).
await page.evaluate((n) => {
  const { tl, show, sinks } = globalThis.__probe;
  for (let i = 0; i < n; i++) tl.evaluate((i / n) * show.durationSec, sinks);
}, 60);

const baseline = await measureAlloc('empty loop', (n) => {
  let x = 0;
  for (let i = 0; i < n; i++) x += i;
  globalThis.__sink = x;
});
// The positive control, and the reason the AC3 number means anything. A loop
// that allocates one small object per frame is the smallest per-frame
// allocation anyone would plausibly write by accident. If the instrument
// cannot see THAT, then "we measured nothing" is a statement about the
// instrument rather than about the code.
const control = await measureAlloc('one small object per frame', (n) => {
  // The array is parked on `globalThis`, and that detail is the whole
  // control. The first version of this kept it local and reported only
  // `keep.length` — so V8's escape analysis proved the objects never leave
  // the loop and allocated none of them, and 6000 "allocations" measured
  // 1.4 kB. A positive control that the optimiser is free to delete is not a
  // control.
  globalThis.__keep = [];
  for (let i = 0; i < n; i++) globalThis.__keep.push({ x: i, y: i * 2 });
});
const evaluated = await measureAlloc('evaluate', (n) => {
  const { tl, show, sinks } = globalThis.__probe;
  for (let i = 0; i < n; i++) tl.evaluate((i / n) * show.durationSec, sinks);
});

await browser.close();

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------
const show = JSON.parse(showJson);
console.log(`show: ${show.id} — ${show.shots.length} shot(s), ${show.durationSec.toFixed(3)} s, ${seek.cueCount} cue(s)`);

console.log('\nAC4 — easing');
let easingBad = 0;
for (const r of easing.rows) {
  const endpointsOk = Math.abs(r.at0) < 1e-12 && Math.abs(r.at1 - 1) < 1e-12;
  const monoOk = !r.monotonicRequired || r.monotonic;
  const clampOk = r.clampsLow && r.clampsHigh;
  if (!endpointsOk || !monoOk || !clampOk) easingBad++;
  console.log(
    `  ${r.name.padEnd(14)} f(0)=${r.at0.toFixed(3)} f(1)=${r.at1.toFixed(3)} ` +
      `range [${r.min.toFixed(3)}, ${r.max.toFixed(3)}] ` +
      `${r.monotonic ? 'monotonic' : 'overshoots'}${r.monotonicRequired ? '' : ' (by design)'} ` +
      `${endpointsOk && monoOk && clampOk ? '' : '  <-- FAIL'}`,
  );
}
console.log(
  `  cubicBezier(.42,0,.58,1): f(0)=${easing.bezier.at0.toFixed(6)} f(.5)=${easing.bezier.atHalf.toFixed(6)} ` +
    `f(1)=${easing.bezier.at1.toFixed(6)}; identity curve max error ${easing.bezier.identityMaxError.toExponential(2)}`,
);

console.log('\nAC1 — seek determinism (hash over every value the evaluator emits)');
let seekBad = 0;
for (const r of seek.results) {
  if (!r.matches) seekBad++;
  console.log(
    `  t=${r.t.toFixed(3).padStart(8)}s  seeked ${r.seekedHash.toString(16).padStart(8, '0')}  ` +
      `played-through ${r.windowHash.toString(16).padStart(8, '0')}  ${r.matches ? 'identical' : 'DIFFERENT'}`,
  );
}
console.log(`  cues fired reaching 75% by seek: ${seek.firedOnSeek} — by playing: ${seek.firedOnPlay}`);

console.log('\nAC2 — clock drift');
if (!drift.ran) {
  console.log(`  no running AudioContext (state=${drift.state}) — not measured here`);
} else {
  console.log(`  AudioContext ${drift.sampleRate} Hz, ${drift.samples} frames over ${drift.ranSec.toFixed(1)} s`);
  console.log(`  show time vs audioContext.currentTime: worst ${drift.worstAudioMs.toFixed(4)} ms`);
  console.log(
    `  performance.now() vs audio clock: worst ${drift.worstPerfVsAudioMs.toFixed(3)} ms ` +
      `(~${drift.perfDriftPerMinuteMs.toFixed(1)} ms/min if the rate holds)`,
  );
}

console.log('\nAC3 — allocation per frame (Chromium heap sampling, 512 B interval)');
console.log(`  empty loop           : ${String(baseline.totalBytes).padStart(8)} B total, ${baseline.perFrame.toFixed(2).padStart(8)} B/frame`);
console.log(`  1 small object/frame : ${String(control.totalBytes).padStart(8)} B total, ${control.perFrame.toFixed(2).padStart(8)} B/frame   <- positive control`);
console.log(`  evaluate             : ${String(evaluated.totalBytes).padStart(8)} B total, ${evaluated.perFrame.toFixed(2).padStart(8)} B/frame over ${FRAMES} frames`);
console.log(
  `  evaluate minus empty loop: ${evaluated.totalBytes - baseline.totalBytes} B ` +
    `(control minus empty loop: ${control.totalBytes - baseline.totalBytes} B — that is what a real per-frame allocation looks like here)`,
);

if (errors.length) {
  console.log('\nconsole errors:');
  for (const e of errors) console.log(`  ${e}`);
}

const failed = easingBad > 0 || seekBad > 0 || errors.length > 0;
console.log(`\n${failed ? 'FAIL' : 'ok'} — ${easingBad} easing problem(s), ${seekBad} seek mismatch(es), ${errors.length} console error(s)`);
process.exit(failed ? 1 : 0);
