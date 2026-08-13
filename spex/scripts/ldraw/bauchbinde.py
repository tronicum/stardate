#!/usr/bin/env python3
"""Burn a lower third into an Atlas plate: the site, its UNESCO year, its criteria.

The picture alone does not say what it is or why it is on the list. Everything
here comes from the committed Wikidata snapshot (CC0) -- never from the World
Heritage Centre, whose terms forbid republication. A missing year is printed as
a missing year.
"""
import json, sys, pathlib
from PIL import Image, ImageDraw, ImageFont

ROMAN = {"i":"i","ii":"ii","iii":"iii","iv":"iv","v":"v","vi":"vi",
         "vii":"vii","viii":"viii","ix":"ix","x":"x"}

def font(sz, bold=False):
    for p in ("/usr/share/fonts/truetype/dejavu/DejaVuSans%s.ttf" % ("-Bold" if bold else ""),
              "/usr/share/fonts/truetype/liberation/LiberationSans%s.ttf" % ("-Bold" if bold else "")):
        if pathlib.Path(p).exists():
            return ImageFont.truetype(p, sz)
    return ImageFont.load_default()

def _subline(meta):
    year, crit = meta.get("year"), meta.get("crit") or []
    line = ("UNESCO-Welterbe seit %d" % year) if year else "UNESCO-Welterbe — Jahr im Wikidata-Snapshot nicht belegt"
    if crit:
        line += "   ·   Kriterien " + ", ".join("(%s)" % ROMAN.get(c, c) for c in crit)
    if meta.get("states"):
        line += "   ·   " + ", ".join(meta["states"])
    return line


def draw(img_path, meta, out_path):
    im = Image.open(img_path).convert("RGB")
    W, H = im.size
    bar_h = int(H * 0.145)
    y0 = H - bar_h - int(H * 0.055)
    pad = int(W * 0.022)
    f1, f2 = font(int(bar_h * 0.34), True), font(int(bar_h * 0.215))
    scratch = ImageDraw.Draw(Image.new("RGB", (8, 8)))
    # the bar is as wide as its longest line: a fixed 62 % clipped Memphis
    name = meta["name"]
    while scratch.textlength(name, font=f1) > W - 2 * pad and f1.size > 12:
        f1 = font(f1.size - 1, True)
    ov = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(ov)
    bar_w = int(min(W, max(scratch.textlength(name, font=f1), 0) + 2 * pad))
    _sub = _subline(meta)
    bar_w = int(min(W, max(bar_w, scratch.textlength(_sub, font=f2) + 2 * pad)))
    d.rectangle([0, y0, bar_w, y0 + bar_h], fill=(8, 10, 12, 184))
    d.rectangle([0, y0, int(W * 0.0045), y0 + bar_h], fill=(214, 178, 92, 255))
    im = Image.alpha_composite(im.convert("RGBA"), ov).convert("RGB")
    d = ImageDraw.Draw(im)
    d.text((pad, y0 + int(bar_h * 0.16)), meta["name"], font=f1, fill=(244, 244, 240))
    line = _subline(meta)
    d.text((pad, y0 + int(bar_h * 0.60)), line, font=f2, fill=(198, 200, 204))
    im.save(out_path)
    return out_path

if __name__ == "__main__":
    metas = json.loads(pathlib.Path(sys.argv[1]).read_text())
    indir, outdir = pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
    outdir.mkdir(parents=True, exist_ok=True)
    for p in sorted(indir.glob("*.png")):
        m = metas.get(p.stem)
        if not m:
            print("kein Datensatz:", p.stem); continue
        draw(p, m, outdir / p.name)
        print(p.stem, "->", m.get("year") or "kein Jahr")
