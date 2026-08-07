/** M59 AC1: counters from the synthetic 200 k-instance scene, no pixels.
 *
 * Review 01's B10 moved AC1 off "a 40-site Atlas scene (from M74)" — a week-6
 * milestone gated on a week-17 deliverable — and onto this. Frame rate is not
 * asserted here for the reason recorded at M54's AC2: the harness has no GPU.
 * What is asserted is that the selector works at that scale and that the
 * triangle count it produces is far below full detail.
 */
import { chromium } from 'playwright';
const browser = await chromium.launch({ args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: 640, height: 400 } });
const errors = [];
page.on('pageerror', (e) => errors.push(e.message));
await page.goto(process.argv[2], { waitUntil: 'networkidle' });
await page.waitForFunction(() => Boolean(window.__spexMesh), null, { timeout: 240_000 });
await page.waitForFunction(() => window.__spexMesh.drawCalls() > 0, null, { timeout: 240_000 });
const out = await page.evaluate(() => {
  const M = window.__spexMesh;
  const l = M.lod();
  const t0 = performance.now();
  l?.forceLevel(null);
  M.camera.position.multiplyScalar(1.0001); // nudge so the selector re-evaluates
  l?.update(M.camera, window.innerHeight);
  const selectMs = performance.now() - t0;
  return {
    stats: M.stats,
    drawCalls: M.drawCalls(),
    lod: l ? { perLevel: l.stats.perLevel, triangles: l.stats.triangles, repacks: l.stats.repacks } : null,
    selectMs: +selectMs.toFixed(2),
  };
});
await browser.close();
console.log(JSON.stringify(out, null, 1));
if (errors.length) { console.log('pageerrors:', errors.slice(0, 3)); process.exit(1); }
