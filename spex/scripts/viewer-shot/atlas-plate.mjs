#!/usr/bin/env node
/** A LEGIBLE picture of an Atlas site: exposure pulled down, camera pulled back.
 *
 * atlas-shot.mjs screenshots the viewer's defaults, and the defaults are wrong
 * for this material -- a 157 m cathedral and a 55 m pyramid do not want the
 * same framing, and 0.28 exposure blows out pale limestone. This one drives
 * the viewer's own exposure control and dollies the camera out with real wheel
 * events until the whole model is inside the frame.
 */
import { chromium } from 'playwright';
const [base, outDir, expo, zoom, ...slugs] = process.argv.slice(2);
const browser = await chromium.launch();
for (const slug of slugs) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(`${base}/d/atlas-${slug}/?quality=low`, { waitUntil: 'networkidle' });
  await page.waitForSelector('canvas', { timeout: 240000 });
  await page.waitForTimeout(9000);
  const set = await page.evaluate((e) => {
    const i = document.getElementById('exposure');
    if (!i) return null;
    i.value = String(e);
    i.dispatchEvent(new Event('input', { bubbles: true }));
    return i.value;
  }, expo);
  const box = await page.locator('canvas').boundingBox();
  const cx = box.x + box.width / 2, cy = box.y + box.height / 2;
  await page.mouse.move(cx, cy);
  for (let k = 0; k < Number(zoom); k++) { await page.mouse.wheel(0, 120); await page.waitForTimeout(120); }
  await page.waitForTimeout(2500);
  await page.screenshot({ path: `${outDir}/${slug}.png`, timeout: 300000 });
  console.log(`${slug} exposure=${set} wheel=${zoom}`);
  await page.close();
}
await browser.close();
