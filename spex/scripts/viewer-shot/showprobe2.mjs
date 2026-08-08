#!/usr/bin/env node
/** Scratch diagnostic for M66. Not an acceptance test — a question.
 *
 * Two things the first full run of `showrun.mjs` photographed and could not
 * explain: the opening frame is not black, and two renders of the same t=0
 * state differ by 87 levels across a loop. Both are about *shared renderer
 * state*, so this prints it rather than reasoning about it.
 */
import { chromium } from 'playwright';

const url = process.argv[2];
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 640, height: 400 } });
page.on('pageerror', (e) => console.log('pageerror', String(e)));
await page.goto(`${url}?duration=endless`, { waitUntil: 'networkidle' });
await page.waitForFunction(() => !!window.__spexShow, null, { timeout: 120000 });
await page.waitForTimeout(3500);

const out = await page.evaluate(async () => {
  const s = window.__spexShow;
  const frame = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  const gl = s.renderer.getContext();
  const w = s.renderer.domElement.width;
  const h = s.renderer.domElement.height;
  const px = (x, y) => {
    const b = new Uint8Array(4);
    gl.readPixels(x, y, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, b);
    return [b[0], b[1], b[2]];
  };
  const state = () => {
    const p = s.post();
    const ground = s.scene.getObjectByName('ground');
    return {
      exposure: p.exposure,
      vignette: p.vignette,
      grade: p.gradeStrength,
      grain: p.grain,
      bloom: [p.bloomThreshold, p.bloomStrength, p.bloomRadius],
      tier: s.quality(),
      groundColor: ground ? ground.material.color.toArray() : null,
      groundVisible: ground ? ground.visible : null,
      cam: s.camera.position.toArray().map((v) => +v.toFixed(3)),
      fov: s.camera.fov,
      near: s.camera.near,
      far: s.camera.far,
      blur: s.director.blur,
      // Low in the frame (ground) and high in the frame (background).
      groundPx: px(Math.floor(w / 2), Math.floor(h * 0.15)),
      skyPx: px(Math.floor(w / 2), Math.floor(h * 0.9)),
    };
  };

  s.setPlaying(false);
  s.seek(0);
  await frame();
  const before = state();

  // Render the same frame twice, once without the ground: whatever remains is
  // not the ground, whatever changes is. M59's technique, third milestone in a
  // row that needs it.
  const ground = s.scene.getObjectByName('ground');
  ground.visible = false;
  await frame();
  const noGround = state();
  ground.visible = true;
  await frame();
  // And once with the environment removed entirely.
  const env = s.scene.environment;
  s.scene.environment = null;
  for (const sc of s.scenes) for (const g of sc.groups) {
    const mats = Array.isArray(g.mesh.material) ? g.mesh.material : [g.mesh.material];
    for (const m of mats) { m.envMap = null; m.needsUpdate = true; }
  }
  ground.material.envMap = null; ground.material.needsUpdate = true;
  await frame();
  const noEnv = state();
  s.scene.environment = env;

  s.clock.seek(s.show.durationSec - 0.4);
  s.setPlaying(true);
  const c0 = s.clock.cycle;
  const t0 = performance.now();
  while (s.clock.cycle === c0 && performance.now() - t0 < 60000) await frame();
  const atLoop = state();
  s.setPlaying(false);
  s.seek(0);
  await frame();
  const after = state();

  return { before, noGround, noEnv, atLoop, after, looped: s.clock.cycle > c0 };
});

console.log(JSON.stringify(out, null, 2));
await browser.close();
