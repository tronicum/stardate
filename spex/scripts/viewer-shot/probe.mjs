/** Counters only, no screenshot.
 *
 * At 50 000 instances a software rasteriser cannot produce a full-resolution
 * frame inside any sane timeout — but the numbers M55 actually asserts (group
 * count, draw calls, the transform-pass cost) are all available from the page
 * without ever reading pixels back. So this exists: same page, same hooks,
 * no `page.screenshot`.
 */
import { chromium } from 'playwright';
const [, , url] = process.argv;
const browser = await chromium.launch({ args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: 640, height: 400 } });
const errors = [];
page.on('pageerror', (e) => errors.push(e.message));
await page.goto(url, { waitUntil: 'networkidle' });
await page.waitForFunction(() => Boolean(window.__spexMesh), null, { timeout: 120_000 });
// Let a handful of frames go by so renderer.info.render.calls is a real count.
await page.waitForFunction(() => window.__spexMesh.drawCalls() > 0, null, { timeout: 120_000 });
const out = await page.evaluate(() => ({
  stats: window.__spexMesh.stats,
  drawCalls: window.__spexMesh.drawCalls(),
  transforms: window.__spexMesh.benchTransforms(5),
  parts: window.__spexMesh.stats.parts,
}));
await browser.close();
console.log(JSON.stringify(out, null, 2));
if (errors.length) { console.log('pageerrors:', errors); process.exit(1); }
