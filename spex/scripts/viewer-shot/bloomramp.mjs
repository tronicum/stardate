/** M58 AC2 and AC3.
 *
 * AC2: ramp the bloom threshold from 1.0 to 0.2 over 30 frames and check the
 * result is a *smooth, visible* change — measured as mean frame luminance,
 * not judged by eye. A threshold ramp that produces a flat luminance curve
 * means bloom is reading a signal that has already been clipped, which is
 * exactly the defect the rev 3 corrections were about.
 *
 * AC3: hide every brick and confirm the frame is neither black nor NaN. The
 * first six seconds of Act I are almost empty, and a post chain that divides
 * by an average luminance of zero produces a black frame at precisely the
 * moment the piece opens.
 */
import { chromium } from 'playwright';
import { PNG } from 'pngjs';
import { readFileSync } from 'node:fs';

const [, , url, prefix] = process.argv;
const browser = await chromium.launch({ args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: 480, height: 360 } });
const errors = [];
page.on('pageerror', (e) => errors.push(e.message));
page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
await page.goto(url, { waitUntil: 'networkidle' });
await page.waitForTimeout(6000);

const meanLuma = (file) => {
  const png = PNG.sync.read(readFileSync(file));
  let sum = 0;
  // Skip the top strip: the HUD is opaque and would flatten the curve.
  for (let y = 200; y < png.height; y++) {
    for (let x = 0; x < png.width; x++) {
      const i = (png.width * y + x) * 4;
      sum += 0.2126 * png.data[i] + 0.7152 * png.data[i + 1] + 0.0722 * png.data[i + 2];
    }
  }
  return sum / (png.width * (png.height - 200));
};

const luma = [];
const FRAMES = 30;
for (let f = 0; f < FRAMES; f++) {
  const threshold = 1.0 - (0.8 * f) / (FRAMES - 1);
  await page.evaluate(`window.__spexMesh.post().bloomThreshold = ${threshold}`);
  await page.waitForTimeout(120);
  const file = `${prefix}-bloom-${String(f).padStart(2, '0')}.png`;
  await page.screenshot({ path: file });
  luma.push({ threshold: +threshold.toFixed(3), luma: +meanLuma(file).toFixed(3) });
}
console.log('bloom threshold 1.0 -> 0.2, mean frame luminance:');
for (const s of luma) console.log(`  ${s.threshold.toFixed(2)}  ${s.luma.toFixed(2)}`);
const rise = luma[luma.length - 1].luma - luma[0].luma;
// Monotonic within noise: no step may go backwards by more than a code value.
const backsteps = luma.filter((s, i) => i > 0 && s.luma < luma[i - 1].luma - 1.0).length;
console.log(`total rise ${rise.toFixed(2)} luma, ${backsteps} non-monotonic steps`);

// AC3: the almost-empty scene.
await page.evaluate(() => {
  const M = window.__spexMesh;
  M.groups.forEach((g) => (g.mesh.visible = false));
  M.edges.setVisible(false);
  M.scene.getObjectByName('ground').visible = false;
});
await page.waitForTimeout(1200);
await page.screenshot({ path: `${prefix}-empty.png` });
const emptyLuma = meanLuma(`${prefix}-empty.png`);
console.log(`empty scene mean luminance ${emptyLuma.toFixed(3)} (0.00 would be a black frame)`);

await browser.close();
if (errors.length) { console.log('errors:', errors.slice(0, 3)); process.exit(1); }
process.exit(rise > 3 && backsteps === 0 && emptyLuma > 0.5 ? 0 : 1);
