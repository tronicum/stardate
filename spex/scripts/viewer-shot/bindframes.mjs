#!/usr/bin/env node
/** M71 — rung 5. The three things the binding puts on screen.
 *
 *   node scripts/viewer-shot/bindframes.mjs http://127.0.0.1:8150/ /tmp/m71shots
 *
 * The gate, an entry lift, and the same frame without it. That last pair is
 * the point: an emissive addition is easy to claim and easy to not actually
 * make, so each lit frame is shot twice — once with the lift at 1 and once at
 * 0, from the identical camera and show time — and the difference in mean
 * luminance is the measurement. Counting bright pixels would measure the
 * bloom's spread rather than the lift.
 */
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync } from 'node:fs';
import { PNG } from 'pngjs';

const url = process.argv[2] ?? 'http://127.0.0.1:8150/';
const outDir = process.argv[3] ?? '/tmp/m71shots';
mkdirSync(outDir, { recursive: true });

function meanLuma(buffer) {
  const png = PNG.sync.read(buffer);
  let sum = 0;
  for (let i = 0; i < png.data.length; i += 4) {
    sum += 0.2126 * png.data[i] + 0.7152 * png.data[i + 1] + 0.0722 * png.data[i + 2];
  }
  return sum / (png.data.length / 4);
}

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 960, height: 600 } });
const warnings = [];
page.on('console', (m) => {
  const t = m.text();
  if (/SwiftShader|GL Driver Message|Automatic fallback/.test(t)) return;
  if (m.type() === 'error' || m.type() === 'warning') warnings.push(t);
});
page.on('pageerror', (e) => warnings.push(String(e)));

// No `--autoplay-policy` flag here, deliberately: this is the gate as a
// visitor meets it.
await page.goto(`${url}?duration=240`, { waitUntil: 'networkidle' });
await page.waitForFunction(() => !!window.__spexShow, null, { timeout: 120000 });
await page.waitForTimeout(1500);
const gateShown = await page.evaluate(
  () => document.getElementById('show-gate')?.style.display !== 'none',
);
writeFileSync(`${outDir}/m71-gate.png`, await page.screenshot());

await page.evaluate(() => window.__spexShow.begin());
await page.waitForTimeout(1200);

const shots = [];
for (const [name, at] of [
  ['m71-lift-brick', 31.8],
  ['m71-lift-monolith', 120.0],
  ['m71-lift-monolith-late', 190.0],
]) {
  // **The show is paused and seeked first.** The first version shot the two
  // frames a second apart while the piece kept playing, so the camera moved
  // between them and the "difference the lift makes" came out *negative* — it
  // was measuring a dolly. A pair of frames that differ in one thing has to
  // differ in one thing.
  //
  // The lift is then held by re-writing it every 30 ms, because the binder
  // decays it by design and a frame here is 250 ms: one frame of decay is a
  // quarter of the effect. What is held is the binder's own `lift` map, so
  // what reaches the shader is the same path a real entry takes.
  const setup = await page.evaluate(async (t) => {
    const s = window.__spexShow;
    s.setPlaying(false);
    s.seek(t);
    await new Promise((r) => setTimeout(r, 1500));
    window.__hold = setInterval(() => {
      for (const v of [0, 1, 2, 3]) s.binder.lift.set(v, 1);
    }, 30);
    await new Promise((r) => setTimeout(r, 1200));
    return { scenes: s.visibleScenes(), shot: s.activeShotId(), section: s.binder.section };
  }, at);
  const withLift = await page.screenshot();
  writeFileSync(`${outDir}/${name}.png`, withLift);

  await page.evaluate(async () => {
    const s = window.__spexShow;
    clearInterval(window.__hold);
    window.__hold = setInterval(() => {
      for (const v of [0, 1, 2, 3]) s.binder.lift.set(v, 0);
    }, 30);
    await new Promise((r) => setTimeout(r, 1500));
  });
  const withoutLift = await page.screenshot();
  writeFileSync(`${outDir}/${name}-unlit.png`, withoutLift);
  await page.evaluate(() => clearInterval(window.__hold));

  const a = meanLuma(withLift);
  const b = meanLuma(withoutLift);
  shots.push({
    name, at, scenes: setup.scenes, shot: setup.shot, section: setup.section,
    litLuma: +a.toFixed(4), unlitLuma: +b.toFixed(4), deltaLuma: +(a - b).toFixed(4),
  });
  console.log(
    `${name}  t=${at}s  ${setup.shot} [${setup.scenes.join(',')}]  ` +
      `luma ${a.toFixed(3)} lit / ${b.toFixed(3)} unlit  \u0394 ${(a - b).toFixed(3)}`,
  );
}

writeFileSync(`${outDir}/m71-frames.json`, JSON.stringify({ gateShown, shots, warnings }, null, 2));
console.log(`\ngate shown before begin(): ${gateShown}`);
console.log(`console warnings/errors: ${warnings.length}`);
for (const w of warnings.slice(0, 5)) console.log(`  ! ${w}`);
await browser.close();
