#!/usr/bin/env python3
"""Plan and elevation of a built `.ldr`, as a picture, in a second.

    ./target/release/spex build recipes/heritage/jelling.json -o /tmp/jelling.ldr
    python3 scripts/ldraw/atlas-elevation.py /tmp/jelling.ldr -o /tmp/jelling.png

WHY NOT THE RENDERER
--------------------
Checking a massing is checking proportions and layout: is the tower taller than
the range, is the church actually between the two mounds, does the palisade
enclose anything. A lit, shaded, post-processed frame answers those questions
too, and on this container's software rasteriser it takes two to four minutes
per site. Forty sites is three hours of waiting to find out that a coordinate
sign was wrong.

So this projects the real placement coordinates -- read out of the real emitted
`.ldr`, not out of the recipe -- orthographically, twice: the plan from above
and the elevation from the south. No lighting, no perspective, no LOD. It is a
drawing, and a drawing is what a massing wants.

The renderer still gets the last word. This is the instrument for the forty
iterations before that.
"""
import argparse
import pathlib
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

LDU_PER_STUD = 20.0
LDU_PER_MM = 1 / 0.4  # 1 LDU = 0.4 mm


def read_ldr(path):
    """(x, y, z, colour) per type-1 line. LDraw is +Y DOWN."""
    pts = []
    for line in pathlib.Path(path).read_text().splitlines():
        t = line.split()
        if len(t) < 15 or t[0] != "1":
            continue
        try:
            pts.append((float(t[2]), float(t[3]), float(t[4]), int(t[1])))
        except ValueError:
            continue
    return np.array([(p[0], p[1], p[2]) for p in pts]), [p[3] for p in pts]


def ldraw_colors(config_path):
    out = {}
    try:
        text = pathlib.Path(config_path).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return out
    for line in text.splitlines():
        t = line.split()
        if len(t) < 8 or t[1] != "!COLOUR":
            continue
        try:
            hexv = t[t.index("VALUE") + 1].lstrip("#")
            out[int(t[t.index("CODE") + 1])] = tuple(int(hexv[i:i + 2], 16) for i in (0, 2, 4))
        except (ValueError, IndexError):
            continue
    return out


def project(draw, pts, cols, palette, u_idx, v_idx, flip_v, box, label, spm):
    x0, y0, w, h = box
    u, v = pts[:, u_idx], pts[:, v_idx]
    if flip_v:
        v = -v
    umin, umax, vmin, vmax = u.min(), u.max(), v.min(), v.max()
    span = max(umax - umin, vmax - vmin, 1.0)
    scale = (min(w, h) - 24) / span
    # One dot per placement, sized so a dense wall reads as a surface and a
    # single runestone still reads as a mark.
    r = max(1.0, scale * LDU_PER_STUD * 0.45)
    for i in range(len(u)):
        px = x0 + 12 + (u[i] - umin) * scale
        py = y0 + h - 12 - (v[i] - vmin) * scale
        c = palette.get(cols[i], (200, 200, 200))
        draw.ellipse([px - r, py - r, px + r, py + r], fill=c)
    metres = span / LDU_PER_STUD / spm
    return f"{label}   {metres:,.0f} m across   {len(u):,} Steine"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ldr")
    ap.add_argument("-o", "--out", required=True)
    ap.add_argument("--spm", type=float, default=1.0, help="studs per metre, for the scale bar")
    ap.add_argument("--title", default="")
    ap.add_argument("--ldconfig", default=".ldraw-cache/LDConfig.ldr")
    args = ap.parse_args()

    pts, cols = read_ldr(args.ldr)
    if not len(pts):
        sys.exit(f"{args.ldr} has no type-1 lines")
    palette = ldraw_colors(args.ldconfig)

    W, H = 1400, 760
    img = Image.new("RGB", (W, H), (11, 15, 22))
    d = ImageDraw.Draw(img)
    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 15)
        big = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 20)
    except OSError:
        font = big = ImageFont.load_default()

    # Plan: x across, z up the page. Elevation: x across, y up (LDraw's +Y is
    # down, so it is flipped).
    cap1 = project(d, pts, cols, palette, 0, 2, True, (0, 60, W // 2, H - 110), "GRUNDRISS (von oben)", args.spm)
    cap2 = project(d, pts, cols, palette, 0, 1, True, (W // 2, 60, W // 2, H - 110), "ANSICHT (von Sueden)", args.spm)

    d.text((16, 16), args.title or pathlib.Path(args.ldr).stem, fill=(230, 236, 245), font=big)
    d.text((16, H - 44), cap1, fill=(160, 176, 196), font=font)
    d.text((W // 2 + 16, H - 44), cap2, fill=(160, 176, 196), font=font)
    d.line([(W // 2, 56), (W // 2, H - 56)], fill=(40, 50, 64), width=1)
    img.save(args.out)
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
