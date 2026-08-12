#!/usr/bin/env python3
"""M75, ladder rung 5 — the contact sheet, which is mandatory and is for a person.

    ./target/release/spex flag DK --width-studs 37 -o /tmp/flag-dk.json
    python3 scripts/flags/contact-sheet.py /tmp/flag-*.json -o /tmp/flaggen.png

Reads the *recipe*, not the specification: the recipe is what the pipeline
will actually build, so this is a picture of the bricks and not a picture of
the construction sheet. Every cell is filled with the real `LDConfig.ldr`
value of the LDraw colour the quantiser chose, so what you are looking at is
the flag as the palette can express it — which is the whole question M75 asks.

No lighting, no bevels, no stud tops. A rendering would be prettier and would
answer a different question; this one has to be readable as *colour*, because
the number underneath it is a colour distance.
"""
import argparse
import json
import pathlib
import sys

from PIL import Image, ImageDraw, ImageFont

CELL = 12  # px per stud
GAP = 26  # px of caption under each flag


def ldraw_colors(config_path):
    """code -> (name, (r, g, b)) from the real file."""
    out = {}
    for line in pathlib.Path(config_path).read_text(encoding="utf-8", errors="replace").splitlines():
        t = line.split()
        if len(t) < 8 or t[1] != "!COLOUR":
            continue
        try:
            code = int(t[t.index("CODE") + 1])
            hexv = t[t.index("VALUE") + 1].lstrip("#")
            out[code] = (t[2], tuple(int(hexv[i:i + 2], 16) for i in (0, 2, 4)))
        except (ValueError, IndexError):
            continue
    return out


def render(recipe, colors):
    cells = recipe["steps"][0]["params"]["cells"]
    h, w = len(cells), len(cells[0])
    img = Image.new("RGB", (w * CELL, h * CELL), (11, 15, 22))
    d = ImageDraw.Draw(img)
    for r, row in enumerate(cells):
        for c, code in enumerate(row):
            rgb = colors.get(code, (255, 0, 255))[1]
            d.rectangle([c * CELL, r * CELL, (c + 1) * CELL - 1, (r + 1) * CELL - 1], fill=rgb)
    return img


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("recipes", nargs="+")
    ap.add_argument("-o", "--out", required=True)
    ap.add_argument("--ldconfig", default=".ldraw-cache/LDConfig.ldr")
    args = ap.parse_args()

    colors = ldraw_colors(args.ldconfig)
    if not colors:
        sys.exit(f"no colours parsed from {args.ldconfig}")

    tiles = []
    for path in args.recipes:
        recipe = json.loads(pathlib.Path(path).read_text())
        img = render(recipe, colors)
        used = sorted({c for row in recipe["steps"][0]["params"]["cells"] for c in row})
        caption = "{}   {} x {} Studs   {}".format(
            recipe["id"].replace("flag-", "").upper(),
            img.width // CELL,
            img.height // CELL,
            ", ".join(f"{c} {colors.get(c, ('?',))[0]}" for c in used),
        )
        tiles.append((img, caption))

    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 13)
    except OSError:
        font = ImageFont.load_default()

    pad = 18
    width = max(t.width for t, _ in tiles) + 2 * pad
    height = sum(t.height + GAP + pad for t, _ in tiles) + pad
    sheet = Image.new("RGB", (width, height), (11, 15, 22))
    d = ImageDraw.Draw(sheet)
    y = pad
    for img, caption in tiles:
        sheet.paste(img, (pad, y))
        d.text((pad, y + img.height + 6), caption, fill=(200, 210, 224), font=font)
        y += img.height + GAP + pad
    sheet.save(args.out)
    print(f"wrote {args.out}  ({width} x {height})")


if __name__ == "__main__":
    main()
