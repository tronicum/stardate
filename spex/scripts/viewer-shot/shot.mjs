/** Rung 5 of the verification ladder: a real browser, a real screenshot, and
 * counters read out of the running page rather than inferred from the pixels.
 *
 * Every viewer-visible milestone has to pass through here. "It looks right on
 * my screen" is not a result anyone else can check six months later; a PNG
 * plus the renderer's own numbers is.
 *
 * Usage:
 *   node scripts/viewer-shot/shot.mjs <url> <out.png> [--width 1600] [--height 1000]
 *                                     [--settle 4000] [--expect-mesh]
 *
 * Exits non-zero on any console error, any page error, any failed request, or
 * — with --expect-mesh — if the page did not actually take the mesh path.
 */
import { chromium } from 'playwright';
import { writeFileSync } from 'node:fs';

const [, , url, out, ...rest] = process.argv;
if (!url || !out) {
  console.error('usage: shot.mjs <url> <out.png> [--width N] [--height N] [--settle MS] [--expect-mesh] [--bench] [--no-shadows]');
  process.exit(2);
}
const arg = (name, fallback) => {
  const i = rest.indexOf(`--${name}`);
  return i === -1 ? fallback : Number(rest[i + 1]);
};
const width = arg('width', 1600);
const height = arg('height', 1000);
const settle = arg('settle', 4000);
const expectMesh = rest.includes('--expect-mesh');

const browser = await chromium.launch({ args: ['--no-sandbox', '--use-gl=swiftshader'] });
const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 1 });

/** The viewer probes for four manifests that are *supposed* to be absent most
 * of the time — that absence is how it picks a render mode. A 404 on one of
 * these is the feature working, not a broken asset, so it is filtered out of
 * every collector below (the browser logs it as a console error too, not just
 * a failed request). Anything else 4xx/5xx is real. */
const OPTIONAL_MANIFESTS = ['/mesh.json', '/sequence.json', '/nodes.json', '/meta.json'];
const isOptional = (url) => OPTIONAL_MANIFESTS.some((m) => url.endsWith(m));

const consoleErrors = [];
const warnings = [];
const failedRequests = [];
page.on('console', (msg) => {
  if (isOptional(msg.location()?.url ?? '')) return;
  if (msg.type() === 'error') consoleErrors.push(msg.text());
  if (msg.type() === 'warning') warnings.push(msg.text());
});
page.on('pageerror', (err) => consoleErrors.push(`pageerror: ${err.message}`));
page.on('requestfailed', (req) => {
  if (!isOptional(req.url())) failedRequests.push(`${req.url()} — ${req.failure()?.errorText}`);
});
page.on('response', (res) => {
  if (res.status() >= 400 && !isOptional(res.url())) {
    failedRequests.push(`${res.url()} — HTTP ${res.status()}`);
  }
});

await page.goto(url, { waitUntil: 'networkidle' });

// Software rasteriser escape hatch. The shadow pass re-submits every triangle
// in the scene, so on SwiftShader a 50 000-instance scene doubles from "slow"
// to "the screenshot call times out". Counters and the transform benchmark are
// unaffected — they are what this harness actually asserts (see README) — but
// rung 5 still wants a picture, and this is how a picture is obtainable at
// that scale without a GPU. Never pass it for a milestone's hero shot.
if (rest.includes('--no-shadows')) {
  await page.evaluate(() => {
    if (window.__spexMesh) window.__spexMesh.renderer.shadowMap.enabled = false;
  });
}
// Long enough for the fps counter to have a real window behind it, not one
// frame of startup jitter.
await page.waitForTimeout(settle);

const bench = rest.includes('--bench');
const mesh = await page.evaluate((doBench) => {
  const m = window.__spexMesh;
  if (!m) return null;
  return {
    stats: m.stats,
    fps: m.fps(),
    drawCalls: m.drawCalls(),
    // M55 AC3: the real cost of rewriting every instance's transform once.
    transforms: doBench ? m.benchTransforms() : null,
  };
}, bench);
const hud = await page.evaluate(() => document.getElementById('hud')?.innerText ?? '');

await page.screenshot({ path: out });
await browser.close();

const report = { url, out, width, height, mesh, hud, consoleErrors, warnings, failedRequests };
writeFileSync(out.replace(/\.png$/, '.json'), JSON.stringify(report, null, 2));

console.log(`screenshot: ${out}`);
if (mesh) {
  console.log(`  mesh mode: ${mesh.stats.instances} instances, ${mesh.stats.parts} parts, ` +
    `${mesh.stats.drawnTriangles} triangles drawn (${mesh.stats.uniqueTriangles} unique), ` +
    `${mesh.drawCalls} draw calls, ${mesh.fps.toFixed(1)} fps`);
  if (mesh.stats.groups !== undefined) {
    console.log(`  ${mesh.stats.groups} instance groups -> ${mesh.drawCalls} draw calls ` +
      `(${(mesh.drawCalls / mesh.stats.parts).toFixed(2)} x distinct parts)`);
  }
  if (mesh.transforms) {
    console.log(`  transform pass over ${mesh.transforms.instances} instances, median of ` +
      `${mesh.transforms.runs}: compose ${mesh.transforms.composeMs.toFixed(2)} ms, ` +
      `matrix ${mesh.transforms.matrixMs.toFixed(2)} ms`);
  }
} else {
  console.log('  point mode (no mesh bundle)');
}
console.log(`  hud: ${hud.replace(/\n/g, ' | ')}`);
console.log(`  console errors: ${consoleErrors.length}, warnings: ${warnings.length}, failed requests: ${failedRequests.length}`);
for (const e of consoleErrors) console.log(`    ERROR ${e}`);
for (const w of warnings) console.log(`    WARN  ${w}`);
for (const f of failedRequests) console.log(`    FAIL  ${f}`);

let bad = consoleErrors.length > 0 || failedRequests.length > 0;
if (expectMesh && !mesh) {
  console.log('    ERROR --expect-mesh, but the page did not take the mesh path');
  bad = true;
}
process.exit(bad ? 1 : 0);
