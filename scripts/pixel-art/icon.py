#!/usr/bin/env python3
"""Renders the Android launcher icon (the "Ancestor" glyph) into android/res/.

    python3 scripts/pixel-art/icon.py
"""
import os
import sys

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from maps import A
from render import GRID, render

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
RES = os.path.join(REPO, "android", "res")
SPRITE = "Ancestor"
BG = (0x0E, 0x11, 0x16)  # the game's background, as in web/index.html
# density -> px per dp, as a multiple of mdpi's 1.
DENSITIES = {"mdpi": 1, "hdpi": 1.5, "xhdpi": 2, "xxhdpi": 3, "xxxhdpi": 4}


def sprite():
    rows, pal = A[SPRITE]
    px, _ = render(SPRITE, rows, pal)
    img = Image.new("RGBA", (GRID, GRID))
    for y in range(GRID):
        for x in range(GRID):
            if px[y][x]:
                img.putpixel((x, y), px[y][x] + (255,))
    return img


def centred(spr, size, scale, bg):
    out = Image.new("RGBA", (size, size), bg)
    big = spr.resize((GRID * scale,) * 2, Image.NEAREST)
    off = (size - GRID * scale) // 2
    out.alpha_composite(big, (off, off))
    return out


def write(path, img):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    img.save(path)


def main():
    spr = sprite()
    for name, k in DENSITIES.items():
        d = os.path.join(RES, f"mipmap-{name}")
        # Adaptive foreground: a 108dp layer, the sprite 64dp inside its 66dp safe zone.
        write(os.path.join(d, "ic_launcher_foreground.png"),
              centred(spr, int(108 * k), int(4 * k), (0, 0, 0, 0)))
        # Legacy icon (Android 7): 48dp, background baked in.
        write(os.path.join(d, "ic_launcher.png"), centred(spr, int(48 * k), int(2 * k), BG + (255,)))
    write_text(os.path.join(RES, "mipmap-anydpi-v26", "ic_launcher.xml"), """\
<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
</adaptive-icon>
""")
    write_text(os.path.join(RES, "values", "ic_launcher.xml"), f"""\
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <color name="ic_launcher_background">#{BG[0]:02X}{BG[1]:02X}{BG[2]:02X}</color>
</resources>
""")
    print(f"wrote the {SPRITE} icon to {RES}")


def write_text(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, "w").write(text)


if __name__ == "__main__":
    main()
