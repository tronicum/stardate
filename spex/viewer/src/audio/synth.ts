/** M69 — the voices and the pulse, synthesised.
 *
 * A browser has no MIDI sound generator. `AudioContext` is not a General MIDI
 * device; a `.mid` is a list of notes and something still has to make a sound.
 * Shipping a soundfont would be several megabytes with its own licence, which
 * is against both the loading budget and the no-audio-assets rule — so this
 * file is the instrument, and it was always going to exist.
 *
 * # Why the voices sustain
 *
 * The four contrapuntal voices are a soft **additive organ**: partials 1, 2,
 * 3, 4, 6 and 8, with a short chiff at the onset. Not a pluck, not a piano.
 *
 * That is the one decision here that is about the music rather than about
 * taste. Counterpoint is the perception of several lines *at once*, and a
 * line you can only hear at its attack is not a line — a plucked or struck
 * voice decays through the bar and the listener is left tracking a sequence
 * of events rather than following four simultaneous melodies. Every real
 * instrument fugues were written for sustains: organ, strings, voices.
 *
 * The chiff is what keeps a sustaining voice from turning to mud: an entry
 * has to be *audible as an entry*, and with no transient at all a new voice
 * simply thickens the texture without announcing itself.
 *
 * # Partial weights per voice
 *
 * The four voices are not four copies at different pitches. The soprano is
 * brighter and the bass carries more of its fundamental, which is how a real
 * ensemble sits — a bass with the soprano's partials fights it for the same
 * frequency band, and the two lines stop being separable exactly where the
 * counterpoint needs them most.
 */

/** Harmonic numbers, in order. Not 1..8: the fifth and seventh partials are
 * left out because they are the ones that make an additive tone read as
 * "electronic" rather than "pipe" — 5 is a major third above two octaves and
 * 7 is a flat seventh, and both fight the temperament of whatever is being
 * played. */
export const PARTIALS = [1, 2, 3, 4, 6, 8];

/** Per-voice partial weights, soprano to bass. */
export const VOICE_TIMBRES: number[][] = [
  [1.0, 0.42, 0.24, 0.14, 0.07, 0.04], // soprano — brightest
  [1.0, 0.38, 0.20, 0.11, 0.05, 0.03], // alto
  [1.0, 0.34, 0.16, 0.09, 0.04, 0.02], // tenor
  [1.0, 0.28, 0.12, 0.06, 0.02, 0.01], // bass — mostly fundamental
];

/** Stereo placement, soprano to bass. Spread, not centred: four lines in the
 * same place is one line four times as loud. Modest, because a fugue is one
 * ensemble rather than four soloists. */
export const VOICE_PAN = [-0.35, -0.12, 0.12, 0.35];

export interface Envelope {
  attackSec: number;
  decaySec: number;
  sustain: number;
  releaseSec: number;
}

/** The organ envelope. A slow-ish attack and a long release, which is what
 * "it sustains" means in numbers. */
export const VOICE_ENVELOPE: Envelope = {
  attackSec: 0.035,
  decaySec: 0.18,
  sustain: 0.72,
  releaseSec: 0.28,
};

/** One sounding note. Holds every node it made, because stopping a voice
 * means stopping all of them and disconnecting all of them — an oscillator
 * left running is a few percent of a CPU that never comes back. */
export class SynthVoice {
  private readonly ctx: BaseAudioContext;
  private readonly oscillators: OscillatorNode[] = [];
  private readonly gain: GainNode;
  private readonly panner: StereoPannerNode;
  private stopped = false;

  constructor(
    ctx: BaseAudioContext,
    destination: AudioNode,
    voice: number,
    frequency: number,
    velocity: number,
    startTime: number,
    env: Envelope = VOICE_ENVELOPE,
  ) {
    this.ctx = ctx;
    const timbre = VOICE_TIMBRES[voice % VOICE_TIMBRES.length];

    this.gain = ctx.createGain();
    this.panner = ctx.createStereoPanner();
    this.panner.pan.value = VOICE_PAN[voice % VOICE_PAN.length];
    this.gain.connect(this.panner);
    this.panner.connect(destination);

    // The partial bank. Weights are normalised so a voice's loudness does not
    // depend on how many partials its timbre happens to use — otherwise
    // changing a timbre changes the mix.
    const sum = timbre.reduce((a, b) => a + b, 0);
    for (let i = 0; i < PARTIALS.length; i++) {
      const osc = ctx.createOscillator();
      osc.type = 'sine';
      osc.frequency.value = frequency * PARTIALS[i];
      const g = ctx.createGain();
      g.gain.value = (timbre[i] / sum) * velocity;
      osc.connect(g);
      g.connect(this.gain);
      osc.start(startTime);
      this.oscillators.push(osc);
    }

    // ADSR. `setValueAtTime(0)` first, because a gain that has never been
    // scheduled starts at 1 and the first thing anyone would hear is a click.
    const g = this.gain.gain;
    g.setValueAtTime(0, startTime);
    // The chiff: a fast overshoot before the body of the note. It is what
    // makes an entry announce itself in a texture of sustaining voices.
    g.linearRampToValueAtTime(1.18, startTime + env.attackSec * 0.45);
    g.linearRampToValueAtTime(1.0, startTime + env.attackSec);
    g.linearRampToValueAtTime(env.sustain, startTime + env.attackSec + env.decaySec);
  }

  /** Release, and schedule the teardown. `endTime` is when the key lifts; the
   * sound continues for the release. */
  stop(endTime: number, env: Envelope = VOICE_ENVELOPE) {
    if (this.stopped) return;
    this.stopped = true;
    const g = this.gain.gain;
    const t = Math.max(endTime, this.ctx.currentTime);
    g.cancelScheduledValues(t);
    g.setValueAtTime(g.value, t);
    // An exponential release sounds like an instrument and a linear one
    // sounds like a fader. `linearRampToValueAtTime(0)` at the end because
    // `exponentialRampToValueAtTime` cannot reach zero.
    g.setTargetAtTime(0.0001, t, env.releaseSec / 3);
    g.linearRampToValueAtTime(0, t + env.releaseSec);
    for (const o of this.oscillators) o.stop(t + env.releaseSec + 0.02);
  }

  /** Cut immediately with a short ramp — what a seek does. Never an abrupt
   * disconnect: that is a click, and a piece that clicks every time someone
   * scrubs it is a piece nobody scrubs. */
  flush(atTime: number, rampSec = 0.02) {
    if (this.stopped) return;
    this.stopped = true;
    const g = this.gain.gain;
    const t = Math.max(atTime, this.ctx.currentTime);
    g.cancelScheduledValues(t);
    g.setValueAtTime(g.value, t);
    g.linearRampToValueAtTime(0, t + rampSec);
    for (const o of this.oscillators) o.stop(t + rampSec + 0.01);
  }

  disconnect() {
    for (const o of this.oscillators) {
      try {
        o.disconnect();
      } catch {
        /* already gone */
      }
    }
    this.gain.disconnect();
    this.panner.disconnect();
  }
}

// ------------------------------------------------------------------ pulse

/** The Act IV percussion. All three are standard synthesis and all three are
 * one-shots, so none of them needs a voice-stealing policy. */
export const PULSE = {
  /** A pitch-swept sine plus a click. The sweep is what makes it a kick
   * rather than a low note: 120 Hz down to 45 in 60 ms. */
  kick(ctx: BaseAudioContext, dest: AudioNode, at: number, gain = 1.0) {
    const osc = ctx.createOscillator();
    const g = ctx.createGain();
    osc.type = 'sine';
    osc.frequency.setValueAtTime(120, at);
    osc.frequency.exponentialRampToValueAtTime(45, at + 0.06);
    g.gain.setValueAtTime(0, at);
    g.gain.linearRampToValueAtTime(gain, at + 0.004);
    g.gain.exponentialRampToValueAtTime(0.0001, at + 0.32);
    osc.connect(g);
    g.connect(dest);
    osc.start(at);
    osc.stop(at + 0.36);

    // The click: two milliseconds of high sine, which is what a listener
    // actually hears as the attack.
    const click = ctx.createOscillator();
    const cg = ctx.createGain();
    click.type = 'triangle';
    click.frequency.value = 1400;
    cg.gain.setValueAtTime(gain * 0.35, at);
    cg.gain.exponentialRampToValueAtTime(0.0001, at + 0.012);
    click.connect(cg);
    cg.connect(dest);
    click.start(at);
    click.stop(at + 0.02);
  },

  /** Filtered noise burst. */
  hat(ctx: BaseAudioContext, dest: AudioNode, at: number, gain = 0.35, open = false) {
    const src = noiseSource(ctx, open ? 0.22 : 0.05);
    const hp = ctx.createBiquadFilter();
    hp.type = 'highpass';
    hp.frequency.value = 7000;
    const g = ctx.createGain();
    g.gain.setValueAtTime(gain, at);
    g.gain.exponentialRampToValueAtTime(0.0001, at + (open ? 0.22 : 0.05));
    src.connect(hp);
    hp.connect(g);
    g.connect(dest);
    src.start(at);
    src.stop(at + (open ? 0.24 : 0.07));
  },

  /** Three short bursts a few milliseconds apart — which is what a clap is,
   * and why one burst never sounds like one. */
  clap(ctx: BaseAudioContext, dest: AudioNode, at: number, gain = 0.5) {
    for (const [i, offset] of [0, 0.011, 0.022].entries()) {
      const src = noiseSource(ctx, 0.12);
      const bp = ctx.createBiquadFilter();
      bp.type = 'bandpass';
      bp.frequency.value = 1500;
      bp.Q.value = 1.2;
      const g = ctx.createGain();
      const level = gain * (i === 2 ? 1 : 0.6);
      g.gain.setValueAtTime(level, at + offset);
      g.gain.exponentialRampToValueAtTime(0.0001, at + offset + (i === 2 ? 0.12 : 0.03));
      src.connect(bp);
      bp.connect(g);
      g.connect(dest);
      src.start(at + offset);
      src.stop(at + offset + 0.14);
    }
  },
};

/** White noise as a buffer source. Generated per call and short, because a
 * shared looping buffer makes every hat in the piece the identical hat. */
function noiseSource(ctx: BaseAudioContext, seconds: number): AudioBufferSourceNode {
  const n = Math.max(1, Math.floor(seconds * ctx.sampleRate));
  const buf = ctx.createBuffer(1, n, ctx.sampleRate);
  const d = buf.getChannelData(0);
  for (let i = 0; i < n; i++) d[i] = Math.random() * 2 - 1;
  const src = ctx.createBufferSource();
  src.buffer = buf;
  return src;
}

/** MIDI note number to frequency, A4 = 440. */
export function midiToFrequency(midi: number): number {
  return 440 * Math.pow(2, (midi - 69) / 12);
}
