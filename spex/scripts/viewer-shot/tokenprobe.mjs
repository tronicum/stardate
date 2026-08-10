#!/usr/bin/env node
/** A4-S03's tokens: do they move, do they stay on the lattice, and does a seek
 * land them where playing there would have?
 *
 *   node scripts/viewer-shot/tokenprobe.mjs http://127.0.0.1:8220/
 *
 * Three claims, three measurements. "They move" is the one the shot exists for
 * and the one a still frame cannot show — a screenshot of a token flow and a
 * screenshot of forty-eight bricks lying on a grid are the same picture.
 *
 * The positions are read off `group.matrices` — the authoritative transforms
 * the writer flushed and the GPU is given — rather than recomputed from
 * `TokenFlow.positionAt`, which would be testing the generator against itself
 * and would pass even if `player.ts` never called it. Note `matrices` and not
 * a LOD mesh's `instanceMatrix`: since M59 those are re-packed copies whose
 * row order changes with the level, so "the matrix of instance i" only lives
 * in one place.
 */
import { chromium } from 'playwright';
import { attachConsole } from './absence.mjs';

const url = process.argv[2];
if (!url) { console.error('usage: tokenprobe.mjs <viewer-url>'); process.exit(2); }

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 640, height: 400 } });
const errors = [];
attachConsole(page, errors);
await page.goto(url, { waitUntil: 'networkidle' });
await page.waitForFunction(() => !!window.__spexShow, null, { timeout: 120000 });
// The same door the audience uses — M71 put a gesture gate in front of the
// piece, and a probe that goes around it measures the title card.
await page.evaluate(() => window.__spexShow.begin());
await page.waitForTimeout(3000);

/** Every token's world position at one show time, in bundle order. */
async function sample(sec) {
  return page.evaluate(async (t) => {
    const s = window.__spexShow;
    s.setPlaying(false);
    s.seek(t);
    // Three frames, not one: a cue fires on the frame after the seek, the
    // generator runs after the tracks, and the writer flushes after that.
    for (let i = 0; i < 3; i++) {
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    }
    const scene = s.scenes.find((x) => x.id === 'tokenStrom');
    if (!scene) return { error: 'no tokenStrom scene' };
    if (!scene.root.visible) return { error: `tokenStrom not visible at ${t}s` };
    const byId = new Map();
    for (const grp of scene.groups) {
      for (let i = 0; i < grp.ids.length; i++) {
        byId.set(grp.ids[i], [
          grp.matrices[i * 16 + 12],
          grp.matrices[i * 16 + 13],
          grp.matrices[i * 16 + 14],
        ]);
      }
    }
    // Bundle order, so index i is the same token in every sample.
    const pos = scene.instanceIds.map((id) => byId.get(id) ?? null);
    return {
      shot: s.activeShotId(),
      generator: scene.assembly ? scene.assemblyShot : null,
      pos,
    };
  }, sec);
}

const A = await sample(222);
const B = await sample(226);
const A2 = await sample(222);

for (const [label, r] of [['t=222', A], ['t=226', B]]) {
  if (r.error) { console.log(`FAIL: ${label}: ${r.error}`); await browser.close(); process.exit(1); }
  if (!r.pos.length || r.pos.some((p) => !p)) {
    console.log(`FAIL: ${label}: ${r.pos.length} instance(s), some without a matrix`);
    await browser.close();
    process.exit(1);
  }
}

let moved = 0;
let maxStep = 0;
let arc = 0;
for (let i = 0; i < A.pos.length; i++) {
  const d = Math.hypot(
    B.pos[i][0] - A.pos[i][0],
    B.pos[i][1] - A.pos[i][1],
    B.pos[i][2] - A.pos[i][2],
  );
  if (d > 1) moved++;
  if (d > maxStep) maxStep = d;
}

// The lattice spans x 10804..11316, z -252..260 mm; a token must stay on it,
// and its arc must stay within a few millimetres of the plane at y=12.
let outside = 0;
for (const p of A.pos.concat(B.pos)) {
  if (p[0] < 10790 || p[0] > 11330 || p[2] < -266 || p[2] > 274) outside++;
  if (p[1] - 12 > arc) arc = p[1] - 12;
}

let drift = 0;
for (let i = 0; i < A.pos.length; i++) {
  drift = Math.max(drift, Math.hypot(
    A2.pos[i][0] - A.pos[i][0],
    A2.pos[i][1] - A.pos[i][1],
    A2.pos[i][2] - A.pos[i][2],
  ));
}

console.log(`shot              ${A.shot}   generator from ${A.generator}`);
console.log(`tokens            ${A.pos.length}`);
console.log(`moved in 4 s      ${moved} of ${A.pos.length}   (largest step ${maxStep.toFixed(1)} mm)`);
console.log(`outside lattice   ${outside}`);
console.log(`highest arc       ${arc.toFixed(2)} mm above the plane`);
console.log(`seek drift        ${drift.toFixed(6)} mm   (same time, twice)`);
console.log(`console errors    ${errors.length}`);
for (const e of errors.slice(0, 5)) console.log(`  ! ${e}`);
const ok =
  moved >= A.pos.length * 0.8 &&
  outside === 0 &&
  arc > 0.5 &&
  drift < 1e-6 &&
  errors.length === 0;
console.log(ok ? 'PASS' : 'FAIL');
await browser.close();
process.exit(ok ? 0 : 1);
