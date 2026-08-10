#!/usr/bin/env node
/** A watchable recording of a show, for people who are not sitting at the
 * machine that renders it.
 *
 *   spex show demos/matrix --port 8118 --no-open &
 *   node scripts/viewer-shot/showvideo.mjs http://127.0.0.1:8118/ /tmp/m66-video \
 *     [cut] [frames] [fps]
 *
 * Not a test. `showrun.mjs` measures and `showframes.mjs` documents; this is
 * the one that produces something to *watch*, which on a software rasteriser
 * running at two or three frames a second cannot be done by screen-recording
 * the thing live.
 *
 * So the show is paused and stepped: seek, render, capture, repeat. Show time
 * advances by a fixed step per captured frame, which means the result plays at
 * a rate this container's speed has no influence on — the recording is a
 * property of the piece, not of the machine. The trade is that anything
 * derived from *frame* time rather than show time (the dither, the materialise
 * flash decay) is stepped too.
 */
import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';

const url = process.argv[2];
const outDir = process.argv[3];
const cut = process.argv[4] ?? 'endless';
const FRAMES = Number(process.argv[5] ?? 300);
const FPS = Number(process.argv[6] ?? 15);
// Optional, because the cost of a frame on a software rasteriser is entirely
// its pixel count: a four-minute recording at 960x540 is hours and at 640x360
// is not. The recording's *timing* is unaffected — show time is stepped, so
// the only thing the viewport changes is how long the machine takes.
const WIDTH = Number(process.argv[7] ?? 960);
const HEIGHT = Number(process.argv[8] ?? 540);
const QUALITY = process.argv[9] ?? 'medium';
if (!url || !outDir) {
  console.error('usage: showvideo.mjs <viewer-url> <out-dir> [cut] [frames] [fps] [width] [height]');
  process.exit(2);
}
mkdirSync(outDir, { recursive: true });

/** Headless Chromium does **not** use the GPU by default — it falls back to
 * SwiftShader, which is a software rasteriser, and then a faster machine buys
 * nothing at all. That is worth stating because the symptom is indistinguishable
 * from "rendering is just slow": this script measured 0.4 frames a second on a
 * GPU-less container at 640x360 and 0.2 on a real workstation at 1920x1080 —
 * nine times the pixels for half the rate, i.e. the workstation was doing the
 * same software rasterising, only better.
 *
 * So: ask for the GPU, and then **print which renderer actually answered**, so
 * nobody has to infer it from a frame rate. `SPEX_HEADED=1` runs a visible
 * window, which on macOS is the most reliable way to get hardware
 * acceleration when the headless path refuses.
 */
const HEADED = !!process.env.SPEX_HEADED;
const browser = await chromium.launch({
  headless: !HEADED,
  args: [
    '--enable-gpu',
    '--ignore-gpu-blocklist',
    '--enable-webgl',
    '--use-angle=default',
  ],
});
const page = await browser.newPage({ viewport: { width: WIDTH, height: HEIGHT } });
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto(`${url}?duration=${cut}&quality=${QUALITY}`, { waitUntil: 'networkidle' });
await page.waitForFunction(() => !!window.__spexShow, null, { timeout: 120000 });
// M71 put a gate in front of the piece — no browser will start an
// `AudioContext` without a gesture, and the piece is four voices. `begin()`
// is exactly what the button calls, so a harness that uses it is screening
// through the same door rather than around it. Without this line every frame
// below would be a photograph of the title card.
await page.evaluate(() => window.__spexShow.begin());
await page.waitForTimeout(3000);

// Which rasteriser is actually doing the work. `UNMASKED_RENDERER_WEBGL` is
// what the driver calls itself; "SwiftShader" or "llvmpipe" in that string
// means every frame below is being drawn on the CPU.
const renderer = await page.evaluate(() => {
  const gl = document.createElement('canvas').getContext('webgl2')
    || document.createElement('canvas').getContext('webgl');
  if (!gl) return 'no WebGL context at all';
  const ext = gl.getExtension('WEBGL_debug_renderer_info');
  return ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER);
});
const software = /swiftshader|llvmpipe|software/i.test(renderer);
console.log(`renderer: ${renderer}${software ? '  <-- SOFTWARE. The GPU is not being used; try SPEX_HEADED=1.' : ''}`);

const durationSec = await page.evaluate(() => {
  const s = window.__spexShow;
  s.setPlaying(false);
  return s.show.durationSec;
});
const step = durationSec / (FRAMES - 1);
console.log(`${cut}: ${durationSec.toFixed(3)} s in ${FRAMES} frames (${step.toFixed(3)} s/frame), played at ${FPS} fps = ${(FRAMES / FPS).toFixed(1)} s of video, ${(durationSec / (FRAMES / FPS)).toFixed(2)}x real time`);

/** PNG or JPEG — and this is **not** where the time goes. Measured, because
 * the guess was wrong.
 *
 * The guess was that a 1920x1080 screenshot is two million pixels being
 * PNG-compressed on the CPU, and that JPEG would therefore be much faster.
 * Measured on this project's GPU-less container at 1920x1080: **22.4 s per
 * capture as JPEG, 22.7 s as PNG.** Within noise of each other. The encoder
 * is not the cost — `Page.captureScreenshot` makes the browser *produce a
 * fresh frame* and hand it over, so a capture is a second render, and on a
 * software rasteriser that is the whole bill.
 *
 * So `SPEX_JPEG=1` buys **disk, not time**: 7200 frames at 1080p is 6-10 GB
 * as PNG and under one as JPEG, and they are h264-encoded immediately
 * afterwards anyway, so quality 92 loses nothing that survives the mux.
 * Default stays PNG, the honest archival format.
 *
 * If the time is what you want back, `showlive.mjs` is the answer: it records
 * the piece playing instead of photographing it frame by frame, and on a
 * machine with a real GPU that is four minutes rather than an hour.
 */
const JPEG = !!process.env.SPEX_JPEG;
const shotOpts = JPEG ? { type: 'jpeg', quality: 92 } : { type: 'png' };
const ext = JPEG ? 'jpg' : 'png';

const t0 = Date.now();
// Where the time actually goes, split at the seam that matters: driving the
// show and waiting for frames, against getting the pixels out. Guessing which
// half dominates is how an hour gets spent on the wrong one.
let msDrive = 0;
let msShot = 0;
for (let i = 0; i < FRAMES; i++) {
  const a = Date.now();
  await page.evaluate(async (sec) => {
    const s = window.__spexShow;
    s.seek(sec);
    // Two full frames: the LOD selector and the edge gate both decide from the
    // camera the previous frame left behind.
    for (let k = 0; k < 2; k++) {
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    }
  }, i * step);
  const b = Date.now();
  await page.screenshot({ path: `${outDir}/f${String(i).padStart(4, '0')}.${ext}`, ...shotOpts });
  const c = Date.now();
  msDrive += b - a;
  msShot += c - b;
  if (i % 50 === 0) {
    const rate = (i + 1) / ((Date.now() - t0) / 1000);
    const n = i + 1;
    console.log(
      `  ${i}/${FRAMES}  ${rate.toFixed(1)} frames/s  eta ${((FRAMES - i) / rate / 60).toFixed(1)} min` +
        `  [render+settle ${(msDrive / n).toFixed(0)} ms, capture ${(msShot / n).toFixed(0)} ms/frame]`,
    );
  }
}
console.log(
  `captured ${FRAMES} ${ext.toUpperCase()} frames in ${((Date.now() - t0) / 1000 / 60).toFixed(1)} min ` +
    `(render+settle ${(msDrive / FRAMES).toFixed(0)} ms, capture ${(msShot / FRAMES).toFixed(0)} ms per frame); ` +
    `console errors ${errors.length}`,
);
await browser.close();
