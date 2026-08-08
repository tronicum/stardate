//! M68 — the standard MIDI file writer.
//!
//! Hand-rolled, and the reason is not stubbornness about dependencies. Rev 4
//! of the spec made **one artefact the whole score**: the browser plays this
//! file and a person opens *the same file* in a DAW, so review and runtime
//! cannot disagree. A format that central is worth being able to read end to
//! end, and a type-1 SMF is a header chunk, a length, and a list of
//! delta-timed events. It is about a hundred and fifty lines.
//!
//! # Type 1, one track per voice
//!
//! Type 0 puts everything in one track and would play identically. Type 1
//! keeps the four voices apart, which is what makes the file *editable* — open
//! it in Logic and there are four named staves, not one pile of notes. Since
//! the point of emitting MIDI at all is that a human can look at it, the
//! division is the feature.
//!
//! Track 0 carries tempo and time signature and no notes, which is the
//! convention every DAW expects of a type-1 file.
//!
//! # Ticks
//!
//! 960 per quarter note. Divisible by 2, 3, 4, 5, 6 and 8, so every duration
//! this piece uses — halves, quarters, eighths, and the augmentation by 2 and
//! diminution by 4 the plan asks for — lands on a whole tick. A rounding error
//! in a score is a note that starts a millisecond late forever.

use crate::counterpoint::{Realisation, VOICE_NAMES};

/// General MIDI percussion, on channel 10 (index 9). Not invented numbers: a
/// drum track that opens in a DAW as a drum track is the whole reason the
/// pulse lives in the score file at all.
pub const GM_KICK: u8 = 36;
pub const GM_CLAP: u8 = 39;
pub const GM_HAT_CLOSED: u8 = 42;
pub const GM_HAT_OPEN: u8 = 46;
/// MIDI channel index for percussion. 9 is channel 10 in one-based counting.
pub const PULSE_CHANNEL: u8 = 9;
/// The marker written at the final accent. The camera binds to this text and
/// to nothing else, so the Kick has exactly one definition in the whole work.
pub const KICK_MARKER: &str = "KICK";

/// Ticks per quarter note. See the module header.
pub const TICKS_PER_BEAT: u32 = 960;

/// A MIDI variable-length quantity: seven bits per byte, high bit set on every
/// byte but the last.
fn write_vlq(out: &mut Vec<u8>, mut value: u32) {
    let mut buf = [0u8; 4];
    let mut i = 0;
    buf[i] = (value & 0x7F) as u8;
    value >>= 7;
    while value > 0 {
        i += 1;
        buf[i] = ((value & 0x7F) as u8) | 0x80;
        value >>= 7;
    }
    for j in (0..=i).rev() {
        out.push(buf[j]);
    }
}

fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(id);
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(body);
    out
}

fn meta(out: &mut Vec<u8>, delta: u32, kind: u8, data: &[u8]) {
    write_vlq(out, delta);
    out.push(0xFF);
    out.push(kind);
    write_vlq(out, data.len() as u32);
    out.extend_from_slice(data);
}

/// One note-on or note-off, as an absolute-tick event to be sorted later.
struct Event {
    tick: u32,
    /// Note-offs sort before note-ons at the same tick, so a repeated pitch
    /// re-articulates instead of being silently swallowed by its own release.
    order: u8,
    status: u8,
    data1: u8,
    data2: u8,
}

/// The Act IV percussive layer, as a channel-10 track.
///
/// # Why the pulse is in the score and not in the runtime
///
/// M69 synthesises the kick, the hat and the clap, so a runtime generator
/// reading `PulseSpec` would have worked. It would also have meant that the
/// file a person opens in a DAW is missing an entire layer of the piece — and
/// rev 4's whole point is that there is one artefact. A General MIDI drum
/// track costs nothing extra: the numbers are standard, every DAW names them,
/// and the browser needs one branch on the channel.
///
/// # The pattern
///
/// `bpm_multiplier` is 2, so the grid is eighth notes against the fugue's 84
/// and quarter notes against the pulse's own 168 — a half-time feel, which is
/// what makes a techno layer sit *under* counterpoint rather than fight it.
/// One pulse bar is therefore two score beats: kick on its 1 and 3, clap on
/// the backbeats, closed hat on every eighth of the doubled grid with the last
/// one opened.
fn pulse_track(r: &Realisation) -> Vec<u8> {
    let mut events: Vec<Event> = Vec::new();
    let beats_per_sec = r.bpm / 60.0;
    let start_beat = r.pulse.enter_at_sec * beats_per_sec;
    let end_beat = r.pulse.final_accent_at_sec * beats_per_sec;
    // One pulse bar in score beats. At bpm_multiplier 2 that is two beats.
    let bar = 4.0 / r.pulse.bpm_multiplier;
    let step = bar / 8.0; // an eighth of the doubled grid

    let mut hit = |events: &mut Vec<Event>, beat: f64, note: u8, velocity: u8| {
        let on = (beat * TICKS_PER_BEAT as f64).round() as u32;
        events.push(Event { tick: on, order: 1, status: 0x90 | PULSE_CHANNEL, data1: note, data2: velocity });
        // A percussion note-off is a formality — the sound is a one-shot and
        // its length is in the synthesis, not in the score. It is written
        // anyway, one tick later, because a DAW that never sees a note-off
        // draws a note that lasts to the end of the piece.
        events.push(Event { tick: on + 1, order: 0, status: 0x80 | PULSE_CHANNEL, data1: note, data2: 0 });
    };

    let mut b = start_beat;
    while b < end_beat - 1e-9 {
        for i in 0..8 {
            let at = b + i as f64 * step;
            if at >= end_beat - 1e-9 {
                break;
            }
            match i {
                0 => hit(&mut events, at, GM_KICK, 110),
                4 => hit(&mut events, at, GM_KICK, 96),
                2 | 6 => hit(&mut events, at, GM_CLAP, 88),
                7 => hit(&mut events, at, GM_HAT_OPEN, 70),
                _ => hit(&mut events, at, GM_HAT_CLOSED, 64),
            }
            if i % 2 == 1 && i != 7 {
                hit(&mut events, at, GM_HAT_CLOSED, 52);
            }
        }
        b += bar;
    }
    // The Kick. Kick and clap together at full velocity, at the one moment the
    // whole piece ends on.
    hit(&mut events, end_beat, GM_KICK, 127);
    hit(&mut events, end_beat, GM_CLAP, 127);

    events.sort_by_key(|e| (e.tick, e.order, e.data1));
    let mut body = Vec::new();
    meta(&mut body, 0, 0x03, b"Pulse");
    let mut last = 0u32;
    for e in &events {
        write_vlq(&mut body, e.tick - last);
        last = e.tick;
        body.push(e.status);
        body.push(e.data1);
        body.push(e.data2);
    }
    meta(&mut body, 0, 0x2F, &[]);
    body
}

/// Writes a realised score as a type-1 standard MIDI file.
pub fn to_smf(r: &Realisation) -> Vec<u8> {
    let mut tracks: Vec<Vec<u8>> = Vec::new();

    // Track 0: tempo map, no notes.
    let mut t0 = Vec::new();
    meta(&mut t0, 0, 0x03, b"Die Geschichtliche Matrix");
    let us_per_beat = (60_000_000.0 / r.bpm).round() as u32;
    meta(&mut t0, 0, 0x51, &us_per_beat.to_be_bytes()[1..]);
    // Time signature: numerator, denominator as a power of two, MIDI clocks
    // per metronome click, 32nds per quarter.
    let denom_pow = (r.beats_per_bar as u32).trailing_zeros().max(2).min(2); // 4/4
    let _ = denom_pow;
    meta(&mut t0, 0, 0x58, &[r.beats_per_bar as u8, 2, 24, 8]);

    // The form, as MIDI markers (meta 0x06) on the conductor track.
    //
    // **This is not documentation, it is the runtime's section list.** M71
    // binds a HUD card to each section boundary, and the alternative — a
    // second table of bar numbers beside the score — is two things that have
    // to be kept in step and eventually are not. Writing them here means a DAW
    // shows the form in its marker lane *and* the browser reads the same
    // bytes: one artefact, as rev 4 asked for.
    let mut marker_last = 0u32;
    let mut markers: Vec<(u32, String)> =
        r.sections.iter().map(|m| ((m.at_beat * TICKS_PER_BEAT as f64).round() as u32, m.label.clone())).collect();
    let kick_tick = (r.pulse.final_accent_at_sec * r.bpm / 60.0 * TICKS_PER_BEAT as f64).round() as u32;
    markers.push((kick_tick, KICK_MARKER.to_string()));
    markers.sort_by_key(|(t, _)| *t);
    for (tick, label) in &markers {
        meta(&mut t0, tick - marker_last, 0x06, label.as_bytes());
        marker_last = *tick;
    }
    meta(&mut t0, 0, 0x2F, &[]);
    tracks.push(chunk(b"MTrk", &t0));

    for voice in 0..4 {
        let mut events: Vec<Event> = Vec::new();
        for n in r.notes.iter().filter(|n| n.voice == voice) {
            let on = (n.at_beat * TICKS_PER_BEAT as f64).round() as u32;
            let off = (n.end_beat() * TICKS_PER_BEAT as f64).round() as u32;
            if off <= on {
                continue;
            }
            let midi = n.midi.clamp(0, 127) as u8;
            // Velocity by voice: the subject-carrying inner voices a little
            // stronger than the filling. A flat 100 across four parts is what
            // makes generated MIDI sound like generated MIDI.
            let velocity = match voice {
                0 => 92,
                1 => 88,
                2 => 84,
                _ => 90,
            };
            events.push(Event { tick: on, order: 1, status: 0x90 | voice as u8, data1: midi, data2: velocity });
            events.push(Event { tick: off, order: 0, status: 0x80 | voice as u8, data1: midi, data2: 0 });
        }
        events.sort_by_key(|e| (e.tick, e.order, e.data1));

        let mut body = Vec::new();
        meta(&mut body, 0, 0x03, VOICE_NAMES[voice].as_bytes());
        let mut last = 0u32;
        for e in &events {
            write_vlq(&mut body, e.tick - last);
            last = e.tick;
            body.push(e.status);
            body.push(e.data1);
            body.push(e.data2);
        }
        meta(&mut body, 0, 0x2F, &[]);
        tracks.push(chunk(b"MTrk", &body));
    }

    tracks.push(chunk(b"MTrk", &pulse_track(r)));

    let mut header = Vec::new();
    header.extend_from_slice(&1u16.to_be_bytes()); // format 1
    header.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
    header.extend_from_slice(&(TICKS_PER_BEAT as u16).to_be_bytes());

    let mut out = chunk(b"MThd", &header);
    for t in tracks {
        out.extend_from_slice(&t);
    }
    out
}

/// Reads a type-1 SMF back into `(voice, tick, midi, duration)` tuples.
///
/// This exists for AC1, and the fact that it exists is the point: the
/// acceptance criterion is that *the emitted score* has no parallel fifths,
/// not that the thing the generator was holding in memory had none. Analysing
/// the in-memory `Realisation` would test the generator against itself and
/// would pass even if the writer dropped every second note.
pub fn read_smf(bytes: &[u8]) -> Result<Vec<(usize, u32, i32, u32)>, String> {
    let mut pos = 0usize;
    let take = |pos: &mut usize, n: usize| -> Result<&[u8], String> {
        if *pos + n > bytes.len() {
            return Err(format!("truncated at {pos}"));
        }
        let s = &bytes[*pos..*pos + n];
        *pos += n;
        Ok(s)
    };
    if take(&mut pos, 4)? != b"MThd" {
        return Err("not an SMF".into());
    }
    let len = u32::from_be_bytes(take(&mut pos, 4)?.try_into().unwrap()) as usize;
    let head = take(&mut pos, len)?;
    let format = u16::from_be_bytes(head[0..2].try_into().unwrap());
    if format != 1 {
        return Err(format!("expected a type-1 file, got type {format}"));
    }
    let track_count = u16::from_be_bytes(head[2..4].try_into().unwrap()) as usize;

    let mut out = Vec::new();
    for _ in 0..track_count {
        if take(&mut pos, 4)? != b"MTrk" {
            return Err("expected MTrk".into());
        }
        let len = u32::from_be_bytes(take(&mut pos, 4)?.try_into().unwrap()) as usize;
        let end = pos + len;
        let mut tick = 0u32;
        let mut running = 0u8;
        let mut open: Vec<(usize, i32, u32)> = Vec::new();
        while pos < end {
            // delta
            let mut delta = 0u32;
            loop {
                let b = take(&mut pos, 1)?[0];
                delta = (delta << 7) | (b & 0x7F) as u32;
                if b & 0x80 == 0 {
                    break;
                }
            }
            tick += delta;
            let mut status = bytes[pos];
            if status < 0x80 {
                status = running; // running status
            } else {
                pos += 1;
                running = status;
            }
            match status & 0xF0 {
                0x80 | 0x90 => {
                    let d1 = take(&mut pos, 1)?[0] as i32;
                    let d2 = take(&mut pos, 1)?[0];
                    let ch = (status & 0x0F) as usize;
                    if status & 0xF0 == 0x90 && d2 > 0 {
                        open.push((ch, d1, tick));
                    } else if let Some(i) =
                        open.iter().position(|(c, n, _)| *c == ch && *n == d1)
                    {
                        let (c, n, start) = open.remove(i);
                        out.push((c, start, n, tick - start));
                    }
                }
                0xA0 | 0xB0 | 0xE0 => {
                    take(&mut pos, 2)?;
                }
                0xC0 | 0xD0 => {
                    take(&mut pos, 1)?;
                }
                0xF0 => {
                    if status == 0xFF {
                        take(&mut pos, 1)?; // meta type
                    }
                    let mut mlen = 0u32;
                    loop {
                        let b = take(&mut pos, 1)?[0];
                        mlen = (mlen << 7) | (b & 0x7F) as u32;
                        if b & 0x80 == 0 {
                            break;
                        }
                    }
                    take(&mut pos, mlen as usize)?;
                }
                _ => return Err(format!("unhandled status {status:#04x} at {pos}")),
            }
        }
        pos = end;
    }
    out.sort_by_key(|(v, t, n, _)| (*t, *v, *n));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_length_quantities_match_the_midi_specs_own_examples() {
        // Straight from the SMF specification's table. If this is wrong every
        // note in the file is at the wrong time, and nothing else would say so.
        let cases: &[(u32, &[u8])] = &[
            (0x00000000, &[0x00]),
            (0x00000040, &[0x40]),
            (0x0000007F, &[0x7F]),
            (0x00000080, &[0x81, 0x00]),
            (0x00002000, &[0xC0, 0x00]),
            (0x00003FFF, &[0xFF, 0x7F]),
            (0x00004000, &[0x81, 0x80, 0x00]),
            (0x00100000, &[0xC0, 0x80, 0x00]),
            (0x001FFFFF, &[0xFF, 0xFF, 0x7F]),
            (0x00200000, &[0x81, 0x80, 0x80, 0x00]),
            (0x0FFFFFFF, &[0xFF, 0xFF, 0xFF, 0x7F]),
        ];
        for (value, expected) in cases {
            let mut out = Vec::new();
            write_vlq(&mut out, *value);
            assert_eq!(&out[..], *expected, "vlq of {value:#x}");
        }
    }

    #[test]
    fn ticks_per_beat_divides_every_duration_the_piece_uses() {
        for d in [1.0, 0.5, 0.25, 2.0, 4.0, 1.0 / 3.0] {
            let ticks = d * TICKS_PER_BEAT as f64;
            if d == 1.0 / 3.0 {
                assert_eq!(ticks, 320.0, "even a triplet is whole");
            } else {
                assert_eq!(ticks.fract(), 0.0, "{d} beats is not a whole number of ticks");
            }
        }
    }
}
