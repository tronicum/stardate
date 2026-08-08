#!/usr/bin/env node
/** M64 — the runtime assembly, against the baked one it replaces.
 *
 *   spex mesh-model ldraw-scenes/monolith.ldr -o /tmp/mono
 *   spex serve /tmp/mono --port 8099 --no-open &
 *   node scripts/viewer-shot/assembly.mjs http://127.0.0.1:8099/ /tmp/m64
 *
 * Four questions:
 *
 *   port  does the TypeScript splitmix reproduce the Rust one, bit for bit?
 *   AC1   does the runtime assembly land where the baked one does?
 *   AC2   what does the per-frame transform pass cost?
 *   AC3   does `0 STEP` order actually look different from index order?
 *
 * The first is the load-bearing one. Everything else in this milestone rests
 * on two languages agreeing about a pseudo-random sequence, and they are
 * pinned to `docs/fugen/fixtures/assembly-scatter.json` — a file *neither*
 * generates at test time — rather than to each other, because two
 * implementations compared only against each other can drift together.
 */

import { chromium } from 'playwright';
import { attachConsole } from './absence.mjs';
import { build } from 'esbuild';
import { PNG } from 'pngjs';
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const url = process.argv[2];
const outDir = process.argv[3];
if (!url || !outDir) {
  console.error('usage: assembly.mjs <viewer-url> <out-dir>');
  process.exit(2);
}
mkdirSync(outDir, { recursive: true });

const fixture = JSON.parse(
  readFileSync(resolve(here, '../../docs/fugen/fixtures/assembly-scatter.json'), 'utf8'),
);

const bundled = await build({
  entryPoints: [resolve(here, 'choreography-entry.ts')],
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
const page = await browser.newPage({ viewport: { width: 640, height: 400 } });
// The 404s that are the viewer's mode test answering "no" are not errors
// (absence.mjs). Phase 3's rung 6 surfaced it: probes printed clean numbers
// and then FAIL, on demos nothing was wrong with.
const byDesign = attachConsole(page, errors);
await page.goto(url, { waitUntil: 'networkidle' });
await page.waitForFunction(() => !!window.__spexMesh, null, { timeout: 60000 });
await page.waitForTimeout(3500);
await page.addScriptTag({ content: script });

// ---------------------------------------------------------------------------
// The port, and AC1
// ---------------------------------------------------------------------------
const agreement = await page.evaluate((fx) => {
  const { startOffsetLdu, startOffsetMm, AssemblyChoreography } = globalThis.__spexChoreo;

  let worstScatter = 0;
  let components = 0;
  for (const c of fx.cases) {
    c.offsetsLdu.forEach((want, i) => {
      const got = startOffsetLdu(i, c.editionSeed);
      for (let k = 0; k < 3; k++) {
        worstScatter = Math.max(worstScatter, Math.abs(got[k] - want[k]));
        components++;
      }
    });
  }

  // AC1. The baked demo has NO stagger — one eased lerp for everything — so
  // that is the configuration the runtime has to reproduce. The comparison
  // runs in millimetres, +Y up, which means it also exercises the LDU->mm
  // mirror: the one conversion in this project that has silently inverted a
  // whole library before.
  const mm = fx.lduMm;
  const finalsMm = fx.monolithFinalsLdu.flatMap(([x, y, z]) => [x * mm, -y * mm, z * mm]);
  const ids = fx.monolithFinalsLdu.map((_, i) => `p/${i}`);
  const choreo = new AssemblyChoreography({ ids, finals: finalsMm, stagger: 0, editionSeed: 0 });

  let worstBaked = 0;
  const out = [0, 0, 0];
  for (const frame of fx.bakedMonolith) {
    frame.positionsLdu.forEach((want, i) => {
      choreo.positionAt(i, frame.t01, out);
      const wantMm = [want[0] * mm, -want[1] * mm, want[2] * mm];
      for (let k = 0; k < 3; k++) worstBaked = Math.max(worstBaked, Math.abs(out[k] - wantMm[k]));
    });
  }

  // And the end state: at t01 = 1 every instance must be exactly where the
  // bundle already put it, or the shot ends on a scene that is subtly not the
  // model.
  let worstFinal = 0;
  for (let i = 0; i < ids.length; i++) {
    choreo.positionAt(i, 1, out);
    for (let k = 0; k < 3; k++) worstFinal = Math.max(worstFinal, Math.abs(out[k] - finalsMm[i * 3 + k]));
  }

  // Sanity: the mirror is actually being applied. The scatter starts ABOVE
  // the final position in the viewer's +Y-up frame.
  const up = startOffsetMm(0, 0)[1];

  return { worstScatter, components, worstBaked, worstFinal, upMm: up, frames: fx.bakedMonolith.length };
}, fixture);

// ---------------------------------------------------------------------------
// AC3 — build-step order vs index order, on a scene that has real 0 STEP lines
// ---------------------------------------------------------------------------
const stagger = await page.evaluate(async () => {
  const { AssemblyChoreography } = globalThis.__spexChoreo;
  const m = window.__spexMesh;
  const base = location.pathname.replace(/\/$/, '') + '/tileset';
  let manifest = null;
  for (const candidate of [`${base}/mesh.json`, './tileset/mesh.json', './mesh.json']) {
    try {
      const r = await fetch(candidate);
      if (r.ok) { manifest = await r.json(); break; }
    } catch { /* keep trying */ }
  }
  if (!manifest) return { ok: false, why: 'could not fetch mesh.json' };

  const ids = manifest.instanceIds;
  const steps = manifest.instanceBuildSteps ?? null;

  // Final positions come from the groups the viewer already built, so this
  // measures the real scene rather than a reconstruction of it.
  const finals = new Float32Array(ids.length * 3);
  const index = new Map(ids.map((id, i) => [id, i]));
  for (const g of m.groups) {
    for (let i = 0; i < g.ids.length; i++) {
      const at = index.get(g.ids[i]);
      if (at === undefined) continue;
      const e = g.matrices;
      finals[at * 3] = e[i * 16 + 12];
      finals[at * 3 + 1] = e[i * 16 + 13];
      finals[at * 3 + 2] = e[i * 16 + 14];
    }
  }

  const byIndex = new AssemblyChoreography({ ids, finals, stagger: 0.55 });
  const byStep = steps
    ? new AssemblyChoreography({ ids, finals, order: steps, stagger: 0.55 })
    : null;

  const out = [0, 0, 0];
  const out2 = [0, 0, 0];
  const samples = [];
  for (const t of [0.25, 0.5, 0.75]) {
    let worst = 0;
    let moved = 0;
    for (let i = 0; i < ids.length; i++) {
      byIndex.positionAt(i, t, out);
      if (byStep) {
        byStep.positionAt(i, t, out2);
        const d = Math.hypot(out[0] - out2[0], out[1] - out2[1], out[2] - out2[2]);
        if (d > 0.01) moved++;
        worst = Math.max(worst, d);
      }
    }
    samples.push({ t, worstMm: worst, movedInstances: moved });
  }

  return {
    ok: true,
    instances: ids.length,
    hasBuildSteps: !!steps,
    distinctSteps: steps ? Array.from(new Set(steps)).length : 0,
    samples,
  };
});

// ---------------------------------------------------------------------------
// AC2 — what the per-frame transform pass costs, and a picture
// ---------------------------------------------------------------------------
const bench = await page.evaluate(() => window.__spexMesh.benchTransforms(9));

const shots = await page.evaluate(async () => {
  const { AssemblyChoreography } = globalThis.__spexChoreo;
  const m = window.__spexMesh;
  const post = m.post();
  const THREE_pos = m.camera.position.clone();
  const ids = [];
  const finals = [];
  for (const g of m.groups) {
    for (let i = 0; i < g.ids.length; i++) {
      ids.push(g.ids[i]);
      finals.push(g.matrices[i * 16 + 12], g.matrices[i * 16 + 13], g.matrices[i * 16 + 14]);
    }
  }
  const choreo = new AssemblyChoreography({ ids, finals, stagger: 0.55 });
  const pos = THREE_pos.clone();
  const quat = { set: () => {} , x:0,y:0,z:0,w:1 };
  // The writer wants real three objects; borrow the ones the viewer has.
  const p = m.camera.position.clone();
  const q = m.camera.quaternion.clone();

  const gl = m.renderer.getContext();
  const w = m.renderer.domElement.width;
  const h = m.renderer.domElement.height;
  const buf = new Uint8Array(w * h * 4);
  const b64 = (bytes) => {
    let s = '';
    for (let i = 0; i < bytes.length; i += 8192) s += String.fromCharCode.apply(null, bytes.subarray(i, i + 8192));
    return btoa(s);
  };

  const out = {};
  for (const t of [0, 0.5, 1]) {
    choreo.apply(t, m.writer, p, q);
    m.writer.flush();
    // Load-bearing, and the first version of this harness did not do it: since
    // M59 the LOD selector is what copies `group.matrices` into the meshes the
    // GPU actually reads. Without this the numbers above all passed and every
    // screenshot was of a car that had never moved.
    m.lod()?.update(m.camera, h);
    m.edges.update(m.camera, h);
    post.render(t);
    gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, buf);
    out[String(t)] = { w, h, data: b64(buf) };
  }
  return out;
});

await browser.close();

for (const [t, shot] of Object.entries(shots)) {
  const png = new PNG({ width: shot.w, height: shot.h });
  const raw = Buffer.from(shot.data, 'base64');
  for (let y = 0; y < shot.h; y++) {
    const src = (shot.h - 1 - y) * shot.w * 4;
    raw.copy(png.data, y * shot.w * 4, src, src + shot.w * 4);
  }
  const path = join(outDir, `assembly-t${t.replace('.', '')}.png`);
  writeFileSync(path, PNG.sync.write(png));
  console.log(`wrote ${path}`);
}

// ---------------------------------------------------------------------------
console.log(`\nthe port — TypeScript splitmix64 against the Rust fixture`);
console.log(`  ${agreement.components} components, ${fixture.cases.length} editions: worst |TS - Rust| = ${agreement.worstScatter.toExponential(3)} LDU`);

console.log(`\nAC1 — runtime assembly vs the baked one (stagger 0, the baked demo's own configuration)`);
console.log(`  ${agreement.frames} frames x 9 parts: worst position difference ${agreement.worstBaked.toExponential(3)} mm (asked for < 0.01)`);
console.log(`  at t01 = 1, worst distance from the bundle's own placement: ${agreement.worstFinal.toExponential(3)} mm`);
console.log(`  scatter starts ${agreement.upMm.toFixed(1)} mm above the final position — positive, so the LDU->mm mirror is applied`);

console.log(`\nAC2 — per-frame transform cost (median of 9 full passes, ${bench.instances ?? '?'} instances)`);
console.log(`  compose (position/quaternion/scale): ${bench.composeMs.toFixed(2)} ms`);
console.log(`  matrix  (what a curve produces):     ${bench.matrixMs.toFixed(2)} ms`);
console.log(`  no frame rate is asserted: this container has no GPU, and the CPU pass is what the show's budget is built on`);

console.log(`\nAC3 — build-step order vs index order`);
if (!stagger.ok) {
  console.log(`  not measured: ${stagger.why}`);
} else if (!stagger.hasBuildSteps) {
  console.log(`  this scene has no 0 STEP markers (${stagger.instances} instances) — nothing to compare`);
} else {
  console.log(`  ${stagger.instances} instances, ${stagger.distinctSteps} distinct build steps`);
  for (const s of stagger.samples) {
    console.log(`  t01=${s.t}: ${s.movedInstances} instance(s) in a different place, worst ${s.worstMm.toFixed(1)} mm apart`);
  }
}

if (errors.length) {
  console.log('\nconsole errors:');
  for (const e of errors) console.log(`  ${e}`);
}

const failed =
  agreement.worstScatter > 1e-9 ||
  agreement.worstBaked >= 0.01 ||
  agreement.worstFinal >= 0.01 ||
  agreement.upMm <= 0 ||
  errors.length > 0;
console.log(`\n${failed ? 'FAIL' : 'ok'}`);
process.exit(failed ? 1 : 0);
