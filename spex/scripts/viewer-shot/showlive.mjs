#!/usr/bin/env node
/** A recording made by *watching* the show, rather than by stepping it.
 *
 *   spex show demos/matrix --port 8200 --no-open &
 *   node scripts/viewer-shot/showlive.mjs http://127.0.0.1:8200/ /tmp/filmframes \
 *     [cut] [fps] [width] [height] [quality]
 *
 * `showvideo.mjs` pauses the piece and photographs it one frame at a time.
 * That was the only way to record it on a software rasteriser, and its
 * timing is a property of the piece rather than of the machine — but it pays
 * a full page capture per frame, and a page capture is not a cheap read of
 * the last framebuffer: `Page.captureScreenshot` makes the browser *produce a
 * new frame* and hand it over. Measured on this project's GPU-less container
 * at 1920x1080: 22.4 s per capture as JPEG and 22.7 s as PNG. Identical. The
 * encoder was never the cost; the re-render was.
 *
 * So on a machine whose GPU can actually draw the piece at rate — and the
 * viewer measured 80 fps on one — the honest thing is to stop photographing
 * it and simply record what it puts on screen. `Page.startScreencast` is the
 * browser's own compositor pushing finished frames out as they are presented:
 * no per-frame round trip, no second render, and the DOM half of the picture
 * (title cards, caption, credits crawl, mixer) is included, which a
 * `canvas.captureStream()` would have silently dropped.
 *
 * # The trade, stated rather than hidden
 *
 * Stepped capture guarantees **every** frame and no wall clock touches it.
 * This guarantees **the timing** and drops frames if the machine cannot keep
 * up: frames arrive when they arrive, each with the compositor's own
 * timestamp, and are written to a concat list with real durations. A slow
 * machine therefore yields a judder-free but frame-starved recording rather
 * than a slow-motion one — and the run prints how many frames it actually
 * got against how many it asked for, so nobody has to infer it.
 *
 * Sound is unchanged: `filmaudio.mjs` renders the same score offline and
 * ffmpeg puts the two together.
 */
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync, createWriteStream } from 'node:fs';

const url = process.argv[2];
const outDir = process.argv[3];
const cut = process.argv[4] ?? 'endless';
const FPS = Number(process.argv[5] ?? 30);
const WIDTH = Number(process.argv[6] ?? 1920);
const HEIGHT = Number(process.argv[7] ?? 1080);
const QUALITY = process.argv[8] ?? 'high';
const JPEG_Q = Number(process.env.SPEX_JPEG_Q ?? 92);
if (!url || !outDir) {
  console.error('usage: showlive.mjs <viewer-url> <out-dir> [cut] [fps] [width] [height] [quality]');
  process.exit(2);
}
mkdirSync(`${outDir}/raw`, { recursive: true });

const HEADED = !!process.env.SPEX_HEADED;
const browser = await chromium.launch({
  headless: !HEADED,
  args: ['--enable-gpu', '--ignore-gpu-blocklist', '--enable-webgl', '--use-angle=default'],
});
const page = await browser.newPage({ viewport: { width: WIDTH, height: HEIGHT } });
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto(`${url}?duration=${cut}&quality=${QUALITY}`, { waitUntil: 'networkidle' });
await page.waitForFunction(() => !!window.__spexShow, null, { timeout: 120000 });
await page.evaluate(() => window.__spexShow.begin());
await page.waitForTimeout(3000);

const renderer = await page.evaluate(() => {
  const gl = document.createElement('canvas').getContext('webgl2')
    || document.createElement('canvas').getContext('webgl');
  if (!gl) return 'no WebGL context at all';
  const ext = gl.getExtension('WEBGL_debug_renderer_info');
  return ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER);
});
const software = /swiftshader|llvmpipe|software/i.test(renderer);
console.log(`renderer: ${renderer}${software ? '  <-- SOFTWARE.' : ''}`);
if (software) {
  console.log('  A software rasteriser cannot draw this at rate. This script will record what it manages');
  console.log('  and say so; `showvideo.mjs` is the one that gets every frame on such a machine.');
}

const durationSec = await page.evaluate(() => window.__spexShow.show.durationSec);
const expected = Math.round(durationSec * FPS);
console.log(`${cut}: ${durationSec.toFixed(3)} s at ${FPS} fps = ${expected} frames, recorded in real time`);

// Frames arrive from the compositor and are written as they come. Holding a
// four-minute 1080p recording in memory to reorder it afterwards would be
// about a gigabyte and a half of base64 for no gain.
const cdp = await page.context().newCDPSession(page);
const frames = []; // { ts, file }
let firstTs = null;
let dropped = 0;

cdp.on('Page.screencastFrame', async ({ data, metadata, sessionId }) => {
  try {
    if (firstTs === null) firstTs = metadata.timestamp;
    const idx = frames.length;
    const file = `raw/${String(idx).padStart(5, '0')}.jpg`;
    writeFileSync(`${outDir}/${file}`, Buffer.from(data, 'base64'));
    frames.push({ ts: metadata.timestamp - firstTs, file });
  } catch {
    dropped++;
  }
  // Unacknowledged frames stop the stream, so this has to happen even on a
  // write error — an unacked screencast goes quiet and looks like a hang.
  try { await cdp.send('Page.screencastFrameAck', { sessionId }); } catch { /* closed */ }
});

await page.evaluate(() => {
  const s = window.__spexShow;
  s.seek(0);
  s.setPlaying(true);
});
await cdp.send('Page.startScreencast', {
  format: 'jpeg',
  quality: JPEG_Q,
  maxWidth: WIDTH,
  maxHeight: HEIGHT,
  everyNthFrame: 1,
});

const t0 = Date.now();
let lastReport = 0;
while (Date.now() - t0 < durationSec * 1000) {
  await page.waitForTimeout(500);
  const el = (Date.now() - t0) / 1000;
  if (el - lastReport >= 15) {
    lastReport = el;
    console.log(
      `  ${el.toFixed(0)}/${durationSec.toFixed(0)} s  ${frames.length} frames  ` +
        `${(frames.length / el).toFixed(1)} frames/s  (asking ${FPS})`,
    );
  }
}
await cdp.send('Page.stopScreencast');
await page.evaluate(() => window.__spexShow.setPlaying(false));
await page.waitForTimeout(500);

/** ffmpeg's concat demuxer, with each frame's real on-screen duration.
 *
 * This is what makes a variable arrival rate into a correct constant-rate
 * video: `-r <fps>` on the output resamples it, duplicating a frame that was
 * held and dropping one that was superseded, and the *timing* stays what the
 * compositor measured. Writing f0000.jpg .. f7199.jpg by duplication instead
 * would say the same thing in ten times the disk.
 */
const list = createWriteStream(`${outDir}/frames.ffconcat`);
list.write('ffconcat version 1.0\n');
for (let i = 0; i < frames.length; i++) {
  const next = i + 1 < frames.length ? frames[i + 1].ts : durationSec;
  const dur = Math.max(1 / (FPS * 4), next - frames[i].ts);
  list.write(`file ${frames[i].file}\nduration ${dur.toFixed(6)}\n`);
}
// The concat demuxer ignores the last entry's duration unless the file is
// named once more after it.
if (frames.length) list.write(`file ${frames[frames.length - 1].file}\n`);
await new Promise((r) => list.end(r));

const wall = (Date.now() - t0) / 1000;
const rate = frames.length / wall;
console.log(
  `recorded ${frames.length} frames in ${wall.toFixed(1)} s = ${rate.toFixed(1)} frames/s ` +
    `(${((frames.length / expected) * 100).toFixed(0)} % of ${expected} asked for)` +
    (dropped ? `, ${dropped} write failure(s)` : '') +
    `; console errors ${errors.length}`,
);
if (rate < FPS * 0.9) {
  console.log(`  Below the asked rate: the mux will hold frames rather than invent them. Use showvideo.mjs`);
  console.log(`  if every frame matters more than the wall-clock hour.`);
}
console.log(`\nmux:\n  ffmpeg -y -f concat -safe 0 -i ${outDir}/frames.ffconcat -i /tmp/film/film.wav \\\n` +
  `    -r ${FPS} -c:v libx264 -pix_fmt yuv420p -crf 18 -c:a aac -b:a 192k -shortest out.mp4`);
writeFileSync(`${outDir}/recording.json`, JSON.stringify({
  url, cut, fps: FPS, width: WIDTH, height: HEIGHT, quality: QUALITY,
  renderer, software, durationSec, expected, captured: frames.length,
  measuredFps: +rate.toFixed(2), consoleErrors: errors.length,
}, null, 2));
await browser.close();
