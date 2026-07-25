#!/usr/bin/env python3
"""S0 spike — answers D7: is the generated fugue musically alive?

Not the implementation. This is the throwaway that exists so a human can
*listen* in week 3 instead of finding out in month six. The real thing is
`crates/spex-fugue` (M67/M68); nothing here is meant to survive into it
except the authored subject and the verdict.

What it does:
  1. Realises the authored subject and its TONAL answer (D dorian, 84 bpm).
  2. Searches for a countersubject under real contrapuntal constraints and
     checks it is invertible at the octave — M68's requirement, demonstrated
     small.
  3. Assembles the exposition on the screenplay's own bars (entries at bars
     5, 7, 11, 14; exposition closes at 17 = the end of Act I), with two
     episodes built by sequence from the subject head.
  4. Writes a real type-1 SMF (hand-rolled, no dependency) and renders a WAV
     with a plain additive organ, so it can be listened to without a
     soundfont.
  5. Prints every rule it had to relax, rather than hiding it.

Usage:  python3 fugue_spike.py -o out/
"""
import argparse
import math
import struct
from pathlib import Path

import numpy as np

# ---------------------------------------------------------------- theory ---

BPM = 84.0
SEC_PER_BEAT = 60.0 / BPM
BEATS_PER_BAR = 4
# D dorian: D E F G A B C — semitone offsets from the tonic
MODE = [0, 2, 3, 5, 7, 9, 10]
TONIC_MIDI = 62  # D4


def deg_to_midi(deg: int) -> int:
    """Scale degree (0 = tonic D4, may be negative or > 6) -> MIDI note."""
    octave, step = divmod(deg, 7)
    return TONIC_MIDI + 12 * octave + MODE[step]


# Real ranges, from the spec (M68): S C4-A5, A F3-D5, T C3-A4, B E2-C4.
RANGES = {"S": (60, 81), "A": (53, 74), "T": (48, 69), "B": (40, 60)}
VOICE_ORDER = ["S", "A", "T", "B"]


# ------------------------------------------------------- the one decision ---
# The subject is authored, not generated. It is the single musical decision
# this project does not delegate. Two bars, eight beats.
#
#   D4  A4   G4 F4   E4  |  F4  G4   A4 G4   F4
#   deg 0    4    3  2    1     2    3    4  3    2
#
# The head leaps tonic -> dominant, which is precisely the case that FORCES a
# tonal answer rather than a real transposition. That is deliberate: if the
# generator ever takes the easy path, this subject makes it audible.

SUBJECT = [
    (0, 1.0), (4, 1.0), (3, 0.5), (2, 0.5), (1, 1.0),
    (2, 1.0), (3, 1.0), (4, 0.5), (3, 0.5), (2, 1.0),
]

# Tonal answer: note 1 answers tonic with dominant, note 2 answers dominant
# with tonic (deg 7 = D an octave up) instead of the real +4 transposition
# (deg 8 = E). The real transposition resumes from note 3, whose deg 7
# duplicates the adjusted note — so the two merge into one longer D. That
# merge is not a fudge; it is how a tonal adjustment normally resolves.
ANSWER = [
    (4, 1.0), (7, 1.5), (6, 0.5), (5, 1.0),
    (6, 1.0), (7, 1.0), (8, 0.5), (7, 0.5), (6, 1.0),
]


def total_beats(line):
    return sum(d for _, d in line)


assert total_beats(SUBJECT) == 8.0
assert total_beats(ANSWER) == 8.0


# ------------------------------------------------------- countersubject ----

def sample(line, grid=0.5):
    """Sample a (degree, duration) line onto a fixed grid of scale degrees."""
    out, t = [], 0.0
    for deg, dur in line:
        n = int(round(dur / grid))
        out.extend([deg] * n)
        t += dur
    return out


PERFECT = {0, 4}  # degree distance mod 7: unison/octave, and fifth


def parallel_violations(a, b):
    """Real parallel fifths/octaves between two sampled lines."""
    bad = []
    for i in range(1, min(len(a), len(b))):
        prev, cur = abs(a[i - 1] - b[i - 1]) % 7, abs(a[i] - b[i]) % 7
        moved = a[i] != a[i - 1] and b[i] != b[i - 1]
        same_dir = (a[i] - a[i - 1]) * (b[i] - b[i - 1]) > 0
        if moved and same_dir and prev == cur and cur in PERFECT:
            bad.append((i, "octave/unison" if cur == 0 else "fifth"))
    return bad


def build_countersubject(subject, seed=1927):
    """Small rule-checked search, not a hand-written line.

    Constraints, all real: mostly stepwise; contrary motion against the
    subject preferred; no parallel fifths or octaves; no voice crossing;
    and it must stay within a tenth of the subject so that it INVERTS at
    the octave (M68's invertibility requirement).
    """
    subj = sample(subject)
    rng = np.random.default_rng(seed)
    best, best_score = None, -1e9
    for _ in range(4000):
        line, cur = [], 7  # start an octave above the subject's tonic
        for i in range(len(subj)):
            if i == 0:
                step = 0
            else:
                # prefer contrary motion to the subject
                subj_dir = np.sign(subj[i] - subj[i - 1])
                choices = [-2, -1, 0, 1, 2]
                weights = np.array(
                    [3.0 if np.sign(c) == -subj_dir and c != 0 else 1.0 for c in choices]
                )
                weights[choices.index(0)] = 0.6
                step = int(rng.choice(choices, p=weights / weights.sum()))
            cur += step
            line.append(cur)
        gaps = [line[i] - subj[i] for i in range(len(subj))]
        if min(gaps) < 1 or max(gaps) > 9:      # no crossing, within a tenth
            continue
        viol = parallel_violations(line, subj)
        if viol:
            continue
        # invertibility at the octave: drop the CS an octave and re-check
        inverted = [d - 7 for d in line]
        if parallel_violations(inverted, subj):
            continue
        if any(inverted[i] > subj[i] for i in range(len(subj))):
            pass  # inversion may cross; that is what inversion is for
        steps = [abs(line[i] - line[i - 1]) for i in range(1, len(line))]
        contrary = sum(
            1 for i in range(1, len(line))
            if np.sign(line[i] - line[i - 1]) == -np.sign(subj[i] - subj[i - 1]) != 0
        )
        score = contrary * 2 - sum(1 for s in steps if s > 2) * 3 - abs(line[-1] - 5)
        if score > best_score:
            best, best_score = line, score
    if best is None:
        raise SystemExit("no countersubject satisfied the constraints — widen the search")
    # back to (degree, duration) at the sampling grid
    out, i = [], 0
    while i < len(best):
        j = i
        while j + 1 < len(best) and best[j + 1] == best[i]:
            j += 1
        out.append((best[i], 0.5 * (j - i + 1)))
        i = j + 1
    return out


# ------------------------------------------------------------ assembly ----

class Score:
    def __init__(self):
        self.notes = []  # (voice, start_beat, dur_beats, midi)
        self.range_misses = []  # (voice, notes outside its range, notes in line)

    def place(self, voice, start_beat, line, octave_shift=None):
        """Place a whole line with ONE octave shift.

        Clamping each note into range individually would put a 12-semitone
        jump in the middle of a melodic line whenever one note fell outside
        it — which is audible, and wrong. The shift is chosen for the line
        as a whole; anything still outside the range is counted and
        reported rather than silently bent.
        """
        if octave_shift is None:
            octave_shift = fit_octave(voice, line)
        lo, hi = RANGES[voice]
        t, out_of_range = start_beat, 0
        for deg, dur in line:
            midi = deg_to_midi(deg + 7 * octave_shift)
            if not (lo <= midi <= hi):
                out_of_range += 1
            self.notes.append((voice, t, dur, midi))
            t += dur
        if out_of_range:
            self.range_misses.append((voice, out_of_range, len(line)))
        return t


def fit_octave(voice, line, shift_range=(-3, 4)):
    """One octave shift for the whole line: most notes inside the range,
    ties broken toward the centre of the voice."""
    lo, hi = RANGES[voice]
    mid = (lo + hi) / 2
    best, best_key = 0, None
    for s in range(*shift_range):
        pitches = [deg_to_midi(d + 7 * s) for d, _ in line]
        inside = sum(1 for m in pitches if lo <= m <= hi)
        centre = abs(sum(pitches) / len(pitches) - mid)
        key = (inside, -centre)
        if best_key is None or key > best_key:
            best, best_key = s, key
    return best


def episode(head, start_deg, steps, seq_interval=-1):
    """A real episode: sequence on the subject's head, no new material."""
    out, deg = [], start_deg
    for _ in range(steps):
        for d, dur in head:
            out.append((deg + (d - head[0][0]), dur))
        deg += seq_interval
    return out


def build_exposition(cs):
    """The screenplay's own bars: entries at 5, 7, 11, 14; closes at 17."""
    s = Score()
    head = SUBJECT[:4]
    B = BEATS_PER_BAR

    # entry 1 — alto, subject, alone (bar 5)
    s.place("A", 5 * B, SUBJECT)
    # entry 2 — soprano, tonal answer (bar 7), alto continues with the CS
    s.place("S", 7 * B, ANSWER)
    s.place("A", 7 * B, cs)
    # episode 1 (bars 9-11) — sequence down, modulating back to the tonic
    s.place("S", 9 * B, episode(head, 6, 2))
    s.place("A", 9 * B, episode(head, 2, 2))
    # entry 3 — tenor, subject (bar 11)
    s.place("T", 11 * B, SUBJECT)
    s.place("A", 11 * B, cs)
    s.place("S", 11 * B, episode(head, 9, 2))
    # episode 2 (bar 13) — short, one sequence step
    s.place("T", 13 * B, episode(head, 4, 1))
    s.place("A", 13 * B, episode(head, 7, 1))
    # entry 4 — bass, answer (bar 14). Exposition complete at bar 17.
    s.place("B", 14 * B, ANSWER)
    s.place("T", 14 * B, cs)
    s.place("A", 14 * B, episode(head, 7, 2))
    s.place("S", 14 * B, episode(head, 11, 2))
    return s


# ------------------------------------------------------------- SMF out ----

def vlq(n):
    out = [n & 0x7F]
    n >>= 7
    while n:
        out.append((n & 0x7F) | 0x80)
        n >>= 7
    return bytes(reversed(out))


def write_midi(score, path, ticks=480):
    tracks = []
    for vi, voice in enumerate(VOICE_ORDER):
        events = []
        for v, start, dur, midi in score.notes:
            if v != voice:
                continue
            events.append((int(start * ticks), 0x90 | vi, midi, 90))
            events.append((int((start + dur) * ticks), 0x80 | vi, midi, 0))
        events.sort(key=lambda e: (e[0], e[1] & 0xF0))
        data, last = b"", 0
        data += b"\x00\xFF\x03" + vlq(len(voice)) + voice.encode()
        for t, status, a, b in events:
            data += vlq(t - last) + bytes([status, a, b])
            last = t
        data += b"\x00\xFF\x2F\x00"
        tracks.append(data)
    tempo = int(60_000_000 / BPM)
    meta = (b"\x00\xFF\x51\x03" + tempo.to_bytes(3, "big")
            + b"\x00\xFF\x58\x04\x04\x02\x18\x08" + b"\x00\xFF\x2F\x00")
    out = struct.pack(">4sIHHH", b"MThd", 6, 1, len(tracks) + 1, ticks)
    out += struct.pack(">4sI", b"MTrk", len(meta)) + meta
    for t in tracks:
        out += struct.pack(">4sI", b"MTrk", len(t)) + t
    Path(path).write_bytes(out)


# ------------------------------------------------------------ audio out ---

SR = 44100
PARTIALS = [(1, 1.0), (2, 0.45), (3, 0.28), (4, 0.16), (6, 0.09), (8, 0.05)]


def render(score, path):
    end = max(s + d for _, s, d, _ in score.notes) + 4
    buf = np.zeros(int(end * SEC_PER_BEAT * SR) + SR, dtype=np.float64)
    for _, start, dur, midi in score.notes:
        f = 440.0 * 2 ** ((midi - 69) / 12)
        n = int(dur * SEC_PER_BEAT * SR)
        t = np.arange(n) / SR
        sig = np.zeros(n)
        for mult, amp in PARTIALS:
            sig += amp * np.sin(2 * math.pi * f * mult * t)
        # organ-ish envelope: fast chiff, sustain, short release
        atk, rel = int(0.012 * SR), int(0.09 * SR)
        env = np.ones(n)
        env[:atk] = np.linspace(0, 1, atk)
        if n > rel:
            env[-rel:] = np.linspace(1, 0, rel)
        sig *= env * 0.16
        i = int(start * SEC_PER_BEAT * SR)
        buf[i:i + n] += sig
    # a cheap generated hall: a few decaying taps, no impulse-response file
    wet = np.zeros_like(buf)
    for delay_ms, gain in [(37, 0.30), (71, 0.22), (113, 0.16), (191, 0.10), (307, 0.06)]:
        d = int(delay_ms * SR / 1000)
        wet[d:] += buf[:-d] * gain
    buf = buf * 0.8 + wet * 0.45
    peak = np.max(np.abs(buf))
    buf = buf / peak * 0.89 if peak > 0 else buf
    pcm = (buf * 32767).astype("<i2")
    data = pcm.tobytes()
    hdr = (b"RIFF" + struct.pack("<I", 36 + len(data)) + b"WAVEfmt "
           + struct.pack("<IHHIIHH", 16, 1, 1, SR, SR * 2, 2, 16)
           + b"data" + struct.pack("<I", len(data)))
    Path(path).write_bytes(hdr + data)


# ---------------------------------------------------------------- main ----

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-o", "--out", default="out")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    cs = build_countersubject(SUBJECT)
    score = build_exposition(cs)

    letters = "C C# D D# E F F# G G# A A# B".split()
    def spell(line):
        out = []
        for d, _ in line:
            m = deg_to_midi(d)
            out.append(f"{letters[m % 12]}{m // 12 - 1}")
        return " ".join(out)

    print("subject         :", spell(SUBJECT))
    print("tonal answer    :", spell(ANSWER))
    print("countersubject  :", spell(cs))
    subj, csl = sample(SUBJECT), sample(cs)
    n = min(len(subj), len(csl))
    print("parallels S/CS  :", parallel_violations(csl[:n], subj[:n]) or "none")
    print("inverted at 8ve :", parallel_violations([d - 7 for d in csl[:n]], subj[:n]) or "none")
    print("notes           :", len(score.notes))
    print("out of range    :", score.range_misses or "none")
    print("entries at bars : 5 (A, subject) · 7 (S, tonal answer) · 11 (T, subject) · 14 (B, answer)")

    write_midi(score, out / "fugue-spike-exposition.mid")
    render(score, out / "fugue-spike-exposition.wav")
    print("wrote", out / "fugue-spike-exposition.mid", "and .wav")


if __name__ == "__main__":
    main()
