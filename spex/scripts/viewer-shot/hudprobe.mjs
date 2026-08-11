#!/usr/bin/env node
/** Does every word on screen belong to the shot it is on top of?
 *
 *   node scripts/viewer-shot/hudprobe.mjs http://127.0.0.1:8250/
 *
 * A1-S05 raises `monolith-metrics` — "1 : 4 : 9.20 — 73.6 mm — 9 real parts" —
 * and its own track fades the line back out, 0 -> 1 -> 1 -> 0 across the shot.
 * The line was nevertheless on screen over Uruk, over the coin, over Rome and
 * over both patents, right through to the last act, for two independent
 * reasons: a track only writes while its own shot is live, so the closing key
 * at t=1 is never the last thing written; and a seek re-applies `hud` cues by
 * kind, because they are state, without re-running the track that would take
 * them down.
 *
 * It was found by measuring the lower-right corner of 27 documentation frames,
 * not by looking at them — a hairline of type at the edge of frame is exactly
 * what an eye stops seeing after the third picture. So this is a measurement
 * and not a screenshot: it reads the elements' own computed opacity at every
 * shot boundary and asserts that each is lit only inside the shots that
 * address it.
 */
import { chromium } from 'playwright';
import { attachConsole } from './absence.mjs';

const url = process.argv[2];
if (!url) { console.error('usage: hudprobe.mjs <viewer-url>'); process.exit(2); }

/** element -> the shot ids allowed to have it lit. `seed-point` is addressed
 * by three shots an act and a half apart, which is why the runtime tracks
 * ownership rather than clearing everything at every boundary. */
const OWNERS = {
  'monolith-metrics': ['A1-S05'],
  'seed-point': ['A1-S01', 'A1-S02', 'A4-KICK'],
};

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 640, height: 400 } });
const errors = [];
attachConsole(page, errors);
await page.goto(url, { waitUntil: 'networkidle' });
await page.waitForFunction(() => !!window.__spexShow, null, { timeout: 120000 });
await page.evaluate(() => window.__spexShow.begin());
await page.waitForTimeout(3000);

const shots = await page.evaluate(() => window.__spexShow.show.shots.map((s) => ({
  id: s.id, start: s.startSec, end: s.endSec,
})));

/** Two samples per shot — a fifth in and four fifths in — because the defect
 * this exists for is a value left behind at a boundary, and sampling only the
 * middle of a shot would miss a line that lights up late and stays. */
const marks = [];
for (const s of shots) {
  marks.push([s.id, s.start + 0.2 * (s.end - s.start)]);
  marks.push([s.id, s.start + 0.8 * (s.end - s.start)]);
}

const rows = [];
for (const [shotId, t] of marks) {
  const lit = await page.evaluate(async ({ sec, names }) => {
    const s = window.__spexShow;
    s.setPlaying(false);
    s.seek(sec);
    for (let i = 0; i < 3; i++) await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const out = {};
    for (const n of names) {
      // `hud.ts`'s `el()` sets an **id**, not a class — the one thing that made
      // the first run of this probe report "lit in 0 of its own samples" and
      // still PASS, which is the shape of a test that measures nothing.
      const ids = { 'monolith-metrics': 'show-metrics', 'seed-point': 'show-seed-point', caption: 'show-caption' };
      const el = document.getElementById(ids[n] ?? n) ?? document.querySelector(`[data-element="${n}"]`);
      out[n] = el ? Number(getComputedStyle(el).opacity) : null;
    }
    return { shot: s.activeShotId(), values: out };
  }, { sec: t, names: Object.keys(OWNERS) });
  rows.push({ shotId, t, ...lit });
}

let strays = 0;
for (const r of rows) {
  for (const [name, allowed] of Object.entries(OWNERS)) {
    const v = r.values[name];
    if (v === null) continue;
    const owned = allowed.includes(r.shot ?? r.shotId);
    if (!owned && v > 0.01) {
      strays++;
      console.log(`  ! ${name} at ${v.toFixed(3)} during ${r.shot} (t=${r.t.toFixed(1)}s) — owned by ${allowed.join(', ')}`);
    }
  }
}

for (const [name, allowed] of Object.entries(OWNERS)) {
  const inside = rows.filter((r) => allowed.includes(r.shot ?? r.shotId) && (r.values[name] ?? 0) > 0.01).length;
  console.log(`${name.padEnd(18)} lit in ${inside} of its own samples, ${strays === 0 ? 'nowhere else' : 'and elsewhere'}`);
}
console.log(`samples            ${rows.length} (two per shot)`);
console.log(`strays             ${strays}`);
console.log(`console errors     ${errors.length}`);
// A probe that finds nothing lit anywhere has not proved that nothing strays;
// it has proved that it cannot see. Require both halves.
const litSomewhere = Object.entries(OWNERS).some(([name, allowed]) =>
  rows.some((r) => allowed.includes(r.shot ?? r.shotId) && (r.values[name] ?? 0) > 0.01));
console.log(`sees the HUD       ${litSomewhere ? 'yes' : 'NO — the selectors are wrong'}`);
const ok = strays === 0 && litSomewhere && errors.length === 0;
console.log(ok ? 'PASS' : 'FAIL');
await browser.close();
process.exit(ok ? 0 : 1);
