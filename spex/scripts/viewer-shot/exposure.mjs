#!/usr/bin/env node
/** Deciding an exposure by measuring it, rather than by looking at one frame.
 *
 *   spex show <show-dir> --port 8111 --no-open &
 *   node scripts/viewer-shot/exposure.mjs http://127.0.0.1:8111/ /tmp/expo 73.4 \
 *     '[0.7,0.55,0.45,0.35,0.28,0.22]'
 *
 * This has now been the deciding instrument twice — A4-S01b's last brick and
 * A1-S06's sarsens — so it is a file rather than a thing retyped each time.
 *
 * # The number to read is the spread, not the median
 *
 * A blown-out frame in this piece is almost never a *clipped* frame. The grade
 * pass ends in ACES, and ACES has a long shoulder: push a lit stone surface up
 * and it does not slam into 255, it slides into the part of the curve where a
 * face turned 40 degrees away from the light and a face turned straight at it
 * both come out at 207. Nothing is clipped, no histogram warns, and the object
 * reads as a white cut-out with a silhouette and no sides.
 *
 * So the two numbers this prints are the median (which says *how bright*) and
 * p95 - p5 (which says *how much of the object is still distinguishable from
 * the rest of it*). The rule that has held both times:
 *
 *   - the median should land near the material's own sRGB value, which is
 *     `baseColor` from `mesh.json` through the sRGB transfer function; and
 *   - past the exposure where the spread stops growing, darkening buys
 *     nothing — the shoulder is already behind you.
 *
 * Both together, because either alone can be satisfied by a picture nobody
 * wants: an object at exactly its albedo under no light has a perfect median
 * and no spread at all.
 *
 * `lum > 40` is the object/background split. It works because every frame this
 * has been pointed at is a lit object on the piece's near-black ground; a shot
 * with a bright background needs a different mask and should say so here.
 */
import { chromium } from 'playwright';

const [base, outDir, timeArg, stepsArg] = process.argv.slice(2);
if (!base || !outDir || !timeArg || !stepsArg) {
  console.error('usage: exposure.mjs <base-url> <out-dir> <show-seconds> <json-array-of-exposures>');
  process.exit(2);
}
const t = Number(timeArg);
const steps = JSON.parse(stepsArg);

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 960, height: 600 } });
// `?quality=low` because this measures the grade, not the LOD selector, and a
// software rasteriser spends minutes per frame at the other tiers. `?mute=1`
// because a headless Chromium has no audio device.
await page.goto(`${base}?quality=low&mute=1`, { waitUntil: 'networkidle' });
await page.waitForFunction(() => !!window.__spexShow, null, { timeout: 180000 });
await page.evaluate(() => window.__spexShow.begin());
await page.waitForTimeout(3000);

for (const e of steps) {
  await page.evaluate(async ([sec, expo]) => {
    const s = window.__spexShow;
    s.setPlaying(false);
    s.seek(sec);
    // The exposure is set *after* the seek on purpose: the seek re-evaluates
    // every live track, and a post track would otherwise write the authored
    // value straight back over the one being tested.
    s.post().exposure = expo;
    for (let i = 0; i < 3; i++) {
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    }
  }, [t, e]);
  await page.waitForTimeout(200);
  await page.screenshot({ path: `${outDir}/expo-${e}.png`, timeout: 120000 });
  console.log(`exposure ${e} -> ${outDir}/expo-${e}.png`);
}
await browser.close();
console.log(`\nnow measure them, e.g.:\n  python3 -c "` +
  `from PIL import Image; import numpy as np, glob\n` +
  `for f in sorted(glob.glob('${outDir}/expo-*.png')):\n` +
  `    a=np.asarray(Image.open(f).convert('RGB')).astype(int)\n` +
  `    l=(0.2126*a[:,:,0]+0.7152*a[:,:,1]+0.0722*a[:,:,2]); s=l[l>40]\n` +
  `    p5,p50,p95=np.percentile(s,[5,50,95])\n` +
  `    print(f'{f} median {p50:.1f} spread {p95-p5:.1f}')"`);
