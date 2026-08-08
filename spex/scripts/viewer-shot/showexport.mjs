#!/usr/bin/env node
/** M66 AC2 — does the static export play where it is put?
 *
 *   spex show-export demos/matrix -o /tmp/m66-static
 *   node scripts/viewer-shot/showexport.mjs /tmp/m66-static
 *
 * Three hostings, asked the same question: does the show reach `__spexShow`
 * with no console errors, having actually fetched its data?
 *
 *   1. a static server at the domain root         http://127.0.0.1:PORT/
 *   2. the same files under a deep subpath        http://127.0.0.1:PORT/a/b/c/
 *   3. `file://`
 *
 * (2) is the real criterion — it is what a GitHub Pages project site does, and
 * it is what `export_static.rs`'s relative-path discipline exists for. (3) is
 * in the spec and is measured here rather than assumed, because the answer
 * turns out to be no and the reason is not this export's doing.
 */
import { chromium } from 'playwright';
import { attachConsole } from './absence.mjs';
import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { join, extname, resolve } from 'node:path';

const dir = resolve(process.argv[2] ?? '/tmp/m66-static');
const PORT = 8129;
const PREFIX = '/a/b/c';

const MIME = {
  '.html': 'text/html', '.js': 'text/javascript', '.json': 'application/json',
  '.bin': 'application/octet-stream', '.css': 'text/css',
};

const server = createServer(async (req, res) => {
  let path = decodeURIComponent(req.url.split('?')[0]);
  // The same bytes served twice: once at the root, once under a deep prefix.
  if (path.startsWith(PREFIX)) path = path.slice(PREFIX.length) || '/';
  if (path.endsWith('/')) path += 'index.html';
  const file = join(dir, path);
  try {
    await stat(file);
    res.writeHead(200, { 'content-type': MIME[extname(file)] ?? 'application/octet-stream' });
    res.end(await readFile(file));
  } catch {
    res.writeHead(404).end('not found');
  }
});
await new Promise((r) => server.listen(PORT, '127.0.0.1', r));

const browser = await chromium.launch();

async function tryUrl(label, url) {
  const errors = [];
  const requests = [];
  const page = await browser.newPage({ viewport: { width: 640, height: 400 } });
  // The 404s that are the viewer's mode test answering "no" are not errors
  // (absence.mjs). Phase 3's rung 6 surfaced it: probes printed clean numbers
  // and then FAIL, on demos nothing was wrong with.
  const byDesign = attachConsole(page, errors);
  page.on('requestfailed', (r) => requests.push(`${r.url()} ${r.failure()?.errorText}`));
  page.on('response', (r) => { if (r.status() >= 400) requests.push(`${r.url()} ${r.status()}`); });
  let ok = false;
  let info = null;
  try {
    await page.goto(url, { waitUntil: 'load' });
    await page.waitForFunction(() => !!window.__spexShow, null, { timeout: 60000 });
    await page.waitForTimeout(2500);
    info = await page.evaluate(() => {
      const s = window.__spexShow;
      return {
        title: s.show.title,
        durationSec: s.show.durationSec,
        scenes: s.scenes.map((x) => `${x.id}:${x.instanceIds.length}`),
        drawCalls: s.drawCalls(),
      };
    });
    ok = true;
  } catch (e) {
    info = { failed: String(e).split('\n')[0] };
  }
  console.log(`\n${label}\n  ${url}`);
  console.log(`  loaded: ${ok}`);
  if (info) console.log(`  ${JSON.stringify(info)}`);
  console.log(`  console errors: ${errors.length}${errors.length ? `\n    ! ${errors.slice(0, 3).join('\n    ! ')}` : ''}`);
  if (requests.length) console.log(`  failed requests: ${requests.length}\n    ! ${requests.slice(0, 3).join('\n    ! ')}`);
  await page.close();
  return ok;
}

const root = await tryUrl('1. static server, domain root', `http://127.0.0.1:${PORT}/`);
const sub = await tryUrl('2. static server, deep subpath', `http://127.0.0.1:${PORT}${PREFIX}/`);
const file = await tryUrl('3. file://', `file://${dir}/index.html`);

console.log(`\nroot ${root ? 'ok' : 'FAIL'} · subpath ${sub ? 'ok' : 'FAIL'} · file:// ${file ? 'ok' : 'FAIL'}`);
await browser.close();
server.close();
