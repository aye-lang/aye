# Makes the images of the paint and layout fixtures (decoded.html,
# layout/images.html). Needs Pillow; run from anywhere:
#
#     python3 browser/tests/paint/img/make_images.py
#
# The images are committed, so the tests do not need Python.
import math
import os

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))


def photo(w, h):
    """A smooth color field with a few sharp shapes."""
    im = Image.new("RGB", (w, h))
    px = im.load()
    for y in range(h):
        for x in range(w):
            r = int(255 * x / (w - 1))
            g = int(255 * y / (h - 1))
            b = int(128 + 127 * math.sin((x + y) / 6.0))
            if (x - w * 0.7) ** 2 + (y - h * 0.4) ** 2 < (h * 0.2) ** 2:
                r, g, b = 255, 255, 255
            if w * 0.1 < x < w * 0.3 and h * 0.6 < y < h * 0.9:
                r, g, b = 20, 20, 20
            px[x, y] = (r, g, b)
    return im


def badge(size):
    """A disc that fades out at its edge, on a transparent background."""
    im = Image.new("RGBA", (size, size))
    px = im.load()
    c = (size - 1) / 2.0
    for y in range(size):
        for x in range(size):
            d = math.hypot(x - c, y - c) / c
            a = 0 if d >= 1.0 else int(255 * min(1.0, (1.0 - d) * 3.0))
            px[x, y] = (200, 30, 90, a)
    return im


def icon():
    """A 16x16 palette image with a transparent color."""
    im = Image.new("P", (16, 16), 0)
    im.putpalette([255, 0, 255, 0, 120, 255, 255, 200, 0] + [0] * (253 * 3))
    px = im.load()
    for y in range(16):
        for x in range(16):
            if 3 <= x <= 12 and 3 <= y <= 12:
                px[x, y] = 1 if (x + y) % 4 < 2 else 2
    return im


photo(64, 48).save(os.path.join(HERE, "photo.png"))
photo(64, 48).save(os.path.join(HERE, "photo.jpg"), quality=92, subsampling=2)
badge(24).save(os.path.join(HERE, "badge.png"))
icon().save(os.path.join(HERE, "icon.gif"), transparency=0)
