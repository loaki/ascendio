import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from helpers import grid, set, hline, vline, rect, ellipse, ellipse_ring, px, mirror_right, rows

TEMPLATES = {}
def add(name, g, pal):
    TEMPLATES[name] = (rows(g), pal)

# Fallback silhouette per clade, used for any taxon without a specific
# sprite. Same colour family as render.rs's group_color() so an unfinished
# taxon still reads as belonging to its clade.

def basal():
    g = grid()
    ellipse(g, 6, 6.5, 4.2, 3.4, 'a')
    px(g, [(4,5),(8,5),(6,4)], 'b')
    add('Basal', g, {'a':0x3FB8AF,'b':0x2C8A83})

def spiralia():
    g = grid()
    ellipse(g, 6, 6, 4.6, 2.0, 'a')
    ellipse_ring(g, 6, 6, 2.4, 2.4, 'b', 1.0)
    add('Spiralia', g, {'a':0x9B7EDE,'b':0x7657B5})

def ecdysozoa():
    g = grid()
    ellipse(g, 6, 6, 3.0, 2.4, 'a')
    for (lx,ly) in [(2,4),(1,6),(2,8),(10,4),(11,6),(10,8)]:
        set(g, lx, ly, 'b')
    add('Ecdysozoa', g, {'a':0xE8A33D,'b':0xB87D26})

def deuterostome():
    g = grid()
    import math
    cx,cy=6,6
    for i in range(5):
        ang=-math.pi/2+i*2*math.pi/5
        for r in range(5):
            set(g, int(cx+r*math.cos(ang)), int(cy+r*math.sin(ang)), 'a')
    ellipse(g, cx, cy, 1.2, 1.2, 'a')
    add('Deuterostome', g, {'a':0xE86A5E})

def fish():
    g = grid()
    ellipse(g, 6, 6, 3.6, 2.2, 'a')
    px(g, [(1,6),(0,4),(0,8)], 'a')
    px(g, [(6,3),(6,9)], 'a')
    px(g, [(8,5)], 'k')
    add('Fish', g, {'a':0x4A9FE0,'k':0x0E2436})

def tetrapod():
    g = grid()
    ellipse(g, 6, 6.5, 3.8, 2.0, 'a')
    for (lx,ly) in [(3,8),(2,9),(8,8),(9,9)]:
        set(g, lx, ly, 'a')
    px(g, [(9,5)], 'k')
    add('Tetrapod', g, {'a':0x5CC26B,'k':0x113B18})

def reptile():
    g = grid()
    ellipse(g, 6, 6.5, 4.0, 1.8, 'a')
    px(g, [(1,6),(0,5),(0,7)], 'a')
    for (lx,ly) in [(4,8),(3,9),(8,8),(9,9)]:
        set(g, lx, ly, 'a')
    px(g, [(9,5)], 'k')
    add('Reptile', g, {'a':0xE05E8A,'k':0x4A1626})

def mammal():
    g = grid()
    ellipse(g, 6, 6.5, 3.6, 2.4, 'a')
    ellipse(g, 9.4, 5, 1.4, 1.2, 'a')
    for (lx,ly) in [(4,8),(4,10),(8,8),(8,10)]:
        set(g, lx, ly, 'a')
    px(g, [(10,4)], 'k')
    add('Mammal', g, {'a':0xF0C674,'k':0x4A3A14})

def ancestor():
    """Backbone / internal-node glyph: an abstract branching helix, not a
    real animal. These are inferred common ancestors, so they intentionally
    read as different from the depicted-species sprites around them."""
    g = grid()
    vline(g, 1, 10, 5, 'a'); vline(g, 1, 10, 6, 'a')
    for y in [2,4,6,8,10]:
        hline(g, 3, 5, y, 'a' if y % 4 else 'b')
        hline(g, 6, 8, y+1 if y+1<12 else y, 'a' if y % 4 else 'b')
    add('Ancestor', g, {'a':0x8A94A3,'b':0x5E6774})

for fn in [basal, spiralia, ecdysozoa, deuterostome, fish, tetrapod, reptile, mammal, ancestor]:
    fn()
print(f"{len(TEMPLATES)} templates defined")
