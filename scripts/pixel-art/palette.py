"""The planet palette: every colour a sprite may use.

Taken from `src/backdrop.rs` (rock, ash, snow, vegetation, sea, sky, lava,
trunk) plus creature accents muted to the same saturation, so animals look
painted with the same paints as the world they live in.
"""
import colorsys

PLANET = [
    (90, 82, 96), (74, 66, 80), (62, 56, 72), (46, 42, 54), (42, 38, 50), (28, 26, 34),
    (70, 58, 60), (48, 40, 44),
    (230, 240, 246), (223, 238, 246), (184, 202, 214), (140, 160, 178),
    (106, 122, 74), (79, 95, 56), (79, 138, 58), (58, 106, 44), (47, 122, 42), (32, 90, 30), (26, 60, 24),
    (159, 224, 224), (191, 232, 240), (79, 138, 128), (47, 127, 146), (30, 58, 68), (22, 48, 58), (14, 36, 44),
    (4, 16, 28),
    (126, 176, 214), (86, 122, 170), (48, 70, 112), (10, 26, 58),
    (255, 190, 90), (255, 176, 72), (255, 106, 40), (200, 62, 28), (150, 40, 24),
    (224, 168, 120), (200, 144, 112), (192, 80, 58),
    (42, 30, 20), (96, 70, 48), (150, 112, 74), (206, 170, 110), (236, 214, 150),
    (228, 140, 160), (186, 96, 128), (120, 56, 88),
    (196, 150, 226), (150, 104, 190), (98, 66, 138), (60, 40, 92),
    (220, 110, 84), (168, 66, 56), (112, 40, 40),
    (244, 241, 232), (16, 20, 32),
    (150, 100, 76), (110, 72, 54), (150, 92, 62),
    (190, 130, 80), (140, 92, 52), (200, 162, 96), (160, 124, 70),
]

# Outline inks: the darkest shade of each palette family.
INKS = [(28, 26, 34), (14, 36, 44), (26, 40, 20), (42, 30, 20), (40, 18, 28), (30, 22, 50), (16, 20, 32)]

# Map letters with a fixed colour: eye white, eye (pupil), glint, ivory, mouth.
FIXED = {'e': (244, 241, 232), 'k': (16, 20, 32), 'w': (244, 241, 232), 't': (244, 241, 232), 'm': (16, 20, 32)}

BAYER = [0.03, 0.53, 0.16, 0.66, 0.78, 0.28, 0.91, 0.41, 0.22, 0.72, 0.09, 0.59, 0.97, 0.47, 0.84, 0.34]


def rgb(h):
    return ((h >> 16) & 255, (h >> 8) & 255, h & 255)


def _dist(a, b):
    return 2 * (a[0] - b[0]) ** 2 + 4 * (a[1] - b[1]) ** 2 + 3 * (a[2] - b[2]) ** 2


def nearest(c, pool=PLANET):
    return min(pool, key=lambda p: _dist(p, c))


def _shift(c, mul, dh, ds):
    h, l, s = colorsys.rgb_to_hls(*[v / 255 for v in c])
    h = (h + dh / 360) % 1
    r, g, b = colorsys.hls_to_rgb(h, max(0, min(1, l * mul)), max(0, min(1, s + ds)))
    return (int(r * 255), int(g * 255), int(b * 255))


def _hue_gap(a, b):
    ha = colorsys.rgb_to_hls(*[v / 255 for v in a])[0] * 360
    hb = colorsys.rgb_to_hls(*[v / 255 for v in b])[0] * 360
    return abs((ha - hb + 540) % 360 - 180)


def _sat(c):
    return colorsys.rgb_to_hls(*[v / 255 for v in c])[2]


def ramp(hexc):
    """(shadow, base, light, ink), each snapped to the palette."""
    base = nearest(rgb(hexc))
    hue = colorsys.rgb_to_hls(*[v / 255 for v in base])[0] * 360
    cool = (250 - hue + 540) % 360 - 180
    warm = (55 - hue + 540) % 360 - 180
    others = [p for p in PLANET if p != base]
    # Shadows lean cool, but never swing a warm colour round through red.
    shadow = nearest(_shift(base, 0.66, cool * (0.12 if abs(cool) < 90 else 0.03), 0.0), others)
    if _hue_gap(shadow, base) > 12 or _sat(shadow) - _sat(base) > 0.2:
        shadow = nearest((base[0] * 0.66, base[1] * 0.64, base[2] * 0.72), others)
    light = nearest(_shift(base, 1.28, warm * 0.10, -0.05), others)
    ink = nearest(_shift(base, 0.32, cool * 0.1, 0.0), INKS)
    return shadow, base, light, ink
