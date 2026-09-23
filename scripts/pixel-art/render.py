"""Renders a 16x16 material map in the "rough ink" style.

Two tones per material from the planet palette, speckled like the
backdrop's rock, a few chipped edge pixels, grit on the shadow side and a
dark outline that breaks in places. Deterministic: the noise is seeded by
the animal's name.
"""
import math

from palette import BAYER, FIXED, ramp

GRID = 16
EYE = 'k'


def _hash(x, y, seed):
    h = (x * 374761393 + y * 668265263 + seed * 2246822519) & 0xFFFFFFFF
    h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
    return ((h ^ (h >> 16)) & 0xFFFF) / 65535.0


def seed_of(name):
    return sum(ord(ch) * (i + 1) for i, ch in enumerate(name))


def _light(inside):
    """0..1 per pixel: how much it faces a light from the top left."""
    n = len(inside)
    k = [(dx, dy, math.exp(-(dx * dx + dy * dy) / 2.2)) for dx in range(-2, 3) for dy in range(-2, 3)]
    tot = sum(w for _, _, w in k)
    blur = [[sum(w for dx, dy, w in k if 0 <= x + dx < n and 0 <= y + dy < n and inside[y + dy][x + dx]) / tot
             for x in range(n)] for y in range(n)]

    def at(x, y):
        return blur[y][x] if 0 <= x < n and 0 <= y < n else 0.0

    return [[max(0.0, min(1.0, 0.52 + (at(x + 1, y + 1) - at(x - 1, y - 1)) * 1.3)) if inside[y][x] else 0.0
             for x in range(n)] for y in range(n)]


def render(name, rows, pal):
    """Returns (pixels, eyes): a GRID x GRID grid of RGB tuples or None, and
    the set of (x, y) that are eye pixels (morphs recolour those)."""
    seed = seed_of(name)
    n = GRID
    inside = [[rows[y][x] != '.' for x in range(n)] for y in range(n)]

    def isin(x, y):
        return 0 <= x < n and 0 <= y < n and inside[y][x]

    light = _light(inside)
    ramps = {c: ramp(h) for c, h in pal.items()}
    px = [[None] * n for _ in range(n)]
    eyes = set()
    for y in range(n):
        for x in range(n):
            c = rows[y][x]
            if c == '.':
                continue
            if c in FIXED and c not in pal:
                px[y][x] = FIXED[c]
                if c == EYE:
                    eyes.add((x, y))
                continue
            edge = not all(isin(x + dx, y + dy) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
            if edge and _hash(x, y, seed) < 0.10 and light[y][x] < 0.55:
                continue  # chipped edge, shadow side only
            shadow, base, _, _ = ramps[c]
            noise = _hash(x >> 1, y, seed + 7) - 0.5
            v = light[y][x] + noise * 0.5 + (BAYER[(y & 3) * 4 + (x & 3)] - 0.5) * 0.15
            px[y][x] = shadow if v < 0.5 else base
    # grit knocked off the shadow side
    for y in range(n):
        for x in range(n):
            if inside[y][x] or px[y][x]:
                continue
            for a, b in ((x - 1, y), (x, y - 1)):
                if isin(a, b) and rows[b][a] in ramps and _hash(x, y, seed + 3) < 0.10:
                    px[y][x] = ramps[rows[b][a]][0]
                    break
    # worn outline
    drawn = [[px[y][x] is not None for x in range(n)] for y in range(n)]
    for y in range(n):
        for x in range(n):
            if drawn[y][x]:
                continue
            if (isin(x - 1, y) and isin(x + 1, y)) or (isin(x, y - 1) and isin(x, y + 1)):
                continue  # keep the gap between two limbs open
            nbs = [(x + dx, y + dy) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
                   if 0 <= x + dx < n and 0 <= y + dy < n and drawn[y + dy][x + dx] and inside[y + dy][x + dx]]
            if not nbs or _hash(x, y, seed + 11) < 0.18:
                continue
            a, b = nbs[0]
            c = rows[b][a]
            px[y][x] = ramps[c][3] if c in ramps else FIXED['k']
    return px, eyes
