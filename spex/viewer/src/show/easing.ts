/** M62 — the easing library.
 *
 * Named, pure, `(t: number) => number` on [0,1]. Nothing here reads a clock,
 * a scene or a document; that is what makes them testable in isolation and
 * what makes a curve mean the same thing in the fugue, the camera and the
 * HUD.
 *
 * `cubicInOut` is a **port** of `crates/spex-cli/src/brick.rs`'s
 * `ease_in_out_cubic`, not a re-derivation. The existing `brick-assembly`
 * demo settles nine bricks with that exact curve, and M64's acceptance
 * criterion is that the runtime assembly agrees with the baked one to within
 * 0.01 mm. Two independently written cubics agree to about three decimals,
 * which is four orders of magnitude short.
 *
 * The whole set is exported through `EASINGS` so a resolved document's
 * `easing` string maps to a function by lookup rather than a `switch` that
 * someone has to remember to extend.
 */

export type EasingFn = (t: number) => number;

const clamp01 = (t: number) => (t < 0 ? 0 : t > 1 ? 1 : t);

export const linear: EasingFn = (t) => clamp01(t);

export const quadIn: EasingFn = (t) => {
  t = clamp01(t);
  return t * t;
};
export const quadOut: EasingFn = (t) => {
  t = clamp01(t);
  return 1 - (1 - t) * (1 - t);
};
export const quadInOut: EasingFn = (t) => {
  t = clamp01(t);
  return t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
};

export const cubicIn: EasingFn = (t) => {
  t = clamp01(t);
  return t * t * t;
};
export const cubicOut: EasingFn = (t) => {
  t = clamp01(t);
  return 1 - Math.pow(1 - t, 3);
};
/** Port of `brick.rs::ease_in_out_cubic`. Same branch, same expressions. */
export const cubicInOut: EasingFn = (t) => {
  t = clamp01(t);
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
};

export const quartInOut: EasingFn = (t) => {
  t = clamp01(t);
  return t < 0.5 ? 8 * t * t * t * t : 1 - Math.pow(-2 * t + 2, 4) / 2;
};

// The exponential family is the one place the textbook definition is not
// self-consistent at the endpoints: 2^(-10*0) is 1, not 0. The special cases
// are load-bearing, not defensive.
export const expoIn: EasingFn = (t) => {
  t = clamp01(t);
  return t === 0 ? 0 : Math.pow(2, 10 * t - 10);
};
export const expoOut: EasingFn = (t) => {
  t = clamp01(t);
  return t === 1 ? 1 : 1 - Math.pow(2, -10 * t);
};
export const expoInOut: EasingFn = (t) => {
  t = clamp01(t);
  if (t === 0) return 0;
  if (t === 1) return 1;
  return t < 0.5 ? Math.pow(2, 20 * t - 10) / 2 : (2 - Math.pow(2, -20 * t + 10)) / 2;
};

export const circInOut: EasingFn = (t) => {
  t = clamp01(t);
  return t < 0.5
    ? (1 - Math.sqrt(1 - Math.pow(2 * t, 2))) / 2
    : (Math.sqrt(1 - Math.pow(-2 * t + 2, 2)) + 1) / 2;
};

/** Overshoots past 1 and comes back. Not monotonic, on purpose. */
export const backOut: EasingFn = (t) => {
  t = clamp01(t);
  const c1 = 1.70158;
  const c3 = c1 + 1;
  return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
};

/** Also not monotonic. Both endpoints are still exact. */
export const elasticOut: EasingFn = (t) => {
  t = clamp01(t);
  if (t === 0) return 0;
  if (t === 1) return 1;
  const c4 = (2 * Math.PI) / 3;
  return Math.pow(2, -10 * t) * Math.sin((t * 10 - 0.75) * c4) + 1;
};

export const bounceOut: EasingFn = (t) => {
  t = clamp01(t);
  const n1 = 7.5625;
  const d1 = 2.75;
  if (t < 1 / d1) return n1 * t * t;
  if (t < 2 / d1) return n1 * (t -= 1.5 / d1) * t + 0.75;
  if (t < 2.5 / d1) return n1 * (t -= 2.25 / d1) * t + 0.9375;
  return n1 * (t -= 2.625 / d1) * t + 0.984375;
};

/** Hold, then jump. A1-S03's edge lines arrive in one frame; a fade would
 * make legibility gradual, and the shot is about it not being gradual. */
export const step: EasingFn = (t) => (t >= 1 ? 1 : 0);

export const smootherstep: EasingFn = (t) => {
  t = clamp01(t);
  return t * t * t * (t * (t * 6 - 15) + 10);
};

/** CSS-style cubic Bézier through (0,0) and (1,1) with two control points.
 *
 * Newton–Raphson on x, falling back to bisection when the derivative is flat
 * — which it genuinely is for curves like `cubicBezier(1, 0, 1, 1)`, where
 * Newton alone wanders off and returns a y for the wrong x.
 */
export function cubicBezier(x1: number, y1: number, x2: number, y2: number): EasingFn {
  const a = (a1: number, a2: number) => 1 - 3 * a2 + 3 * a1;
  const b = (a1: number, a2: number) => 3 * a2 - 6 * a1;
  const c = (a1: number) => 3 * a1;
  const calc = (t: number, a1: number, a2: number) =>
    ((a(a1, a2) * t + b(a1, a2)) * t + c(a1)) * t;
  const slope = (t: number, a1: number, a2: number) =>
    3 * a(a1, a2) * t * t + 2 * b(a1, a2) * t + c(a1);

  return (x) => {
    x = clamp01(x);
    if (x1 === y1 && x2 === y2) return x; // the identity curve
    let t = x;
    for (let i = 0; i < 8; i++) {
      const d = slope(t, x1, x2);
      if (Math.abs(d) < 1e-6) break;
      const err = calc(t, x1, x2) - x;
      if (Math.abs(err) < 1e-7) return calc(t, y1, y2);
      t -= err / d;
    }
    let lo = 0;
    let hi = 1;
    t = x;
    for (let i = 0; i < 24; i++) {
      const v = calc(t, x1, x2);
      if (Math.abs(v - x) < 1e-7) break;
      if (v > x) hi = t;
      else lo = t;
      t = (lo + hi) / 2;
    }
    return calc(t, y1, y2);
  };
}

/** Every name a resolved document may use, and the function it means.
 *
 * The keys are exactly `spec/show-resolved.schema.json`'s `easing` enum. A
 * document cannot name a curve that is not here, because the schema is a
 * closed set — and if one ever did, `easingByName` says so out loud rather
 * than quietly playing it linear. */
export const EASINGS = {
  linear,
  quadIn,
  quadOut,
  quadInOut,
  cubicIn,
  cubicOut,
  cubicInOut,
  expoIn,
  expoOut,
  step,
  // Not in the schema's enum, but authored curves and internal use want them.
  quartInOut,
  circInOut,
  backOut,
  elasticOut,
  bounceOut,
  smootherstep,
} as const;

export type EasingName = keyof typeof EASINGS;

export function easingByName(name: string): EasingFn {
  const fn = (EASINGS as Record<string, EasingFn | undefined>)[name];
  if (!fn) {
    // Falling back silently would make a typo in a document look like a
    // deliberate linear ramp, which is exactly the kind of wrong nobody
    // notices until the piece is on a wall.
    throw new Error(`unknown easing ${JSON.stringify(name)} — see EASINGS`);
  }
  return fn;
}
