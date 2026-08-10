/** A4-S03 — a token is emitted by a node, travels one edge, and is absorbed.
 *
 * The shot used to be three bloom pulses on the beat. That is an accent, not a
 * token: it says "something is happening in the lattice" where the screenplay
 * says "each node emits a glyph that travels an edge and is absorbed". The
 * difference matters because the whole act is about the module becoming a unit
 * that MOVES, and a pulse is the one thing that does not move.
 *
 * # Why this is a generator and not a new track kind
 *
 * The first design was a `path` track: a target glob, a list of waypoints, an
 * easing. It would have been megabytes of authored JSON to express what is
 * really one sentence — *tokens hop between the lattice's nodes* — and the
 * document already has the right shape for that sentence. M64's `seed` cue
 * declares a generator and lets the runtime evaluate it, exactly so that a
 * hundred moving things cost one line of screenplay. This is the second user
 * of that decision, and it needed no format change at all: same cue kind, same
 * `positionAt(i, t01, out)` contract the assembly already satisfies.
 *
 * # It is a real walk on the real grid
 *
 * Each token picks a node and a cardinal direction, travels one edge over one
 * hop, and at the far node picks a new direction — a random walk, deterministic
 * from `(editionSeed, instance, hop)` through the same splitmix64 the assembly
 * uses. It is not a set of pre-drawn loops: two renders of one edition are
 * identical, and two different editions are genuinely different traffic.
 *
 * The walk is **reflected at the boundary** rather than wrapped. A token that
 * wraps from one edge of the lattice to the other crosses the whole frame in
 * one frame, which reads as a glitch and not as a token; a reflected one turns
 * around, which is what a bounded network looks like.
 *
 * The arc is the other half of the reading. A token that slides along the
 * lattice plane is hidden by the lattice; one that rises most of a node's
 * spacing over the edge it is crossing is visibly *on a journey between two
 * nodes* — 26 mm against a 32 mm pitch, measured up from a frame, not guessed. The
 * height is a half sine over the hop, so it is zero at both ends — a token is
 * absorbed at the node's own level, not above it.
 */
import { nextFloat, placementSeed } from './choreography';

export interface TokenFlowSpec {
  /** Instance ids of the scene carrying the tokens, in bundle order. */
  ids: string[];
  /** Node (0,0) of the lattice, in millimetres. */
  originMm: [number, number, number];
  /** Distance between adjacent nodes, in millimetres. */
  pitchMm: number;
  /** Nodes per side. The walk is reflected at 0 and `nodes - 1`. */
  nodes: number;
  /** Hops the whole shot is worth. One hop is one edge crossed. */
  hops: number;
  /** Peak height of the travelling arc above the lattice, in millimetres. */
  arcMm: number;
  editionSeed: number;
}

/** The four cardinal steps on the grid, in the order a seeded pick indexes. */
const STEPS: ReadonlyArray<readonly [number, number]> = [
  [1, 0],
  [-1, 0],
  [0, 1],
  [0, -1],
];

/** Reflects a grid coordinate back inside `[0, n - 1]`. */
function reflect(v: number, n: number): number {
  if (v < 0) return -v;
  if (v > n - 1) return 2 * (n - 1) - v;
  return v;
}

export class TokenFlow {
  readonly count: number;
  private readonly spec: TokenFlowSpec;

  constructor(spec: TokenFlowSpec) {
    this.spec = spec;
    this.count = spec.ids.length;
  }

  /** Where token `i` is at `t01` of the shot, in millimetres.
   *
   * Deliberately allocation-free and stateless: the frame loop calls this once
   * per instance per frame, and a seek must land a token exactly where playing
   * to that moment would have. Walking the hops from the start each call is
   * O(hops) — twelve, here — and buys that property outright. A cached
   * incremental walk would be faster and would drift on a seek, which is the
   * bug M66 spent a shot's worth of frames finding in the camera. */
  positionAt(i: number, t01: number, out: [number, number, number]): void {
    const { originMm, pitchMm, nodes, hops, arcMm, editionSeed } = this.spec;
    const t = t01 < 0 ? 0 : t01 > 1 ? 1 : t01;

    // Where this token starts, and its first heading.
    let rng = placementSeed(i, editionSeed);
    let gx = Math.floor(nextFloat(rng) * nodes);
    rng = placementSeed(i + 1013, editionSeed);
    let gz = Math.floor(nextFloat(rng) * nodes);

    const total = t * hops;
    const hop = Math.min(Math.floor(total), hops - 1);
    const u = hops > 0 ? total - hop : 0;

    // Walk the completed hops, then take the one in progress.
    let sx = 0;
    let sz = 0;
    for (let h = 0; h <= hop; h++) {
      const pick = placementSeed(i * 7919 + h, editionSeed);
      const [dx, dz] = STEPS[Math.min(3, Math.floor(nextFloat(pick) * 4))];
      sx = dx;
      sz = dz;
      if (h < hop) {
        gx = reflect(gx + dx, nodes);
        gz = reflect(gz + dz, nodes);
      }
    }
    const nx = reflect(gx + sx, nodes);
    const nz = reflect(gz + sz, nodes);

    const fx = gx + (nx - gx) * u;
    const fz = gz + (nz - gz) * u;

    out[0] = originMm[0] + fx * pitchMm;
    out[1] = originMm[1] + Math.sin(Math.PI * u) * arcMm;
    out[2] = originMm[2] + fz * pitchMm;
  }
}

/** Builds a `TokenFlow` from a `seed` cue's payload, or `null` if the payload
 * is not one. Same shape as `assemblyFromCue`, so `player.ts` can dispatch on
 * `generator` alone and neither generator has to know the other exists. */
export function tokenFlowFromCue(
  payload: Record<string, unknown>,
  ids: string[],
  editionSeed: number,
): TokenFlow | null {
  if (payload.generator !== 'tokens') return null;
  const num = (k: string, fallback: number) =>
    typeof payload[k] === 'number' ? (payload[k] as number) : fallback;
  const origin = Array.isArray(payload.originMm) && payload.originMm.length === 3
    ? (payload.originMm.map(Number) as [number, number, number])
    : [0, 0, 0] as [number, number, number];
  return new TokenFlow({
    ids,
    originMm: origin,
    pitchMm: num('pitchMm', 32),
    nodes: Math.max(2, Math.floor(num('nodes', 17))),
    hops: Math.max(1, Math.floor(num('hops', 12))),
    arcMm: num('arcMm', 8),
    editionSeed,
  });
}
