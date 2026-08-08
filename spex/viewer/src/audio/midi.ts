/** M70 — the event model, and the SMF reader that is the runtime path.
 *
 * # This reader is not a convenience, it is the whole point
 *
 * Rev 4 of the spec deleted a format so that **one file is the score**: M68
 * writes a standard MIDI file, this reads it, and the browser plays what a
 * person opens in a DAW. Two consequences follow, and both are the reason the
 * decision was made:
 *
 * - **The reader cannot silently rot.** It is not an escape hatch that gets
 *   exercised when someone remembers to; nothing plays without it, so every
 *   single run is a test of it.
 * - **Substituting a different, clearly-licensed `.mid` needs no code at
 *   all.** Drop in another file. For a work whose licensing questions are
 *   live (see `docs/fugen/licensing.md`), that is not a small property.
 *
 * # Ticks, seconds, and why both are carried
 *
 * A MIDI file is timed in ticks against a tempo map. Show time is in seconds.
 * Converting once at load and carrying **both** on every event means the
 * scheduler never divides, the HUD can say "bar 37" without a second table,
 * and a tempo change — which this piece does not have but the format does —
 * lands in exactly one function.
 */

/** One note, already resolved to seconds. The scheduler wants nothing else. */
export interface ScoredNote {
  /** MIDI channel, which for a `spex fugue-build` file is the voice. */
  voice: number;
  midi: number;
  velocity: number;
  /** Seconds from the start of the score. */
  atSec: number;
  durationSec: number;
  tick: number;
}

export interface TempoPoint {
  tick: number;
  usPerBeat: number;
  /** Seconds at this tick, accumulated from the points before it. */
  atSec: number;
}

/** A marker meta event (0x06), which for this score is the musical form.
 *
 * M71 put the section plan into the file rather than into a table beside it:
 * a DAW shows the markers in its marker lane, the runtime reads the same
 * bytes, and there is no second list that can drift out of step with the
 * notes. `KICK` is one of them — the final accent has exactly one definition
 * in the whole work, and it is a string in the score. */
export interface ScoreMarker {
  tick: number;
  atSec: number;
  text: string;
}

export interface Score {
  ticksPerBeat: number;
  tempoMap: TempoPoint[];
  notes: ScoredNote[];
  markers: ScoreMarker[];
  /** Track names, in file order. Track 0 is the tempo map. */
  trackNames: string[];
  durationSec: number;
}

/** MIDI channel 10 (index 9): General MIDI percussion. Not a voice.
 *
 * The pulse lives in the score file as a real drum track, so every reader of
 * the file — this one, a DAW, the counterpoint tests — has to know that
 * channel 10 is not a fifth contrapuntal part. Note 36 against note 39 is a
 * minor third that never moves, and a rule checker handed those alongside the
 * fugue reports parallel thirds in a kick drum. */
export const PULSE_CHANNEL = 9;

/** General MIDI percussion numbers this project writes and reads. */
export const GM = { kick: 36, clap: 39, hatClosed: 42, hatOpen: 46 } as const;

/** The marker text the camera Kick binds to. */
export const KICK_MARKER = 'KICK';

class Reader {
  constructor(
    readonly view: DataView,
    public pos = 0,
  ) {}
  u8(): number {
    return this.view.getUint8(this.pos++);
  }
  u16(): number {
    const v = this.view.getUint16(this.pos);
    this.pos += 2;
    return v;
  }
  u32(): number {
    const v = this.view.getUint32(this.pos);
    this.pos += 4;
    return v;
  }
  bytes(n: number): Uint8Array {
    const v = new Uint8Array(this.view.buffer, this.view.byteOffset + this.pos, n);
    this.pos += n;
    return v;
  }
  ascii(n: number): string {
    return String.fromCharCode(...this.bytes(n));
  }
  /** A variable-length quantity: seven bits a byte, high bit continues. */
  vlq(): number {
    let v = 0;
    for (;;) {
      const b = this.u8();
      v = (v << 7) | (b & 0x7f);
      if (!(b & 0x80)) return v;
    }
  }
}

/** Parses a type-0 or type-1 standard MIDI file.
 *
 * Type 0 as well as type 1, even though this project only writes type 1: the
 * point of loading a `.mid` at runtime is that *someone else's* file works,
 * and a DAW export is as likely to be one as the other.
 */
export function parseSmf(data: ArrayBuffer): Score {
  const r = new Reader(new DataView(data));
  if (r.ascii(4) !== 'MThd') throw new Error('not a standard MIDI file (no MThd)');
  const headerLength = r.u32();
  const headerEnd = r.pos + headerLength;
  const format = r.u16();
  const trackCount = r.u16();
  const division = r.u16();
  if (division & 0x8000) {
    // SMPTE timing. The piece is written in bars against a tempo, so this
    // would be a file from a different world; refusing is better than
    // guessing at a frame rate.
    throw new Error('SMPTE-timed MIDI files are not supported; this score is tempo-timed');
  }
  const ticksPerBeat = division;
  r.pos = headerEnd;

  // Pass 1: the tempo map, gathered from every track. In a type-1 file it is
  // conventionally in track 0, but "conventionally" is not "always" and a
  // tempo event anywhere changes the time of every note after it.
  const tempoEvents: { tick: number; usPerBeat: number }[] = [];
  const rawMarkers: { tick: number; text: string }[] = [];
  const trackChunks: { start: number; end: number }[] = [];
  let scan = r.pos;
  for (let t = 0; t < trackCount; t++) {
    const tr = new Reader(new DataView(data), scan);
    if (tr.ascii(4) !== 'MTrk') throw new Error(`track ${t}: expected MTrk`);
    const len = tr.u32();
    trackChunks.push({ start: tr.pos, end: tr.pos + len });
    scan = tr.pos + len;
  }

  const trackNames: string[] = [];
  for (const chunk of trackChunks) {
    const tr = new Reader(new DataView(data), chunk.start);
    let tick = 0;
    let running = 0;
    let name = '';
    while (tr.pos < chunk.end) {
      tick += tr.vlq();
      let status = tr.view.getUint8(tr.pos);
      if (status < 0x80) status = running;
      else {
        tr.pos++;
        running = status;
      }
      const high = status & 0xf0;
      if (high === 0x80 || high === 0x90 || high === 0xa0 || high === 0xb0 || high === 0xe0) {
        tr.pos += 2;
      } else if (high === 0xc0 || high === 0xd0) {
        tr.pos += 1;
      } else if (status === 0xff) {
        const type = tr.u8();
        const len = tr.vlq();
        const payload = tr.bytes(len);
        if (type === 0x51 && len === 3) {
          tempoEvents.push({ tick, usPerBeat: (payload[0] << 16) | (payload[1] << 8) | payload[2] });
        } else if (type === 0x03) {
          name = String.fromCharCode(...payload);
        } else if (type === 0x06) {
          rawMarkers.push({ tick, text: String.fromCharCode(...payload) });
        }
      } else if (status === 0xf0 || status === 0xf7) {
        // `tr.pos += tr.vlq()` is WRONG here and was, for a while. JavaScript
        // evaluates the left-hand reference *before* the right-hand call, so
        // `pos += vlq()` computes `oldPos + length` and throws away the bytes
        // the length itself occupied. See the note on the meta branch below.
        const skip = tr.vlq();
        tr.pos += skip;
      } else {
        throw new Error(`unhandled status ${status.toString(16)} at byte ${tr.pos}`);
      }
    }
    trackNames.push(name);
  }

  tempoEvents.sort((a, b) => a.tick - b.tick);
  if (tempoEvents.length === 0 || tempoEvents[0].tick > 0) {
    // 120 bpm is the MIDI default and the only defensible assumption.
    tempoEvents.unshift({ tick: 0, usPerBeat: 500000 });
  }
  const tempoMap: TempoPoint[] = [];
  let seconds = 0;
  for (let i = 0; i < tempoEvents.length; i++) {
    if (i > 0) {
      const prev = tempoEvents[i - 1];
      seconds += ((tempoEvents[i].tick - prev.tick) / ticksPerBeat) * (prev.usPerBeat / 1e6);
    }
    tempoMap.push({ tick: tempoEvents[i].tick, usPerBeat: tempoEvents[i].usPerBeat, atSec: seconds });
  }

  const markers: ScoreMarker[] = rawMarkers
    .map((m) => ({ ...m, atSec: tickToSeconds(m.tick, ticksPerBeat, tempoMap) }))
    .sort((a, b) => a.atSec - b.atSec);

  // Pass 2: the notes.
  const notes: ScoredNote[] = [];
  for (const chunk of trackChunks) {
    const tr = new Reader(new DataView(data), chunk.start);
    let tick = 0;
    let running = 0;
    const open: { voice: number; midi: number; velocity: number; tick: number }[] = [];
    while (tr.pos < chunk.end) {
      tick += tr.vlq();
      let status = tr.view.getUint8(tr.pos);
      if (status < 0x80) status = running;
      else {
        tr.pos++;
        running = status;
      }
      const high = status & 0xf0;
      const channel = status & 0x0f;
      if (high === 0x90 || high === 0x80) {
        const midi = tr.u8();
        const velocity = tr.u8();
        if (high === 0x90 && velocity > 0) {
          open.push({ voice: channel, midi, velocity, tick });
        } else {
          // Last-in-first-out, so a repeated pitch closes its own note-on
          // rather than the oldest one — which is what makes a re-articulated
          // pedal come out with the right lengths.
          const i = open.map((o) => o.voice === channel && o.midi === midi).lastIndexOf(true);
          if (i >= 0) {
            const [n] = open.splice(i, 1);
            const atSec = tickToSeconds(n.tick, ticksPerBeat, tempoMap);
            notes.push({
              voice: n.voice,
              midi: n.midi,
              velocity: n.velocity / 127,
              atSec,
              durationSec: tickToSeconds(tick, ticksPerBeat, tempoMap) - atSec,
              tick: n.tick,
            });
          }
        }
      } else if (high === 0xa0 || high === 0xb0 || high === 0xe0) {
        tr.pos += 2;
      } else if (high === 0xc0 || high === 0xd0) {
        tr.pos += 1;
      } else if (status === 0xff) {
        tr.u8();
        // **`tr.pos += tr.vlq()` is a bug**, and it cost an afternoon.
        // `a.pos += f()` is `a.pos = a.pos + f()`, and JavaScript reads
        // `a.pos` *before* calling `f` — so when `f` advances `pos` itself,
        // the assignment overwrites that with the stale value plus the
        // result. Here it silently lost the byte the length field occupied,
        // which put the reader one byte behind for the rest of the track:
        // every subsequent delta was read from the middle of some other
        // event, and the parse desynchronised into plausible-looking garbage
        // — note times that were nearly right rather than obviously wrong.
        //
        // The first pass over the same bytes did not have this bug, because
        // it reads the payload with `bytes(len)` rather than skipping it.
        // Two readers of one format, one of them subtly wrong: exactly the
        // thing this project keeps finding.
        const skip = tr.vlq();
        tr.pos += skip;
      } else if (status === 0xf0 || status === 0xf7) {
        const skip = tr.vlq();
        tr.pos += skip;
      }
    }
    // A note-on with no matching note-off is a broken file, not a note that
    // sounds for ever. It ends where the track does.
    for (const n of open) {
      const atSec = tickToSeconds(n.tick, ticksPerBeat, tempoMap);
      notes.push({
        voice: n.voice,
        midi: n.midi,
        velocity: n.velocity / 127,
        atSec,
        durationSec: Math.max(0.05, tickToSeconds(tick, ticksPerBeat, tempoMap) - atSec),
        tick: n.tick,
      });
    }
  }

  notes.sort((a, b) => a.atSec - b.atSec || a.voice - b.voice || a.midi - b.midi);
  const durationSec = notes.reduce((m, n) => Math.max(m, n.atSec + n.durationSec), 0);
  if (format !== 0 && format !== 1) {
    throw new Error(`MIDI format ${format} is not supported`);
  }
  return { ticksPerBeat, tempoMap, notes, markers, trackNames, durationSec };
}

/** Tick to seconds, through the tempo map. */
export function tickToSeconds(tick: number, ticksPerBeat: number, map: TempoPoint[]): number {
  let point = map[0];
  for (const p of map) {
    if (p.tick <= tick) point = p;
    else break;
  }
  return point.atSec + ((tick - point.tick) / ticksPerBeat) * (point.usPerBeat / 1e6);
}

/** Seconds to ticks — the inverse, for seeking by bar. */
export function secondsToTick(sec: number, ticksPerBeat: number, map: TempoPoint[]): number {
  let point = map[0];
  for (const p of map) {
    if (p.atSec <= sec) point = p;
    else break;
  }
  return point.tick + ((sec - point.atSec) * 1e6 * ticksPerBeat) / point.usPerBeat;
}

/** Fetches and parses a score. `null` when there is none — a show without
 * music is still a show, and the viewer's other two modes have no audio at
 * all, so a 404 here is a fact rather than an error. */
export async function fetchScore(baseUrl: string, file = 'fugue.mid'): Promise<Score | null> {
  const res = await fetch(`${baseUrl.replace(/\/$/, '')}/${file}`);
  if (!res.ok) return null;
  return parseSmf(await res.arrayBuffer());
}
