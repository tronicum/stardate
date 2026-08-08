/** M69 — the audio graph.
 *
 * ```
 * per-voice: partial bank -> ADSR -> pan -> voice bus
 * voice bus -> [dry] -----------------------------> mix bus
 *           -> [send] -> ReverbRack (procedural IR) -> mix bus
 * pulse bus -> kick/hat/clap -> saturation (WaveShaper) -> mix bus
 * mix bus   -> 3-band EQ -> limiter -> master -> destination
 * ```
 *
 * # It takes a `BaseAudioContext`, not an `AudioContext`
 *
 * That one type is what makes this milestone verifiable. `OfflineAudioContext`
 * renders faster than real time into a buffer, so "does this clip" and "is the
 * level steady" become arithmetic over samples rather than someone listening
 * and saying it seemed fine. Everything here is therefore built against the
 * base class, and nothing reaches for `ctx.destination`'s live-only behaviour
 * or for `resume()`.
 *
 * # A compressor is not a limiter, and the measurement said so
 *
 * The first version of this file ended in a `DynamicsCompressor` and claimed
 * that made "no cut ever clips" a property of the graph. **It measured 1.376
 * on the first render.** `DynamicsCompressorNode` is a compressor: it has an
 * attack, so transients pass through it, and a 20:1 ratio above a threshold
 * still permits overshoot above that threshold. Four sustaining additive
 * voices landing on the same beat — which in a stretto is the *point* — go
 * straight over.
 *
 * What makes the guarantee real is a **bounded transfer function**. The chain
 * now ends in a `WaveShaper` whose curve is `tanh`-shaped and clamped to
 * ±`CEILING`, so the output cannot exceed it for any input at all: not for a
 * loud passage, not for a mixer setting, not for a quality tier nobody has
 * tried. The compressor stays, doing the job it is actually good at — riding
 * the level so the ceiling is rarely reached — and the ceiling is arithmetic.
 *
 * The master gain sits *before* the ceiling for the same reason: a mixer
 * control after it could reintroduce exactly what it prevents.
 */

import { ReverbRack } from './reverb';
import { GM, PULSE_CHANNEL } from './midi';
import { midiToFrequency, PULSE, SynthVoice, VOICE_ENVELOPE } from './synth';

export interface EngineOptions {
  /** Reverb send level, 0..1. */
  send?: number;
  /** Master level, 0..1. */
  master?: number;
  seed?: number;
}

/** What the mixer's monitor switch can select.
 *
 * Useful for review and harmless to ship, which is the spec's own reason for
 * it — but it is also how anyone checks that the pulse and the counterpoint
 * are two layers rather than one mix that happens to sound busy. */
export type Monitor = 'both' | 'counterpoint' | 'pulse';

/** Every gain change ramps over this. A gain set by assignment is a step, a
 * step is a discontinuity, and a discontinuity is a click — including on a
 * mute, which is the one control a listener uses while something is playing. */
export const MIXER_RAMP_SEC = 0.03;

/** How hard the pulse bus is driven into the waveshaper. */
export const SATURATION_DRIVE = 2.2;

/** The pulse bus's own level, kept as a constant because the monitor switch
 * has to be able to put it back. */
export const PULSE_BUS_GAIN = 0.55;

/** Ramp an `AudioParam` to a value rather than assigning it. */
function ramp(param: AudioParam, to: number, at: number, seconds = MIXER_RAMP_SEC) {
  const t = Math.max(at, 0);
  param.cancelScheduledValues(t);
  param.setValueAtTime(param.value, t);
  param.linearRampToValueAtTime(to, t + seconds);
}

/** The compressor. A high ratio and a fast attack: this rides the level so
 * the ceiling below is rarely reached. It is not the thing that guarantees
 * the ceiling — see the module header. */
export const LIMITER = { threshold: -12, knee: 0, ratio: 20, attack: 0.002, release: 0.12 };

/** The hard ceiling, as an absolute sample value.
 *
 * Not 1.0: a value that touches full scale is a value that clips after any
 * later resample or lossy encode, both of which overshoot. 0.985 is about
 * -0.13 dBFS and is inaudible as a loss. */
export const CEILING = 0.985;

export class AudioEngine {
  readonly ctx: BaseAudioContext;
  readonly voiceBus: GainNode;
  readonly pulseBus: GainNode;
  readonly mixBus: GainNode;
  readonly master: GainNode;
  readonly reverb: ReverbRack;
  readonly limiter: DynamicsCompressorNode;
  /** The bounded transfer function that makes "never clips" true. */
  readonly ceiling: WaveShaperNode;
  readonly eq: BiquadFilterNode[];

  private readonly dry: GainNode;
  private readonly sendGain: GainNode;
  private readonly sendLevel: number;
  private masterLevel: number;
  private muted = false;
  private monitor: Monitor = 'both';
  /** Sounding voices, keyed by `${voice}:${midi}` — a fugue re-articulates
   * the same pitch in the same voice constantly, and a key that is only the
   * pitch would have the second note stop the first. */
  private readonly active = new Map<string, SynthVoice>();

  constructor(ctx: BaseAudioContext, opts: EngineOptions = {}) {
    this.ctx = ctx;

    this.voiceBus = ctx.createGain();
    this.pulseBus = ctx.createGain();
    this.mixBus = ctx.createGain();
    this.master = ctx.createGain();
    this.dry = ctx.createGain();
    this.sendGain = ctx.createGain();

    // Headroom. Four voices at 0.5 each sum to 2.0 in the worst case, and a
    // compressor with headroom to work in sounds like a compressor rather than
    // like a ceiling being hit.
    this.voiceBus.gain.value = 0.42;
    this.pulseBus.gain.value = PULSE_BUS_GAIN;
    this.dry.gain.value = 1;
    this.sendLevel = opts.send ?? 0.28;
    this.sendGain.gain.value = this.sendLevel;
    this.masterLevel = opts.master ?? 0.9;
    this.master.gain.value = this.masterLevel;

    // Voices: dry and send in parallel.
    this.reverb = new ReverbRack(ctx, opts.seed ?? 263865);
    this.voiceBus.connect(this.dry);
    this.dry.connect(this.mixBus);
    this.voiceBus.connect(this.sendGain);
    this.sendGain.connect(this.reverb.input);
    this.reverb.output.connect(this.mixBus);

    // Pulse: saturated, and deliberately *not* sent to the reverb. A kick in
    // a cathedral is mud; the percussion belongs in the front of the room
    // while the voices are in the back of it, and that contrast is most of
    // what makes Act IV feel like a different place.
    const shaper = ctx.createWaveShaper();
    shaper.curve = saturationCurve(SATURATION_DRIVE);
    shaper.oversample = '4x';
    this.pulseBus.connect(shaper);
    shaper.connect(this.mixBus);

    // Master EQ: a gentle low shelf, a presence bell, a high shelf. Three
    // filters in series, each doing one thing, because a single tilt cannot
    // lift the fundamentals and open the top without also thickening the mids
    // where four voices already compete.
    const low = ctx.createBiquadFilter();
    low.type = 'lowshelf';
    low.frequency.value = 160;
    low.gain.value = 1.5;
    const mid = ctx.createBiquadFilter();
    mid.type = 'peaking';
    mid.frequency.value = 900;
    mid.Q.value = 0.9;
    mid.gain.value = -1.5;
    const high = ctx.createBiquadFilter();
    high.type = 'highshelf';
    high.frequency.value = 6000;
    high.gain.value = 1.0;
    this.eq = [low, mid, high];

    this.limiter = ctx.createDynamicsCompressor();
    this.limiter.threshold.value = LIMITER.threshold;
    this.limiter.knee.value = LIMITER.knee;
    this.limiter.ratio.value = LIMITER.ratio;
    this.limiter.attack.value = LIMITER.attack;
    this.limiter.release.value = LIMITER.release;

    this.ceiling = ctx.createWaveShaper();
    this.ceiling.curve = ceilingCurve(CEILING);
    // 'none', deliberately — see `ceilingCurve`. A downsampling filter after
    // the bound is a filter that can exceed it, and it measured 1.44.
    this.ceiling.oversample = 'none';

    this.mixBus.connect(low);
    low.connect(mid);
    mid.connect(high);
    high.connect(this.limiter);
    this.limiter.connect(this.master);
    this.master.connect(this.ceiling);
    this.ceiling.connect(ctx.destination);
  }

  /** Start a note. `at` is an absolute `AudioContext` time — never a delay,
   * and never `currentTime` read inside a render loop. Sample-accurate
   * scheduling is the entire reason WebAudio has a clock of its own.
   *
   * `env` exists for one reason and it is a measurement one: M70's AC1 asks
   * that note onsets *measured from a rendered capture* land within 3 ms of
   * their scored times, and the organ's 35 ms attack is a ramp rather than an
   * edge — an onset detector cannot resolve 3 ms in it. The harness passes a
   * fast-attack envelope so the thing being measured is the *scheduling*, and
   * says so. Nothing in the piece passes it. */
  noteOn(voice: number, midi: number, at: number, velocity = 0.8, env = VOICE_ENVELOPE) {
    // Channel 10 is percussion, by the same General MIDI convention the score
    // is written with. Not a fifth voice with a strange timbre: a drum has no
    // pitch, no envelope to release and no voice-stealing policy, and running
    // it through `SynthVoice` would give it all three.
    if (voice === PULSE_CHANNEL) {
      this.percussion(midi, at, velocity);
      return;
    }
    const key = `${voice}:${midi}`;
    // A re-articulation of a pitch that is still sounding: release the old
    // one rather than leaving it running for ever. This is the stuck-note
    // case M70's AC3 is about, handled where the note is made.
    const existing = this.active.get(key);
    if (existing) {
      existing.flush(at);
      existing.disconnect();
    }
    const v = new SynthVoice(
      this.ctx,
      this.voiceBus,
      voice,
      midiToFrequency(midi),
      velocity,
      at,
      env,
    );
    this.active.set(key, v);
  }

  noteOff(voice: number, midi: number, at: number) {
    // A percussion note-off is a formality the score carries so a DAW draws
    // notes of a sensible length; the sound is a one-shot and has already
    // scheduled its own end.
    if (voice === PULSE_CHANNEL) return;
    const key = `${voice}:${midi}`;
    const v = this.active.get(key);
    if (!v) return;
    v.stop(at, VOICE_ENVELOPE);
    this.active.delete(key);
  }

  /** Release everything with a short ramp — a seek. Never a disconnect: that
   * is a click. */
  flushAll(at: number) {
    for (const v of this.active.values()) {
      v.flush(at);
      v.disconnect();
    }
    this.active.clear();
  }

  get soundingCount(): number {
    return this.active.size;
  }

  // ------------------------------------------------------------------ mixer

  /** Master level, 0..1, ramped. Sits before the ceiling — see the header. */
  setMasterLevel(value: number, at = this.ctx.currentTime) {
    this.masterLevel = Math.min(Math.max(value, 0), 1);
    this.applyMaster(at);
  }

  get masterLevelValue(): number {
    return this.masterLevel;
  }

  /** Mute is a mixer control and **not** the same thing as `?mute=1`.
   *
   * `?mute=1` decides which clock the show reads (M66): no `AudioContext`, so
   * show time comes from `performance.now()`. This turns the output down on a
   * context that is running and still ticking. Confusing the two is how a
   * muted session ends up with a clock that does not move — which M66 already
   * watched happen once. */
  setMuted(muted: boolean, at = this.ctx.currentTime) {
    this.muted = muted;
    this.applyMaster(at);
  }

  get isMuted(): boolean {
    return this.muted;
  }

  /** Which layer is audible. The buses are separate all the way to the mix
   * bus precisely so this is a gain and not a re-route. */
  setMonitor(monitor: Monitor, at = this.ctx.currentTime) {
    this.monitor = monitor;
    ramp(this.dry.gain, monitor === 'pulse' ? 0 : 1, at);
    ramp(this.sendGain.gain, monitor === 'pulse' ? 0 : this.sendLevel, at);
    ramp(this.pulseBus.gain, monitor === 'counterpoint' ? 0 : PULSE_BUS_GAIN, at);
  }

  get monitorValue(): Monitor {
    return this.monitor;
  }

  private applyMaster(at: number) {
    ramp(this.master.gain, this.muted ? 0 : this.masterLevel, at);
  }

  kick(at: number, gain = 1.0) {
    PULSE.kick(this.ctx, this.pulseBus, at, gain);
  }
  hat(at: number, gain = 0.35, open = false) {
    PULSE.hat(this.ctx, this.pulseBus, at, gain, open);
  }
  clap(at: number, gain = 0.5) {
    PULSE.clap(this.ctx, this.pulseBus, at, gain);
  }

  /** The Kick — the drum one, and the camera one. §7 of the screenplay is
   * explicit that these are the same event, so this is deliberately not just
   * a louder `kick()`: it is the single accent the whole piece ends on, and
   * it gets its own name so that binding it to the camera (M71) is one call
   * rather than a magic velocity. */
  /** One General MIDI drum hit. */
  private percussion(midi: number, at: number, velocity: number) {
    switch (midi) {
      case GM.kick:
        PULSE.kick(this.ctx, this.pulseBus, at, velocity);
        return;
      case GM.clap:
        PULSE.clap(this.ctx, this.pulseBus, at, velocity * 0.7);
        return;
      case GM.hatOpen:
        PULSE.hat(this.ctx, this.pulseBus, at, velocity * 0.5, true);
        return;
      case GM.hatClosed:
      default:
        PULSE.hat(this.ctx, this.pulseBus, at, velocity * 0.45, false);
    }
  }

  finalAccent(at: number) {
    PULSE.kick(this.ctx, this.pulseBus, at, 1.0);
    PULSE.clap(this.ctx, this.pulseBus, at, 0.7);
  }

  setSpace(name: string, at: number, seconds = 2.0) {
    this.reverb.select(name, at, seconds);
  }
}

/** How long the mastering chain delays everything, in seconds — measured,
 * not assumed.
 *
 * # A compressor is a delay line, and nothing says so
 *
 * `DynamicsCompressorNode` looks ahead. It has to: a limiter that reacted
 * only to samples it had already passed on could not attenuate the transient
 * that triggered it. The lookahead is implemented as a **pre-delay on the
 * signal path**, so every sample that goes through the compressor comes out
 * late by a fixed amount — and the Web Audio API exposes no way to ask what
 * that amount is. (An early draft had a `latencyTime` attribute; it was
 * removed.)
 *
 * M70 found it the way anything gets found here: by measuring. Onsets taken
 * from the master output were consistently **5.99 ms** late against the
 * score, with a spread of well under a millisecond — a constant, which is
 * never a scheduling error and always a delay line. An impulse through a bare
 * compressor with this project's settings comes out **264 samples** later at
 * 44.1 kHz. The waveshaper adds none, and neither does the EQ.
 *
 * # Why this matters, and it is not the music
 *
 * A constant delay on everything is inaudible: the whole piece is 6 ms late
 * and no listener has a reference. It matters for **M71's AC2**, which asks
 * that the Kick's audio onset and the first frame of the camera Kick land
 * within one frame — 16.7 ms. Six milliseconds of unaccounted latency is over
 * a third of that budget, spent before anyone writes a line of binding code.
 *
 * So it is measured rather than trusted: the number depends on the browser's
 * implementation and on the sample rate, and hard-coding either would be
 * exactly the kind of assertion this project keeps catching itself making.
 */
export async function measureOutputLatency(sampleRate = 44100): Promise<number> {
  const ctx = new OfflineAudioContext(1, Math.ceil(sampleRate * 0.5), sampleRate);
  const buffer = ctx.createBuffer(1, 4, sampleRate);
  buffer.getChannelData(0)[0] = 1;
  const src = ctx.createBufferSource();
  src.buffer = buffer;
  const c = ctx.createDynamicsCompressor();
  c.threshold.value = LIMITER.threshold;
  c.knee.value = LIMITER.knee;
  c.ratio.value = LIMITER.ratio;
  c.attack.value = LIMITER.attack;
  c.release.value = LIMITER.release;
  src.connect(c);
  c.connect(ctx.destination);
  const at = 0.1;
  src.start(at);
  const rendered = await ctx.startRendering();
  const d = rendered.getChannelData(0);
  const from = Math.floor(at * sampleRate * 0.5);
  for (let i = from; i < d.length; i++) {
    if (Math.abs(d[i]) > 1e-7) return i / sampleRate - at;
  }
  return 0;
}

/** The output ceiling, as a curve.
 *
 * `tanh` scaled so the asymptote is exactly `ceiling`: the output cannot leave
 * [-ceiling, +ceiling] for any input, which is the whole point. Gentle in the
 * middle — a signal at half scale passes through almost untouched — and firm
 * at the top.
 *
 * # Two things about `WaveShaperNode` that this got wrong first
 *
 * **The curve is indexed by input over [-1, 1], and inputs outside that range
 * are clamped to its ends.** The first version built the table over ±8, on the
 * theory that a loud input needed somewhere to land — which is exactly
 * backwards: it made an input of 0.125 read the table entry for 1.0 and come
 * out at 0.75. An eightfold gain, dressed as headroom. Every signal was slammed
 * into the knee and the output was a near-square wave. The node's own clamping
 * is the headroom; the table belongs over [-1, 1].
 *
 * **And `oversample` must be `'none'` here.** Oversampling runs the signal
 * through an upsample filter, the curve, and a downsample filter, and that
 * last filter *rings*: on a hard-shaped near-square signal the overshoot
 * measured **1.44 against a ceiling of 0.985** — 46% over, which is the
 * classic Gibbs figure and could not be anything else. Oversampling is right
 * for a saturator, where the goal is to avoid aliasing in something meant to
 * be heard. It is wrong for a ceiling, where the goal is a bound, because a
 * filter after the bound is a filter that can exceed it.
 */
export function ceilingCurve(ceiling: number, samples = 8192): Float32Array<ArrayBuffer> {
  const curve = new Float32Array(new ArrayBuffer(samples * 4));
  for (let i = 0; i < samples; i++) {
    const x = (i / (samples - 1)) * 2 - 1;
    curve[i] = ceiling * Math.tanh(x / ceiling);
  }
  return curve;
}

/** A soft-clip curve. `tanh`-shaped rather than a hard knee, because the
 * point of saturating the pulse bus is density and not distortion. */
export function saturationCurve(drive: number, samples = 4096): Float32Array<ArrayBuffer> {
  const curve = new Float32Array(new ArrayBuffer(samples * 4));
  for (let i = 0; i < samples; i++) {
    const x = (i / (samples - 1)) * 2 - 1;
    curve[i] = Math.tanh(x * drive) / Math.tanh(drive);
  }
  return curve;
}

/** Peak and RMS of a rendered buffer, and how far the RMS moves across it.
 *
 * This is M69's AC2, as a function rather than as a description: "no samples
 * at ±1.0" and "RMS within a 6 dB band" are both statements about numbers,
 * and putting the measurement in the shipped module rather than only in the
 * harness means anyone can ask the same question of any render.
 */
export function analyse(buffer: AudioBuffer, windowSec = 1.0) {
  let peak = 0;
  let sumSquares = 0;
  let count = 0;
  const windows: number[] = [];
  const windowLength = Math.max(1, Math.floor(windowSec * buffer.sampleRate));

  for (let ch = 0; ch < buffer.numberOfChannels; ch++) {
    const d = buffer.getChannelData(ch);
    for (let i = 0; i < d.length; i++) {
      const a = Math.abs(d[i]);
      if (a > peak) peak = a;
      sumSquares += d[i] * d[i];
      count++;
    }
  }

  // Windowed RMS across the run, on channel 0. Silence is excluded rather
  // than averaged in: the piece opens on five bars of nothing, and a band
  // measured through that says the level swings by infinity.
  const d = buffer.getChannelData(0);
  for (let start = 0; start + windowLength <= d.length; start += windowLength) {
    let s = 0;
    for (let i = start; i < start + windowLength; i++) s += d[i] * d[i];
    const rms = Math.sqrt(s / windowLength);
    if (rms > 1e-4) windows.push(rms);
  }

  const rms = Math.sqrt(sumSquares / Math.max(1, count));
  const loud = windows.length ? Math.max(...windows) : 0;
  const quiet = windows.length ? Math.min(...windows) : 0;
  return {
    peak,
    rms,
    /** Samples at or past full scale. Must be zero. */
    clipped: peak >= 1.0,
    windows: windows.length,
    loudestWindowRms: loud,
    quietestWindowRms: quiet,
    /** How far the level moves across the run, in decibels. */
    dynamicRangeDb: quiet > 0 ? 20 * Math.log10(loud / quiet) : Infinity,
  };
}
