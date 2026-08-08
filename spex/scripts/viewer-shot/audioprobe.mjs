#!/usr/bin/env node
/** M69 — the audio graph, measured.
 *
 *   node scripts/viewer-shot/audioprobe.mjs /tmp/m69
 *
 * Three questions:
 *
 *   AC1  does the graph build with no console warnings?
 *   AC2  over a real render: does anything clip, and does the level stay
 *        inside a 6 dB band?
 *   AC3  what does it cost?
 *
 * All three are asked of an `OfflineAudioContext`, which renders faster than
 * real time into a buffer — so "does this clip" is arithmetic over samples
 * rather than someone listening and saying it seemed fine. That is the whole
 * reason `AudioEngine` takes a `BaseAudioContext` instead of an
 * `AudioContext`.
 *
 * The passage is deliberately the *worst case* the piece contains: a stretto,
 * four voices entering on top of each other with the pulse under them. If
 * anything clips, it clips there.
 */
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const outDir = process.argv[2] ?? '/tmp/m69';
mkdirSync(outDir, { recursive: true });

// Bundle the engine for the browser.
execFileSync(
  join(here, 'node_modules/.bin/esbuild'),
  [join(here, 'audio-entry.ts'), '--bundle', '--format=iife', `--outfile=${outDir}/audio-entry.js`],
  { stdio: 'inherit' },
);

const browser = await chromium.launch();
const page = await browser.newPage();
const warnings = [];
page.on('console', (m) => {
  if (m.type() === 'warning' || m.type() === 'error') warnings.push(`${m.type()}: ${m.text()}`);
});
page.on('pageerror', (e) => warnings.push(`pageerror: ${String(e)}`));
await page.goto('about:blank');
await page.addScriptTag({ path: `${outDir}/audio-entry.js` });

const result = await page.evaluate(async () => {
  const A = window.__spexAudio;
  const SR = 44100;
  const SECONDS = 60;

  // AC1, part one: does the graph build at all, and what is in it?
  const probe = new OfflineAudioContext(2, SR, SR);
  const probeEngine = new A.AudioEngine(probe);
  const built = {
    spaces: [...probeEngine.reverb.spaces],
    eqBands: probeEngine.eq.length,
    limiter: {
      threshold: probeEngine.limiter.threshold.value,
      ratio: probeEngine.limiter.ratio.value,
    },
    partials: A.PARTIALS.length,
    voicePan: [...A.VOICE_PAN],
  };

  // The impulse responses, on their own: seeded means reproducible, and that
  // is checkable rather than assertable.
  const irA = A.makeImpulseResponse(probe, A.SPACES.cathedral, 42);
  const irB = A.makeImpulseResponse(probe, A.SPACES.cathedral, 42);
  const irC = A.makeImpulseResponse(probe, A.SPACES.cathedral, 43);
  const same = (x, y) => {
    const a = x.getChannelData(0), b = y.getChannelData(0);
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
    return true;
  };
  const irStereoDecorrelated = !same(
    { getChannelData: () => irA.getChannelData(0) },
    { getChannelData: () => irA.getChannelData(1) },
  );
  const ir = {
    seededReproducible: same(irA, irB),
    differentSeedDiffers: !same(irA, irC),
    stereoDecorrelated: irStereoDecorrelated,
    lengths: Object.fromEntries(
      Object.entries(A.SPACES).map(([k, v]) => [k, +(v.seconds).toFixed(2)]),
    ),
  };

  // AC2: the worst case the piece contains — a stretto, four voices piling in,
  // with the pulse under them.
  const ctx = new OfflineAudioContext(2, SR * SECONDS, SR);
  const engine = new A.AudioEngine(ctx);
  const t0 = performance.now();

  const BPM = 84;
  const beat = 60 / BPM;
  // D Dorian, four voices, entries a bar apart then half a bar apart — the
  // texture thickens exactly the way the real stretto does.
  const SUBJECT = [
    [4, -1, 1], [0, 0, 1], [3, 0, 0.5], [2, 0, 0.5],
    [1, 0, 1], [5, 0, 2], [4, 0, 1], [0, 0, 1],
  ];
  const DORIAN = [0, 2, 3, 5, 7, 9, 10];
  const toMidi = (deg, oct) => 62 + DORIAN[((deg % 7) + 7) % 7] + 12 * (Math.floor(deg / 7) + oct);
  const OCTAVE = [12, 0, -12, -24];
  let entries = 0;
  for (let round = 0; round < 6; round++) {
    for (let v = 0; v < 4; v++) {
      const start = round * 8 * beat + v * 2 * beat;
      if (start > SECONDS - 10) continue;
      entries++;
      let t = start;
      for (const [deg, oct, beats] of SUBJECT) {
        const midi = toMidi(deg, oct) + OCTAVE[v];
        engine.noteOn(v, midi, t, 0.85);
        engine.noteOff(v, midi, t + beats * beat * 0.96);
        t += beats * beat;
      }
    }
  }
  // The pulse, from halfway, at double time — Act IV's texture.
  let pulses = 0;
  for (let t = SECONDS / 2; t < SECONDS - 2; t += beat / 2) {
    engine.kick(t, 0.9);
    engine.hat(t + beat / 4, 0.3);
    pulses++;
  }
  engine.setSpace('gated', SECONDS / 2, 3);
  engine.finalAccent(SECONDS - 1.5);

  const scheduleMs = performance.now() - t0;

  // A second render, with the texture held constant: four voices, no pulse,
  // no entries piling in. AC2's "RMS within a 6 dB band" is a claim about
  // level *stability*, and the passage above deliberately grows from one voice
  // to four plus percussion — which is +6 dB of arithmetic before any music.
  // Measuring only that would report the exposition's own crescendo as a
  // fault. This is the control.
  const steadyCtx = new OfflineAudioContext(2, SR * 20, SR);
  const steady = new A.AudioEngine(steadyCtx);
  for (let bar = 0; bar < 20 / (4 * beat); bar++) {
    for (let v = 0; v < 4; v++) {
      let t = bar * 4 * beat;
      for (const [deg, oct, beats] of SUBJECT) {
        if (t > 19) break;
        const midi = toMidi(deg, oct) + OCTAVE[v];
        steady.noteOn(v, midi, t, 0.85);
        steady.noteOff(v, midi, t + beats * beat * 0.96);
        t += beats * beat;
      }
    }
  }
  const steadyStats = A.analyse(await steadyCtx.startRendering(), 1.0);
  const r0 = performance.now();
  const rendered = await ctx.startRendering();
  const renderMs = performance.now() - r0;

  const stats = A.analyse(rendered, 1.0);
  // The saturation curve is monotonic and bounded, which is what makes it a
  // soft clip rather than a fold.
  const curve = A.saturationCurve(2.2, 64);
  let monotonic = true;
  for (let i = 1; i < curve.length; i++) if (curve[i] < curve[i - 1]) monotonic = false;

  return {
    built,
    ir,
    entries,
    pulses,
    scheduleMs: +scheduleMs.toFixed(1),
    renderMs: +renderMs.toFixed(1),
    realtimeRatio: +(SECONDS / (renderMs / 1000)).toFixed(1),
    seconds: SECONDS,
    peak: +stats.peak.toFixed(5),
    rms: +stats.rms.toFixed(5),
    clipped: stats.clipped,
    windows: stats.windows,
    dynamicRangeDb: +stats.dynamicRangeDb.toFixed(2),
    loudestWindowRms: +stats.loudestWindowRms.toFixed(4),
    quietestWindowRms: +stats.quietestWindowRms.toFixed(4),
    steady: {
      peak: +steadyStats.peak.toFixed(5),
      clipped: steadyStats.clipped,
      windows: steadyStats.windows,
      dynamicRangeDb: +steadyStats.dynamicRangeDb.toFixed(2),
    },
    saturationMonotonic: monotonic,
    saturationBounded: Math.max(...curve.map(Math.abs)) <= 1.0000001,
    // Hand the samples back so the harness can write a WAV a person can play.
    pcm: Array.from(rendered.getChannelData(0).filter((_, i) => i % 1 === 0)).length,
    wav: (() => {
      const L = rendered.getChannelData(0), R = rendered.getChannelData(1);
      const n = L.length;
      const buf = new ArrayBuffer(44 + n * 4);
      const view = new DataView(buf);
      const str = (o, s) => { for (let i = 0; i < s.length; i++) view.setUint8(o + i, s.charCodeAt(i)); };
      str(0, 'RIFF'); view.setUint32(4, 36 + n * 4, true); str(8, 'WAVE');
      str(12, 'fmt '); view.setUint32(16, 16, true); view.setUint16(20, 1, true);
      view.setUint16(22, 2, true); view.setUint32(24, SR, true);
      view.setUint32(28, SR * 4, true); view.setUint16(32, 4, true); view.setUint16(34, 16, true);
      str(36, 'data'); view.setUint32(40, n * 4, true);
      for (let i = 0; i < n; i++) {
        view.setInt16(44 + i * 4, Math.max(-1, Math.min(1, L[i])) * 32767, true);
        view.setInt16(46 + i * 4, Math.max(-1, Math.min(1, R[i])) * 32767, true);
      }
      let bin = '';
      const bytes = new Uint8Array(buf);
      for (let i = 0; i < bytes.length; i += 8192) {
        bin += String.fromCharCode.apply(null, bytes.subarray(i, i + 8192));
      }
      return btoa(bin);
    })(),
  };
});

writeFileSync(`${outDir}/m69-engine.wav`, Buffer.from(result.wav, 'base64'));
delete result.wav;
delete result.pcm;
writeFileSync(`${outDir}/m69-audio.json`, JSON.stringify({ ...result, warnings }, null, 2));

console.log('AC1 — the graph builds');
console.log(`  spaces          ${result.built.spaces.join(', ')}`);
console.log(`  eq bands        ${result.built.eqBands}; limiter ${result.built.limiter.threshold} dB @ ${result.built.limiter.ratio}:1`);
console.log(`  partials        ${result.built.partials} per voice; pan ${result.built.voicePan.join(' ')}`);
console.log(`  console warnings/errors  ${warnings.length}`);
for (const w of warnings.slice(0, 5)) console.log(`    ! ${w}`);
console.log('\n  impulse responses');
console.log(`    seeded reproducible   ${result.ir.seededReproducible}`);
console.log(`    different seed differs ${result.ir.differentSeedDiffers}`);
console.log(`    stereo decorrelated   ${result.ir.stereoDecorrelated}`);
console.log(`    lengths (s)           ${JSON.stringify(result.ir.lengths)}`);

console.log(`\nAC2 — ${result.seconds} s rendered offline (${result.entries} subject entries, ${result.pulses} pulses)`);
console.log(`  peak            ${result.peak}  ${result.clipped ? 'CLIPPED' : '(no sample at full scale)'}`);
console.log(`  overall rms     ${result.rms}`);
console.log(`  level band      ${result.dynamicRangeDb} dB across ${result.windows} one-second windows (loudest ${result.loudestWindowRms}, quietest ${result.quietestWindowRms})`);
console.log(`  saturation      monotonic ${result.saturationMonotonic}, bounded ${result.saturationBounded}`);
console.log(`  control (constant texture, 4 voices, no pulse)`);
console.log(`    peak          ${result.steady.peak}  ${result.steady.clipped ? 'CLIPPED' : 'clean'}`);
console.log(`    level band    ${result.steady.dynamicRangeDb} dB across ${result.steady.windows} windows`);

console.log(`\nAC3 — cost`);
console.log(`  scheduling      ${result.scheduleMs} ms for the whole passage`);
console.log(`  render          ${result.renderMs} ms for ${result.seconds} s = ${result.realtimeRatio}x real time`);
console.log(`\nwrote ${outDir}/m69-engine.wav and m69-audio.json`);

await browser.close();
