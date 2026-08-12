#!/usr/bin/env node
/** M70 — the scheduler, measured against the score it claims to be playing.
 *
 *   spex fugue-build shows/die-geschichtliche-matrix.show.json -o /tmp/f.mid
 *   node scripts/viewer-shot/scheduleprobe.mjs /tmp/f.mid /tmp/m70
 *
 *   AC1  are note onsets, measured from a RENDERED capture, within 3 ms of
 *        their scored times over a four-minute run?
 *   AC2  does seeking to an arbitrary time produce the material that belongs
 *        there — checked against the same passage reached by playing into it?
 *   AC3  are there stuck notes after 100 randomised seek/pause/play?
 *
 * AC1 is the one worth reading the code for. Comparing the scheduler's own
 * idea of when it scheduled a note against the score would test the scheduler
 * against itself; what is measured here is **the audio**. Two substitutions
 * are made for the measurement and both are stated rather than hidden:
 *
 * - **A fast attack.** The organ's 35 ms ramp (M69) is not an edge, and an
 *   instrument that cannot resolve 3 ms cannot be used to measure 3 ms.
 * - **One voice.** Four lines attacking within a few milliseconds of each
 *   other cannot be separated by any detector, and the question is about
 *   scheduling rather than about polyphony. The soprano has 191 of the 475
 *   notes, which is sample enough.
 * - **The tap is the voice bus, not the master output.** The mastering chain
 *   delays everything by a measured 5.99 ms (see `measureOutputLatency`) and
 *   its compressor pumps the level back up after every transient, which a
 *   rise detector reads as an onset. Neither is the scheduler. The latency is
 *   reported here as its own number rather than quietly subtracted, because
 *   M71 has to spend it against a 16.7 ms frame budget.
 */
import { chromium } from 'playwright';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const midPath = process.argv[2] ?? '/tmp/die-geschichtliche-matrix.mid';
const outDir = process.argv[3] ?? '/tmp/m70';
mkdirSync(outDir, { recursive: true });

execFileSync(
  join(here, 'node_modules/.bin/esbuild'),
  [join(here, 'audio-entry.ts'), '--bundle', '--format=iife', `--outfile=${outDir}/audio-entry.js`],
  { stdio: 'inherit' },
);

const midB64 = readFileSync(midPath).toString('base64');
const browser = await chromium.launch();
const page = await browser.newPage();
const warnings = [];
page.on('console', (m) => { if (m.type() === 'warning' || m.type() === 'error') warnings.push(m.text()); });
page.on('pageerror', (e) => warnings.push(String(e)));
await page.goto('about:blank');
await page.addScriptTag({ path: `${outDir}/audio-entry.js` });

const result = await page.evaluate(async (b64) => {
  const A = window.__spexAudio;
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  const score = A.parseSmf(bytes.buffer);

  const SR = 44100;
  /** A fake clock: the harness drives time explicitly, because an
   *  OfflineAudioContext has no wall clock and `setInterval` would never fire
   *  a single note in one. */
  const clock = { time: 0, playing: true };

  /** Render `seconds` of the score starting at show time `from`, pumping the
   *  real scheduler at its real 25 ms cadence. */
  async function render(from, seconds, { fastAttack = true, pumpMs = A.TICK_MS, onlyVoice = null, tap = 'master' } = {}) {
    const ctx = new OfflineAudioContext(1, Math.ceil(SR * seconds), SR);
    const engine = new A.AudioEngine(ctx);
    if (tap === 'voicebus') {
      // Straight off the voice bus: no EQ, no compressor, no ceiling. The
      // reverb is a parallel send and does not feed back into the bus, so
      // this tap is dry whatever the send is set to.
      engine.ceiling.disconnect();
      engine.voiceBus.connect(ctx.destination);
    }
    if (fastAttack) {
      // See the file header: an edge, so an onset detector can find one.
      const env = { attackSec: 0.001, decaySec: 0.05, sustain: 0.6, releaseSec: 0.05 };
      const inner = engine.noteOn.bind(engine);
      engine.noteOn = (v, m, at, vel) => inner(v, m, at, vel, env);
    }
    clock.time = from;
    clock.playing = true;
    if (onlyVoice !== null) {
      const inner = engine.noteOn.bind(engine);
      engine.noteOn = (v, m, at, vel, env) => { if (v === onlyVoice) inner(v, m, at, vel, env); };
      const innerOff = engine.noteOff.bind(engine);
      engine.noteOff = (v, m, at) => { if (v === onlyVoice) innerOff(v, m, at); };
    }
    const sched = new A.Scheduler(engine, score, clock, { cues: A.cuesFromScore(score) });
    sched.seek(from, 0);
    // Pump on the real cadence, with audio time and show time advancing
    // together — exactly what `setInterval` does in production.
    for (let t = 0; t <= seconds; t += pumpMs / 1000) {
      clock.time = from + t;
      sched.pump(t, clock.time);
    }
    const buf = await ctx.startRendering();
    return { buf, scheduled: sched.scheduled, resumed: sched.resumed, pending: sched.pendingCount };
  }

  /** A peak-hold envelope: the running maximum of |x| over `window` samples.
   *
   * This is the second detector in this file and the reason for the change is
   * worth keeping. The first one smoothed |x| with a one-pole filter and took
   * its flux, and it reported **1107 onsets in a part that has 191 notes**.
   * The arithmetic says why it had to: |sin| ripples at twice the fundamental,
   * so at 220 Hz the envelope has a bump every 2.3 ms, and any smoother slow
   * enough not to follow those bumps is also too slow to resolve the 3 ms this
   * milestone is asked about. There is no setting of that filter that works.
   *
   * A running maximum over more than one period has neither problem. It is
   * ripple-free *by construction* — the max over a whole period is the same
   * whichever part of the cycle you are in — and, unlike any low-pass, it
   * rises on the very sample a louder signal arrives, because a maximum has no
   * time constant. The staircase it leaves on the way down is ugly and does
   * not matter: onsets are rises.
   */
  function envelope(d, window) {
    const out = new Float32Array(d.length);
    // Monotonic deque, so the whole thing is O(n) rather than O(n·window).
    const idx = new Int32Array(d.length);
    let head = 0;
    let tail = 0;
    for (let i = 0; i < d.length; i++) {
      const x = Math.abs(d[i]);
      while (tail > head && Math.abs(d[idx[tail - 1]]) <= x) tail--;
      idx[tail++] = i;
      if (idx[head] <= i - window) head++;
      out[i] = Math.abs(d[idx[head]]);
    }
    return out;
  }

  /** Onsets, as sample-accurate times. */
  function onsets(buf, { windowSec = 0.006, riseSec = 0.003, refractorySec = 0.25, floor = 0.10 } = {}) {
    const d = buf.getChannelData(0);
    const sr = buf.sampleRate;
    let peak = 0;
    for (let i = 0; i < d.length; i++) if (Math.abs(d[i]) > peak) peak = Math.abs(d[i]);
    const env = envelope(d, Math.round(windowSec * sr));
    const rise = Math.round(riseSec * sr);
    const refractory = Math.round(refractorySec * sr);
    // Relative to the loudest sample in the run, so the detector does not have
    // a gain baked into it and the same numbers work on a quieter mix.
    //
    // **The refractory period is the setting that matters, and it comes from
    // the music rather than from tuning.** The shortest interval between two
    // soprano onsets in this score is a quaver at 84 bpm — 0.357 s — so any
    // second detection within 0.25 s of the first cannot be a note, and every
    // one of them was the same artefact: where a note's 50 ms release overlaps
    // the next note's attack the two beat against each other, and the beat is
    // a rise. Raising the level threshold instead suppressed those *and* the
    // quietest real entries; raising the refractory suppresses only them. On
    // the full run: at 50 ms, 50 spurious detections and **two real onsets
    // missed**; at 250 ms, 8 spurious and none missed.
    const minJump = floor * peak;
    const out = [];
    let lastAt = -refractory - 1;
    for (let i = rise; i < env.length; i++) {
      if (i - lastAt <= refractory) continue;
      if (env[i] - env[i - rise] <= minJump) continue;
      // Walk back to the first sample of the contiguous rise. That sample is
      // the onset: the envelope was flat or falling before it.
      let j = i;
      while (j > 0 && env[j - 1] < env[j] && i - j < rise * 2) j--;
      out.push(j / sr);
      lastAt = i;
    }
    return out;
  }

  // ---- AC1: four minutes, soprano onsets from the audio against the score.
  const FROM = 0;
  const SECONDS = 240;
  const latencySec = await A.measureOutputLatency(SR);
  const sopranoRun = await render(FROM, SECONDS, { onlyVoice: 0, tap: 'voicebus' });
  const detected = onsets(sopranoRun.buf);
  const scoredOnsets = [...new Set(score.notes.filter((n) => n.voice === 0).map((n) => +n.atSec.toFixed(6)))]
    .filter((t) => t >= FROM && t < FROM + SECONDS - 1)
    .sort((a, b) => a - b);
  // Match *scored* onsets to detected ones, not the other way round: the
  // question is whether every note in the score arrived on time, and matching
  // the detections would quietly excuse a note that produced no sound at all.
  const errors = [];
  let missed = 0;
  for (const s of scoredOnsets) {
    let best = Infinity;
    for (const t of detected) {
      const e = Math.abs(s - t);
      if (e < best) best = e;
      if (t > s + 0.1) break;
    }
    if (best < 0.05) errors.push(best);
    else missed++;
  }
  const sorted = errors.slice().sort((a, b) => a - b);
  const median = sorted.length ? sorted[Math.floor(sorted.length / 2)] : 0;
  const p95 = sorted.length ? sorted[Math.floor(sorted.length * 0.95)] : 0;
  const worst = sorted.length ? sorted[sorted.length - 1] : 0;
  // Detections with no scored note near them. A few are expected at the ends
  // of long notes where the release re-exposes the reverb; many would mean the
  // detector is measuring something other than onsets, which is exactly the
  // failure the first version had.
  let spurious = 0;
  for (const t of detected) {
    if (!scoredOnsets.some((s) => Math.abs(s - t) < 0.05)) spurious++;
  }

  // ---- AC2: seek to five times, compare against the same passage reached by
  // playing into it.
  function bands(data, sampleRate, n = 8) {
    // Cheap signature: energy and zero-crossing rate per band of time. Enough
    // to tell "this is the right music" from "this is different music", which
    // is the whole of the question.
    const step = Math.floor(data.length / n);
    const out = [];
    for (let k = 0; k < n; k++) {
      let energy = 0;
      let crossings = 0;
      for (let i = k * step; i < (k + 1) * step && i < data.length; i++) {
        energy += data[i] * data[i];
        if (i > 0 && (data[i] >= 0) !== (data[i - 1] >= 0)) crossings++;
      }
      out.push([Math.sqrt(energy / step), crossings / step]);
    }
    return out;
  }
  const SILENT = 1e-4;
  /** t = 221.7 s is inside the ten-second caesura before the final chord
   *  (bars 77–80 are empty). It is kept deliberately: seeking into a silence
   *  must produce silence, and the previous run reported that correct result
   *  as a failure because a cosine similarity between two silent windows is
   *  0/0. A silence is now compared as a silence. */
  const SEEKS = [17.3, 61.9, 128.4, 190.0, 221.7];
  const LEAD_IN = 6.0;
  const seekChecks = [];
  for (const at of SEEKS) {
    const window = await render(at, 3.0);
    // The reference is not a fresh render at `at`: it is a render that *played
    // into* `at` from six seconds earlier, so it carries the releases and the
    // reverb tail a real listener would have arrived with. Comparing a seek
    // against another seek would prove nothing at all.
    const lead = await render(at - LEAD_IN, LEAD_IN + 3.0);
    const reference = lead.buf.getChannelData(0).slice(Math.floor(LEAD_IN * SR));
    const observed = window.buf.getChannelData(0);
    let ea = 0;
    let eb = 0;
    for (let i = 0; i < observed.length; i++) ea += observed[i] * observed[i];
    for (let i = 0; i < reference.length; i++) eb += reference[i] * reference[i];
    const rmsA = Math.sqrt(ea / Math.max(1, observed.length));
    const rmsB = Math.sqrt(eb / Math.max(1, reference.length));
    // What the score says is here, which is what "musically correct material
    // for that position" is measured against.
    const scoredHere = score.notes.filter((n) => n.atSec < at + 3.0 && n.atSec + n.durationSec > at).length;
    let similarity;
    if (scoredHere === 0 && rmsA < SILENT) {
      // The score is silent here and so is the seek. That is the right answer,
      // and the reference is *not* the thing to compare it against: a run that
      // played into this moment still has a reverb tail from the bar before,
      // which a fresh seek cannot have and should not have.
      similarity = 1;
    } else {
      const a = bands(observed, SR);
      const b = bands(reference, SR);
      let num = 0, da = 0, db = 0;
      for (let i = 0; i < a.length; i++) {
        for (let j = 0; j < 2; j++) {
          num += a[i][j] * b[i][j];
          da += a[i][j] ** 2;
          db += b[i][j] ** 2;
        }
      }
      similarity = num / (Math.sqrt(da) * Math.sqrt(db) || 1);
    }
    seekChecks.push({
      at,
      notes: window.scheduled,
      resumed: window.resumed,
      scoredHere,
      silent: scoredHere === 0 && rmsA < SILENT,
      referenceTailRms: +rmsB.toFixed(6),
      similarity: +similarity.toFixed(4),
    });
  }

  // ---- AC3: 100 randomised seek/pause/play, then check for stuck notes.
  const ctx3 = new OfflineAudioContext(1, SR, SR);
  const engine3 = new A.AudioEngine(ctx3);
  const sched3 = new A.Scheduler(engine3, score, clock, {});
  let seed = 12345;
  const rnd = () => ((seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff);
  let audioT = 0;
  const ops = [0, 0, 0];
  for (let i = 0; i < 100; i++) {
    const op = Math.floor(rnd() * 3);
    ops[op]++;
    audioT += 0.05 + rnd() * 0.2;
    if (op === 0) {
      clock.time = rnd() * score.durationSec;
      sched3.seek(clock.time, audioT);
    } else if (op === 1) {
      clock.playing = false;
      sched3.pump(audioT, clock.time);
    } else {
      clock.playing = true;
      clock.time += 0.1;
      sched3.pump(audioT, clock.time);
    }
  }
  // Stop, then let time run past every release that was ever scheduled. A
  // note whose release has passed and which is still counted is a leak; a
  // note still counted while its release is in the future is just a note.
  clock.playing = false;
  sched3.seek(clock.time, audioT + 1);
  sched3.pump(audioT + 3600, clock.time);
  const stuck = { pending: sched3.pendingCount, sounding: engine3.soundingCount, ops };

  // ---- AC4: a starved tick must not drop a cue.
  //
  // `setInterval(…, 25)` is a request, not a contract. Measured on this
  // project's own software rasteriser: sixteen pumps in three and a half
  // seconds — a tick of about 220 ms against a lookahead of 150. Windows
  // computed from `now` alone then have gaps, and a cue in a gap used to be
  // discarded in silence by the guard that exists for seeks.
  //
  // That is not a hypothetical: it is how DER KICK came to be scored,
  // playable and never bound. So this pumps the real scheduler at four
  // deliberately bad cadences and counts what arrives against what the score
  // says is in the interval. The cue may be LATE — the binder measures that —
  // but it may not be missing.
  const starved = [];
  {
    const allCues = A.cuesFromScore(score);
    const FROM = 180;
    const SPAN = 60;
    const expected = allCues.filter((c) => c.atSec >= FROM && c.atSec < FROM + SPAN);
    for (const pumpMs of [A.TICK_MS, 100, 250, 500]) {
      const seen = [];
      const ctx = new OfflineAudioContext(1, SR, SR);
      const engine = new A.AudioEngine(ctx);
      clock.time = FROM;
      clock.playing = true;
      const sched = new A.Scheduler(engine, score, clock, {
        cues: allCues,
        onCue: (c) => seen.push(c.atSec),
      });
      sched.seek(FROM, 0);
      for (let t = 0; t <= SPAN; t += pumpMs / 1000) {
        clock.time = FROM + t;
        sched.pump(t, clock.time);
      }
      const got = new Set(seen.map((x) => x.toFixed(6)));
      const missing = expected.filter((c) => !got.has(c.atSec.toFixed(6)));
      starved.push({
        pumpMs,
        lookaheadMs: A.LOOKAHEAD_SEC * 1000,
        expected: expected.length,
        delivered: seen.length,
        missing: missing.length,
        firstMissing: missing.length ? +missing[0].atSec.toFixed(3) : null,
      });
    }
  }

  return {
    ac4: starved,
    score: {
      notes: score.notes.length,
      durationSec: +score.durationSec.toFixed(3),
      ticksPerBeat: score.ticksPerBeat,
      bpm: +(60e6 / score.tempoMap[0].usPerBeat).toFixed(2),
      trackNames: score.trackNames,
      cues: A.cuesFromScore(score).length,
    },
    outputLatencyMs: +(latencySec * 1000).toFixed(3),
    ac1: {
      seconds: SECONDS,
      scoredOnsets: scoredOnsets.length,
      detected: detected.length,
      matched: errors.length,
      missed,
      spurious,
      worstMs: +(worst * 1000).toFixed(3),
      medianMs: +(median * 1000).toFixed(3),
      p95Ms: +(p95 * 1000).toFixed(3),
      scheduled: sopranoRun.scheduled,
    },
    ac2: seekChecks,
    ac3: stuck,
    lookaheadSec: A.LOOKAHEAD_SEC,
    tickMs: A.TICK_MS,
  };
}, midB64);

writeFileSync(`${outDir}/m70-schedule.json`, JSON.stringify({ ...result, warnings }, null, 2));

const ac4ok = result.ac4.every((r) => r.missing === 0);
const ac1ok = result.ac1.worstMs <= 3 && result.ac1.missed === 0;
console.log(`score: ${result.score.notes} notes, ${result.score.durationSec} s at ${result.score.bpm} bpm, ${result.score.ticksPerBeat} ppq`);
console.log(`       tracks ${result.score.trackNames.filter(Boolean).join(', ')}; ${result.score.cues} cues derived`);
console.log(`scheduler: ${result.tickMs} ms tick, ${result.lookaheadSec * 1000} ms lookahead\n`);
console.log(`mastering chain delays everything by ${result.outputLatencyMs} ms (measured; the compressor's lookahead)\n`);
console.log(`AC1 — soprano onsets measured from ${result.ac1.seconds} s of rendered audio, tapped at the voice bus`);
console.log(`  ${result.ac1.scoredOnsets} scored onsets, ${result.ac1.detected} detected, ${result.ac1.matched} matched, ${result.ac1.missed} missed, ${result.ac1.spurious} spurious`);
console.log(`  error: median ${result.ac1.medianMs} ms, p95 ${result.ac1.p95Ms} ms, worst ${result.ac1.worstMs} ms  ${ac1ok ? 'PASS (<= 3 ms, none missed)' : 'FAIL'}`);
console.log(`\nAC2 — seek, then compare against the same passage played into`);
for (const s of result.ac2) {
  console.log(`  t=${s.at}s  ${s.notes} notes + ${s.resumed} resumed  similarity ${s.similarity}${s.silent ? `  (scored silence, and silent; the played-in reference still has ${s.referenceTailRms} rms of reverb tail)` : ''}`);
}
console.log(`\nAC3 — after 100 randomised seek/pause/play (${result.ac3.ops.join('/')} seek/pause/play)`);
console.log(`  pending ${result.ac3.pending}, sounding ${result.ac3.sounding}  ${result.ac3.pending === 0 && result.ac3.sounding === 0 ? 'no stuck notes' : 'STUCK'}`);
console.log(`\nAC4 — a starved tick, against a ${result.lookaheadSec * 1000} ms lookahead`);
for (const r of result.ac4) {
  console.log(`  tick ${String(r.pumpMs).padStart(4)} ms  ${r.delivered} of ${r.expected} cues delivered, ` +
    `${r.missing} missing${r.firstMissing !== null ? ` (first at ${r.firstMissing}s)` : ''}` +
    `  ${r.missing === 0 ? 'ok' : 'DROPPED'}`);
}

console.log(`\nconsole warnings/errors: ${warnings.length}`);
for (const w of warnings.slice(0, 5)) console.log(`  ! ${w}`);
await browser.close();
