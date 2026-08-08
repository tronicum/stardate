/** Absence is how this viewer decides what it is looking at — and a probe that
 * counts absence as an error cries wolf for ever.
 *
 * `main.ts` picks between three render modes with one branch each: it asks for
 * `show-resolved.json`, then for `mesh.json`, and takes whichever answers. A
 * 404 there is a *fact*, and the modules say so in as many words
 * (`fetchResolvedShow`, `fetchMeshBundle`, `fetchCutsIndex`, `fetchScore`).
 * The browser still logs every failed request as a console error, so since M66
 * every probe that gates on "zero console errors" has been failing on a plain
 * mesh bundle for two 404s that are the design working.
 *
 * Phase 3's rung 6 is what surfaced it: `dissolve.mjs` printed a clean set of
 * numbers and then `FAIL`, on a demo nothing was wrong with.
 *
 * So: attach this to a page, and it separates the console errors that mean
 * something from the ones that are the viewer asking a question and getting
 * "no". It matches on the *URL of the failed response*, not on the console
 * text — the text is the same sentence for every 404 there has ever been.
 */

/** Paths whose absence is a documented answer rather than a fault. */
export const ABSENT_BY_DESIGN = [
  /\/show-resolved(-[a-z0-9]+)?\.json$/,
  /\/cuts\.json$/,
  /\/mesh\.json$/,
  /\/nodes\.json$/,
  /\/meta\.json$/,
  /\/sequence\.json$/,
  /\/fugue\.mid$/,
  /\/favicon\.ico$/,
];

/** Watches a page and sorts its console errors into the two kinds.
 *
 * Returns `{ errors, byDesign, all }` — live arrays, read them after the run.
 * `errors` is what a probe should gate on.
 */
export function watchConsole(page, { extraByDesign = [] } = {}) {
  const patterns = [...ABSENT_BY_DESIGN, ...extraByDesign];
  const errors = [];
  const byDesign = [];
  const all = [];
  /** How many designed 404s have been seen but not yet matched to a console
   * line. The two arrive as separate events and in no guaranteed order, so
   * this is a count rather than a correlation — which is enough, because the
   * question is only how many of the 404 lines to discount. */
  let pendingDesigned = 0;
  const drain = () => {
    for (let i = errors.length - 1; i >= 0 && pendingDesigned > 0; i--) {
      if (/Failed to load resource/.test(errors[i])) {
        byDesign.push(errors.splice(i, 1)[0]);
        pendingDesigned--;
      }
    }
  };

  page.on('response', (r) => {
    if (r.status() !== 404) return;
    if (!patterns.some((p) => p.test(new URL(r.url()).pathname))) return;
    pendingDesigned++;
    drain();
  });
  page.on('console', (m) => {
    if (m.type() !== 'error') return;
    all.push(m.text());
    errors.push(m.text());
    drain();
  });
  page.on('pageerror', (e) => {
    all.push(String(e));
    errors.push(String(e));
  });

  return { errors, byDesign, all };
}

/** The same thing, pushing into a probe's own `errors` array.
 *
 * One line to replace two, so an existing probe keeps its own variable and its
 * own verdict and only stops counting the viewer's mode test as a fault.
 * Returns the array of discounted lines, for printing.
 */
export function attachConsole(page, errors, opts = {}) {
  const w = watchConsole(page, opts);
  // `watchConsole` owns its own arrays; mirror the real errors into the
  // caller's. A getter would be neater and would not survive `errors.length`
  // being read by code that has no idea this exists.
  const push = w.errors.push.bind(w.errors);
  w.errors.push = (...items) => {
    errors.push(...items);
    return push(...items);
  };
  const splice = w.errors.splice.bind(w.errors);
  w.errors.splice = (...args) => {
    const removed = splice(...args);
    for (const r of removed) {
      const i = errors.indexOf(r);
      if (i >= 0) errors.splice(i, 1);
    }
    return removed;
  };
  return w.byDesign;
}
