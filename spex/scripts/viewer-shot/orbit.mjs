/** M57 AC2 + AC3: orbit the camera and watch the conditional-edge test work.
 *
 * A conditional edge is drawn only when both its control points project to
 * the same side of the line — which means the *number* drawn must change as
 * the camera moves. A constant count is proof the test is not running, and
 * that is a bug a still screenshot cannot show. This orbits 12 angles,
 * records the count at each, and writes a contact sheet of the frames.
 *
 * It also re-shoots at 0.5x and 50x the default camera distance (AC3), where
 * depth precision is worst at both ends.
 */
import { chromium } from 'playwright';
const [, , url, prefix] = process.argv;
const browser = await chromium.launch({ args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: 700, height: 700 } });
const errors = [];
page.on('pageerror', (e) => errors.push(e.message));
await page.goto(url, { waitUntil: 'networkidle' });
await page.waitForTimeout(4000);
await page.evaluate(() => { window.__spexMesh.renderer.shadowMap.enabled = false; });

const sets = [];
for (let i = 0; i < 12; i++) {
  const a = (i / 12) * Math.PI * 2;
  await page.evaluate(`window.__spexMesh.orbitTo(${a})`);
  await page.waitForTimeout(700);
  sets.push(await page.evaluate(() => window.__spexMesh.conditionalEdgesDrawn()));
  await page.screenshot({ path: `${prefix}-orbit-${String(i).padStart(2, '0')}.png` });
}
// The SET, not the count. A cylinder has exactly two silhouette edges from
// every direction, so a correct renderer holds the count constant while the
// identity of the two rotates with the camera.
const sigs = sets.map((s) => s.slice().sort().join(','));
console.log('conditional edges drawn per angle (count):', sets.map((s) => s.length).join(' '));
sigs.forEach((s, i) => console.log(`  angle ${String(i).padStart(2)}: ${s || '(none)'}`));
const distinct = new Set(sigs).size;
console.log(`distinct edge sets across 12 angles: ${distinct} (1 would mean the test never runs)`);

// AC3: the depth extremes.
for (const [name, scale] of [['near', 0.5], ['far', 50]]) {
  await page.evaluate(`(() => {
    const M = window.__spexMesh, c = M.camera, t = M.controls.target;
    const d = c.position.clone().sub(t);
    c.position.copy(t).add(d.multiplyScalar(${scale}));
    c.updateProjectionMatrix(); M.controls.update();
  })()`);
  await page.waitForTimeout(900);
  await page.screenshot({ path: `${prefix}-${name}.png` });
  await page.evaluate(`(() => {
    const M = window.__spexMesh, c = M.camera, t = M.controls.target;
    const d = c.position.clone().sub(t);
    c.position.copy(t).add(d.multiplyScalar(1 / ${scale}));
    c.updateProjectionMatrix(); M.controls.update();
  })()`);
  console.log(`${name} (x${scale}) written`);
}
await browser.close();
if (errors.length) { console.log('pageerrors:', errors); process.exit(1); }
process.exit(distinct > 1 ? 0 : 1);
