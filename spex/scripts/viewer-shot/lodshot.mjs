import { chromium } from 'playwright';
const browser = await chromium.launch({ args: ['--no-sandbox'] });
const page = await browser.newPage({ viewport: { width: 1200, height: 500 } });
await page.goto(process.argv[2], { waitUntil: 'networkidle' });
await page.waitForTimeout(7000);
// Edges off. Forcing a level keeps the object at full screen size, where the
// edge pass is legitimately still on — so a forced LOD1 would show stud
// *outlines* with no studs behind them. That never happens in normal
// operation (edges gate off at 56 px, LOD1 starts at 44 px, and the bands do
// not overlap), and leaving them on here would document a bug that is not
// there.
await page.evaluate(() => window.__spexMesh.edges.setVisible(false));
for (const lvl of [0, 1, 2]) {
  await page.evaluate(`window.__spexMesh.lod().forceLevel(${lvl})`);
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `/tmp/m59-level-${lvl}.png`, clip: { x: 380, y: 60, width: 440, height: 420 } });
}
await browser.close(); console.log('ok');
