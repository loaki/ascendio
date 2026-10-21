#!/usr/bin/env python3
"""Renders the Google Play featured graphic (1024x500) into store/: the
title over a line of animals climbing the tree of life.

    python3 scripts/pixel-art/store.py
"""
import math
import os
import sys

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from icon import BG, REPO, STORE, sprite_of, write
from palette import BAYER
from render import GRID

W, H = 1024, 500
TITLE = "ASCENDIO"
TEXT = (0xE6, 0xEA, 0xF0)
SHADOW = (0x5B, 0xC8, 0xF5)  # the game's DNA accent
# Root to tip, the climb along the bottom.
CLIMB = ["Jellyfish", "Trilobite", "Ammonite", "Shark", "Tiktaalik", "T. rex", "Mammoth", "Human"]
# 5x7 capitals, just the title's.
GLYPHS = {
    "A": ["01110", "10001", "10001", "11111", "10001", "10001", "10001"],
    "S": ["01111", "10000", "10000", "01110", "00001", "00001", "11110"],
    "C": ["01111", "10000", "10000", "10000", "10000", "10000", "01111"],
    "E": ["11111", "10000", "10000", "11110", "10000", "10000", "11111"],
    "N": ["10001", "11001", "10101", "10011", "10001", "10001", "10001"],
    "D": ["11110", "10001", "10001", "10001", "10001", "10001", "11110"],
    "I": ["11111", "00100", "00100", "00100", "00100", "00100", "11111"],
    "O": ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
}


def backdrop():
    """The background with a dithered glow rising from the bottom, on a coarse grid like the game's."""
    cell = 8
    img = Image.new("RGBA", (W, H), BG + (255,))
    glow = (0x16, 0x2A, 0x36)
    for gy in range(H // cell):
        for gx in range(W // cell):
            t = (gy * cell / H) ** 2 * 0.9
            if t > BAYER[(gy % 4) * 4 + gx % 4]:
                img.paste(glow + (255,), (gx * cell, gy * cell, (gx + 1) * cell, (gy + 1) * cell))
    return img


def title(img, scale, y):
    w = (len(TITLE) * 6 - 1) * scale
    x0 = (W - w) // 2
    for colour, off in ((SHADOW, scale // 2), (TEXT, 0)):
        for i, ch in enumerate(TITLE):
            for r, row in enumerate(GLYPHS[ch]):
                for c, bit in enumerate(row):
                    if bit == "1":
                        x = x0 + (i * 6 + c) * scale + off
                        img.paste(colour + (255,), (x, y + r * scale + off, x + scale, y + (r + 1) * scale + off))


def main():
    img = backdrop()
    title(img, 12, 70)
    scale = 6
    size = GRID * scale
    n = len(CLIMB)
    for i, name in enumerate(CLIMB):
        t = i / (n - 1)
        x = int(60 + t * (W - 120 - size))
        y = int(H - 40 - size - math.sin(t * math.pi / 2) * 140)
        img.alpha_composite(sprite_of(name).resize((size, size), Image.NEAREST), (x, y))
    write(os.path.join(STORE, "featured.png"), img.convert("RGB"))
    print(f"wrote the featured graphic to {os.path.relpath(STORE, REPO)}")


if __name__ == "__main__":
    main()
