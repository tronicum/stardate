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
/** Optional substring filter over the mark names. Documenting a 16-shot piece
 * takes minutes on a software rasteriser, and re-checking one act should not
 * cost the other two. */
const only = process.argv[4] ?? '';
if (!url || !outDir) {
  console.error('usage: showframes.mjs <viewer-url> <out-dir> [name-filter]');
  process.exit(2);
}
mkdirSync(outDir, { recursive: true });

/** One frame per authored moment, at the seconds the 4:00 cut puts them at.
 *
 * These are timestamps into the canonical 240 s resolution and they move
 * whenever `baseDurationBars` does — Act II pushed it from 17 to 37, Act III
 * to 57 and Act IV to 84, and each time every number below changed. That is
 * the cost of documenting a piece by wall-clock second rather than by bar,
 * and it is paid here rather than in the document, which stays in bars.
 */
const MARKS = [
  ['a1s01-point', 2.8],
  ['a1s02-swarm', 10],
  ['a1s03-crossfade', 24],
  ['a1s04-assembly', 38],
  ['a1s05-monolith', 55],
  ['a1s06-stonehenge', 72],
  ['a2s01-uruk', 80],
  ['a2s02-bulla', 91.6],
  ['a2s03-tokens', 103],
  ['a2s04-sardis', 114],
  ['a2s05-face', 123],
  ['a3s01-rom-weit', 128],
  ['a3s01-stempel', 132],
  ['a3s02-batima', 138],
  ['a3s03-kiddicraft', 150],
  ['a3s04-saeule', 161.2],
  ['a3s04b-klemme', 164],
  ['a3s05-feld', 178],
  ['a4s01-inkpour', 194],
  ['a4s01b-letzterstein', 198.2],
  ['a4s01b-entfaerbt', 201.3],
  ['a4s02-punkte', 207],
  ['a4s02-gitter', 219],
  ['a4s03-tokens', 224],
  ['a4s03-tokens-spaet', 229],
  ['a4s04-saettigung', 236],
  ['a4kick-ende', 239.95],
  ['loop-erster', 0.05],
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
  for (const [name, t] of marks.filter(([n]) => n.includes(only))) {
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
