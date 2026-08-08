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

const browser = await chromium.launch();
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

const durationSec = await page.evaluate(() => {
  const s = window.__spexShow;
  s.setPlaying(false);
  return s.show.durationSec;
});
const step = durationSec / (FRAMES - 1);
console.log(`${cut}: ${durationSec.toFixed(3)} s in ${FRAMES} frames (${step.toFixed(3)} s/frame), played at ${FPS} fps = ${(FRAMES / FPS).toFixed(1)} s of video, ${(durationSec / (FRAMES / FPS)).toFixed(2)}x real time`);

const t0 = Date.now();
for (let i = 0; i < FRAMES; i++) {
  await page.evaluate(async (sec) => {
    const s = window.__spexShow;
    s.seek(sec);
    // Two full frames: the LOD selector and the edge gate both decide from the
    // camera the previous frame left behind.
    for (let k = 0; k < 2; k++) {
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    }
  }, i * step);
  await page.screenshot({ path: `${outDir}/f${String(i).padStart(4, '0')}.png` });
  if (i % 50 === 0) {
    const rate = (i + 1) / ((Date.now() - t0) / 1000);
    console.log(`  ${i}/${FRAMES}  ${rate.toFixed(1)} frames/s  eta ${((FRAMES - i) / rate / 60).toFixed(1)} min`);
  }
}
console.log(`captured ${FRAMES} frames in ${((Date.now() - t0) / 1000 / 60).toFixed(1)} min; console errors ${errors.length}`);
await browser.close();
