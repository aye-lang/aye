#!/usr/bin/env python3
"""Writes the ICO/CUR fixtures of browser/image's tests (no dependencies).

Run it from anywhere: python3 browser/tests/image/make_ico_fixtures.py
It writes ico_*.ico / .cur and their expected ico_*.rgba here, and replaces
the ico_ lines of manifest.txt (make_fixtures.py runs it too). Every byte
is written by hand: the PNG entries are RGBA8 with filter 0, the BMP
entries a BITMAPINFOHEADER, a palette, bottom-up XOR rows and a 1-bpp AND
mask, rows padded to 4 bytes.

Expected pixels: a 32-bpp entry whose alpha is not all 0 keeps its BGRA
and ignores the mask; any other BMP entry is opaque where the AND mask
bit is 0 and transparent black where it is 1. The decoder picks the
largest entry no larger than 64x64 (more bits per pixel wins a tie), or
the largest entry when every one is larger.
"""

import os
import struct
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))


def png(w, h, px):
    """px[y][x] = (r, g, b, a). An RGBA8 PNG."""
    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))
    raw = b"".join(b"\x00" + bytes(c for p in row for c in p) for row in px)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))


def pad4(row):
    return row + b"\x00" * (-len(row) % 4)


def pack_bits(values, bpp):
    out = bytearray()
    acc = 0
    n = 0
    for v in values:
        acc = (acc << bpp) | v
        n += bpp
        if n == 8:
            out.append(acc)
            acc = 0
            n = 0
    if n:
        out.append(acc << (8 - n))
    return bytes(out)


def bmp(w, h, bpp, px=None, idx=None, palette=None, mask=None, clr_used=0):
    """A BMP icon entry and its expected RGBA. px[y][x] = (r, g, b, a) for
    24/32 bpp; idx[y][x] into palette [(r, g, b)] for 1/4/8 bpp; mask[y][x]
    is 1 where the AND mask makes the pixel transparent."""
    mask = mask or [[0] * w for _ in range(h)]
    ncolors = 0 if bpp > 8 else (clr_used or len(palette))
    out = struct.pack("<IiiHHIIiiII", 40, w, 2 * h, 1, bpp, 0, 0, 0, 0, clr_used, 0)
    if bpp <= 8:
        pal = list(palette) + [(0, 0, 0)] * (ncolors - len(palette))
        out += b"".join(bytes([b, g, r, 0]) for r, g, b in pal)
    for y in reversed(range(h)):
        if bpp <= 8:
            out += pad4(pack_bits(idx[y], bpp))
        elif bpp == 24:
            out += pad4(b"".join(bytes([b, g, r]) for r, g, b, a in px[y]))
        else:
            out += pad4(b"".join(bytes([b, g, r, a]) for r, g, b, a in px[y]))
    for y in reversed(range(h)):
        out += pad4(pack_bits(mask[y], 1))
    use_alpha = bpp == 32 and any(p[3] for row in px for p in row)
    rgba = bytearray()
    for y in range(h):
        for x in range(w):
            if bpp <= 8:
                r, g, b = palette[idx[y][x]]
                a = 255
            else:
                r, g, b, a = px[y][x]
            if use_alpha:
                rgba += bytes([r, g, b, a])
            elif mask[y][x]:
                rgba += bytes(4)
            else:
                rgba += bytes([r, g, b, 255])
    return out, bytes(rgba)


def ico(entries, kind=1):
    """entries: [(w, h, bpp, data)]. Directory entries say 0 for 256."""
    out = struct.pack("<HHH", 0, kind, len(entries))
    offset = 6 + 16 * len(entries)
    body = b""
    for w, h, bpp, data in entries:
        planes, bits = (1, bpp) if kind == 1 else (w // 2, h // 2)  # CUR: hotspot
        out += struct.pack("<BBBBHHII", w & 255, h & 255, 0, 0, planes, bits, len(data), offset)
        offset += len(data)
        body += data
    return out + body


def colors(w, h, seed, alpha=None):
    def a(x, y):
        return 255 if alpha is None else alpha(x, y)
    return [[((x * 37 + seed * 11) & 255, (y * 59 + seed * 7) & 255, (x * y * 13 + seed) & 255, a(x, y))
             for x in range(w)] for y in range(h)]


def checker(w, h):
    return [[1 if (x + 2 * y) % 5 == 0 else 0 for x in range(w)] for y in range(h)]


def make(save):
    p = colors(6, 5, 1, lambda x, y: (x * 50 + y * 3) & 255)
    data = png(6, 5, p)
    save("ico_png.ico", ico([(6, 5, 32, data)]), 6, 5, "exact", b"".join(bytes(c) for row in p for c in row))

    data, rgba = bmp(7, 5, 32, px=colors(7, 5, 2, lambda x, y: (x * 40 + y * 7 + 1) & 255), mask=checker(7, 5))
    save("ico_bmp32.ico", ico([(7, 5, 32, data)]), 7, 5, "exact", rgba)

    data, rgba = bmp(7, 5, 32, px=colors(7, 5, 3, lambda x, y: 0), mask=checker(7, 5))
    save("ico_bmp32_mask.ico", ico([(7, 5, 32, data)]), 7, 5, "exact", rgba)

    data, rgba = bmp(5, 3, 24, px=colors(5, 3, 4), mask=checker(5, 3))
    save("ico_bmp24.ico", ico([(5, 3, 24, data)]), 5, 3, "exact", rgba)

    pal8 = [(i * 30, 255 - i * 20, i * i * 5) for i in range(7)]
    idx8 = [[(x + y * 3) % 7 for x in range(5)] for y in range(4)]
    data, rgba = bmp(5, 4, 8, idx=idx8, palette=pal8, mask=checker(5, 4), clr_used=7)
    save("ico_bmp8_mask.ico", ico([(5, 4, 8, data)]), 5, 4, "exact", rgba)
    good8 = ico([(5, 4, 8, data)])

    pal4 = [(i * 16, 255 - i * 16, (i * 77) & 255) for i in range(16)]
    data, rgba = bmp(9, 3, 4, idx=[[(x * 5 + y) % 16 for x in range(9)] for y in range(3)], palette=pal4,
                     mask=checker(9, 3))
    save("ico_bmp4.ico", ico([(9, 3, 4, data)]), 9, 3, "exact", rgba)

    data, rgba = bmp(33, 2, 1, idx=[[(x // 3 + y) % 2 for x in range(33)] for y in range(2)],
                     palette=[(10, 20, 30), (200, 150, 100)], mask=checker(33, 2))
    save("ico_bmp1.ico", ico([(33, 2, 1, data)]), 33, 2, "exact", rgba)

    # Picks the 48x48 32-bpp entry: the largest up to 64, and more bits
    # than the other 48x48.
    small, _ = bmp(16, 16, 32, px=colors(16, 16, 5, lambda x, y: 255))
    pal = [(i, 255 - i, 128) for i in range(256)]
    mid8, _ = bmp(48, 48, 8, idx=[[(x + y) % 256 for x in range(48)] for y in range(48)], palette=pal)
    mid32, rgba = bmp(48, 48, 32, px=colors(48, 48, 6, lambda x, y: (x * 5 + y) & 255))
    big = png(96, 96, colors(96, 96, 7))
    save("ico_multi.ico", ico([(16, 16, 32, small), (48, 48, 8, mid8), (48, 48, 32, mid32), (96, 96, 32, big)]),
         48, 48, "exact", rgba)

    # Every entry is above 64: the largest (256, written 0) wins.
    mid, _ = bmp(72, 72, 32, px=colors(72, 72, 8, lambda x, y: 255))
    p = colors(256, 256, 9, lambda x, y: (x + y) & 255)
    save("ico_multi_large.ico", ico([(72, 72, 32, mid), (256, 256, 32, png(256, 256, p))]), 256, 256, "exact",
         b"".join(bytes(c) for row in p for c in row))

    data, rgba = bmp(4, 4, 32, px=colors(4, 4, 10, lambda x, y: 200))
    save("ico_cursor.cur", ico([(4, 4, 32, data)], kind=2), 4, 4, "exact", rgba)

    save("ico_bad_truncated.ico", good8[:len(good8) * 2 // 3], 0, 0, "invalid")
    save("ico_bad_no_entries.ico", struct.pack("<HHH", 0, 1, 0), 0, 0, "invalid")
    bad = bytearray(good8)
    bad[18:22] = struct.pack("<I", len(good8) + 10)
    save("ico_bad_offset.ico", bytes(bad), 0, 0, "invalid")
    data, _ = bmp(5, 4, 8, idx=idx8, palette=pal8, clr_used=7)
    bad = bytearray(data)
    bad[14:16] = struct.pack("<H", 16)  # 16 bpp: not an icon depth
    save("ico_bad_bpp.ico", ico([(5, 4, 8, bytes(bad))]), 0, 0, "unsupported")


def main():
    lines = []

    def save(name, data, width, height, check, rgba=None):
        with open(os.path.join(HERE, name), "wb") as f:
            f.write(data)
        if rgba is not None:
            assert len(rgba) == width * height * 4, name
            with open(os.path.join(HERE, name.rsplit(".", 1)[0] + ".rgba"), "wb") as f:
                f.write(rgba)
        lines.append("%s %d %d %s" % (name, width, height, check))

    make(save)
    path = os.path.join(HERE, "manifest.txt")
    old = []
    if os.path.exists(path):
        with open(path) as f:
            old = [l for l in f.read().split("\n") if l and not l.startswith("ico_")]
    with open(path, "w") as f:
        f.write("\n".join(old + lines) + "\n")


if __name__ == "__main__":
    main()
