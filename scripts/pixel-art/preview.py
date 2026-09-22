#!/usr/bin/env python3
"""Renders every sprite def to a contact-sheet PNG for visual review.

    python3 scripts/pixel-art/preview.py [out.png]

Needs Pillow (`pip install pillow`). Not needed to build or run the game --
this is purely an authoring aid.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from PIL import Image, ImageDraw, ImageFont

from animals import ANIMALS
from templates import TEMPLATES

ALL = {**ANIMALS, **TEMPLATES}
SCALE = 10
CELL = 12 * SCALE
PAD = 6
LABEL_H = 14
COLS = 8


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "contact_sheet.png"
    names = sorted(ALL.keys())
    rows_n = (len(names) + COLS - 1) // COLS

    sheet = Image.new(
        "RGBA",
        (COLS * (CELL + PAD) + PAD, rows_n * (CELL + LABEL_H + PAD) + PAD),
        (14, 17, 22, 255),
    )
    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 11)
    except OSError:
        font = ImageFont.load_default()

    for i, name in enumerate(names):
        rows, pal = ALL[name]
        cx, cy = i % COLS, i // COLS
        ox, oy = PAD + cx * (CELL + PAD), PAD + cy * (CELL + LABEL_H + PAD)

        img = Image.new("RGBA", (12, 12), (0, 0, 0, 0))
        for y, row in enumerate(rows):
            for x, ch in enumerate(row):
                if ch == ".":
                    continue
                hexv = pal[ch]
                img.putpixel((x, y), ((hexv >> 16) & 0xFF, (hexv >> 8) & 0xFF, hexv & 0xFF, 255))
        big = img.resize((CELL, CELL), Image.NEAREST)

        sheet.alpha_composite(big, (ox, oy))
        draw.rectangle([ox, oy, ox + CELL, oy + CELL], outline=(60, 70, 85, 255))
        draw.text((ox, oy + CELL + 1), name, font=font, fill=(220, 225, 235, 255))

    sheet.save(out)
    print(f"{len(names)} sprites -> {out}")


if __name__ == "__main__":
    main()
