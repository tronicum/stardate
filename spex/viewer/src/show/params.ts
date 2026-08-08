/** M66 — the eight URL parameters, parsed in one place.
 *
 * A screening is a URL. That is not a convenience: the piece has four
 * durations, a seed per edition, a quality tier and a debug overlay, and the
 * only way a projectionist, an archivist or a future maintainer can pin all of
 * those down and hand them to someone else is a line of text. So the
 * parameters are the show's real interface, and they live here rather than
 * being read three characters at a time wherever they happen to be needed.
 *
 * # Nothing is silently ignored
 *
 * `?duration=250` is a typo, not a request. The temptation is to fall back to
 * the default and say nothing, which is exactly the failure this project keeps
 * finding: a screening runs for four minutes when someone asked for ten, and
 * nothing anywhere reports it. So every rejected value lands in `warnings`,
 * which the console prints and the director HUD shows.
 *
 * And nothing here throws. A URL is user input; a mistyped parameter must not
 * produce a blank page.
 */

import type { QualityTier } from '../mesh/post';

/** `?duration=` — which cut to play.
 *
 * A **string, not an enum of four**. The spec names 240, 600, 3600 and
 * endless, and those are the four the piece is authored for — but
 * `spex show-build --duration` takes any beat-integral length, and the first
 * thing a whitelist of four did was reject `?duration=120` for a directory
 * that contained a 120-second cut. The list that decides what is valid is the
 * one in `cuts.json`, because that is the list of cuts that exist.
 *
 * `endless` is not a length: it is the canonical cut with looping declared,
 * which is why it sits in this parameter rather than beside it. */
export type CutLabel = string;
/** The four the piece is authored for — documentation and the CLI's own
 * suggestions, not a filter. */
export const CANONICAL_CUT_LABELS: readonly string[] = ['240', '600', '3600', 'endless'];

export interface ShowParams {
  /** `?t=` — seek here on load. Seconds. */
  seekSec: number | null;
  /** `?duration=` — which resolved cut to play, if the directory has several. */
  cut: CutLabel | null;
  /** `?seed=` — edition seed, overriding the resolved document's own. */
  seed: number | null;
  /** `?quality=` — overrides the two-second benchmark. */
  quality: QualityTier | null;
  /** `?mute=1` — start with no `AudioContext` at all.
   *
   * Not a gain of zero. Until M71 there is nothing to hear, and what this
   * really controls is **which clock the show reads**: with an AudioContext,
   * `ShowClock` runs on the audio hardware's own oscillator; without one it
   * falls back to `performance.now()`. That is the observable difference
   * today, it is exactly the hook M71's autoplay policy needs, and it is
   * documented as that rather than as a volume control it is not. */
  muted: boolean;
  /** `?free=1` — the mouse drives the camera; the timeline keeps running. */
  freeCamera: boolean;
  /** `?loop=0` — play once and hold the final frame. `null` means "do what
   * the resolved document says", which for an `endless` cut is to loop. */
  loop: boolean | null;
  /** `?director=1` — the director HUD. */
  director: boolean;
  /** Every value that was rejected, and why. Never empty silently. */
  warnings: string[];
}

/** A `?flag=1`-style boolean. Present-but-empty (`?director`) counts as true —
 * a URL typed by hand drops the `=1` often enough that treating it as false
 * would be a trap. `0`, `false` and `no` are the ways to say no. */
function boolParam(raw: string | null): boolean | null {
  if (raw === null) return null;
  const v = raw.trim().toLowerCase();
  if (v === '' || v === '1' || v === 'true' || v === 'yes') return true;
  if (v === '0' || v === 'false' || v === 'no') return false;
  return null;
}

export function parseShowParams(
  search: string = typeof location === 'undefined' ? '' : location.search,
): ShowParams {
  const q = new URLSearchParams(search);
  const warnings: string[] = [];

  const out: ShowParams = {
    seekSec: null,
    cut: null,
    seed: null,
    quality: null,
    muted: false,
    freeCamera: false,
    loop: null,
    director: false,
    warnings,
  };

  const t = q.get('t');
  if (t !== null) {
    const v = Number(t);
    // `Number('')` is 0 and `Number('abc')` is NaN — the first is a real
    // request to seek to the start, the second is a typo, and they must not
    // land in the same branch.
    if (t.trim() === '' || !Number.isFinite(v) || v < 0) {
      warnings.push(`?t=${t} is not a time in seconds; ignored`);
    } else {
      out.seekSec = v;
    }
  }

  const duration = q.get('duration');
  if (duration !== null) {
    // Not validated here — `chooseCut` checks it against the cuts that were
    // actually built, which is the only list that can be right.
    if (duration.trim() === '') warnings.push('?duration= is empty; ignored');
    else out.cut = duration.trim();
  }

  const seed = q.get('seed');
  if (seed !== null) {
    const v = Number(seed);
    if (!Number.isFinite(v) || !Number.isInteger(v) || v < 0) {
      warnings.push(`?seed=${seed} is not a non-negative integer; ignored`);
    } else {
      out.seed = v;
    }
  }

  const quality = q.get('quality');
  if (quality !== null) {
    if (quality === 'low' || quality === 'medium' || quality === 'high') {
      out.quality = quality;
    } else {
      warnings.push(`?quality=${quality} is not low, medium or high; ignored`);
    }
  }

  for (const [name, key] of [
    ['mute', 'muted'],
    ['free', 'freeCamera'],
    ['director', 'director'],
  ] as const) {
    const raw = q.get(name);
    if (raw === null) continue;
    const v = boolParam(raw);
    if (v === null) warnings.push(`?${name}=${raw} is not a boolean; ignored`);
    else (out as unknown as Record<string, boolean>)[key] = v;
  }

  const loop = q.get('loop');
  if (loop !== null) {
    const v = boolParam(loop);
    if (v === null) warnings.push(`?loop=${loop} is not a boolean; ignored`);
    else out.loop = v;
  }

  return out;
}

/** Renders a parameter set back to a query string — the round trip a
 * projectionist actually needs: set it up by hand, then copy a URL that
 * reproduces exactly this. Only non-default values appear. */
export function showParamsToQuery(p: ShowParams): string {
  const q = new URLSearchParams();
  if (p.seekSec !== null) q.set('t', String(p.seekSec));
  if (p.cut !== null) q.set('duration', p.cut);
  if (p.seed !== null) q.set('seed', String(p.seed));
  if (p.quality !== null) q.set('quality', p.quality);
  if (p.muted) q.set('mute', '1');
  if (p.freeCamera) q.set('free', '1');
  if (p.loop === false) q.set('loop', '0');
  if (p.director) q.set('director', '1');
  const s = q.toString();
  return s ? `?${s}` : '';
}

/** The show directory's index of available cuts (`cuts.json`).
 *
 * A show directory holds one resolved document per cut, and every one of them
 * shares the same `bundles/` — the geometry does not change between a
 * four-minute and a sixty-minute screening, only the timeline does. This file
 * is how the viewer knows which cuts were actually built without probing for
 * four filenames and treating three 404s as normal. */
export interface CutsIndex {
  version: number;
  default: string;
  cuts: Array<{ label: string; durationSec: number; endless: boolean; file: string }>;
}

export const CUTS_INDEX_VERSION = 1;

/** Fetches `cuts.json`, or `null` when there is none.
 *
 * Absence is not an error: a directory built before M66, or by hand, still has
 * a `show-resolved.json` and still plays. It just has one cut. */
export async function fetchCutsIndex(baseUrl: string): Promise<CutsIndex | null> {
  const res = await fetch(`${baseUrl.replace(/\/$/, '')}/cuts.json`);
  if (!res.ok) return null;
  const index = (await res.json()) as CutsIndex;
  if (index.version !== CUTS_INDEX_VERSION) {
    throw new Error(
      `cuts.json is version ${index.version}, this viewer reads ${CUTS_INDEX_VERSION}. ` +
        'Rebuild the show directory with `spex show-build`.',
    );
  }
  return index;
}

/** Which file to load, given the index and `?duration=`.
 *
 * Returns the default cut and a warning when the requested one was not built —
 * playing the wrong length loudly beats refusing to play at all, and beats
 * playing it quietly by a much larger margin. */
export function chooseCut(
  index: CutsIndex | null,
  wanted: CutLabel | null,
  warnings: string[],
): string {
  if (!index) {
    if (wanted !== null) {
      warnings.push(`?duration=${wanted}: this show directory has only one cut (no cuts.json)`);
    }
    return 'show-resolved.json';
  }
  const fallback = index.cuts.find((c) => c.label === index.default) ?? index.cuts[0];
  if (wanted === null) return fallback?.file ?? 'show-resolved.json';
  const hit = index.cuts.find((c) => c.label === wanted);
  if (hit) return hit.file;
  warnings.push(
    `?duration=${wanted} was not built into this show directory ` +
      `(it has ${index.cuts.map((c) => c.label).join(', ')}); playing ${fallback?.label} instead`,
  );
  return fallback?.file ?? 'show-resolved.json';
}
