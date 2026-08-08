#!/usr/bin/env node
/** M66 — the act, run.
 *
 *   spex show-build shows/die-geschichtliche-matrix.show.json -o demos/matrix \
 *     --duration 240 --duration 120 --endless --skip-unbuildable
 *   spex show demos/matrix --port 8110 --no-open &
 *   node scripts/viewer-shot/showrun.mjs http://127.0.0.1:8110/ /tmp/m66
 *
 * Three questions, one per acceptance criterion:
 *
 *   AC1  does the canonical cut run end to end — every shot reached, the
 *        arithmetic landing on the declared duration, no console errors — and
 *        does it loop *cleanly*?
 *   AC3  does each URL parameter do the thing it says, one at a time?
 *   L5   and what does it look like?
 *
 * # "Cleanly" is the interesting word
 *
 * A loop that merely does not crash is not clean. The test here is stronger
 * and it is a pixel test: render the frame at t=0 on the first cycle, let the
 * clock wrap, render the frame at t=0 again, and require the two to be
 * identical. That catches the whole class of defect where a shot-scoped track
 * leaves shared state behind — the vignette A1-S06 raises, the outline opacity
 * A1-S03 sets, the dissolve A1-S04 writes — none of which anything rewrites at
 * t=0, because the opening shot has no reason to mention them.
 *
 * Each parameter gets its own page load. Sharing one would mean a parameter
 * could pass because of what a previous one left behind, which is the same
 * mistake in a different costume.
 */

import { chromium } from 'playwright';
import { PNG } from 'pngjs';
import { mkdirSync, writeFileSync } from 'node:fs';

const url = process.argv[2];
const outDir = process.argv[3];
if (!url || !outDir) {
  console.error('usage: showrun.mjs <viewer-url> <out-dir>');
  process.exit(2);
}
mkdirSync(outDir, { recursive: true });

const browser = await chromium.launch();
const VIEWPORT = { width: 640, height: 400 };
/** Maximum per-channel difference two renders of the same state may show
 * without meaning anything: the grade pass's own dither. */
const DITHER_FLOOR = 2;

/** Opens the show, waits for it to be running, and returns the page plus the
 * console errors it produced. Every check gets a fresh one. */
async function open(query = '') {
  const errors = [];
  const page = await browser.newPage({ viewport: VIEWPORT });
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto(url + query, { waitUntil: 'networkidle' });
  await page.waitForFunction(() => !!window.__spexShow, null, { timeout: 120000 });
  // Past the two-second quality benchmark, so nothing is rebuilt mid-measure.
  await page.waitForTimeout(3000);
  return { page, errors };
}

const report = { ac1: null, ac3: [], errors: [] };

// ---------------------------------------------------------------------- AC1
//
// Split in two, because the two halves cost three orders of magnitude apart.
//
//   The arithmetic and the shot coverage are questions about the *evaluator*.
//   They need no frames at all, so they are asked at 120 sample points through
//   the 240 s cut and cost milliseconds.
//
//   Whether cues fire, voices enter in order and the piece loops is a question
//   about *playback*, and there is no honest way to ask it except by playing.
//   So that half runs the endless cut — 48.571 s — in real time, once through.
//   Seeking cannot answer it: `fireCues` deliberately does not replay what a
//   jump skipped, so a sweep of seeks fires no audio cue at all and would
//   report zero voices for a piece whose whole structure is four of them.

{
  const { page, errors } = await open('?director=1');
  const arithmetic = await page.evaluate(() => {
    const s = window.__spexShow;
    const N = 120;
    const seen = [];
    const gaps = [];
    for (let i = 0; i <= N; i++) {
      const t = (i / N) * s.show.durationSec;
      const live = s.timeline.activeShots(t);
      if (live.length === 0) gaps.push(+t.toFixed(3));
      const id = live[0]?.shot.id ?? null;
      if (id && seen[seen.length - 1] !== id) seen.push(id);
    }
    return {
      durationSec: s.show.durationSec,
      sumOfShots: s.show.shots.reduce((n, x) => n + x.durationSec, 0),
      lastEnd: s.show.shots[s.show.shots.length - 1].endSec,
      beatAligned: s.show.beatAligned,
      contiguous: s.show.shots.every((x, i, a) => i === 0 || Math.abs(x.startSec - a[i - 1].endSec) < 1e-9),
      reached: seen,
      shotCount: s.show.shots.length,
      uncoveredSamples: gaps,
      warnings: s.warnings,
    };
  });
  report.ac1 = arithmetic;
  report.errors.push(...errors);
  await page.close();
}

{
  const { page, errors } = await open('?duration=endless&director=1');
  const played = await page.evaluate(async () => {
    const s = window.__spexShow;
    const gl = s.renderer.getContext();
    const w = s.renderer.domElement.width;
    const h = s.renderer.domElement.height;
    const b64 = (bytes) => {
      let str = '';
      for (let i = 0; i < bytes.length; i += 8192) str += String.fromCharCode.apply(null, bytes.subarray(i, i + 8192));
      return btoa(str);
    };
    const grab = () => {
      const buf = new Uint8Array(w * h * 4);
      gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, buf);
      return b64(buf);
    };
    const frame = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));

    // The reference frame: paused, seeked to 0, rendered.
    s.setPlaying(false);
    s.seek(0);
    await frame();
    const before = grab();

    // Now play, for real, from 0, until the clock has wrapped once.
    const seen = [];
    s.seek(0);
    s.setPlaying(true);
    const startCycle = s.clock.cycle;
    const t0 = performance.now();
    let frames = 0;
    // Voices are sampled *during* the cycle, not after it. The loop handler
    // clears them — a new cycle is a new performance and the four voices enter
    // again — so reading the list once the wrap has happened reports none, and
    // the first version of this harness duly reported a four-voice fugue as
    // having no voices at all. The instrument was wrong, not the piece.
    let voices = [];
    while (s.clock.cycle === startCycle && performance.now() - t0 < 90000) {
      await frame();
      frames++;
      const id = s.activeShotId();
      if (id && seen[seen.length - 1] !== id) seen.push(id);
      const v = s.voices();
      if (v.length > voices.length) voices = v;
    }
    const looped = s.clock.cycle > startCycle;

    // And the same reference frame again, after the wrap.
    s.setPlaying(false);
    s.seek(0);
    await frame();
    const after = grab();

    return {
      cutSec: s.show.durationSec,
      endless: s.clock.endless,
      looped,
      cycles: s.clock.cycle,
      framesRendered: frames,
      realSeconds: +((performance.now() - t0) / 1000).toFixed(1),
      reached: seen,
      voicesDuringCycle: voices,
      voicesAfterLoop: s.voices(),
      identicalAcrossLoop: before === after,
      width: w,
      height: h,
      beforePng: before,
      afterPng: after,
    };
  });

  // A difference image, and an amplitude — because "identical: false" is a
  // fact and not yet a diagnosis.
  //
  // Bit-equality is the wrong criterion and cannot be met: `post.ts`'s grade
  // pass dithers and grains from wall-clock elapsed time, on purpose, so two
  // renders of the same state differ by +-1 everywhere by design. What a
  // state leak looks like is different in kind, not degree: a vignette left at
  // 0.55 instead of 0.35, or a radial blur left running, moves whole regions
  // by tens of levels. Before the seek-blur fix this number was 76; the dither
  // floor is 2.
  if (!played.identicalAcrossLoop) {
    const aPng = rawToPng(played.beforePng, played.width, played.height);
    const bPng = rawToPng(played.afterPng, played.width, played.height);
    const a = PNG.sync.read(aPng);
    const b = PNG.sync.read(bPng);
    let differing = 0;
    let maxDelta = 0;
    for (let i = 0; i < a.data.length; i += 4) {
      const d = Math.max(
        Math.abs(a.data[i] - b.data[i]),
        Math.abs(a.data[i + 1] - b.data[i + 1]),
        Math.abs(a.data[i + 2] - b.data[i + 2]),
      );
      if (d > 0) differing++;
      if (d > maxDelta) maxDelta = d;
    }
    played.loopDiff = {
      differingPixels: differing,
      ofPixels: a.width * a.height,
      maxChannelDelta: maxDelta,
      /** The 8-bit dither floor. Above this, something is being carried
       * across the loop that should not be. */
      ditherFloor: DITHER_FLOOR,
      cleanWithinDither: maxDelta <= DITHER_FLOOR,
    };
    writeFileSync(`${outDir}/m66-loop-cycle0.png`, aPng);
    writeFileSync(`${outDir}/m66-loop-cycle1.png`, bPng);
  }
  delete played.beforePng;
  delete played.afterPng;

  report.playback = played;
  report.errors.push(...errors);
  await page.close();
}

// ----------------------------------------------------------------- L5: looks

{
  const { page, errors } = await open('?director=1');
  const marks = [
    ['s01-point', 3],
    ['s02-swarm', 12],
    ['s03-crossfade', 32],
    ['s04-assembly', 70],
    ['s05-monolith', 110],
    ['s06-dolly', 200],
  ];
  for (const [name, t] of marks) {
    await page.evaluate(async (sec) => {
      const s = window.__spexShow;
      s.setPlaying(false);
      s.seek(sec);
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    }, t);
    await page.waitForTimeout(250);
    await page.screenshot({ path: `${outDir}/m66-${name}.png` });
  }
  report.errors.push(...errors);
  await page.close();
}

// ---------------------------------------------------------------------- AC3

/** One parameter, one page load, one measured claim. */
const checks = [
  // Not `time === 30`: the show is playing, and `open` deliberately settles
  // for three seconds first, so by the time anything can be read the clock has
  // moved on. What `?t=` claims is that playback *started* at 30, and the
  // measurable form of that is "the clock is at or past 30 and has not
  // wrapped" — checked against the control run two rows down, which starts at
  // 0 and is still well under 30 at the same moment.
  ['?t=30', 'seek', (s) => ({ time: s.clock.time, cycle: s.clock.cycle }), (r) => r.time >= 30 && r.cycle === 0],
  ['', 'no seek (control)', (s) => ({ time: s.clock.time }), (r) => r.time < 30],
  ['?duration=120', 'cut', (s) => ({ dur: s.show.durationSec }), (r) => Math.abs(r.dur - 120) < 1e-6],
  ['?duration=endless', 'cut', (s) => ({ dur: s.show.durationSec, endless: s.show.endless }), (r) => r.endless === true],
  ['?duration=250', 'bad cut', (s) => ({ dur: s.show.durationSec, warnings: s.warnings }), (r) => r.warnings.some((w) => w.includes('250'))],
  ['?quality=low', 'quality', (s) => ({ tier: s.quality() }), (r) => r.tier === 'low'],
  ['?mute=1', 'mute', (s) => ({ src: s.clock.source }), (r) => r.src === 'performance'],
  ['', 'no mute (control)', (s) => ({ src: s.clock.source }), () => true],
  ['?free=1', 'free camera', (s) => ({ free: s.director.isFree, controls: s.controls.enabled }), (r) => r.free === true && r.controls === true],
  ['?loop=0', 'loop off', (s) => ({ endless: s.clock.endless, docEndless: s.show.endless }), (r) => r.endless === false],
  ['?duration=endless&loop=0', 'loop off beats the document', (s) => ({ endless: s.clock.endless, docEndless: s.show.endless }), (r) => r.docEndless === true && r.endless === false],
  ['?director=1', 'director HUD', null, null],
  ['?seed=99', 'seed', null, null],
];

for (const [query, name, probe, ok] of checks) {
  const { page, errors } = await open(query);
  let result;
  if (name === 'director HUD') {
    result = await page.evaluate(() => {
      const el = document.getElementById('show-director');
      return {
        visible: !!el && getComputedStyle(el).display !== 'none',
        chars: el ? el.textContent.length : 0,
        mentionsShot: !!el && /A1-S0\d/.test(el.textContent),
      };
    });
    result.pass = result.visible && result.chars > 60 && result.mentionsShot;
    await page.screenshot({ path: `${outDir}/m66-director.png` });
  } else if (name === 'seed') {
    // A seed is only real if it moves something. A1-S04's assembly is the one
    // generator in the act, so this compares where the same instance starts
    // under the document's own seed and under ?seed=99.
    result = await page.evaluate(async () => {
      const s = window.__spexShow;
      const shot = s.show.shots.find((x) => x.id === 'A1-S04');
      s.setPlaying(false);
      s.seek(shot.startSec + 0.2);
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const mono = s.scenes.find((x) => x.id === 'monolith');
      const g = mono.groups[0];
      return { seed: s.params.seed, x: g.matrices[12], y: g.matrices[13], z: g.matrices[14] };
    });
    const base = await (async () => {
      const { page: p2 } = await open('');
      const v = await p2.evaluate(async () => {
        const s = window.__spexShow;
        const shot = s.show.shots.find((x) => x.id === 'A1-S04');
        s.setPlaying(false);
        s.seek(shot.startSec + 0.2);
        await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
        const mono = s.scenes.find((x) => x.id === 'monolith');
        const g = mono.groups[0];
        return { seed: s.show.seed, x: g.matrices[12], y: g.matrices[13], z: g.matrices[14] };
      });
      await p2.close();
      return v;
    })();
    result.baseline = base;
    result.movedMm = Math.hypot(result.x - base.x, result.y - base.y, result.z - base.z);
    result.pass = result.movedMm > 0.5;
  } else {
    result = await page.evaluate(`(${probe.toString()})(window.__spexShow)`);
    result.pass = ok(result);
  }
  report.ac3.push({ query: query || '(none)', name, ...result, errors });
  report.errors.push(...errors);
  await page.close();
}

await browser.close();

writeFileSync(`${outDir}/m66-showrun.json`, JSON.stringify(report, null, 2));

const ac1 = report.ac1;
const pb = report.playback;
console.log('AC1 — the canonical cut, as arithmetic');
console.log(`  duration        ${ac1.durationSec.toFixed(3)} s; shots sum to ${ac1.sumOfShots.toFixed(3)} s; last shot ends ${ac1.lastEnd.toFixed(3)} s`);
console.log(`  beat-aligned    ${ac1.beatAligned}; contiguous ${ac1.contiguous}; uncovered samples ${ac1.uncoveredSamples.length}`);
console.log(`  shots reached   ${ac1.reached.join(' -> ')}  (${ac1.reached.length}/${ac1.shotCount})`);
if (ac1.warnings.length) console.log(`  warnings        ${ac1.warnings.join(' | ')}`);
console.log('AC1 — the endless cut, played');
console.log(`  played          ${pb.cutSec.toFixed(3)} s of show in ${pb.realSeconds} s real, ${pb.framesRendered} frames (${(pb.framesRendered / pb.realSeconds).toFixed(1)} fps)`);
console.log(`  shots reached   ${pb.reached.join(' -> ')}`);
console.log(`  voices fired    ${pb.voicesDuringCycle.join(' | ') || '—'}`);
console.log(`  looped          ${pb.looped} (endless=${pb.endless}, cycles=${pb.cycles}); voices after loop ${JSON.stringify(pb.voicesAfterLoop)}`);
console.log(`  clean loop      ${
  pb.identicalAcrossLoop
    ? 'frames bit-identical at t=0'
    : pb.loopDiff.cleanWithinDither
      ? `clean within the dither floor (max delta ${pb.loopDiff.maxChannelDelta} <= ${pb.loopDiff.ditherFloor}, ${pb.loopDiff.differingPixels}/${pb.loopDiff.ofPixels} px)`
      : `LEAKS: ${JSON.stringify(pb.loopDiff)}`
}`);
console.log(`  console errors  ${report.errors.length}`);
for (const e of report.errors.slice(0, 8)) console.log(`    ! ${e}`);
console.log('\nAC3 — one parameter at a time');
for (const c of report.ac3) {
  const { query, name, pass, errors: _e, ...rest } = c;
  console.log(`  ${pass ? 'ok  ' : 'FAIL'} ${query.padEnd(28)} ${name.padEnd(28)} ${JSON.stringify(rest)}`);
}

/** Raw RGBA rows come out of `readPixels` bottom-up; PNG wants top-down. */
function rawToPng(b64, width, height) {
  const raw = Buffer.from(b64, 'base64');
  const png = new PNG({ width, height });
  for (let y = 0; y < height; y++) {
    raw.copy(png.data, y * width * 4, (height - 1 - y) * width * 4, (height - y) * width * 4);
  }
  return PNG.sync.write(png);
}
