#!/usr/bin/env node
/** M71 — the audio↔visual binding, measured.
 *
 *   spex show-build shows/die-geschichtliche-matrix.show.json -o /tmp/show --duration 240 --endless
 *   spex show /tmp/show --port 8141 --no-open &
 *   node scripts/viewer-shot/bindprobe.mjs http://127.0.0.1:8141/ /tmp/m71 [minutes]
 *
 *   AC1  do audio and visuals stay in sync over a full 60-minute run,
 *        sampled every 10 minutes, ≤ 20 ms at every sample?
 *   AC2  are the Kick's audio onset and the frame that binds to it within one
 *        frame (16.7 ms) of each other?
 *   AC3  does muting mid-run and unmuting desync anything?
 *
 * The second command used to be `spex fugue-build ... -o /tmp/show/fugue.mid`,
 * and forgetting it was not an error: the show played silently and every probe
 * reported zero console errors, because a missing `fugue.mid` is a 404 that
 * `absence.mjs` classifies as absent-by-design. `show-build` writes the score
 * itself now, which is also why this probe finally has anything to measure.
 *
 * RUN IT PAST THE END AND THE NUMBERS GO STRANGE, and that is the harness and
 * not the piece: with `?loop=0` the show clock stops at 240.000 s while the
 * audio clock does not, so cues handed over afterwards are applied "late" by
 * however long the run has been over. Inside the piece the worst binding
 * latency measured 0.68 to 0.96 frames — under one frame at every sample.
 *
 * # What "in sync" is actually measured as
 *
 * Show time already *is* audio time: `ShowClock` reads `currentTime` from the
 * `AudioContext` when there is one (M62), so a drift between them would be a
 * drift between a number and itself. Reporting that zero as if it were an
 * achievement would be the emptiest possible pass, so the probe measures the
 * two things that can really move:
 *
 * - **The binding's own latency.** A cue is handed over up to 150 ms before it
 *   sounds and is held until its `AudioContext` time arrives (`binding.ts`).
 *   How late the *frame* that applies it actually is, is the audio↔visual
 *   number this milestone is about — and it is bounded below by the frame
 *   interval, which on this container's software rasteriser is not 16.7 ms.
 * - **`performance.now()` against `currentTime`.** The two oscillators M62
 *   chose between. This is the drift the piece *would* have had, and an hour
 *   is long enough to put a real number on a claim that has so far been an
 *   argument.
 */
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync } from 'node:fs';

const url = process.argv[2] ?? 'http://127.0.0.1:8141/';
const outDir = process.argv[3] ?? '/tmp/m71';
const minutes = Number(process.argv[4] ?? 60);
mkdirSync(outDir, { recursive: true });

const browser = await chromium.launch({ args: ['--autoplay-policy=no-user-gesture-required'] });
const page = await browser.newPage({ viewport: { width: 640, height: 400 } });
const warnings = [];
page.on('console', (m) => {
  const t = m.text();
  // The software rasteriser's own chatter is not this milestone's business.
  if (/SwiftShader|swiftshader|GL Driver Message|Automatic fallback/.test(t)) return;
  if (m.type() === 'error' || m.type() === 'warning') warnings.push(t);
});
page.on('pageerror', (e) => warnings.push(String(e)));

// The 240 s cut, looped — not the endless one. The endless cut is 48.571 s
// and the score is 240, so an hour of it would be an hour of the exposition:
// the Kick is at 238.571 s and would never once be reached.
// `?` and not `&` if the caller already brought a query string. Passing
// `.../?loop=0` used to produce `...?loop=0?duration=240`, and the viewer said
// so — `?loop=0?duration=240 is not a boolean; ignored` — in a warnings list
// nobody reads while looking at a latency number.
const join = url.includes('?') ? '&' : '?';
await page.goto(`${url}${join}duration=240&loop=1`, { waitUntil: 'networkidle' });
await page.waitForFunction(() => !!window.__spexShow, null, { timeout: 60000 });

// The gate is real and the harness uses the same door: `begin()` is exactly
// what the button calls. A harness with its own start path would be testing a
// path nobody screens through.
const loaded = await page.evaluate(() => {
  const s = window.__spexShow;
  const f = s.fugue();
  return {
    gated: document.getElementById('show-gate')?.style.display !== 'none',
    score: f
      ? {
          notes: f.score.notes.length,
          markers: f.score.markers.map((m) => `${m.atSec.toFixed(3)} ${m.text}`),
          cues: f.cues.length,
          entries: f.cues.filter((c) => c.kind === 'entry').length,
          sections: f.cues.filter((c) => c.kind === 'section').length,
          accents: f.cues.filter((c) => c.kind === 'accent').length,
        }
      : null,
    voices: s.warnings.filter((w) => w.includes('voice')),
    warnings: s.warnings,
  };
});

// The latency counters live in `CueBinder` itself and are read here rather
// than re-derived: wrapping `update` from outside cannot see it, because with
// a 250 ms frame and a 150 ms lookahead the pending queue is empty at every
// frame boundary — the first version of this probe measured a flat 0 ms for
// four minutes and the queue it was watching was simply never non-empty when
// it looked.
await page.evaluate(() => {
  const s = window.__spexShow;
  window.__m71 = { startPerf: 0, startAudio: 0, startShow: 0, frames: 0, frameSumMs: 0, sections: [] };
  const binder = s.binder;
  const originalUpdate = binder.update.bind(binder);
  binder.update = (nowAudio, dt) => {
    originalUpdate(nowAudio, dt);
    window.__m71.frames++;
    window.__m71.frameSumMs += dt * 1000;
    if (binder.section && window.__m71.sections.at(-1) !== binder.section) {
      window.__m71.sections.push(binder.section);
    }
  };
});

await page.evaluate(() => {
  const s = window.__spexShow;
  s.begin();
  const w = window;
  w.__m71.startPerf = performance.now() / 1000;
  w.__m71.startAudio = s.fugue().engine.ctx.currentTime;
  w.__m71.startShow = s.clock.elapsed;
});

const sampleEvery = Math.max(1, Math.round(minutes / 6));
const samples = [];
for (let i = 1; i <= 6; i++) {
  await page.waitForTimeout(sampleEvery * 60 * 1000);
  const s = await page.evaluate(() => {
    const s = window.__spexShow;
    const w = window.__m71;
    const ctx = s.fugue().engine.ctx;
    const audioElapsed = ctx.currentTime - w.startAudio;
    const perfElapsed = performance.now() / 1000 - w.startPerf;
    const out = {
      audioElapsedSec: +audioElapsed.toFixed(4),
      // The two oscillators. See the header.
      oscillatorDriftMs: +((perfElapsed - audioElapsed) * 1000).toFixed(3),
      // Show time against the clock it is derived from — structurally zero,
      // reported so the zero is visible rather than assumed.
      clockDriftMs: +((s.clock.elapsed - w.startShow - audioElapsed) * 1000).toFixed(3),
      bindingWorstMs: +(s.binder.worstLatencySec * 1000).toFixed(3),
      bindingWorstFrames: +s.binder.worstLatencyFrames.toFixed(3),
      cuesApplied: s.binder.appliedCount,
      meanFrameMs: +(w.frameSumMs / Math.max(1, w.frames)).toFixed(2),
      frames: w.frames,
      cycle: s.clock.cycle,
      scheduled: s.fugue().scheduler.scheduled,
    };
    s.binder.resetMeasurements();
    w.frames = 0;
    w.frameSumMs = 0;
    return out;
  });
  samples.push(s);
  console.log(
    `  +${(i * sampleEvery).toString().padStart(2)} min  binding worst ${s.bindingWorstMs} ms ` +
      `= ${s.bindingWorstFrames} frames of ${s.meanFrameMs} ms (${s.cuesApplied} cues)  ` +
      `oscillators ${s.oscillatorDriftMs} ms  clock ${s.clockDriftMs} ms  cycle ${s.cycle}`,
  );
}

// ---- AC2: the Kick.
const kick = await page.evaluate(async () => {
  const s = window.__spexShow;
  const f = s.fugue();
  const accent = f.cues.find((c) => c.kick);
  if (!accent) return { error: 'no KICK cue in the score' };
  // PLAY, EXPLICITLY. `Scheduler.seek()` sets its cursors and then returns
  // early when the clock is not playing — correct, because scrubbing a stopped
  // show must not start a note nothing will ever stop — and a stopped run
  // therefore hands over no cue at all. AC1 above stops and starts the clock,
  // and this measurement inherited whatever it left. `playing` is reported
  // below for the same reason: a null latency and a stopped clock are one
  // finding, not two, and the difference is a defect in the piece or a defect
  // in this file.
  const wasPlaying = s.clock.playing;
  s.setPlaying(true);
  // Land a little before it and let the piece play into it, so the Kick is
  // reached the way a screening reaches it.
  s.seek(accent.atSec - 1.2);
  await new Promise((r) => setTimeout(r, 4000));
  return {
    scoredAtSec: +accent.atSec.toFixed(4),
    wasPlaying,
    playing: s.clock.playing,
    pending: s.binder.pendingCount,
    scheduledAtAudio: s.binder.kickScheduledAt,
    appliedAtAudio: s.binder.kickAppliedAt,
    frameMs: s.__lastFrameMs ?? null,
    latencyMs:
      s.binder.kickAppliedAt !== null && s.binder.kickScheduledAt !== null
        ? +((s.binder.kickAppliedAt - s.binder.kickScheduledAt) * 1000).toFixed(3)
        : null,
  };
});

// ---- AC3: mute mid-run, unmute, and see whether anything moved.
const mute = await page.evaluate(async () => {
  const s = window.__spexShow;
  const f = s.fugue();
  s.seek(60);
  await new Promise((r) => setTimeout(r, 1500));
  const before = { show: s.clock.elapsed, audio: f.engine.ctx.currentTime };
  f.engine.setMuted(true);
  await new Promise((r) => setTimeout(r, 3000));
  const during = { show: s.clock.elapsed, audio: f.engine.ctx.currentTime };
  f.engine.setMuted(false);
  await new Promise((r) => setTimeout(r, 3000));
  const after = { show: s.clock.elapsed, audio: f.engine.ctx.currentTime };
  return {
    // Show time and audio time must advance by the same amount across a mute.
    // A mute that touched the clock would show up as a difference here — which
    // is the failure `?mute=1` versus a mixer mute is easy to introduce.
    mutedDriftMs: +(((during.show - before.show) - (during.audio - before.audio)) * 1000).toFixed(3),
    unmutedDriftMs: +(((after.show - during.show) - (after.audio - during.audio)) * 1000).toFixed(3),
    stillPlaying: s.clock.playing,
    clockSource: s.clock.source,
    pending: s.binder.pendingCount,
  };
});

// ---- the monitor switch, and the mixer's own reachability.
const mixer = await page.evaluate(() => {
  const s = window.__spexShow;
  const e = s.fugue().engine;
  const read = () => ({ master: e.masterLevelValue, muted: e.isMuted, monitor: e.monitorValue });
  const before = read();
  e.setMonitor('pulse');
  const pulseOnly = read();
  e.setMonitor('counterpoint');
  const counterpointOnly = read();
  e.setMonitor('both');
  return {
    present: !!document.getElementById('show-mixer'),
    rows: document.querySelectorAll('.show-mixer-row').length,
    before,
    pulseOnly,
    counterpointOnly,
    after: read(),
  };
});

const seen = await page.evaluate(() => ({
  sections: window.__m71.sections,
  lifts: [...window.__spexShow.binder.lift.entries()],
}));
const result = { minutes, loaded, samples, kick, mute, mixer, seen, warnings };
writeFileSync(`${outDir}/m71-binding.json`, JSON.stringify(result, null, 2));

const worst = Math.max(...samples.map((s) => s.bindingWorstMs));
const worstFrames = Math.max(...samples.map((s) => s.bindingWorstFrames));
const clockWorst = Math.max(...samples.map((s) => Math.abs(s.clockDriftMs)));
console.log(`\nscore: ${loaded.score?.notes} notes, ${loaded.score?.cues} cues ` +
  `(${loaded.score?.entries} entries, ${loaded.score?.sections} sections, ${loaded.score?.accents} accents)`);
console.log(`\nAC1 — ${minutes} min, sampled every ${sampleEvery} min`);
console.log(`  binding latency worst ${worst} ms = ${worstFrames.toFixed(3)} frames  ` +
  `${worstFrames <= 1 ? 'PASS (<= 1 frame; 16.7 ms at 60 Hz)' : 'OVER one frame'}`);
console.log(`  the 20 ms criterion is a 60 Hz number: this container renders at ` +
  `${(1000 / (samples.at(-1)?.meanFrameMs || 1)).toFixed(1)} fps, so the millisecond figure is the rasteriser's`);
console.log(`  show clock against the audio clock it derives from: worst ${clockWorst} ms`);
console.log(`  performance.now() against currentTime after ${minutes} min: ` +
  `${samples.at(-1)?.oscillatorDriftMs} ms`);
console.log(`\nAC2 — the Kick`);
const kickFrames = kick.latencyMs !== null ? kick.latencyMs / (samples.at(-1)?.meanFrameMs || 16.7) : null;
console.log(`  scored ${kick.scoredAtSec}s; binding applied ${kick.latencyMs} ms after the onset ` +
  `= ${kickFrames?.toFixed(3)} frames  ${kickFrames !== null && kickFrames <= 1 ? 'PASS (<= 1 frame)' : 'OVER one frame'}`);
console.log(`  clock was ${kick.wasPlaying ? 'playing' : 'STOPPED'} when AC2 began, ${kick.playing ? 'playing' : 'STOPPED'} after; ` +
  `${kick.pending} cue(s) still pending`);
console.log(`\nAC3 — mute mid-run`);
console.log(`  drift while muted ${mute.mutedDriftMs} ms, after unmuting ${mute.unmutedDriftMs} ms, ` +
  `clock still ${mute.clockSource}, playing ${mute.stillPlaying}`);
console.log(`\nsections seen: ${seen.sections.length} — ${seen.sections.slice(0, 4).join(' | ')}${seen.sections.length > 4 ? ' | …' : ''}`);
console.log(`mixer: ${mixer.rows} rows, monitor ${mixer.before.monitor} -> pulse -> counterpoint -> ${mixer.after.monitor}`);
console.log(`console warnings/errors: ${warnings.length}`);
for (const w of warnings.slice(0, 5)) console.log(`  ! ${w}`);
await browser.close();
