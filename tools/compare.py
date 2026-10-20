"""Tile pet photos above real-app screenshots, for checking likeness.

    python tools/compare.py --out drafts/looks/compare-m2-<name>.png \
        --photos <photo>... --shots <png>...

Photos are scaled to 360 px tall, screenshots scaled up nearest-neighbour to
the same height; each tile has its file name under it. The output must go
under drafts/looks/compare-*.png, which git ignores (the photos are private).
Dev tool only (needs Pillow); nothing here ships.
"""
import argparse
import os
import sys

from PIL import Image, ImageDraw

H = 360
GAP = 12
LABEL = 18
BG = (24, 24, 28)


def tile(path, nearest):
    im = Image.open(path).convert("RGB")
    w = max(1, round(im.width * H / im.height))
    return im.resize((w, H), Image.NEAREST if nearest else Image.LANCZOS)


def row(paths, nearest):
    return [(os.path.basename(p), tile(p, nearest)) for p in paths]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--photos", nargs="*", default=[])
    ap.add_argument("--shots", nargs="+", required=True)
    a = ap.parse_args()
    out = a.out.replace("\\", "/")
    if not (os.path.basename(out).startswith("compare-") and "/drafts/looks/" in "/" + out):
        sys.exit("refusing: output must be drafts/looks/compare-*.png (git-ignored)")
    rows = [r for r in (row(a.photos, False), row(a.shots, True)) if r]
    width = max(sum(im.width for _, im in r) + GAP * (len(r) + 1) for r in rows)
    height = len(rows) * (H + LABEL + GAP) + GAP
    canvas = Image.new("RGB", (width, height), BG)
    draw = ImageDraw.Draw(canvas)
    y = GAP
    for r in rows:
        x = GAP
        for name, im in r:
            canvas.paste(im, (x, y))
            draw.text((x, y + H + 2), name, fill=(200, 200, 200))
            x += im.width + GAP
        y += H + LABEL + GAP
    canvas.save(a.out)
    print(f"wrote {a.out} ({width}x{height})")


if __name__ == "__main__":
    main()
