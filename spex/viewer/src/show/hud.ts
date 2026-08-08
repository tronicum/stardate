/** M66 — the HUD, as DOM.
 *
 * Every word the piece puts on screen is an HTML element over the canvas, not
 * a texture inside it. That is the same decision `#labels` made in M15 and it
 * is worth restating, because for a work whose subject is *legibility* the
 * typography is not decoration:
 *
 * - A browser hints, kerns and subpixel-positions text that a canvas atlas
 *   cannot, at any resolution, at zero cost.
 * - The text stays selectable and readable by a screen reader, which for an
 *   archival work is the difference between a caption and a picture of one.
 * - It survives the post chain untouched. A caption pushed through bloom,
 *   ACES and a grain pass is a caption with a halo and a wobble.
 *
 * The cost is that the HUD cannot occlude or be occluded by geometry. Nothing
 * in the screenplay asks it to.
 *
 * # Elements are addressed by name, and an unknown name is not an error
 *
 * A `hud` track carries `element: "monolith-metrics"` and a 0..1 value; a
 * `hud` cue carries `{element, text}`. Four names have real layout —
 * `seed-point`, `caption`, `monolith-metrics`, `credits` — and anything else
 * becomes a generic card. That is deliberate: M80's Atlas chronicle cards and
 * M84's credits are authored in the document, and a document from a later
 * version of the piece should degrade to "that card is plainer than intended",
 * never to a blank page or a thrown exception. Same principle as
 * `assemblyFromCue`.
 *
 * # `seed-point` is why this file exists at all
 *
 * A1-S01 is two bars of black with one Terminalgrün point at frame centre, and
 * that point is **a HUD element rather than geometry** — which is what lets it
 * be one *device* pixel rather than one projected millimetre. The last frame
 * of the piece has to be byte-identical to the first, and a projected
 * millimetre is a different number of pixels on every screen the work is ever
 * shown on. There is no mesh in the opening shot.
 */

import type { ResolvedShow, ResolvedShot } from './resolved';

/** How long a movement title card is on screen, in bars — and capped at a
 * quarter of the movement, so a short act does not spend its opening under its
 * own title. 2.5 bars at 84 bpm is 7.1 s: a beat to fade in, a bar and a half
 * to read, a beat to leave. */
export const TITLE_CARD_BARS = 2.5;
export const TITLE_CARD_MAX_FRACTION = 0.25;

/** Terminalgrün, the piece's own signal colour, as sRGB for CSS.
 *
 * The palette in the document is **linear** (every colour in this project is,
 * see `mesh/bundle.ts`), and CSS is sRGB — so this is the one place the
 * conversion has to happen, and it happens explicitly rather than by handing a
 * linear triple to a browser that will read it as sRGB and render the piece's
 * signal colour a stop and a half too dark. */
export function linearToCss(rgb: readonly [number, number, number]): string {
  const enc = (c: number) => {
    const v = c <= 0.0031308 ? c * 12.92 : 1.055 * Math.pow(c, 1 / 2.4) - 0.055;
    return Math.round(Math.min(1, Math.max(0, v)) * 255);
  };
  return `rgb(${enc(rgb[0])}, ${enc(rgb[1])}, ${enc(rgb[2])})`;
}

/** What the director HUD reports. Everything here is *measured* — read off
 * the renderer's own counters or the clock — rather than assumed, which is
 * what makes `?director=1` usable as an instrument and not just a caption. */
export interface DirectorInfo {
  shot: ResolvedShot | null;
  timeSec: number;
  durationSec: number;
  cycle: number;
  fps: number;
  drawCalls: number;
  instances: number;
  /** Fugue voices that have entered so far, newest last — from the `audio`
   * cues the timeline has actually fired, not from reading ahead. */
  voices: string[];
  clockSource: 'audio' | 'performance';
  cut: string;
  seed: number;
  warnings: readonly string[];
}

interface MovementSpan {
  id: string;
  numeral: string;
  title: string;
  startSec: number;
  endSec: number;
}

function el(tag: string, id?: string, parent?: HTMLElement): HTMLDivElement {
  const node = document.createElement(tag) as HTMLDivElement;
  if (id) node.id = id;
  if (parent) parent.appendChild(node);
  return node;
}

export class ShowHud {
  readonly root: HTMLDivElement;

  private readonly seedPoint: HTMLDivElement;
  private readonly titleCard: HTMLDivElement;
  private readonly titleNumeral: HTMLDivElement;
  private readonly titleText: HTMLDivElement;
  private readonly caption: HTMLDivElement;
  private readonly metrics: HTMLDivElement;
  private readonly cardHost: HTMLDivElement;
  private readonly credits: HTMLDivElement;
  private readonly creditsInner: HTMLDivElement;
  private readonly director: HTMLDivElement;
  /** Lazily created for element names the layout does not know. */
  private readonly cards = new Map<string, HTMLDivElement>();

  private readonly movements: MovementSpan[];
  private readonly titleCardSec: number;
  private readonly showDirector: boolean;

  constructor(show: ResolvedShow, opts: { director: boolean; parent?: HTMLElement }) {
    this.showDirector = opts.director;
    const parent = opts.parent ?? document.getElementById('app') ?? document.body;

    this.root = el('div', 'show-hud', parent);
    this.seedPoint = el('div', 'show-seed-point', this.root);
    this.titleCard = el('div', 'show-title-card', this.root);
    this.titleNumeral = el('div', 'show-title-numeral', this.titleCard);
    this.titleText = el('div', 'show-title-text', this.titleCard);
    this.caption = el('div', 'show-caption', this.root);
    this.metrics = el('div', 'show-metrics', this.root);
    this.cardHost = el('div', 'show-cards', this.root);
    this.credits = el('div', 'show-credits', this.root);
    this.creditsInner = el('div', 'show-credits-inner', this.credits);
    this.director = el('div', 'show-director', parent);
    this.director.style.display = opts.director ? 'block' : 'none';

    const green = show.palette?.terminalgruen ?? [0, 0.79, 0.03];
    this.seedPoint.style.background = linearToCss(green as [number, number, number]);
    this.seedPoint.style.setProperty('--seed-color', linearToCss(green as [number, number, number]));
    // One *device* pixel. `devicePixelRatio` is read once: it changes only when
    // a window moves between screens, and `resize` is where that is handled.
    this.setPixelRatio(typeof devicePixelRatio === 'number' ? devicePixelRatio : 1);

    this.movements = movementSpans(show);
    const barSec = show.tempo?.barSeconds ?? 20 / 7;
    const shortest = this.movements.reduce(
      (m, s) => Math.min(m, s.endSec - s.startSec),
      Number.POSITIVE_INFINITY,
    );
    this.titleCardSec = Math.min(
      TITLE_CARD_BARS * barSec,
      Number.isFinite(shortest) ? shortest * TITLE_CARD_MAX_FRACTION : TITLE_CARD_BARS * barSec,
    );

    this.creditsInner.textContent = [...(show.credits?.lines ?? []), '', ...(show.credits?.required ?? [])]
      .join('\n');

    // Everything starts hidden. A HUD element that has never been addressed by
    // a track must not appear, or the opening frame — which is supposed to be
    // black with one point in it — is a title card instead.
    for (const node of [this.seedPoint, this.titleCard, this.caption, this.metrics, this.credits]) {
      node.style.opacity = '0';
    }
  }

  setPixelRatio(dpr: number) {
    const px = 1 / Math.max(dpr, 0.5);
    this.seedPoint.style.width = `${px}px`;
    this.seedPoint.style.height = `${px}px`;
  }

  /** A `hud` track's value, 0..1. */
  setValue(element: string, value: number): void {
    const v = value < 0 ? 0 : value > 1 ? 1 : value;
    switch (element) {
      case 'seed-point': {
        this.seedPoint.style.opacity = String(v);
        // The pulse is a glow, not a size change: growing the element would
        // make the point more than one pixel, which is the one thing this
        // element must never be.
        this.seedPoint.style.boxShadow =
          v > 0 ? `0 0 ${(2 + v * 10).toFixed(2)}px ${(v * 2.5).toFixed(2)}px var(--seed-color)` : 'none';
        return;
      }
      case 'caption':
        this.caption.style.opacity = String(v);
        return;
      case 'monolith-metrics':
        this.metrics.style.opacity = String(v);
        return;
      case 'credits': {
        this.credits.style.opacity = String(v > 0 ? 1 : 0);
        // A crawl is a position, not a fade: the value is how far through the
        // roll we are. M84 owns the timing; this owns the transform.
        this.creditsInner.style.transform = `translateY(${(100 - v * 200).toFixed(3)}%)`;
        return;
      }
      default:
        this.card(element).style.opacity = String(v);
    }
  }

  /** A `hud` cue's text. */
  setText(element: string, text: string): void {
    switch (element) {
      case 'caption':
        this.caption.textContent = text;
        return;
      case 'monolith-metrics':
        this.metrics.textContent = text;
        return;
      case 'seed-point':
        return; // a point has no text, and saying so beats rendering one
      default:
        this.card(element).textContent = text;
    }
  }

  private card(name: string): HTMLDivElement {
    let node = this.cards.get(name);
    if (!node) {
      node = el('div', undefined, this.cardHost);
      node.className = 'show-card';
      node.dataset.element = name;
      node.style.opacity = '0';
      this.cards.set(name, node);
    }
    return node;
  }

  /** The movement title card, driven by time rather than by a track.
   *
   * A track would mean every movement carrying the same four keyframes, which
   * is four numbers per act that can disagree with the act's actual boundary.
   * The boundary is already in the resolved document. */
  updateTitleCard(timeSec: number): void {
    const m = this.movements.find((s) => timeSec >= s.startSec && timeSec < s.endSec);
    if (!m) {
      this.titleCard.style.opacity = '0';
      return;
    }
    if (this.titleNumeral.textContent !== m.numeral || this.titleText.textContent !== m.title) {
      this.titleNumeral.textContent = m.numeral;
      this.titleText.textContent = m.title;
    }
    const local = timeSec - m.startSec;
    const span = this.titleCardSec;
    const fade = span * 0.3;
    let a = 0;
    if (local < span) {
      a = local < fade ? local / fade : local > span - fade ? (span - local) / fade : 1;
    }
    this.titleCard.style.opacity = (a < 0 ? 0 : a > 1 ? 1 : a).toFixed(3);
  }

  setDirector(info: DirectorInfo): void {
    if (!this.showDirector) return;
    const shot = info.shot;
    const rows = [
      `${info.timeSec.toFixed(2)} / ${info.durationSec.toFixed(2)} s${info.cycle ? `  · cycle ${info.cycle}` : ''}`,
      `cut ${info.cut}  · seed ${info.seed}  · clock ${info.clockSource}`,
      shot ? `${shot.id}  ${shot.title}` : '—',
      shot
        ? `  ${shot.movementId}  tier ${shot.tier}  ${shot.startSec.toFixed(2)}–${shot.endSec.toFixed(2)} s` +
          `${shot.durationBars !== undefined ? `  (${shot.durationBars} bars)` : ''}`
        : '',
      `${info.fps.toFixed(0)} fps  · ${info.drawCalls} draw calls  · ${info.instances.toLocaleString()} instances`,
      `voices: ${info.voices.length ? info.voices.join(', ') : '—'}`,
    ];
    if (shot?.note) rows.push('', wrap(shot.note, 72));
    if (info.warnings.length) rows.push('', ...info.warnings.map((w) => `! ${w}`));
    this.director.textContent = rows.filter((r) => r !== '').join('\n');
  }

  dispose(): void {
    this.root.remove();
    this.director.remove();
  }
}

/** Each movement's real span, read off the shots rather than restated. */
function movementSpans(show: ResolvedShow): MovementSpan[] {
  const out: MovementSpan[] = [];
  for (const shot of show.shots) {
    const last = out[out.length - 1];
    if (last && last.id === shot.movementId) {
      last.endSec = shot.endSec;
      continue;
    }
    out.push({
      id: shot.movementId,
      numeral: shot.romanNumeral ?? '',
      title: shot.movementTitle ?? '',
      startSec: shot.startSec,
      endSec: shot.endSec,
    });
  }
  return out;
}

/** Hard-wraps at a column, for the monospace director panel. `white-space:
 * pre` there is what keeps the numbers in columns, and it also means a long
 * `note` would otherwise run off the screen. */
function wrap(text: string, cols: number): string {
  const words = text.split(/\s+/);
  const lines: string[] = [];
  let line = '';
  for (const w of words) {
    if (line.length + w.length + 1 > cols && line) {
      lines.push(line);
      line = w;
    } else {
      line = line ? `${line} ${w}` : w;
    }
  }
  if (line) lines.push(line);
  return lines.join('\n');
}
