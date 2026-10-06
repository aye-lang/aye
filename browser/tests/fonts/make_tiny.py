# Makes tiny.ttf, tiny.woff and tiny.woff2 (fontTools and brotli):
#     python3 make_tiny.py tiny
import sys
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont
from fontTools.ttLib import woff2
from fontTools.ttLib.tables._g_l_y_f import GlyphComponent

out = sys.argv[1]

def box(x0, y0, x1, y1):
    p = TTGlyphPen(None)
    p.moveTo((x0, y0)); p.lineTo((x0, y1)); p.lineTo((x1, y1)); p.lineTo((x1, y0)); p.closePath()
    return p.glyph()

def shapeA():
    # Big and small deltas, curves (off-curve points), two contours.
    p = TTGlyphPen(None)
    p.moveTo((50, 0)); p.lineTo((300, 700)); p.qCurveTo((310, 712), (320, 700)); p.lineTo((590, 0)); p.lineTo((470, 0))
    p.lineTo((400, 200)); p.lineTo((220, 200)); p.lineTo((150, 0)); p.closePath()
    p.moveTo((250, 290)); p.lineTo((370, 290)); p.lineTo((311, 470)); p.closePath()
    return p.glyph()

def shapeB():
    p = TTGlyphPen(None)
    p.moveTo((80, 0)); p.lineTo((80, 700)); p.qCurveTo((560, 700), (560, 530), (420, 380)); p.qCurveTo((600, 330), (600, 180), (300, 0)); p.closePath()
    p.moveTo((-1200, 3000)); p.lineTo((-1190, 3003)); p.lineTo((2500, -900)); p.closePath()
    return p.glyph()

fb = FontBuilder(1000, isTTF=True)
order = [".notdef", "space", "A", "B", "C", "D"]
fb.setupGlyphOrder(order)
fb.setupCharacterMap({0x20: "space", 0x41: "A", 0x42: "B", 0x43: "C", 0x44: "D"})
glyphs = {".notdef": box(50, 0, 450, 700), "space": TTGlyphPen(None).glyph(), "A": shapeA(), "B": shapeB(), "D": box(100, -100, 500, 600)}
fb.setupGlyf(glyphs)
glyf = fb.font["glyf"]
# C: a composite of A and D (D scaled and offset).
from fontTools.ttLib.tables._g_l_y_f import Glyph
c = Glyph()
c.numberOfContours = -1
a = GlyphComponent(); a.glyphName = "A"; a.x = 0; a.y = 0; a.flags = 0
d = GlyphComponent(); d.glyphName = "D"; d.x = 650; d.y = 20; d.flags = 0; d.transform = [[0.5, 0], [0, 0.75]]
c.components = [a, d]
glyf["C"] = c
fb.setupHorizontalMetrics({".notdef": (500, 50), "space": (300, 0), "A": (640, 50), "B": (650, 80), "C": (1300, 0), "D": (600, 100)})
for name in order:
    g = glyf[name]
    g.recalcBounds(glyf)
fb.setupHorizontalHeader(ascent=800, descent=-200)
fb.setupNameTable({"familyName": "Tiny", "styleName": "Regular"})
fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
fb.setupPost()
fb.font.save(out + ".ttf")
f = TTFont(out + ".ttf")
f.flavor = "woff"
f.save(out + ".woff")
woff2.compress(out + ".ttf", out + ".woff2", transform_tables={"glyf", "loca", "hmtx"})
print("ok")
