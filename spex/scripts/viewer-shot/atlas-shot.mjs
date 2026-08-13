#!/usr/bin/env node
/** A picture of an Atlas site, from the gallery, in the real renderer.
 *
 *   spex gallery demos --port 8125 --no-open &
 *   node scripts/viewer-shot/atlas-shot.mjs http://127.0.0.1:8125 /tmp/shots kolosseum giza
 *
 * The elevation drawing (`scripts/ldraw/atlas-elevation.py`) is the instrument
 * for the forty iterations; this is the one that gets the last word.
 */
import { chromium } from 'playwright';
const [base, outDir, ...slugs] = process.argv.slice(2);
const browser = await chromium.launch();
for (const slug of slugs) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(`${base}/d/atlas-${slug}/?quality=low`, { waitUntil: 'networkidle' });
  // The mesh path has no `__spexShow`; wait for the canvas to have drawn.
  await page.waitForSelector('canvas', { timeout: 180000 });
  await page.waitForTimeout(12000);
  await page.screenshot({ path: `${outDir}/atlas-${slug}.png`, timeout: 240000 });
  console.log(`${slug} -> ${outDir}/atlas-${slug}.png`);
  await page.close();
}
await browser.close();
