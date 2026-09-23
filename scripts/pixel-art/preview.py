#!/usr/bin/env python3
"""Contact sheet of rendered sprites with their silhouettes.

    python3 scripts/pixel-art/preview.py out.png [Name ...]
"""
import os
import sys

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from maps import A, check
from render import GRID, render


def sheet(path, names, scale=8, cols=10):
    cell = GRID * scale + 12
    rows = (len(names) + cols - 1) // cols
    out = Image.new('RGBA', (cell * cols, cell * rows * 2), (14, 30, 38, 255))
    for i, name in enumerate(names):
        rows_, pal = A[name]
        px, _ = render(name, rows_, pal)
        spr = Image.new('RGBA', (GRID, GRID), (0, 0, 0, 0))
        sil = Image.new('RGBA', (GRID, GRID), (200, 210, 220, 255))
        for y in range(GRID):
            for x in range(GRID):
                if px[y][x]:
                    spr.putpixel((x, y), px[y][x] + (255,))
                if rows_[y][x] != '.':
                    sil.putpixel((x, y), (16, 20, 32, 255))
        x0, y0 = (i % cols) * cell + 6, (i // cols) * cell * 2 + 6
        out.alpha_composite(spr.resize((GRID * scale,) * 2, Image.NEAREST), (x0, y0))
        out.alpha_composite(sil.resize((GRID * scale,) * 2, Image.NEAREST), (x0, y0 + cell))
    out.save(path)


if __name__ == '__main__':
    check()
    names = sys.argv[2:] or list(A)
    sheet(sys.argv[1], names)
