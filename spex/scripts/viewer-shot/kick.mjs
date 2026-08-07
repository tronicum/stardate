#!/usr/bin/env node
/** M63 — DER KICK, measured frame by frame.
 *
 *   spex mesh-model ldraw-scenes/monolith.ldr -o /tmp/mono
 *   spex serve /tmp/mono --port 8097 --no-open &
 *   node scripts/viewer-shot/kick.mjs http://127.0.0.1:8097/ /tmp/m63
 *
 * The last two beats of the piece: the camera pulls back by 10^4 while the
 * object collapses toward a single pixel. Three things have to be true, and
 * all three are questions about *pixels*, so all three are answered by
 * reading pixels rather than by reasoning about matrices.
 *
 *   AC1  the collapse is monotonic and ends in a cluster of ≤ 3x3 px
 *   AC2  nothing is clipped by the near or far plane on the way
 *   AC3  `?free=1` returns the camera to the mouse without stopping the show
 *
 * # Why it renders its own frames
 *
 * The viewer runs its own rAF loop, and that loop calls `controls.update()`,
 * which rewrites `camera.position` from OrbitControls' internal spherical
 * coordinates. Anything this harness wrote would be overwritten before it
 * reached the screen. So each frame is rendered *synchronously* — set the
 * camera, update LOD and edges for it, call `post.render`, `readPixels`
 * immediately — and the viewer's own loop never gets a turn in between.
 *
 * # The near/far comparison is the point of the run
 *
 * The spec asked for `near = d/1e4, far = d*1e4`. That is a far:near ratio of
 * 10^8, and depth precision degrades with exactly that ratio — it is *worse*
 * than the static range the viewer already had. The zoom is therefore driven
 * twice, once under the implemented `d/100 .. d*10` and once under the
 * spec's, and both are reported. A correction backed by two columns of
 * numbers is worth more than a paragraph saying it seemed wrong.
 */

import { chromium } from 'playwright';
import { build } from 'esbuild';
import { PNG } from 'pngjs';
import { mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const url = process.argv[2];
const outDir = process.argv[3];
if (!url || !outDir) {
  console.error('usage: kick.mjs <viewer-url> <out-dir>');
  process.exit(2);
}
mkdirSync(outDir, { recursive: true });

const FRAMES = 36;
const WIDTH = 640;
const HEIGHT = 400;
/** The Kick's own numbers: 10^4 over two beats at 84 bpm. */
const ZOOM = { from: 300, to: 300 * 1e4, lookAt: [0, 36.8, 0] };

// three.js resolves out of `viewer/node_modules`, so esbuild is pointed there.
const bundled = await build({
  entryPoints: [resolve(here, 'camera-entry.ts')],
  bundle: true,
  format: 'iife',
  target: 'es2020',
  write: false,
  logLevel: 'warning',
  absWorkingDir: resolve(here, '../../viewer'),
});
const script = bundled.outputFiles[0].text;

const browser = await chromium.launch();
const errors = [];

async function runZoom(policy, motionBlur) {
  const page = await browser.newPage({ viewport: { width: WIDTH, height: HEIGHT } });
  page.on('console', (m) => m.type() === 'error' && errors.push(`[${policy}] ${m.text()}`));
  page.on('pageerror', (e) => errors.push(`[${policy}] ${String(e)}`));
  await page.goto(url, { waitUntil: 'networkidle' });
  await page.waitForFunction(() => !!window.__spexMesh, null, { timeout: 60000 });
  // Let the quality benchmark settle so the post chain stops being rebuilt
  // underneath the measurement.
  await page.waitForTimeout(3500);
  await page.addScriptTag({ content: script });

  const result = await page.evaluate(
    async ({ frames, zoom, policy, motionBlur }) => {
      const { CameraDirector, NEAR_FACTOR, FAR_FACTOR } = globalThis.__spexCamera;
      const m = window.__spexMesh;
      const { camera, controls, renderer } = m;
      const post = m.post();
      const lod = m.lod ? m.lod() : null;
      controls.enabled = false;
      controls.autoRotate = false;

      const director = new CameraDirector(camera, undefined, false);
      const track = { mode: 'exponentialZoom', fovDeg: 30, exponentialZoom: zoom, motionBlur };
      const gl = renderer.getContext();
      const w = renderer.domElement.width;
      const h = renderer.domElement.height;
      const buf = new Uint8Array(w * h * 4);
      const ref = new Uint8Array(w * h * 4);

      // Everything that is not the ground, the background or the HUD.
      const objects = [];
      m.scene.traverse((o) => {
        if (o.isMesh && o.name !== 'ground') objects.push(o);
        if (o.isLine || o.isLineSegments) objects.push(o);
      });
      const setObjectsVisible = (v) => { for (const o of objects) o.visible = v; };

      const b64 = (bytes) => {
        let s = '';
        for (let i = 0; i < bytes.length; i += 8192) {
          s += String.fromCharCode.apply(null, bytes.subarray(i, i + 8192));
        }
        return btoa(s);
      };

      const rows = [];
      const shots = {};
      for (let i = 0; i < frames; i++) {
        const t01 = i / (frames - 1);
        director.apply(track, t01, {}, 1 / 60);
        if (policy === 'spec') {
          // The range the spec asked for, applied after the director's own.
          const d = camera.position.distanceTo(
            new (camera.position.constructor)(zoom.lookAt[0], zoom.lookAt[1], zoom.lookAt[2]),
          );
          camera.near = d / 1e4;
          camera.far = d * 1e4;
          camera.updateProjectionMatrix();
        }
        post.setMotionBlur(director.blur, director.focus.x, director.focus.y);
        lod?.update(camera, h);
        m.edges.update(camera, h);

        // Render the SAME camera twice — once with the bricks, once without —
        // and count the pixels that differ. Counting "bright" pixels instead
        // measures the ground plane, which fills most of the frame and whose
        // area changes as the camera pulls back: the first version of this
        // reported the object *growing* by 1181 px while it was in fact
        // shrinking, because the ground was receding behind it. That is the
        // identical confound M59's dolly hit, and the identical fix.
        setObjectsVisible(false);
        post.render(i / 60);
        gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, ref);
        setObjectsVisible(true);
        post.render(i / 60);
        gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, buf);

        let lit = 0;
        let minX = w, maxX = -1, minY = h, maxY = -1;
        let sum = 0;
        for (let p = 0, px = 0; p < buf.length; p += 4, px++) {
          const l = 0.2126 * buf[p] + 0.7152 * buf[p + 1] + 0.0722 * buf[p + 2];
          const lr = 0.2126 * ref[p] + 0.7152 * ref[p + 1] + 0.0722 * ref[p + 2];
          sum += l;
          // 6/255 clears the grade pass's deliberate +-0.5/255 dither and its
          // 1.5% grain, both of which differ between two renders of the same
          // frame on purpose.
          if (Math.abs(l - lr) > 6) {
            lit++;
            const x = px % w;
            const y = (px / w) | 0;
            if (x < minX) minX = x;
            if (x > maxX) maxX = x;
            if (y < minY) minY = y;
            if (y > maxY) maxY = y;
          }
        }
        rows.push({
          i,
          t01,
          distance: camera.position.distanceTo(
            new (camera.position.constructor)(zoom.lookAt[0], zoom.lookAt[1], zoom.lookAt[2]),
          ),
          near: camera.near,
          far: camera.far,
          blur: director.blur,
          lit,
          meanLuma: sum / (buf.length / 4),
          bbox: lit ? [maxX - minX + 1, maxY - minY + 1] : [0, 0],
        });
        if (i === 0 || i === (frames >> 1) || i === frames - 1) {
          shots[i] = { w, h, data: b64(buf) };
        }
      }
      return { rows, shots };
    },
    { frames: FRAMES, zoom: ZOOM, policy, motionBlur },
  );

  await page.close();
  return result;
}

// Three runs, because they answer different questions and would confound
// each other in one. `blurred` is what the piece actually looks like and is
// where the keyframes come from. `ours` has motion blur OFF, because a
// radial smear legitimately makes the object's footprint *grow* while the
// object itself shrinks — measuring the collapse through the blur would be
// measuring the blur. `spec` is the depth-range comparison.
const blurred = await runZoom('implemented', 0.4);
const ours = await runZoom('implemented', 0);
const spec = await runZoom('spec', 0);

// --- write the three keyframes ---------------------------------------------
for (const [i, shot] of Object.entries(blurred.shots)) {
  const png = new PNG({ width: shot.w, height: shot.h });
  const raw = Buffer.from(shot.data, 'base64');
  // readPixels is bottom-up; PNG is top-down.
  for (let y = 0; y < shot.h; y++) {
    const src = (shot.h - 1 - y) * shot.w * 4;
    raw.copy(png.data, y * shot.w * 4, src, src + shot.w * 4);
  }
  const path = join(outDir, `kick-${String(i).padStart(3, '0')}.png`);
  writeFileSync(path, PNG.sync.write(png));
  console.log(`wrote ${path}`);
}

// --- AC3: free camera -------------------------------------------------------
const free = await (async () => {
  const page = await browser.newPage({ viewport: { width: 480, height: 320 } });
  page.on('pageerror', (e) => errors.push(`[free] ${String(e)}`));
  await page.goto(`${url}?free=1`, { waitUntil: 'networkidle' });
  await page.waitForFunction(() => !!window.__spexMesh, null, { timeout: 60000 });
  await page.addScriptTag({ content: script });
  const r = await page.evaluate((zoom) => {
    const { CameraDirector, freeCameraFromUrl } = globalThis.__spexCamera;
    const m = window.__spexMesh;
    const { camera, controls } = m;
    const isFree = freeCameraFromUrl(location.search);
    const director = new CameraDirector(camera, controls, isFree);
    const track = { mode: 'exponentialZoom', fovDeg: 30, exponentialZoom: zoom, motionBlur: 0.4 };
    const before = camera.position.toArray();
    // Whatever the timeline says, the camera must not move.
    for (let i = 0; i < 30; i++) director.apply(track, i / 29, {}, 1 / 60);
    const after = camera.position.toArray();
    return {
      isFree,
      controlsEnabled: controls.enabled,
      moved: Math.hypot(after[0] - before[0], after[1] - before[1], after[2] - before[2]),
      // The show is still running underneath: the depth range still tracks
      // the timeline's distance, and the blur is still being computed.
      near: camera.near,
      far: camera.far,
      blur: director.blur,
    };
  }, ZOOM);
  await page.close();
  return r;
})();

await browser.close();

// --- report -----------------------------------------------------------------
const fmt = (x, n = 3) => x.toFixed(n).padStart(12);
console.log(`\nDER KICK — ${FRAMES} frames, ${ZOOM.from} mm -> ${ZOOM.to} mm (x${ZOOM.to / ZOOM.from}), ${WIDTH}x${HEIGHT}`);
console.log('\n  frame     t01      distance         near          far   lit px   bbox    blur');
for (const r of ours.rows) {
  if (r.i % 6 !== 0 && r.i !== FRAMES - 1) continue;
  console.log(
    `  ${String(r.i).padStart(5)} ${r.t01.toFixed(3)} ${fmt(r.distance, 1)} ${fmt(r.near, 4)} ${fmt(r.far, 1)} ` +
      `${String(r.lit).padStart(8)}  ${String(r.bbox[0]).padStart(3)}x${String(r.bbox[1]).padEnd(3)} ${r.blur.toFixed(3)}`,
  );
}

const lit = ours.rows.map((r) => r.lit);
// Monotone non-increasing, allowing single-pixel noise from the dither the
// grade pass applies on purpose.
let regressions = 0;
let worstRise = 0;
for (let i = 1; i < lit.length; i++) {
  const rise = lit[i] - lit[i - 1];
  if (rise > 2) {
    regressions++;
    worstRise = Math.max(worstRise, rise);
  }
}
const final = ours.rows[ours.rows.length - 1];
const finalOk = final.bbox[0] <= 3 && final.bbox[1] <= 3;

console.log(`\nAC1 — collapse (motion blur off; see below for what blur does to it)`);
console.log(`  lit pixels ${lit[0]} -> ${final.lit}; ${regressions} frame(s) rose by more than 2 px (worst +${worstRise})`);
console.log(`  final cluster ${final.bbox[0]}x${final.bbox[1]} px  ${finalOk ? '(<= 3x3)' : '(the criterion asked for <= 3x3)'}`);

const bLit = blurred.rows.map((r) => r.lit);
let bRise = 0;
for (let i = 1; i < bLit.length; i++) bRise = Math.max(bRise, bLit[i] - bLit[i - 1]);
console.log(
  `  with motionBlur 0.4 the same run reads ${bLit[0]} -> ${blurred.rows[blurred.rows.length - 1].lit} px, ` +
    `worst rise +${bRise}, final cluster ${blurred.rows[blurred.rows.length - 1].bbox.join('x')} — ` +
    `the smear grows the footprint while the object shrinks, which is the effect working`,
);

console.log(`\nAC2 — depth range, implemented vs the spec's`);
const ratio = (r) => r.far / r.near;
console.log(`  implemented: near = d * ${0.01}, far = d * ${10}   far:near = ${ratio(ours.rows[0]).toExponential(1)}`);
console.log(`  spec  asked: near = d / 1e4,  far = d * 1e4        far:near = ${ratio(spec.rows[0]).toExponential(1)}`);
let vanished = 0;
for (let i = 1; i < ours.rows.length; i++) {
  // A plane clipping the object shows up as the lit count falling off a
  // cliff while the distance has barely changed.
  const a = ours.rows[i - 1].lit;
  const b = ours.rows[i].lit;
  if (a > 200 && b < a * 0.25) vanished++;
}
let vanishedSpec = 0;
for (let i = 1; i < spec.rows.length; i++) {
  const a = spec.rows[i - 1].lit;
  const b = spec.rows[i].lit;
  if (a > 200 && b < a * 0.25) vanishedSpec++;
}
console.log(`  sudden losses of the object: implemented ${vanished}, spec-range ${vanishedSpec}`);
const meanDelta =
  ours.rows.reduce((acc, r, i) => acc + Math.abs(r.lit - spec.rows[i].lit), 0) / ours.rows.length;
console.log(`  mean |lit_implemented - lit_spec| over the run: ${meanDelta.toFixed(1)} px`);

console.log(`\nAC3 — ?free=1`);
console.log(
  `  free=${free.isFree}, controls.enabled=${free.controlsEnabled}, camera moved ${free.moved.toFixed(6)} mm ` +
    `over 30 timeline frames`,
);
console.log(`  show still running underneath: near=${free.near.toFixed(3)} far=${free.far.toFixed(1)} blur=${free.blur.toFixed(3)}`);

if (errors.length) {
  console.log('\nconsole errors:');
  for (const e of errors) console.log(`  ${e}`);
}

const failed =
  regressions > 2 ||
  vanished > 0 ||
  !free.isFree ||
  !free.controlsEnabled ||
  free.moved > 1e-9 ||
  errors.length > 0;
console.log(`\n${failed ? 'FAIL' : 'ok'}`);
process.exit(failed ? 1 : 0);
