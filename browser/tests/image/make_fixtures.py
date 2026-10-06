#!/usr/bin/env python3
"""Writes the image fixtures of browser/image's tests into this directory.

Run it from anywhere: python3 browser/tests/image/make_fixtures.py
(needs Pillow with WebP; cjpeg and cwebp are used for a few files when they
are there). The tests themselves only read the files it writes:

- NAME.png / .gif / .jpg / .webp: an image, and NAME.rgba its expected
  pixels (raw RGBA8, rows top to bottom, no header).
- manifest.txt: one line per image, "FILE WIDTH HEIGHT CHECK", where CHECK
  is `exact` (the decoder must match NAME.rgba byte for byte), `jpeg` (a
  tolerance, see browser/image/tests.ore) or the error the decoder must
  return (`invalid`, `unsupported`; WIDTH and HEIGHT are 0 then).

PNGs are encoded here (chunks and zlib written by hand) so that every color
type, bit depth, filter, interlacing and compression mode is covered and the
expected pixels follow from the samples. GIFs come from Pillow and from an
LZW encoder here (code size growth, a full table, offsets, interlacing).
JPEGs come from Pillow (libjpeg-turbo) and cjpeg, WebPs from Pillow and
cwebp (libwebp); their expected pixels are Pillow's own decoding. A YCCK
JPEG and WebPs with filtered raw alpha are written here.
"""

import io
import os
import random
import shutil
import struct
import subprocess
import zlib

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
manifest = []


def save(name, data, width, height, check, rgba=None):
    with open(os.path.join(HERE, name), "wb") as f:
        f.write(data)
    if rgba is not None:
        assert len(rgba) == width * height * 4, name
        with open(os.path.join(HERE, name.rsplit(".", 1)[0] + ".rgba"), "wb") as f:
            f.write(bytes(rgba))
    manifest.append("%s %d %d %s" % (name, width, height, check))


# ---------------------------------------------------------------- PNG

CHANNELS = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}
DEPTHS = {0: [1, 2, 4, 8, 16], 2: [8, 16], 3: [1, 2, 4, 8], 4: [8, 16], 6: [8, 16]}
ADAM7 = [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)]


def chunk(kind, data):
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))


def pack_row(samples, depth):
    """Samples (ints) of one row as bytes: low depths packed high bits first."""
    if depth == 16:
        return b"".join(struct.pack(">H", s) for s in samples)
    if depth == 8:
        return bytes(samples)
    out = bytearray()
    per = 8 // depth
    for i in range(0, len(samples), per):
        b = 0
        group = samples[i:i + per]
        for k, s in enumerate(group):
            b |= s << (8 - depth * (k + 1))
        out.append(b)
    return bytes(out)


def paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


def filter_row(ftype, row, prev, bpp):
    out = bytearray([ftype])
    for i, x in enumerate(row):
        a = row[i - bpp] if i >= bpp else 0
        b = prev[i] if prev is not None else 0
        c = prev[i - bpp] if prev is not None and i >= bpp else 0
        pred = [0, a, b, (a + b) // 2, paeth(a, b, c)][ftype]
        out.append((x - pred) & 0xFF)
    return bytes(out)


def to8(v, depth):
    if depth == 16:
        return v >> 8
    return v * 255 // ((1 << depth) - 1)


def make_png(width, height, color, depth, samples, interlace=False, filters="cycle",
             level=6, strategy=zlib.Z_DEFAULT_STRATEGY, idat_size=None, palette=None,
             trns=None, mem_level=9):
    """samples[y][x] is the tuple of a pixel's samples. Returns (png, rgba)."""
    ch = CHANNELS[color]
    bpp = max(1, ch * depth // 8)
    raw = bytearray()
    passes = ADAM7 if interlace else [(0, 0, 1, 1)]
    rowno = 0
    for x0, y0, dx, dy in passes:
        xs = list(range(x0, width, dx))
        ys = list(range(y0, height, dy))
        if not xs or not ys:
            continue
        prev = None
        for y in ys:
            flat = [s for x in xs for s in samples[y][x]]
            row = pack_row(flat, depth)
            if filters == "cycle":
                ftype = rowno % 5
            else:
                ftype = filters
            rowno += 1
            raw += filter_row(ftype, row, prev, bpp)
            prev = row
    comp = zlib.compressobj(level, zlib.DEFLATED, 15, mem_level, strategy)
    z = comp.compress(bytes(raw)) + comp.flush()
    assert zlib.decompress(z) == bytes(raw)
    out = b"\x89PNG\r\n\x1a\n"
    out += chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, depth, color, 0, 0, 1 if interlace else 0))
    out += chunk(b"tEXt", b"Comment\x00an ancillary chunk to skip")
    if palette is not None:
        out += chunk(b"PLTE", bytes(c for rgb in palette for c in rgb))
    if trns is not None:
        out += chunk(b"tRNS", trns)
    step = idat_size or len(z)
    for i in range(0, len(z), step):
        out += chunk(b"IDAT", z[i:i + step])
    out += chunk(b"IEND", b"")

    rgba = bytearray()
    for y in range(height):
        for x in range(width):
            s = samples[y][x]
            if color == 0:
                g = to8(s[0], depth)
                a = 0 if trns is not None and struct.unpack(">H", trns[:2])[0] == s[0] else 255
                rgba += bytes([g, g, g, a])
            elif color == 2:
                a = 255
                if trns is not None and struct.unpack(">HHH", trns[:6]) == tuple(s):
                    a = 0
                rgba += bytes([to8(s[0], depth), to8(s[1], depth), to8(s[2], depth), a])
            elif color == 3:
                r, g, b = palette[s[0]]
                a = trns[s[0]] if trns is not None and s[0] < len(trns) else 255
                rgba += bytes([r, g, b, a])
            elif color == 4:
                g = to8(s[0], depth)
                rgba += bytes([g, g, g, to8(s[1], depth)])
            else:
                rgba += bytes(to8(v, depth) for v in s)
    return out, rgba


def png_samples(width, height, color, depth, seed):
    """Gradients plus noise, so that every filter has work to do."""
    rnd = random.Random(seed)
    top = (1 << depth) - 1
    ch = CHANNELS[color]
    rows = []
    for y in range(height):
        row = []
        for x in range(width):
            px = []
            for c in range(ch):
                v = ((x * 37 + y * 23 + c * 61) * top) // (37 * width + 23 * height + 61 * 3 + 1)
                if rnd.random() < 0.3:
                    v = rnd.randint(0, top)
                px.append(v)
            if color == 3:
                px = [min(px[0], palette_size(depth) - 1)]
            row.append(tuple(px))
        rows.append(row)
    return rows


def palette_size(depth):
    return min(1 << depth, 200)


def make_palette(n, seed):
    rnd = random.Random(seed)
    return [(rnd.randint(0, 255), rnd.randint(0, 255), rnd.randint(0, 255)) for _ in range(n)]


def make_pngs():
    sizes = [(33, 17), (7, 5), (1, 1), (13, 9), (20, 16), (3, 11), (16, 16), (9, 2)]
    k = 0
    for color in sorted(DEPTHS):
        for depth in DEPTHS[color]:
            for interlace in (False, True):
                w, h = sizes[k % len(sizes)]
                k += 1
                samples = png_samples(w, h, color, depth, k)
                palette = make_palette(palette_size(depth), k) if color == 3 else None
                data, rgba = make_png(w, h, color, depth, samples, interlace=interlace, palette=palette)
                name = "png_c%d_d%d_%s_%dx%d.png" % (color, depth, "i" if interlace else "n", w, h)
                save(name, data, w, h, "exact", rgba)
            # tRNS: a key color (types 0, 2) that occurs in the image, or
            # alpha for some palette entries (type 3).
            if color in (0, 2, 3):
                w, h = 17, 11
                samples = png_samples(w, h, color, depth, 100 + k)
                key = samples[3][4]
                palette = None
                if color == 0:
                    trns = struct.pack(">H", key[0])
                elif color == 2:
                    trns = struct.pack(">HHH", *key)
                else:
                    palette = make_palette(palette_size(depth), k)
                    trns = bytes((i * 97) % 256 for i in range(min(len(palette), 5)))
                for interlace in (False, True):
                    data, rgba = make_png(w, h, color, depth, samples, interlace=interlace,
                                          palette=palette, trns=trns)
                    name = "png_c%d_d%d_%s_trns.png" % (color, depth, "i" if interlace else "n")
                    save(name, data, w, h, "exact", rgba)

    # Each filter on its own, on 8-bit RGBA (4 bytes per pixel), 16-bit
    # RGB (6) and 2-bit gray (less than a byte per pixel).
    for color, depth in ((6, 8), (2, 16), (0, 2)):
        samples = png_samples(19, 13, color, depth, color * 10 + depth)
        for f in range(5):
            data, rgba = make_png(19, 13, color, depth, samples, filters=f)
            save("png_filter%d_c%d_d%d.png" % (f, color, depth), data, 19, 13, "exact", rgba)

    # Compression: stored blocks (several: a small memLevel makes small
    # blocks), fixed Huffman, dynamic Huffman, and IDAT in many chunks.
    samples = png_samples(40, 30, 6, 8, 7)
    data, rgba = make_png(40, 30, 6, 8, samples, level=0, mem_level=1)
    assert stored_blocks(data) > 1
    save("png_zstored.png", data, 40, 30, "exact", rgba)
    data, rgba = make_png(40, 30, 6, 8, samples, level=9, strategy=zlib.Z_FIXED)
    assert first_block_type(data) == 1
    save("png_zfixed.png", data, 40, 30, "exact", rgba)
    data, rgba = make_png(40, 30, 6, 8, samples, level=9)
    assert first_block_type(data) == 2
    save("png_zdynamic.png", data, 40, 30, "exact", rgba)
    data, rgba = make_png(40, 30, 6, 8, samples, level=9, idat_size=37, interlace=True)
    save("png_idat_split.png", data, 40, 30, "exact", rgba)

    # Pillow's own PNGs of a photo-like image.
    photo = photo_image(40, 30)
    for mode in ("RGB", "RGBA", "P", "L"):
        im = photo.convert(mode) if mode != "RGBA" else with_alpha(photo)
        if mode == "P":
            im = photo.quantize(64)
        buf = io.BytesIO()
        im.save(buf, "PNG", optimize=True)
        save("png_pil_%s.png" % mode.lower(), buf.getvalue(), 40, 30, "exact", im.convert("RGBA").tobytes())

    # Bad files.
    good, _ = make_png(20, 16, 6, 8, png_samples(20, 16, 6, 8, 1))
    save("png_bad_truncated.png", good[:len(good) // 2], 0, 0, "invalid")
    save("png_bad_signature.png", b"\x89PNX" + good[4:], 0, 0, "unsupported")
    zero = bytearray(good)
    zero[16:20] = b"\x00\x00\x00\x00"
    save("png_bad_zero_width.png", bytes(zero), 0, 0, "invalid")
    huge = bytearray(good)
    huge[16:24] = struct.pack(">II", 20000, 20000)
    save("png_bad_huge.png", bytes(huge), 0, 0, "unsupported")
    bad = bytearray(good)
    idat = bad.index(b"IDAT")
    for i in range(idat + 10, idat + 40):
        bad[i] ^= 0x5A
    save("png_bad_deflate.png", bytes(bad), 0, 0, "invalid")
    raw = b"\x07" + bytes(20 * 4)
    raw = raw * 16
    z = zlib.compress(raw)
    filt = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 20, 16, 8, 6, 0, 0, 0))
    filt += chunk(b"IDAT", z) + chunk(b"IEND", b"")
    save("png_bad_filter.png", filt, 0, 0, "invalid")
    nopal = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 4, 4, 8, 3, 0, 0, 0))
    nopal += chunk(b"IDAT", zlib.compress(b"\x00\x00\x00\x00\x00" * 4)) + chunk(b"IEND", b"")
    save("png_bad_no_palette.png", nopal, 0, 0, "invalid")


def zlib_stream(png):
    """The concatenated IDAT data of a PNG."""
    pos, z = 8, b""
    while pos < len(png):
        n = struct.unpack(">I", png[pos:pos + 4])[0]
        if png[pos + 4:pos + 8] == b"IDAT":
            z += png[pos + 8:pos + 8 + n]
        pos += 12 + n
    return z


def first_block_type(png):
    return (zlib_stream(png)[2] >> 1) & 3


def stored_blocks(png):
    """How many blocks the deflate data has, all of them stored."""
    z = zlib_stream(png)
    pos, n = 2, 0
    while True:
        head = z[pos]
        assert (head >> 1) & 3 == 0
        length = struct.unpack("<H", z[pos + 1:pos + 3])[0]
        pos += 5 + length
        n += 1
        if head & 1:
            return n


def photo_image(w, h, seed=5):
    """A smooth color image with some edges and noise, like a photo."""
    rnd = random.Random(seed)
    im = Image.new("RGB", (w, h))
    px = im.load()
    for y in range(h):
        for x in range(w):
            r = int(255 * x / max(1, w - 1))
            g = int(255 * y / max(1, h - 1))
            b = int(128 + 100 * ((x - w / 2) * (y - h / 2)) / (w * h / 4 + 1))
            if (x - w * 0.6) ** 2 + (y - h * 0.4) ** 2 < (min(w, h) * 0.25) ** 2:
                r, g, b = 230, 40, 30
            n = rnd.randint(-6, 6)
            px[x, y] = tuple(max(0, min(255, v + n)) for v in (r, g, b))
    return im


def with_alpha(im):
    out = im.convert("RGBA")
    px = out.load()
    for y in range(im.size[1]):
        for x in range(im.size[0]):
            r, g, b, _ = px[x, y]
            px[x, y] = (r, g, b, (x * 255) // max(1, im.size[0] - 1))
    return out


# ---------------------------------------------------------------- GIF


def lzw_encode(indices, min_size, clear_when_full):
    """GIF LZW: codes least significant bit first, the code size growing
    as the table does. A full table either stays as it is (the decoder
    must keep going without adding entries) or is cleared."""
    clear, end = 1 << min_size, (1 << min_size) + 1
    out = bytearray()
    acc = nbits = 0

    def emit(code, size):
        nonlocal acc, nbits
        acc |= code << nbits
        nbits += size
        while nbits >= 8:
            out.append(acc & 0xFF)
            acc >>= 8
            nbits -= 8

    def reset():
        return {}, min_size + 1, clear + 2

    table, size, nxt = reset()
    emit(clear, size)
    prefix = None
    for b in indices:
        if prefix is None:
            prefix = b
            continue
        if (prefix, b) in table:
            prefix = table[(prefix, b)]
            continue
        emit(prefix, size)
        if nxt < 4096:
            table[(prefix, b)] = nxt
            nxt += 1
            if nxt > (1 << size) and size < 12:
                size += 1
        elif clear_when_full:
            emit(clear, size)
            table, size, nxt = reset()
        prefix = b
    emit(prefix, size)
    emit(end, size)
    if nbits:
        out.append(acc & 0xFF)
    return bytes(out)


def sub_blocks(data):
    out = bytearray()
    for i in range(0, len(data), 255):
        part = data[i:i + 255]
        out.append(len(part))
        out += part
    return bytes(out) + b"\x00"


def table_bits(n):
    """The size field of a color table of at least n entries."""
    bits = 0
    while (2 << bits) < n:
        bits += 1
    return bits


def color_table(palette, bits):
    out = bytearray(c for rgb in palette for c in rgb)
    out += bytes(3 * (2 << bits) - len(out))
    return bytes(out)


def make_gif(screen_w, screen_h, frame, version=b"GIF89a", global_palette=None, local_palette=None,
             left=0, top=0, interlace=False, transparent=None, clear_when_full=True, min_size=None):
    """frame: rows of palette indices. Returns (gif, rgba)."""
    fw, fh = len(frame[0]), len(frame)
    palette = local_palette or global_palette
    out = bytearray(version)
    flags = 0
    if global_palette:
        gbits = table_bits(len(global_palette))
        flags = 0x80 | (7 << 4) | gbits
    out += struct.pack("<HHBBB", screen_w, screen_h, flags, 0, 0)
    if global_palette:
        out += color_table(global_palette, gbits)
    if transparent is not None:
        out += b"\x21\xF9\x04\x01\x00\x00" + bytes([transparent]) + b"\x00"
    out += b"\x21\xFE" + sub_blocks(b"a comment extension to skip")
    flags = 0
    if local_palette:
        lbits = table_bits(len(local_palette))
        flags = 0x80 | lbits
    if interlace:
        flags |= 0x40
    out += b"\x2C" + struct.pack("<HHHHB", left, top, fw, fh, flags)
    if local_palette:
        out += color_table(local_palette, lbits)
    order = list(range(fh))
    if interlace:
        order = list(range(0, fh, 8)) + list(range(4, fh, 8)) + list(range(2, fh, 4)) + list(range(1, fh, 2))
    indices = [i for y in order for i in frame[y]]
    if min_size is None:
        min_size = max(2, table_bits(len(palette)) + 1)
    out.append(min_size)
    out += sub_blocks(lzw_encode(indices, min_size, clear_when_full))
    out += b"\x3B"

    rgba = bytearray(screen_w * screen_h * 4)
    for y in range(fh):
        for x in range(fw):
            cx, cy = left + x, top + y
            if cx >= screen_w or cy >= screen_h:
                continue
            i = frame[y][x]
            if i == transparent:
                continue
            at = (cy * screen_w + cx) * 4
            rgba[at:at + 4] = bytes(palette[i]) + b"\xff"
    return bytes(out), bytes(rgba)


def pil_rgba(data):
    """Pillow's decoding of the first frame, transparent pixels as 0."""
    im = Image.open(io.BytesIO(data))
    im.seek(0)
    rgba = bytearray(im.convert("RGBA").tobytes())
    for i in range(0, len(rgba), 4):
        if rgba[i + 3] == 0:
            rgba[i:i + 4] = b"\x00\x00\x00\x00"
    return bytes(rgba)


def make_gifs():
    rnd = random.Random(9)

    # Noise with 256 colors: about one code per pixel, so the table fills
    # (4096 entries) and stays full, or is cleared.
    pal = make_palette(256, 1)
    noise = [[rnd.randrange(256) for _ in range(96)] for _ in range(96)]
    data, rgba = make_gif(96, 96, noise, global_palette=pal, clear_when_full=False)
    assert pil_rgba(data) == rgba
    save("gif_lzw_full_table.gif", data, 96, 96, "exact", rgba)
    noise = [[rnd.randrange(256) for _ in range(80)] for _ in range(80)]
    data, rgba = make_gif(80, 80, noise, global_palette=pal, clear_when_full=True)
    assert pil_rgba(data) == rgba
    save("gif_lzw_clear.gif", data, 80, 80, "exact", rgba)

    # Long runs: the KwKwK case on every run.
    runs = [[(x // 9 + y // 5) % 3 for x in range(37)] for y in range(23)]
    data, rgba = make_gif(37, 23, runs, version=b"GIF87a", global_palette=make_palette(3, 2))
    assert pil_rgba(data) == rgba
    save("gif_87a_runs.gif", data, 37, 23, "exact", rgba)

    # Two colors, the smallest code size.
    two = [[(x * y + x) % 2 for x in range(7)] for y in range(5)]
    data, rgba = make_gif(7, 5, two, global_palette=[(0, 0, 0), (255, 255, 255)])
    save("gif_two_colors.gif", data, 7, 5, "exact", rgba)

    # A frame smaller than the screen at an offset, interlaced, with a
    # local color table and a transparent index.
    frame = [[(x + 2 * y) % 16 for x in range(10)] for y in range(13)]
    data, rgba = make_gif(20, 17, frame, global_palette=make_palette(4, 3), local_palette=make_palette(16, 4),
                          left=3, top=2, interlace=True, transparent=5)
    save("gif_offset_interlaced.gif", data, 20, 17, "exact", rgba)
    # A frame reaching past the screen: clipped.
    data, rgba = make_gif(8, 6, frame, global_palette=make_palette(16, 4), left=2, top=1)
    save("gif_clipped.gif", data, 8, 6, "exact", rgba)

    # Pillow's GIFs: a quantized photo, interlaced (Pillow's default) and
    # not, with transparency, and an animation (the first frame counts).
    photo = photo_image(40, 30).quantize(200)
    for interlace in (True, False):
        buf = io.BytesIO()
        photo.save(buf, "GIF", interlace=interlace)
        save("gif_pil_%s.gif" % ("interlaced" if interlace else "plain"), buf.getvalue(), 40, 30, "exact",
             pil_rgba(buf.getvalue()))
    buf = io.BytesIO()
    photo.save(buf, "GIF", transparency=3)
    data = buf.getvalue()
    assert b"\x21\xF9" in data
    rgba = pil_rgba(data)
    assert any(rgba[i + 3] == 0 for i in range(0, len(rgba), 4))
    save("gif_pil_transparent.gif", data, 40, 30, "exact", rgba)
    frames = [photo_image(24, 18, seed=s).quantize(32) for s in (1, 2, 3)]
    buf = io.BytesIO()
    frames[0].save(buf, "GIF", save_all=True, append_images=frames[1:], duration=100, loop=0)
    save("gif_pil_animated.gif", buf.getvalue(), 24, 18, "exact", pil_rgba(buf.getvalue()))

    good, _ = make_gif(37, 23, runs, global_palette=make_palette(3, 2))
    save("gif_bad_truncated.gif", good[:len(good) - 20], 0, 0, "invalid")
    zero = bytearray(good)
    zero[6:10] = b"\x00\x00\x00\x00"
    save("gif_bad_zero_size.gif", bytes(zero), 0, 0, "invalid")
    save("gif_bad_no_image.gif", good[:13 + 12] + b"\x3B", 0, 0, "invalid")


# ---------------------------------------------------------------- JPEG


def make_jpegs():
    photo = photo_image(40, 30)
    odd = photo_image(33, 17, seed=8)
    cases = [
        ("jpeg_gray_q90", photo.convert("L"), dict(quality=90)),
        ("jpeg_gray_odd_q50", odd.convert("L"), dict(quality=50)),
        ("jpeg_444_q95", photo, dict(quality=95, subsampling=0)),
        ("jpeg_444_q50", odd, dict(quality=50, subsampling=0)),
        ("jpeg_422_q95", odd, dict(quality=95, subsampling=1)),
        ("jpeg_422_q50", photo, dict(quality=50, subsampling=1)),
        ("jpeg_420_q95", photo, dict(quality=95, subsampling=2)),
        ("jpeg_420_odd_q50", odd, dict(quality=50, subsampling=2)),
        ("jpeg_420_restart", photo, dict(quality=80, subsampling=2, restart_marker_blocks=1)),
        ("jpeg_444_restart_rows", odd, dict(quality=80, subsampling=0, restart_marker_rows=1)),
        ("jpeg_420_optimized", photo_image(64, 48, seed=3), dict(quality=75, subsampling=2, optimize=True)),
        ("jpeg_rgb", odd, dict(quality=90, subsampling=0, keep_rgb=True)),
        # Quantization values above 255: 16-bit DQT tables in an SOF1
        # (extended sequential) file.
        ("jpeg_sof1_dqt16", odd, dict(subsampling=2, qtables=[
            [300 if i % 9 == 8 else 2 + i // 8 for i in range(64)],
            [400 if i % 7 == 6 else 3 + i // 8 for i in range(64)],
        ])),
    ]
    for name, im, opts in cases:
        buf = io.BytesIO()
        im.save(buf, "JPEG", **opts)
        data = buf.getvalue()
        if "restart" in name:
            assert b"\xFF\xDD" in data
        if "sof1" in name:
            assert b"\xFF\xC1" in data and b"\xFF\xDB\x00\x83\x10" in data
        dec = Image.open(io.BytesIO(data)).convert("RGBA")
        save(name + ".jpg", data, im.size[0], im.size[1], "jpeg", dec.tobytes())

    # cjpeg for what Pillow cannot ask for: 4:4:0 and 4:1:1 subsampling,
    # and a sequential file with one scan per component.
    cjpeg = shutil.which("cjpeg") or "/opt/homebrew/bin/cjpeg"
    if os.path.exists(cjpeg):
        ppm = io.BytesIO()
        odd.save(ppm, "PPM")
        scans = os.path.join(HERE, "scans.tmp")
        with open(scans, "w") as f:
            f.write("0: 0 63 0 0;\n1: 0 63 0 0;\n2: 0 63 0 0;\n")
        runs = [
            ("jpeg_cjpeg_440", ["-sample", "1x2"]),
            ("jpeg_cjpeg_411", ["-sample", "4x1"]),
            ("jpeg_cjpeg_scans", ["-sample", "2x2", "-scans", scans]),
            ("jpeg_cjpeg_restart", ["-sample", "2x1", "-restart", "2B"]),
        ]
        for name, args in runs:
            data = subprocess.run([cjpeg, "-quality", "85"] + args, input=ppm.getvalue(),
                                  capture_output=True, check=True).stdout
            assert data[:2] == b"\xFF\xD8"
            dec = Image.open(io.BytesIO(data)).convert("RGBA")
            save(name + ".jpg", data, odd.size[0], odd.size[1], "jpeg", dec.tobytes())
        os.remove(scans)

    make_progressive_jpegs(photo, cjpeg)
    make_cmyk_jpegs(photo, odd)
    make_exif_jpegs()

    buf = io.BytesIO()
    photo.save(buf, "JPEG", quality=80)
    good = buf.getvalue()
    save("jpeg_bad_truncated.jpg", good[:len(good) * 2 // 3], 0, 0, "invalid")
    if os.path.exists(cjpeg):
        ppm = io.BytesIO()
        photo.save(ppm, "PPM")
        data = subprocess.run([cjpeg, "-arithmetic"], input=ppm.getvalue(), capture_output=True, check=True).stdout
        assert b"\xFF\xC9" in data
        save("jpeg_bad_arithmetic.jpg", data, 0, 0, "unsupported")
    # A 3-component frame header claiming 4 components (cut short).
    four = bytearray(good)
    sof = four.index(b"\xFF\xC0")
    four[sof + 9] = 4
    save("jpeg_bad_four_components.jpg", bytes(four), 0, 0, "invalid")
    zero = bytearray(good)
    zero[sof + 7:sof + 9] = b"\x00\x00"
    save("jpeg_bad_zero_width.jpg", bytes(zero), 0, 0, "invalid")
    # No Huffman tables at all.
    nodht = bytearray()
    pos = 2
    nodht += good[:2]
    while pos < len(good):
        assert good[pos] == 0xFF
        marker = good[pos + 1]
        n = struct.unpack(">H", good[pos + 2:pos + 4])[0]
        if marker == 0xDA:
            nodht += good[pos:]
            break
        if marker != 0xC4:
            nodht += good[pos:pos + 2 + n]
        pos += 2 + n
    save("jpeg_bad_no_tables.jpg", bytes(nodht), 0, 0, "invalid")


def jpeg_case(name, im, opts):
    buf = io.BytesIO()
    im.save(buf, "JPEG", **opts)
    data = buf.getvalue()
    dec = Image.open(io.BytesIO(data)).convert("RGBA")
    save(name + ".jpg", data, im.size[0], im.size[1], "jpeg", dec.tobytes())
    return data


def make_progressive_jpegs(photo, cjpeg):
    """Progressive JPEGs: spectral selection and successive approximation
    (libjpeg's default script), EOB runs, restarts, one-component scans."""
    odd = photo_image(33, 17, seed=8)
    big = photo_image(96, 64, seed=11)
    for name, im, opts in [
        ("jpeg_prog_420", photo, dict(quality=80, subsampling=2)),
        ("jpeg_prog_444_odd", odd, dict(quality=90, subsampling=0)),
        ("jpeg_prog_422_odd", odd, dict(quality=60, subsampling=1)),
        ("jpeg_prog_gray", photo.convert("L"), dict(quality=85)),
        ("jpeg_prog_big", big, dict(quality=70, subsampling=2)),
        ("jpeg_prog_restart", big, dict(quality=75, subsampling=2, restart_marker_blocks=3)),
    ]:
        data = jpeg_case(name, im, dict(opts, progressive=True))
        assert b"\xFF\xC2" in data, name
        if "restart" in name:
            assert b"\xFF\xDD" in data
    if not os.path.exists(cjpeg):
        return
    ppm = io.BytesIO()
    big.save(ppm, "PPM")
    scans = os.path.join(HERE, "scans.tmp")
    runs = [
        # Spectral selection only, the DC scans not interleaved.
        ("jpeg_prog_spectral", "0: 0 0 0 0;\n1: 0 0 0 0;\n2: 0 0 0 0;\n0: 1 9 0 0;\n0: 10 63 0 0;\n"
                               "1: 1 63 0 0;\n2: 1 63 0 0;\n", ["-sample", "2x1"]),
        # Several refinement passes of DC and AC.
        ("jpeg_prog_refine", "0 1 2: 0 0 0 2;\n0: 1 63 0 3;\n1: 1 63 0 1;\n2: 1 63 0 1;\n"
                             "0 1 2: 0 0 2 1;\n0: 1 63 3 2;\n0: 1 63 2 1;\n0 1 2: 0 0 1 0;\n"
                             "0: 1 63 1 0;\n1: 1 63 1 0;\n2: 1 63 1 0;\n", ["-sample", "2x2"]),
    ]
    for name, script, args in runs:
        with open(scans, "w") as f:
            f.write(script)
        data = subprocess.run([cjpeg, "-quality", "80", "-scans", scans] + args, input=ppm.getvalue(),
                              capture_output=True, check=True).stdout
        assert b"\xFF\xC2" in data, name
        dec = Image.open(io.BytesIO(data)).convert("RGBA")
        save(name + ".jpg", data, big.size[0], big.size[1], "jpeg", dec.tobytes())
    os.remove(scans)


def huffman_codes(counts, values):
    """The codes (value -> (code, length)) of a JPEG Huffman table."""
    codes = {}
    code = 0
    k = 0
    for length in range(1, 17):
        for _ in range(counts[length - 1]):
            codes[values[k]] = (code, length)
            code += 1
            k += 1
        code <<= 1
    return codes


def ycck_jpeg(blocks_x, blocks_y, seed):
    """A baseline YCCK JPEG (Adobe transform 2) of flat 8x8 blocks, encoded
    here: libjpeg's tools cannot write one. Only DC terms: a quantizer of 8
    makes the DC value the sample minus 128, so the file is exact."""
    rnd = random.Random(seed)
    dc_counts = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0]
    dc_values = list(range(12))
    dc = huffman_codes(dc_counts, dc_values)
    out = bytearray(b"\xFF\xD8")

    def segment(marker, body):
        out.extend(struct.pack(">BBH", 0xFF, marker, len(body) + 2) + body)

    segment(0xEE, b"Adobe" + struct.pack(">HHHB", 100, 0, 0, 2))
    segment(0xDB, b"\x00" + bytes([8] * 64))
    segment(0xC0, struct.pack(">BHHB", 8, blocks_y * 8, blocks_x * 8, 4) +
            b"".join(bytes([i + 1, 0x11, 0]) for i in range(4)))
    segment(0xC4, b"\x00" + bytes(dc_counts) + bytes(dc_values) +
            b"\x10" + bytes([1] + [0] * 15) + b"\x00")
    segment(0xDA, b"\x04" + b"".join(bytes([i + 1, 0x00]) for i in range(4)) + b"\x00\x3F\x00")
    bits = []

    def put(code, length):
        bits.extend((code >> (length - 1 - i)) & 1 for i in range(length))

    pred = [0, 0, 0, 0]
    for _ in range(blocks_x * blocks_y):
        for c in range(4):
            v = rnd.randint(0, 255) - 128
            diff = v - pred[c]
            pred[c] = v
            size = abs(diff).bit_length()
            put(*dc[size])
            if size:
                put(diff if diff > 0 else diff + (1 << size) - 1, size)
            put(0, 1)  # EOB
    bits.extend([1] * (-len(bits) % 8))
    for i in range(0, len(bits), 8):
        b = int("".join(map(str, bits[i:i + 8])), 2)
        out.append(b)
        if b == 0xFF:
            out.append(0)
    out.extend(b"\xFF\xD9")
    return bytes(out)


def make_cmyk_jpegs(photo, odd):
    """CMYK (Adobe, inverted samples) and YCCK; Pillow converts them to RGB
    as R = (255 - C) * (255 - K) / 255 on the uninverted samples."""
    for name, im, opts in [
        ("jpeg_cmyk", photo.convert("CMYK"), dict(quality=90)),
        ("jpeg_cmyk_odd_prog", odd.convert("CMYK"), dict(quality=70, progressive=True)),
    ]:
        data = jpeg_case(name, im, opts)
        assert b"Adobe" in data
    data = ycck_jpeg(4, 3, seed=21)
    im = Image.open(io.BytesIO(data))
    assert im.mode == "CMYK"
    save("jpeg_ycck.jpg", data, 32, 24, "jpeg", im.convert("RGBA").tobytes())


def exif_segment(orientation, big_endian):
    """An APP1 Exif segment whose IFD0 has only the Orientation tag."""
    e = ">" if big_endian else "<"
    tiff = (b"MM" if big_endian else b"II") + struct.pack(e + "HI", 42, 8)
    tiff += struct.pack(e + "H", 1) + struct.pack(e + "HHI", 0x0112, 3, 1)
    tiff += struct.pack(e + "H", orientation) + b"\x00\x00" + struct.pack(e + "I", 0)
    body = b"Exif\x00\x00" + tiff
    return struct.pack(">BBH", 0xFF, 0xE1, len(body) + 2) + body


def make_exif_jpegs():
    """The 8 EXIF orientations (little- and big-endian TIFF), on an 11x6
    image: the expected pixels are Pillow's exif_transpose of its decoding."""
    from PIL import ImageOps
    im = photo_image(11, 6, seed=13)
    buf = io.BytesIO()
    im.save(buf, "JPEG", quality=95, subsampling=0)
    plain = buf.getvalue()
    for o in range(1, 9):
        data = plain[:2] + exif_segment(o, o % 2 == 0) + plain[2:]
        dec = ImageOps.exif_transpose(Image.open(io.BytesIO(data))).convert("RGBA")
        save("jpeg_exif_%d.jpg" % o, data, dec.size[0], dec.size[1], "jpeg", dec.tobytes())


# ---------------------------------------------------------------- WebP


def save_webp(name, data, check):
    """A WebP file; the expected pixels are Pillow's (libwebp's) decoding of
    its first frame."""
    assert data[:4] == b"RIFF" and data[8:12] == b"WEBP", name
    im = Image.open(io.BytesIO(data))
    im.seek(0)
    dec = im.convert("RGBA")
    save(name + ".webp", data, dec.size[0], dec.size[1], check, dec.tobytes())


def with_alpha_shape(im, shape):
    """Alpha that the ALPH filters predict well: rows ("rows", the vertical
    filter) or a smooth ramp in both directions ("ramp", the gradient one)."""
    out = im.convert("RGBA")
    px = out.load()
    w, h = im.size
    rnd = random.Random(w * h)
    for y in range(h):
        for x in range(w):
            r, g, b, _ = px[x, y]
            if shape == "rows":
                a = [0, 255, 90, 180, 30][(x * 7 // w) % 5]
            else:
                a = min(255, x * 2 + y * 3 + rnd.randint(0, 1))
            px[x, y] = (r, g, b, a)
    return out


def riff_chunk(kind, body):
    return kind + struct.pack("<I", len(body)) + body + b"\x00" * (len(body) & 1)


def raw_alpha_webp(im, method):
    """A lossy WebP with an uncompressed ALPH chunk filtered by `method` (2
    vertical, 3 gradient), written here: cwebp only picks a filter when it
    compresses the alpha."""
    w, h = im.size
    rgb = pil_webp(im.convert("RGB"), quality=60)
    assert rgb[12:16] == b"VP8 "
    vp8 = rgb[20:20 + struct.unpack("<I", rgb[16:20])[0]]
    alpha = [im.getpixel((x, y))[3] for y in range(h) for x in range(w)]

    def at(x, y):
        return alpha[y * w + x]

    filtered = bytearray()
    for y in range(h):
        for x in range(w):
            if x == 0 and y == 0:
                pred = 0
            elif y == 0:
                pred = at(x - 1, 0)
            elif x == 0:
                pred = at(0, y - 1)
            elif method == 2:
                pred = at(x, y - 1)
            else:
                pred = max(0, min(255, at(x - 1, y) + at(x, y - 1) - at(x - 1, y - 1)))
            filtered.append((at(x, y) - pred) & 255)
    vp8x = struct.pack("<I", 0x10) + struct.pack("<I", w - 1)[:3] + struct.pack("<I", h - 1)[:3]
    body = b"WEBP" + riff_chunk(b"VP8X", vp8x) + riff_chunk(b"ALPH", bytes([method << 2]) + bytes(filtered)) + \
        riff_chunk(b"VP8 ", vp8)
    return b"RIFF" + struct.pack("<I", len(body)) + body


def palette_image(w, h, colors, seed):
    rnd = random.Random(seed)
    pal = make_palette(colors, seed)
    im = Image.new("RGB", (w, h))
    px = im.load()
    for y in range(h):
        for x in range(w):
            px[x, y] = pal[(x // 3 + y // 2 + rnd.randint(0, 1)) % colors]
    return im


def cwebp(im, args):
    src = os.path.join(HERE, "cwebp_in.tmp.png")
    out = os.path.join(HERE, "cwebp_out.tmp.webp")
    im.save(src, "PNG")
    subprocess.run(["cwebp", "-quiet"] + args + [src, "-o", out], check=True)
    with open(out, "rb") as f:
        data = f.read()
    os.remove(src)
    os.remove(out)
    return data


def pil_webp(im, **opts):
    buf = io.BytesIO()
    im.save(buf, "WEBP", **opts)
    return buf.getvalue()


def make_webps():
    photo = photo_image(40, 30)
    odd = photo_image(33, 17, seed=8)
    big = photo_image(128, 96, seed=4)
    # Lossless (VP8L): must match libwebp exactly.
    save_webp("webp_lossless_photo", pil_webp(photo, lossless=True), "exact")
    save_webp("webp_lossless_odd_alpha", pil_webp(with_alpha(odd), lossless=True, exact=True), "exact")
    save_webp("webp_lossless_1x1", pil_webp(Image.new("RGBA", (1, 1), (10, 200, 30, 128)), lossless=True), "exact")
    for colors in (2, 3, 11, 200):
        im = palette_image(33, 17, colors, seed=colors)
        save_webp("webp_lossless_palette_%d" % colors, pil_webp(im, lossless=True), "exact")
    exif = b"Exif\x00\x00II*\x00\x08\x00\x00\x00\x00\x00\x00\x00\x00\x00"
    data = pil_webp(with_alpha(photo), lossless=True, exif=exif)
    assert data[12:16] == b"VP8X"
    save_webp("webp_lossless_vp8x", data, "exact")
    if shutil.which("cwebp"):
        for z in (0, 3, 9):
            save_webp("webp_lossless_big_z%d" % z, cwebp(big, ["-z", str(z)]), "exact")
        save_webp("webp_lossless_near", cwebp(odd, ["-near_lossless", "40"]), "exact")
    # Lossy (VP8): decoded the way libwebp does, so its pixels exactly too.
    save_webp("webp_lossy_q80", pil_webp(photo, quality=80), "exact")
    save_webp("webp_lossy_odd_q30", pil_webp(odd, quality=30), "exact")
    save_webp("webp_lossy_big_q95", pil_webp(big, quality=95), "exact")
    save_webp("webp_lossy_alpha", pil_webp(with_alpha(odd), quality=70), "exact")
    save_webp("webp_lossy_gray", pil_webp(photo.convert("L"), quality=60), "exact")
    if shutil.which("cwebp"):
        for name, im, args in [
            ("webp_lossy_simple_filter", big, ["-q", "50", "-nostrong", "-f", "60"]),
            ("webp_lossy_sharpness", odd, ["-q", "40", "-sharpness", "6", "-f", "80"]),
            ("webp_lossy_no_filter", odd, ["-q", "70", "-f", "0"]),
            ("webp_lossy_one_segment", odd, ["-q", "20", "-segments", "1", "-sns", "0"]),
            ("webp_lossy_big_q10", big, ["-q", "10", "-f", "100"]),
            ("webp_alpha_raw", with_alpha_shape(big, "rows"), ["-q", "60", "-alpha_method", "0", "-alpha_filter", "best"]),
            ("webp_alpha_filter_fast", with_alpha(big), ["-q", "60", "-alpha_filter", "fast", "-alpha_q", "50", "-exact"]),
            ("webp_alpha_filter_best", with_alpha_shape(big, "ramp"), ["-q", "60", "-alpha_filter", "best"]),
        ]:
            save_webp(name, cwebp(im, args), "exact")
    save_webp("webp_alpha_vertical", raw_alpha_webp(with_alpha_shape(odd, "rows"), 2), "exact")
    save_webp("webp_alpha_gradient", raw_alpha_webp(with_alpha_shape(odd, "ramp"), 3), "exact")
    # Animations: the first frame only.
    frames = [photo, photo.transpose(Image.FLIP_LEFT_RIGHT)]
    for name, opts in [("webp_lossless_animated", dict(lossless=True)), ("webp_lossy_animated", dict(quality=75))]:
        buf = io.BytesIO()
        frames[0].save(buf, "WEBP", save_all=True, append_images=frames[1:], duration=100, **opts)
        data = buf.getvalue()
        assert b"ANMF" in data
        save_webp(name, data, "exact")

    good = pil_webp(photo, quality=80)
    save("webp_bad_truncated.webp", good[:len(good) // 2], 0, 0, "invalid")
    bad = bytearray(good)
    bad[12:16] = b"VP9 "
    save("webp_bad_chunk.webp", bytes(bad), 0, 0, "invalid")
    good = pil_webp(photo, lossless=True)
    save("webp_bad_lossless_truncated.webp", good[:len(good) * 2 // 3], 0, 0, "invalid")
    # Bitstreams cut short inside chunks whose sizes say so.
    for name, opts in [("webp_bad_vp8_cut", dict(quality=80)), ("webp_bad_vp8l_cut", dict(lossless=True))]:
        good = pil_webp(photo, **opts)
        size = struct.unpack("<I", good[16:20])[0] // 2 * 2
        cut = good[20:20 + size // 2]
        body = b"WEBP" + riff_chunk(good[12:16], cut)
        save(name + ".webp", b"RIFF" + struct.pack("<I", len(body)) + body, 0, 0, "invalid")


def main():
    for f in os.listdir(HERE):
        if f.endswith((".png", ".gif", ".jpg", ".webp", ".rgba", ".bin")) or f == "manifest.txt":
            os.remove(os.path.join(HERE, f))
    make_pngs()
    make_gifs()
    make_jpegs()
    make_webps()
    save("empty.bin", b"", 0, 0, "unsupported")
    with open(os.path.join(HERE, "manifest.txt"), "w") as f:
        f.write("\n".join(manifest) + "\n")


if __name__ == "__main__":
    main()
