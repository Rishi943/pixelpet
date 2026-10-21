#!/usr/bin/env python3
"""Check a CI desktop screenshot for the pet. Standard library only.

    check_shot.py SHOT.png --bg 3a6ea5       solid background of that colour
    check_shot.py SHOT.png --empty EMPTY.png  any wallpaper: compare with a shot
                                              taken after PixelPet quit

Passes when the pet is there (enough changed pixels in the bottom 300 rows),
nothing else drew (the top-left 100x100 is untouched), and the background
shows inside the pet's bounding box (it is not on a black box).
"""
import struct
import sys
import zlib

BOTTOM = 300
MIN_PET_PIXELS = 200
MIN_BG_INSIDE = 0.15
TOL = 24


def read_png(path):
    data = open(path, "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", f"{path}: not a PNG"
    pos, idat, w = 8, b"", None
    while pos < len(data):
        n, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        if kind == b"IHDR":
            w, h, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
            assert depth == 8 and ctype in (2, 6) and interlace == 0, f"{path}: need 8-bit RGB/RGBA, not interlaced"
        elif kind == b"IDAT":
            idat += body
        pos += 12 + n
    bpp = 3 if ctype == 2 else 4
    raw, stride = zlib.decompress(idat), w * bpp
    rows, prev = [], bytearray(stride)
    for y in range(h):
        f, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b, c = prev[i], prev[i - bpp] if i >= bpp else 0
            if f == 1:
                line[i] = (line[i] + a) & 255
            elif f == 2:
                line[i] = (line[i] + b) & 255
            elif f == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append([tuple(line[x * bpp:x * bpp + 3]) for x in range(w)])
        prev = line
    return w, h, rows


def near(p, q):
    return all(abs(a - b) <= TOL for a, b in zip(p, q))


def main():
    shot = read_png(sys.argv[1])
    w, h, rows = shot
    if sys.argv[2] == "--bg":
        c = bytes.fromhex(sys.argv[3])
        bg = lambda x, y: tuple(c)
    else:
        _, _, empty = read_png(sys.argv[3])
        bg = lambda x, y: empty[y][x]
    problems = []
    if any(not near(rows[y][x], bg(x, y)) for y in range(100) for x in range(100)):
        problems.append("something drew in the top-left corner")
    changed = [(x, y) for y in range(h - BOTTOM, h) for x in range(w) if not near(rows[y][x], bg(x, y))]
    if len(changed) < MIN_PET_PIXELS:
        problems.append(f"no pet: only {len(changed)} changed pixels in the bottom {BOTTOM} rows")
    else:
        xs, ys = [p[0] for p in changed], [p[1] for p in changed]
        x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
        box = (x1 - x0 + 1) * (y1 - y0 + 1)
        inside = sum(near(rows[y][x], bg(x, y)) for y in range(y0, y1 + 1) for x in range(x0, x1 + 1))
        print(f"pet box {x0},{y0} {x1 - x0 + 1}x{y1 - y0 + 1}, {len(changed)} pet pixels, background {inside / box:.0%} of the box")
        if inside / box < MIN_BG_INSIDE:
            problems.append("the pet is on a box: almost no background shows inside its bounding box")
    for p in problems:
        print("FAIL:", p)
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
