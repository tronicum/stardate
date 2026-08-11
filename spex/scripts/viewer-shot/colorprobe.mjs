#!/usr/bin/env node
/** A4-S01b: does the colour actually drain out of the last brick?
 *
 *   node scripts/viewer-shot/colorprobe.mjs http://127.0.0.1:8080/
 *
 * The screenplay's sentence is "held still, no camera move, while the colour
 * drains out of it". Every word of that is measurable and each one is a
 * different measurement, so this makes four:
 *
 *   1. THE BRICK'S HUE GOES AWAY. Read off the bound material, not off the
 *      document — the document is what was asked for and the material is what
 *      the renderer was given. Saturation at the start against saturation at
 *      the drain's end.
 *   2. ITS LIGHT DOES NOT. Draining to black would also pass (1), and would be
 *      the brick dying rather than the brick losing its colour. Rec. 709
 *      luminance of the two, which must agree to a per-mille.
 *   3. THE CAMERA DOES NOT MOVE. The shot's whole claim.
 *   4. IT COMES BACK. A colour track writes only while its shot is live, so
 *      the second time round the loop the brick must open terracotta again —
 *      which is exactly the class of defect `resetSharedState` exists for, and
 *      the reason it needed a line for `color`.
 */
import { chromium } from 'playwright';
import { attachConsole } from './absence.mjs';

const url = process.argv[2];
if (!url) { console.error('usage: colorprobe.mjs <viewer-url>'); process.exit(2); }

/** A4-S01b: 197.857 .. 203.571 s. The drain is authored over the first 0.6. */
const START = 198.2;
const DRAINED = 197.857 + 0.6 * 5.714;

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 480, height: 320 } });
const errors = [];
attachConsole(page, errors);
await page.goto(url, { waitUntil: 'networkidle' });
await page.waitForFunction(() => !!window.__spexShow, null, { timeout: 120000 });
await page.evaluate(() => window.__spexShow.begin());
await page.waitForTimeout(3000);

async function sample(sec) {
  return page.evaluate(async (t) => {
    const s = window.__spexShow;
    s.setPlaying(false);
    s.seek(t);
    for (let i = 0; i < 3; i++) {
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    }
    const scene = s.scenes.find((x) => x.id === 'letzterStein');
    if (!scene) return { error: 'no letzterStein scene' };
    const mat = scene.materials.get(0);
    return {
      shot: s.activeShotId(),
      rgb: [mat.color.r, mat.color.g, mat.color.b],
      cam: s.camera.position.toArray(),
    };
  }, sec);
}

const lum = (c) => 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
const sat = (c) => {
  const hi = Math.max(c[0], c[1], c[2]);
  const lo = Math.min(c[0], c[1], c[2]);
  return hi <= 0 ? 0 : (hi - lo) / hi;
};
const fmt = (c) => `[${c.map((v) => v.toFixed(4)).join(', ')}]`;

const a = await sample(START);
const b = await sample(DRAINED);
// Leave the shot entirely and come back, which is what a loop does.
await sample(20);
const again = await sample(START);

for (const [label, r] of [['start', a], ['drained', b], ['again', again]]) {
  if (r.error) { console.log(`FAIL: ${label}: ${r.error}`); await browser.close(); process.exit(1); }
}

const camMove = Math.hypot(b.cam[0] - a.cam[0], b.cam[1] - a.cam[1], b.cam[2] - a.cam[2]);
const returned = Math.hypot(
  again.rgb[0] - a.rgb[0],
  again.rgb[1] - a.rgb[1],
  again.rgb[2] - a.rgb[2],
);

console.log(`shot                ${a.shot}`);
console.log(`colour at ${START}s     ${fmt(a.rgb)}  sat ${sat(a.rgb).toFixed(3)}  lum ${lum(a.rgb).toFixed(4)}`);
console.log(`colour at ${DRAINED.toFixed(3)}s   ${fmt(b.rgb)}  sat ${sat(b.rgb).toFixed(3)}  lum ${lum(b.rgb).toFixed(4)}`);
console.log(`luminance kept      ${(100 * (1 - Math.abs(lum(b.rgb) - lum(a.rgb)) / lum(a.rgb))).toFixed(2)} %`);
console.log(`camera moved        ${camMove.toFixed(4)} mm`);
console.log(`returns on re-entry ${returned.toFixed(6)} off the opening colour`);
console.log(`console errors      ${errors.length}`);
for (const e of errors.slice(0, 5)) console.log(`  ! ${e}`);

const ok =
  sat(a.rgb) > 0.5 &&
  sat(b.rgb) < 0.02 &&
  Math.abs(lum(b.rgb) - lum(a.rgb)) / lum(a.rgb) < 0.01 &&
  camMove < 1e-6 &&
  returned < 1e-6 &&
  errors.length === 0;
console.log(ok ? 'PASS' : 'FAIL');
await browser.close();
process.exit(ok ? 0 : 1);
