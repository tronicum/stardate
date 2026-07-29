/** Isolation experiments for a rendering question.
 *
 * A screenshot tells you *that* something looks wrong. It never tells you
 * which of the four things that could cause it actually did. This runs the
 * two experiments that have so far settled every such question in this
 * renderer, each by removing exactly one contribution:
 *
 *   depth  — shrink the near/far range by ~3 orders of magnitude. Depth
 *            precision artefacts (z-fighting) change or vanish; a real
 *            geometric edge does not notice.
 *   rough  — force roughness to 1.0 on every brick. A specular highlight
 *            disappears; a wrong vertex normal stays exactly where it was.
 *   flat   — turn shadow casting off. Separates shadow acne from geometry.
 *   apart  — pull the instance stack apart along Y, so surfaces that were
 *            coincident stop being coincident.
 *
 * Usage: node scripts/viewer-shot/isolate.mjs <url> <out-prefix> [x y w h]
 * Writes <prefix>-base.png and one PNG per experiment, all with the same crop.
 *
 * Requires the page to be in mesh mode — it drives `window.__spexMesh`, the
 * debug hook `viewer/src/mesh/render.ts` installs.
 */
import { chromium } from 'playwright';

const [, , url, prefix, ...crop] = process.argv;
if (!url || !prefix) {
  console.error('usage: isolate.mjs <url> <out-prefix> [x y w h]');
  process.exit(2);
}
const clip = crop.length === 4
  ? { x: +crop[0], y: +crop[1], width: +crop[2], height: +crop[3] }
  : undefined;

const browser = await chromium.launch({ args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: 900, height: 900 } });
await page.goto(url, { waitUntil: 'networkidle' });
await page.waitForTimeout(2500);
if (!(await page.evaluate(() => Boolean(window.__spexMesh)))) {
  console.error('page is not in mesh mode — nothing to isolate');
  await browser.close();
  process.exit(2);
}
// A black part on a dark ground shows nothing at all in a 260px crop, so
// every experiment runs over-exposed. This is a diagnostic, not a hero shot.
await page.evaluate(() => { window.__spexMesh.renderer.toneMappingExposure = 6.0; });
await page.waitForTimeout(600);

const shot = async (name) => {
  await page.waitForTimeout(700);
  await page.screenshot({ path: `${prefix}-${name}.png`, clip });
  console.log(`${prefix}-${name}.png`);
};
await shot('base');

await page.evaluate(() => {
  const c = window.__spexMesh.camera;
  const d = c.position.length();
  c.near = d * 0.5; c.far = d * 2; c.updateProjectionMatrix();
});
await shot('depth');

await page.evaluate(() => {
  window.__spexMesh.scene.traverse((o) => {
    if (o.name === 'ground' || !o.material) return;
    for (const m of Array.isArray(o.material) ? o.material : [o.material]) {
      if (m.roughness !== undefined) { m.roughness = 1.0; m.needsUpdate = true; }
    }
  });
});
await shot('rough');

await page.evaluate(() => { window.__spexMesh.renderer.shadowMap.enabled = false; });
await shot('flat');

await page.evaluate(() => {
  // Written straight into the instance matrices: element 13 of a column-major
  // Matrix4 is the Y translation. Going through InstanceWriter would need a
  // THREE.Matrix4 in page scope, and this is a diagnostic, not a feature.
  let n = 0;
  for (const g of window.__spexMesh.groups) {
    const a = g.mesh.instanceMatrix.array;
    for (let i = 0; i < g.ids.length; i++) { a[i * 16 + 13] += n * 3.0; n++; }
    g.mesh.instanceMatrix.needsUpdate = true;
  }
});
await shot('apart');

await browser.close();
