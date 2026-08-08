/** M69 — the reverb, generated rather than loaded.
 *
 * **No audio assets.** That is a rule of this project rather than a
 * preference: an impulse response is a WAV file with a licence, a download
 * and a provenance question, and this piece is one that has to be able to say
 * where every byte in it came from. So the tail is synthesised at startup —
 * exponentially decaying noise, filtered — into an `AudioBuffer`, and the only
 * thing that ships is the arithmetic.
 *
 * # Three spaces, because the piece moves through three
 *
 * The screenplay is not in one room. Acts I–II are archaeological — stone,
 * distance, a long tail. Act III is the patent studio, which is a *small*
 * room with hard surfaces. Act IV is the network, where the reverb gates: a
 * bright tail that stops rather than decays, which is the sound of a machine
 * rather than of a space.
 *
 * The timeline crossfades between them (M71), so all three are built up front
 * and none is swapped in mid-phrase.
 *
 * # Why the noise is seeded
 *
 * An impulse response built from `Math.random()` is a different room every
 * time the page loads. Nobody would hear the difference in isolation and
 * *that is exactly the problem*: two renders of the same edition would differ
 * in a way no test could pin down and no listener could describe. Same seed,
 * same room.
 *
 * This generator does **not** have to match the Rust side — nothing in the
 * score depends on it — so it is a small 32-bit one rather than the splitmix64
 * the choreography is pinned to. What it has to be is *the same twice*.
 */

/** A small, fast, seeded generator. See the header for why this one and not
 * splitmix64: it never has to agree with another language, only with itself. */
export function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export interface SpaceSpec {
  name: string;
  /** Seconds. */
  seconds: number;
  /** How fast the tail falls. Higher is shorter. */
  decay: number;
  /** Low-pass sweep: the tail gets darker as it decays, which is what air and
   * absorption actually do — a tail that stays bright reads as a machine. */
  brightness: number;
  /** A pre-delay in seconds: the gap before the first reflection, which is
   * what makes a space feel large rather than merely long. */
  predelaySec: number;
  /** 0 = decays naturally, 1 = cut off hard at the end. Act IV's gate. */
  gate: number;
}

/** The three rooms of the piece. */
export const SPACES: Record<'cathedral' | 'plate' | 'gated', SpaceSpec> = {
  // Acts I–II: stone, distance, and enough tail that a single point in the
  // dark has somewhere to be.
  cathedral: { name: 'cathedral', seconds: 4.2, decay: 2.6, brightness: 0.35, predelaySec: 0.035, gate: 0 },
  // Act III: a real room with hard surfaces. Short, bright, close.
  plate: { name: 'plate', seconds: 1.4, decay: 4.5, brightness: 0.75, predelaySec: 0.006, gate: 0 },
  // Act IV: a tail that stops instead of ending.
  gated: { name: 'gated', seconds: 0.85, decay: 0.9, brightness: 0.9, predelaySec: 0.004, gate: 1 },
};

/** Builds one impulse response.
 *
 * Stereo, and the two channels are generated from *different* streams of the
 * same seeded generator rather than from one stream copied — an IR whose
 * channels are identical is a mono reverb wearing two speakers, and the
 * decorrelation between them is most of what makes a reverb sound wide.
 */
export function makeImpulseResponse(
  ctx: BaseAudioContext,
  spec: SpaceSpec,
  seed = 263865,
): AudioBuffer {
  const rate = ctx.sampleRate;
  const length = Math.max(1, Math.floor(spec.seconds * rate));
  const buffer = ctx.createBuffer(2, length, rate);
  const predelay = Math.floor(spec.predelaySec * rate);

  for (let ch = 0; ch < 2; ch++) {
    const data = buffer.getChannelData(ch);
    const rand = mulberry32(seed + ch * 7919);
    // A one-pole low-pass whose cutoff falls with the envelope: the state
    // variable is carried across samples, which is what makes it a filter
    // rather than a per-sample attenuation.
    let lp = 0;
    for (let i = 0; i < length; i++) {
      if (i < predelay) {
        data[i] = 0;
        continue;
      }
      const t = (i - predelay) / (length - predelay || 1);
      const envelope = Math.pow(1 - t, spec.decay);
      // The gate: a hard multiplicative window that reaches zero before the
      // buffer does, so the tail is cut rather than faded.
      const gate = spec.gate > 0 ? (t < 0.7 ? 1 : Math.max(0, 1 - (t - 0.7) / 0.05)) : 1;
      const white = rand() * 2 - 1;
      const cutoff = spec.brightness * (1 - t * 0.85);
      lp += (white - lp) * Math.max(0.02, cutoff);
      data[i] = lp * envelope * gate;
    }
  }
  return normalise(buffer);
}

/** Scales an IR so its loudest sample is 1.
 *
 * Without this the convolution's output level depends on the buffer length
 * and the decay constant, so changing the room would change the *mix* — and
 * the crossfade between two rooms would be a volume ramp with a reverb
 * attached. Normalising makes the send level mean one thing.
 */
function normalise(buffer: AudioBuffer): AudioBuffer {
  let peak = 0;
  for (let ch = 0; ch < buffer.numberOfChannels; ch++) {
    const d = buffer.getChannelData(ch);
    for (let i = 0; i < d.length; i++) {
      const a = Math.abs(d[i]);
      if (a > peak) peak = a;
    }
  }
  if (peak > 0 && Math.abs(peak - 1) > 1e-6) {
    const g = 1 / peak;
    for (let ch = 0; ch < buffer.numberOfChannels; ch++) {
      const d = buffer.getChannelData(ch);
      for (let i = 0; i < d.length; i++) d[i] *= g;
    }
  }
  return buffer;
}

/** All three rooms, and a crossfade between them.
 *
 * Three convolvers rather than one whose buffer is swapped: assigning a new
 * `AudioBuffer` to a live `ConvolverNode` truncates whatever tail is still
 * sounding, which at the end of Act II is several seconds of the piece. Three
 * nodes and two gains cost a few hundred kilobytes and change nothing anyone
 * can hear except the seam that is no longer there.
 */
export class ReverbRack {
  readonly input: GainNode;
  readonly output: GainNode;
  private readonly convolvers: ConvolverNode[] = [];
  private readonly gains: GainNode[] = [];
  private readonly names: string[] = [];

  constructor(ctx: BaseAudioContext, seed = 263865) {
    this.input = ctx.createGain();
    this.output = ctx.createGain();
    for (const key of ['cathedral', 'plate', 'gated'] as const) {
      const conv = ctx.createConvolver();
      conv.normalize = false; // we normalised the buffer ourselves, once
      conv.buffer = makeImpulseResponse(ctx, SPACES[key], seed);
      const g = ctx.createGain();
      g.gain.value = key === 'cathedral' ? 1 : 0;
      this.input.connect(conv);
      conv.connect(g);
      g.connect(this.output);
      this.convolvers.push(conv);
      this.gains.push(g);
      this.names.push(key);
    }
  }

  /** Crossfade to one space over `seconds`. Unknown names are ignored rather
   * than thrown: a document from a later version of the piece should lose a
   * room, not the audio. */
  select(name: string, atTime: number, seconds = 2.0) {
    const i = this.names.indexOf(name);
    if (i < 0) return;
    for (let k = 0; k < this.gains.length; k++) {
      const g = this.gains[k].gain;
      g.cancelScheduledValues(atTime);
      g.setValueAtTime(g.value, atTime);
      g.linearRampToValueAtTime(k === i ? 1 : 0, atTime + seconds);
    }
  }

  get spaces(): readonly string[] {
    return this.names;
  }
}
