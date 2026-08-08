#!/usr/bin/env node
/** M65 — the dissolve, measured.
 *
 *   spex mesh-model ldraw-scenes/monolith.ldr -o /tmp/mono
 *   spex serve /tmp/mono --port 8103 --no-open &
 *   node scripts/viewer-shot/dissolve.mjs http://127.0.0.1:8103/ /tmp/m65
 *
 * Two questions:
 *
 *   AC1  is the disappearance smooth and noise-driven, with a visible rim?
 *   AC3  what does the dissolve shader cost?
 *
 * Both are read off real frames. "Smooth" is a statement about a sequence of
 * pixel counts; "a visible rim" is a statement about luminance that is *added*
 * partway through, which is exactly the kind of thing a person looking at one
 * screenshot cannot confirm and a difference image can.
 *
 * As everywhere in this harness, each frame is rendered twice — once with the
 * bricks and once without — and only the pixels that differ are counted. The
 * ground plane is most of the frame and it does not dissolve.
 */

import { chromium } from 'playwright';
import { watchConsole } from './absence.mjs';
import { PNG } from 'pngjs';
import { mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const url = process.argv[2];
const outDir = process.argv[3];
if (!url || !outDir) {
  console.error('usage: dissolve.mjs <viewer-url> <out-dir>');
  process.exit(2);
}
mkdirSync(outDir, { recursive: true });

/** Three seconds at 30 fps, which is what AC1 asks for. */
const FRAMES = 90;

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 640, height: 400 } });
// `errors` excludes the 404s that are the viewer's mode test answering "no" —
// see absence.mjs. This probe printed a clean set of numbers and then FAIL for
// two of them, on a demo nothing was wrong with.
const { errors, byDesign } = watchConsole(page);
await page.goto(url, { waitUntil: 'networkidle' });
await page.waitForFunction(() => !!window.__spexMesh, null, { timeout: 90000 });
await page.waitForTimeout(3500);

const run = await page.evaluate(async (frames) => {
  const m = window.__spexMesh;
  const post = m.post();
  const gl = m.renderer.getContext();
  const w = m.renderer.domElement.width;
  const h = m.renderer.domElement.height;
  const buf = new Uint8Array(w * h * 4);
  const ref = new Uint8Array(w * h * 4);

  const objects = [];
  m.scene.traverse((o) => {
    if ((o.isMesh && o.name !== 'ground') || o.isLine || o.isLineSegments) objects.push(o);
  });
  const setVisible = (v) => { for (const o of objects) o.visible = v; };

  const ids = [];
  for (const g of m.groups) for (const id of g.ids) ids.push(id);

  const b64 = (bytes) => {
    let s = '';
    for (let i = 0; i < bytes.length; i += 8192) s += String.fromCharCode.apply(null, bytes.subarray(i, i + 8192));
    return btoa(s);
  };

  const rows = [];
  const shots = {};
  const timings = [];

  for (let f = 0; f < frames; f++) {
    const amount = f / (frames - 1);
    for (const id of ids) m.writer.setDissolve(id, amount);
    m.writer.flush();
    m.lod()?.update(m.camera, h);
    m.edges.update(m.camera, h);

    setVisible(false);
    post.render(f / 30);
    gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, ref);
    setVisible(true);
    const t0 = performance.now();
    post.render(f / 30);
    gl.finish();
    timings.push(performance.now() - t0);
    gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, buf);

    let lit = 0;
    let sumDelta = 0;
    let brighter = 0;
    for (let p = 0; p < buf.length; p += 4) {
      const l = 0.2126 * buf[p] + 0.7152 * buf[p + 1] + 0.0722 * buf[p + 2];
      const lr = 0.2126 * ref[p] + 0.7152 * ref[p + 1] + 0.0722 * ref[p + 2];
      const d = l - lr;
      if (Math.abs(d) > 6) {
        lit++;
        sumDelta += d;
        // Pixels the object made *brighter* than the empty scene. A solid
        // black brick on a grey ground makes almost none; a lit rim does.
        if (d > 25) brighter++;
      }
    }
    rows.push({ f, amount, lit, meanDelta: lit ? sumDelta / lit : 0, brighter });
    if (f === 0 || f === 30 || f === 55 || f === frames - 1) shots[f] = { w, h, data: b64(buf) };
  }

  // Put it back, or every later check in this session sees a dissolved scene.
  for (const id of ids) m.writer.setDissolve(id, 0);
  m.writer.flush();
  m.lod()?.update(m.camera, h);

  const median = (xs) => [...xs].sort((a, b) => a - b)[xs.length >> 1];
  return {
    rows,
    shots,
    instances: ids.length,
    solidMs: median(timings.slice(0, 3)),
    midMs: median(timings.slice((frames >> 1) - 3, (frames >> 1) + 3)),
    allMs: median(timings),
  };
}, FRAMES);

await browser.close();

for (const [f, shot] of Object.entries(run.shots)) {
  const png = new PNG({ width: shot.w, height: shot.h });
  const raw = Buffer.from(shot.data, 'base64');
  for (let y = 0; y < shot.h; y++) {
    const src = (shot.h - 1 - y) * shot.w * 4;
    raw.copy(png.data, y * shot.w * 4, src, src + shot.w * 4);
  }
  const path = join(outDir, `dissolve-${String(f).padStart(3, '0')}.png`);
  writeFileSync(path, PNG.sync.write(png));
  console.log(`wrote ${path}`);
}

const rows = run.rows;
console.log(`\ndissolve — ${run.instances} instance(s), ${FRAMES} frames (3 s at 30 fps)`);
console.log('\n  frame  amount   lit px  meanDelta  brighter');
for (const r of rows) {
  if (r.f % 10 !== 0 && r.f !== FRAMES - 1) continue;
  console.log(
    `  ${String(r.f).padStart(5)}  ${r.amount.toFixed(3)} ${String(r.lit).padStart(8)} ` +
      `${r.meanDelta.toFixed(1).padStart(10)} ${String(r.brighter).padStart(9)}`,
  );
}

// Smoothness: the biggest single-frame share of the whole object to vanish at
// once. A hard cut would show up as one frame taking a large fraction.
const first = rows[0].lit;
let worstStep = 0;
let worstAt = 0;
for (let i = 1; i < rows.length; i++) {
  const step = Math.abs(rows[i].lit - rows[i - 1].lit) / Math.max(first, 1);
  if (step > worstStep) {
    worstStep = step;
    worstAt = i;
  }
}
const rimPeak = rows.reduce((a, r) => (r.brighter > a.brighter ? r : a), rows[0]);
const rimAtStart = rows[0].brighter;

console.log(`\nAC1 — smoothness and rim`);
console.log(`  lit pixels ${first} -> ${rows[rows.length - 1].lit}`);
console.log(`  largest single-frame change: ${(worstStep * 100).toFixed(2)} % of the object, at frame ${worstAt}`);
console.log(
  `  rim: ${rimAtStart} brighter-than-background pixels when solid, peaking at ${rimPeak.brighter} ` +
    `at frame ${rimPeak.f} (dissolve ${rimPeak.amount.toFixed(2)})`,
);

console.log(`\nAC3 — cost of the dissolve shader`);
console.log(`  median render, dissolve = 0.0 : ${run.solidMs.toFixed(2)} ms`);
console.log(`  median render, dissolve ~ 0.5 : ${run.midMs.toFixed(2)} ms`);
const delta = ((run.midMs - run.solidMs) / Math.max(run.solidMs, 1e-6)) * 100;
console.log(`  difference: ${delta >= 0 ? '+' : ''}${delta.toFixed(1)} %`);
console.log(
  `  read with care: this is a SOFTWARE rasteriser. \`discard\` is close to free on a GPU and\n` +
    `  is a branch on SwiftShader, so the sign of this number is meaningful and its size is not.`,
);

if (errors.length) {
  console.log('\nconsole errors:');
  for (const e of errors) console.log(`  ${e}`);
}
if (byDesign.length) {
  console.log(`\n${byDesign.length} 404(s) for files whose absence is how the viewer picks a mode — not errors.`);
}

const failed =
  errors.length > 0 ||
  rows[rows.length - 1].lit > first * 0.02 ||
  worstStep > 0.25 ||
  rimPeak.brighter <= rimAtStart;
console.log(`\n${failed ? 'FAIL' : 'ok'}`);
process.exit(failed ? 1 : 0);
