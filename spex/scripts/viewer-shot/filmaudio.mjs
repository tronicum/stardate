#!/usr/bin/env node
/** The score, rendered to a WAV the length of a cut — the soundtrack half of
 * a watchable recording.
 *
 *   node scripts/viewer-shot/filmaudio.mjs demos/matrix/fugue.mid /tmp/film 240
 *
 * `showvideo.mjs` steps show time and captures frames, which is the only way
 * to record a piece on a software rasteriser that renders at four frames a
 * second. This is its counterpart: the same score, through the same
 * `AudioEngine`, rendered offline at whatever rate the machine manages and
 * written at 44.1 kHz. Both are properties of the piece rather than of the
 * container, which is why they line up when ffmpeg puts them together.
 *
 * The engine is the shipped one — no fast attack, no single voice, no tap
 * before the mastering chain. Every substitution the *measuring* harnesses
 * make is a substitution for measurement, and this is for listening.
 */
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const midPath = process.argv[2] ?? 'demos/matrix/fugue.mid';
const outDir = process.argv[3] ?? '/tmp/film';
const seconds = Number(process.argv[4] ?? 240);
mkdirSync(outDir, { recursive: true });

execFileSync(
  join(here, 'node_modules/.bin/esbuild'),
  [join(here, 'audio-entry.ts'), '--bundle', '--format=iife', `--outfile=${outDir}/audio-entry.js`],
  { stdio: 'inherit' },
);

const midB64 = readFileSync(midPath).toString('base64');
const browser = await chromium.launch();
const page = await browser.newPage();
page.on('pageerror', (e) => console.log('ERR', String(e)));
await page.goto('about:blank');
await page.addScriptTag({ path: `${outDir}/audio-entry.js` });

const result = await page.evaluate(
  async ({ b64, seconds }) => {
    const A = window.__spexAudio;
    const bin = atob(b64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    const score = A.parseSmf(bytes.buffer);

    const SR = 44100;
    const ctx = new OfflineAudioContext(2, Math.ceil(SR * seconds), SR);
    const engine = new A.AudioEngine(ctx);
    // The harness drives the clock: an OfflineAudioContext has no wall clock,
    // and `setInterval` would never fire a single note inside one.
    const clock = { time: 0, playing: true };
    const sched = new A.Scheduler(engine, score, clock, { cues: A.cuesFromScore(score) });
    sched.seek(0, 0);
    for (let t = 0; t <= seconds; t += A.TICK_MS / 1000) {
      clock.time = t;
      sched.pump(t, t);
    }
    const rendered = await ctx.startRendering();
    const analysis = A.analyse(rendered);

    const L = rendered.getChannelData(0);
    const R = rendered.getChannelData(1);
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
    let s = '';
    const u8 = new Uint8Array(buf);
    for (let i = 0; i < u8.length; i += 8192) s += String.fromCharCode.apply(null, u8.subarray(i, i + 8192));
    return {
      wav: btoa(s),
      notes: score.notes.length,
      scheduled: sched.scheduled,
      markers: score.markers.length,
      peak: +analysis.peak.toFixed(4),
      rms: +analysis.rms.toFixed(4),
      clipped: analysis.clipped,
    };
  },
  { b64: midB64, seconds },
);

writeFileSync(`${outDir}/film.wav`, Buffer.from(result.wav, 'base64'));
delete result.wav;
console.log(`wrote ${outDir}/film.wav — ${seconds} s, ${result.scheduled} of ${result.notes} notes, ` +
  `${result.markers} markers, peak ${result.peak}, rms ${result.rms}, clipped ${result.clipped}`);
await browser.close();
