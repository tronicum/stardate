/** M59 AC2: pull the camera back over 60 frames and prove the LOD transitions
 * are invisible.
 *
 * "Invisible" is made a number: no single frame may differ from its
 * predecessor in mean luminance by more than 3 %. A level switch that shows
 * up as a pop is a luminance step, and this catches it whether it comes from
 * the studs vanishing, from an outline switching off, or from the box
 * replacing a brick.
 *
 * Prints the LOD population per frame alongside, so the frames where a switch
 * actually happened are visible in the log — a run where nothing ever
 * switched would pass trivially and prove nothing.
 */
import { chromium } from 'playwright';
import { PNG } from 'pngjs';
import { readFileSync } from 'node:fs';

const [, , url, prefix] = process.argv;
const FRAMES = 60;
const browser = await chromium.launch({ args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: 320, height: 240 } });
const errors = [];
page.on('pageerror', (e) => errors.push(e.message));
await page.goto(url, { waitUntil: 'networkidle' });
await page.waitForTimeout(6000);
await page.evaluate(() => { window.__spexMesh.renderer.shadowMap.enabled = false;
  window.__spexMesh.scene.traverse((o) => { for (const m of [].concat(o.material ?? [])) if (m) m.needsUpdate = true; }); });

const meanLuma = (file) => {
  const png = PNG.sync.read(readFileSync(file));
  let sum = 0;
  for (let y = 75; y < png.height; y++) {
    for (let x = 0; x < png.width; x++) {
      const i = (png.width * y + x) * 4;
      sum += 0.2126 * png.data[i] + 0.7152 * png.data[i + 1] + 0.0722 * png.data[i + 2];
    }
  }
  return sum / (png.width * (png.height - 75));
};

const rows = [];
for (let f = 0; f < FRAMES; f++) {
  // Geometric dolly: each frame is 1.09x further out, so 60 frames span
  // roughly 1x to 170x and cross every LOD boundary at least once.
  const scale = f === 0 ? 1 : 1.09;
  await page.evaluate(`(() => {
    const M = window.__spexMesh, c = M.camera, t = M.controls.target;
    // The offset is computed into a variable *before* the copy. Written as
    // one chain — c.position.copy(t).add(c.position.clone()...) — JavaScript
    // runs copy(t) first and only then evaluates the argument, so the clone
    // is of the target and every frame parks the camera exactly on it. The
    // dolly then went nowhere and the whole test passed while proving
    // nothing, which is how it was found.
    const d = c.position.clone().sub(t).multiplyScalar(${scale});
    c.position.copy(t).add(d);
    M.controls.update();
  })()`);
  await page.waitForTimeout(60);
  const file = `${prefix}-dolly-${String(f).padStart(2, '0')}.png`;
  await page.screenshot({ path: file });
  const lod = await page.evaluate(() => window.__spexMesh.lod()?.stats.perLevel ?? null);

  // The same camera, with every instance pinned to full detail. The
  // difference between the two is the LOD error and nothing else — the shot
  // is identical, so the dolly's own luminance drift cancels out.
  await page.evaluate(() => window.__spexMesh.lod()?.forceLevel(0));
  await page.waitForTimeout(60);
  const ref = `${prefix}-ref-${String(f).padStart(2, '0')}.png`;
  await page.screenshot({ path: ref });
  await page.evaluate(() => window.__spexMesh.lod()?.forceLevel(null));

  rows.push({ f, luma: meanLuma(file), full: meanLuma(ref), lod });
}

let worst = 0;
let worstAt = -1;
for (const r of rows) {
  r.err = r.full > 0.5 ? Math.abs(r.luma - r.full) / r.full : 0;
  if (r.err > worst) { worst = r.err; worstAt = r.f; }
}
for (const r of rows) {
  if (r.f % 5 === 0 || r.f === worstAt) {
    console.log(
      `  frame ${String(r.f).padStart(2)}  luma ${r.luma.toFixed(2).padStart(6)}` +
      `  full-detail ${r.full.toFixed(2).padStart(6)}  delta ${(r.err * 100).toFixed(2).padStart(5)}%` +
      `  LOD ${r.lod ? r.lod.join('/') : 'n/a'}`,
    );
  }
}
console.log(`largest LOD-induced luminance error: ${(worst * 100).toFixed(2)}% at frame ${worstAt} (AC2 allows 3%)`);
const switched = new Set(rows.map((r) => (r.lod ? r.lod.join('/') : ''))).size;
console.log(`distinct LOD populations across the dolly: ${switched} (1 would mean nothing ever switched)`);

await browser.close();
if (errors.length) { console.log('errors:', errors.slice(0, 3)); process.exit(1); }
process.exit(worst <= 0.03 && switched > 1 ? 0 : 1);
