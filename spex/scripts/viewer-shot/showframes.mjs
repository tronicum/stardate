#!/usr/bin/env node
/** M66, ladder rung 5 — the pictures.
 *
 *   node scripts/viewer-shot/showframes.mjs http://127.0.0.1:8116/ /tmp/m66
 *
 * Separate from `showrun.mjs` because the two want opposite things from the
 * same page. The measurements want `?director=1`, and the HUD it draws covers
 * most of the frame — which is right for an instrument and useless as a record
 * of what the piece looks like. So the frames are shot clean, and the HUD gets
 * exactly one picture of its own.
 *
 * The show is paused and seeked rather than played to each mark: at three
 * frames a second, playing to 3:20 takes three minutes and lands somewhere
 * else. Seeking while paused is legible now that the camera follows a seek
 * (see `CameraDirector.follow`) — before that it produced six pictures of the
 * right shot from the wrong camera, which is exactly the sort of frame that
 * looks fine until someone checks it against the screenplay.
 */
import { chromium } from 'playwright';
import { attachConsole } from './absence.mjs';
import { mkdirSync } from 'node:fs';

const url = process.argv[2];
const outDir = process.argv[3];
if (!url || !outDir) {
  console.error('usage: showframes.mjs <viewer-url> <out-dir>');
  process.exit(2);
}
mkdirSync(outDir, { recursive: true });

const MARKS = [
  ['s01-point', 2.8],
  ['s02-swarm', 15],
  ['s03-crossfade', 45],
  ['s04-assembly', 62],
  ['s04-landed', 95],
  ['s05-monolith', 130],
  ['s06-dolly', 200],
];

const browser = await chromium.launch();
const errors = [];

async function shoot(query, marks, prefix) {
  const page = await browser.newPage({ viewport: { width: 960, height: 600 } });
  // The 404s that are the viewer's mode test answering "no" are not errors
  // (absence.mjs). Phase 3's rung 6 surfaced it: probes printed clean numbers
  // and then FAIL, on demos nothing was wrong with.
  const byDesign = attachConsole(page, errors);
  await page.goto(url + query, { waitUntil: 'networkidle' });
  await page.waitForFunction(() => !!window.__spexShow, null, { timeout: 120000 });
  // M71 put a gate in front of the piece — no browser will start an
  // `AudioContext` without a gesture, and the piece is four voices. `begin()`
  // is exactly what the button calls, so a harness that uses it is screening
  // through the same door rather than around it. Without this line every frame
  // below would be a photograph of the title card.
  await page.evaluate(() => window.__spexShow.begin());
  await page.waitForTimeout(3000);
  for (const [name, t] of marks) {
    const info = await page.evaluate(async (sec) => {
      const s = window.__spexShow;
      s.setPlaying(false);
      s.seek(sec);
      // Three frames, not one: the LOD selector and the edge gate both decide
      // from the camera the *previous* frame left behind, so a single frame
      // after a jump can be drawn at the wrong level.
      for (let i = 0; i < 3; i++) {
        await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      }
      return {
        shot: s.activeShotId(),
        scenes: s.visibleScenes(),
        cam: s.camera.position.toArray().map((v) => +v.toFixed(1)),
        draws: s.drawCalls(),
      };
    }, t);
    await page.waitForTimeout(200);
    await page.screenshot({ path: `${outDir}/${prefix}${name}.png` });
    console.log(`${prefix}${name}  t=${t}s  ${info.shot}  scenes[${info.scenes.join(',')}]  cam ${info.cam}  ${info.draws} draws`);
  }
  await page.close();
}

await shoot('', MARKS, 'm66-');
await shoot('?director=1', [['director', 62]], 'm66-');
await browser.close();
console.log(`console errors: ${errors.length}`);
for (const e of errors.slice(0, 5)) console.log(`  ! ${e}`);
