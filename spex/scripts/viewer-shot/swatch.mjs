/** Reads back the rendered colour of each instance, instead of judging it by eye.
 *
 * "Does chrome read as metal?" is a question about relative luminance, and a
 * screenshot answers it only if someone looks carefully. This projects every
 * instance's centre to screen space, samples the frame there, and prints the
 * sRGB triple — so a material regression is a number that moved, not a
 * feeling that something looks off.
 */
import { chromium } from 'playwright';
import { PNG } from 'pngjs';
import { readFileSync } from 'node:fs';

const browser = await chromium.launch({ args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: 1200, height: 800 } });
await page.goto(process.argv[2], { waitUntil: 'networkidle' });
await page.waitForTimeout(4000);
const probes = await page.evaluate(() => {
  const THREEish = window.__spexMesh;
  const cam = THREEish.camera;
  cam.updateMatrixWorld();
  const out = [];
  for (const g of THREEish.groups) {
    const m = [].concat(g.mesh.material)[0];
    // Instance 1 is the upper course, well clear of the ground. A part's
    // origin sits at its *top* in the output frame (LDraw is Y-down and the
    // bundle flips it), so drop 4 mm to land in the middle of the body
    // rather than on the stud or in the air above it.
    const a = g.mesh.instanceMatrix.array;
    const i = Math.min(1, g.ids.length - 1);
    const p = { x: a[i * 16 + 12], y: a[i * 16 + 13] - 4, z: a[i * 16 + 14] };
    const v = new cam.position.constructor(p.x, p.y, p.z).project(cam);
    out.push({
      name: m.name,
      x: Math.round((v.x * 0.5 + 0.5) * window.innerWidth),
      y: Math.round((-v.y * 0.5 + 0.5) * window.innerHeight),
    });
  }
  return out;
});
await page.screenshot({ path: '/tmp/swatch.png' });
await browser.close();
const png = PNG.sync.read(readFileSync('/tmp/swatch.png'));
const at = (x, y) => { const i = (png.width * y + x) * 4; return [png.data[i], png.data[i + 1], png.data[i + 2]]; };
for (const p of probes) {
  const c = at(p.x, p.y);
  const lum = Math.round(0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]);
  console.log(`${p.name.padEnd(30)} rgb(${c.join(',').padEnd(12)}) luma ${String(lum).padStart(3)}  @${p.x},${p.y}`);
}
