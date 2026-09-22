import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from helpers import grid, set, hline, vline, rect, ellipse, ellipse_ring, px, mirror_right, rows, N

ANIMALS = {}  # name -> (rows, palette_dict[char]=hexint)

def add(name, g, pal):
    ANIMALS[name] = (rows(g), pal)

# ============================================================ Basal =====

def sea_sponge():
    g = grid()
    ellipse(g, 5.5, 4, 3.2, 3.4, 'a')          # barrel body
    rect(g, 3, 4, 8, 9, 'a')
    ellipse(g, 5.5, 9, 2.8, 1.2, 'b')          # base shadow
    px(g, [(4,3),(7,3),(5,6),(8,6),(4,8)], 'k')  # pore holes
    px(g, [(3,2),(8,2)], 'a')
    add('Sea Sponge', g, {'a':0xD9A857,'b':0xA97C34,'k':0x6B4A1E})

def comb_jelly():
    g = grid()
    ellipse(g, 5.5, 5.5, 3.4, 4.2, 'a')
    for i,x in enumerate([2,4,6,8]):
        vline(g, 2, 9, x if x<9 else 8, 'c')
    px(g, [(3,4),(5,3),(7,4)], 'w')
    add('Comb Jelly', g, {'a':0xBFE9F2,'c':0xE7A6E0,'w':0xFFFFFF})

def placozoan():
    g = grid()
    ellipse(g, 5.5, 6.5, 4.6, 2.4, 'a')
    px(g, [(3,6),(7,7),(5,5),(8,6)], 'b')
    add('Placozoan', g, {'a':0xE7C79A,'b':0xC9A26E})

def jellyfish():
    g = grid()
    ellipse(g, 5.5, 3.6, 3.6, 2.6, 'a')
    rect(g, 2, 3, 9, 4, 'a')
    for x in [2,4,5,7,9]:
        vline(g, 5, 10, x, 'b')
    px(g, [(4,3),(7,3)], 'k')
    add('Jellyfish', g, {'a':0xD79BE0,'b':0xA65FC0,'k':0x5A2E6E})

def coral():
    g = grid()
    rect(g, 5, 9, 6, 11, 'b')
    branches = [
        [(5,9),(4,7),(3,5),(2,3)], [(3,5),(2,4),(1,2)],
        [(6,9),(7,7),(8,5),(9,3)], [(8,5),(9,4),(10,2)],
        [(5,9),(5,6),(4,3),(4,1)], [(6,9),(6,6),(7,3),(7,1)],
    ]
    for path in branches:
        for i in range(len(path)-1):
            x0,y0 = path[i]; x1,y1 = path[i+1]
            steps = max(abs(x1-x0),abs(y1-y0),1)
            for s_ in range(steps+1):
                t=s_/steps
                set(g, round(x0+(x1-x0)*t), round(y0+(y1-y0)*t), 'a')
    for tip in [(2,3),(1,2),(9,3),(10,2),(4,1),(7,1)]:
        set(g, *tip, 'c')
    add('Coral', g, {'a':0xE87A5A,'b':0xB2553B,'c':0xF7B08A})

def acoel_worm():
    g = grid()
    ellipse(g, 6, 6, 4.6, 1.6, 'a')
    px(g, [(3,6)], 'k')
    add('Acoel Worm', g, {'a':0xCDA37C,'k':0x6B4A2E})

# ============================================================ Spiralia ==

def flatworm():
    g = grid()
    ellipse(g, 6, 6, 4.8, 1.8, 'a')
    ellipse(g, 6, 6, 3.6, 0.9, 'b')
    px(g, [(3,5),(3,7)], 'k')
    add('Flatworm', g, {'a':0xB98CD1,'b':0x8E5FAE,'k':0x4A2E5E})

def snail():
    g = grid()
    ellipse(g, 4.5, 8.5, 3.6, 1.8, 'a')
    ellipse(g, 6.5, 6, 3.6, 3.6, 'b')
    ellipse_ring(g, 6.5, 6, 2.5, 2.5, 'c', 1.1)
    ellipse_ring(g, 6.5, 6, 1.3, 1.3, 'c', 0.9)
    px(g, [(1,7),(1,9)], 'a')
    px(g, [(1,6)], 'k')
    add('Snail', g, {'a':0xD9B15C,'b':0xE8CE8A,'c':0xB98530,'k':0x4A3416})

def octopus():
    g = grid()
    ellipse(g, 6, 3.6, 3.6, 3.0, 'a')
    px(g, [(4,3),(8,3)], 'k'); px(g, [(4,3)], 'w'); px(g, [(8,3)], 'w')
    for bx in [1,3,5,7,9]:
        vline(g, 6, 10, bx, 'a')
    for bx in [2,4,6,8]:
        vline(g, 6, 9, bx, 'b')
    add('Octopus', g, {'a':0xD1548C,'b':0xA83A6C,'k':0x2A0F1E,'w':0xFFFFFF})

def segmented_worm():
    g = grid()
    ellipse(g, 6, 6, 5.2, 1.9, 'a')
    for x in [1,3,5,7,9,11]:
        vline(g, 4, 8, x, 'b')
    px(g, [(1,5)], 'k')
    add('Segmented Worm', g, {'a':0xC96B4E,'b':0xA0492F,'k':0x3A1A0E})

# ============================================================ Ecdysozoa =

def nematode():
    g = grid()
    pts = [(1,6),(2,5),(3,5),(4,6),(5,7),(6,7),(7,6),(8,5),(9,5),(10,6)]
    for (x,y) in pts:
        set(g, x, y, 'a'); set(g, x, y+1, 'a')
    add('Nematode', g, {'a':0xE0C070})

def horseshoe_crab():
    g = grid()
    ellipse(g, 6, 4.6, 4.6, 3.6, 'a')
    ellipse(g, 6, 8, 3.2, 1.6, 'b')
    hline(g, 5, 7, 10, 'b')
    px(g, [(4,3),(8,3)], 'k')
    add('Horseshoe Crab', g, {'a':0x8C6B4A,'b':0x6E4F32,'k':0x2E1E10})

def scorpion():
    g = grid()
    ellipse(g, 5.5, 6.5, 3.4, 2.4, 'a')
    px(g, [(2,5),(1,4),(2,7),(1,8)], 'b')
    for x,y in [(8,5),(9,4),(10,3),(10,2)]:
        set(g, x, y, 'b')
    set(g, 10, 1, 'k')
    for lx in [3,4,7,8]:
        vline(g, 8, 9, lx, 'a')
    add('Scorpion', g, {'a':0xE0A030,'b':0xB87F20,'k':0x2A1808})

def spider():
    g = grid()
    ellipse(g, 6, 7.2, 2.6, 2.4, 'a')
    ellipse(g, 6, 4.2, 1.8, 1.6, 'a')
    legs = [(1,3),(0,5),(0,7),(1,9),(11,3),(12,5),(12,7),(11,9)]
    for (lx,ly) in legs:
        x0,y0 = 6, 6.5
        steps = 5
        for s_ in range(steps+1):
            t = s_/steps
            set(g, round(x0+(lx-x0)*t), round(y0+(ly-y0)*t), 'c')
    px(g, [(5,4),(7,4)], 'k')
    add('Spider', g, {'a':0x6E3A4A,'c':0xB05A6A,'k':0xE04040})

def crab():
    g = grid()
    ellipse(g, 6, 6, 4.4, 2.6, 'a')
    px(g, [(1,4),(0,3),(2,3)], 'b'); px(g, [(10,4),(11,3),(9,3)], 'b')
    for lx in [2,3,8,9]:
        vline(g, 8, 9, lx, 'b')
    px(g, [(4,5),(7,5)], 'k')
    add('Crab', g, {'a':0xE0553A,'b':0xB03A24,'k':0x3A1006})

def dragonfly():
    g = grid()
    hline(g, 2, 10, 6, 'a'); hline(g, 2, 10, 5, 'a')
    ellipse(g, 3, 3.2, 3.4, 1.4, 'c')
    ellipse(g, 8, 3.2, 3.4, 1.4, 'c')
    ellipse(g, 3, 8.6, 3.4, 1.4, 'c')
    ellipse(g, 8, 8.6, 3.4, 1.4, 'c')
    px(g, [(1,4),(1,5)], 'k')
    add('Dragonfly', g, {'a':0x2E6E5E,'c':0xB9E8DC,'k':0x123028})

def beetle():
    g = grid()
    ellipse(g, 6, 6.5, 4.2, 3.6, 'a')
    vline(g, 3, 10, 6, 'b')
    ellipse(g, 6, 2.8, 1.6, 1.4, 'a')
    px(g, [(5,2),(7,2)], 'k')
    for lx in [1,2,9,10]:
        hline(g, lx, lx+1, 7, 'b')
    add('Beetle', g, {'a':0x3E7A3A,'b':0x275224,'k':0x0F2A0E})

def butterfly():
    g = grid()
    vline(g, 3, 9, 6, 'k')
    # Upper wing: broad, rounded outward
    ellipse(g, 3.2, 4.0, 2.6, 2.0, 'a')
    ellipse(g, 2.6, 3.6, 1.1, 0.9, 'c')
    # Lower wing: smaller, tapered
    ellipse(g, 3.6, 7.6, 1.9, 1.7, 'a')
    ellipse(g, 3.4, 7.8, 0.7, 0.6, 'c')
    mirror_right(g, 6)
    px(g, [(6,2),(6,3)], 'k')
    px(g, [(5,1),(7,1)], 'k')
    add('Butterfly', g, {'a':0xE8862F,'c':0x2A1608,'k':0x1A1008})

def ant():
    g = grid()
    ellipse(g, 2.6, 7, 1.6, 1.4, 'a')    # abdomen
    ellipse(g, 5.6, 6.6, 0.9, 0.8, 'a')  # thorax waist
    ellipse(g, 8, 6, 1.4, 1.3, 'a')      # head
    px(g, [(9,4),(10,3),(9,8),(10,9)], 'b')  # antennae
    for lx, ly in [(3,8),(2,9),(5,8),(5,9),(7,8),(8,9)]:  # 6 legs, clear of the body
        set(g, lx, ly, 'b')
    px(g, [(8,5)], 'k')
    add('Ant', g, {'a':0xB8481E,'b':0x5A2010,'k':0x140804})

# ========================================================= Deuterostome =

def starfish():
    g = grid()
    pts = []
    import math
    cx, cy = 6, 6.2
    for i in range(5):
        ang = -math.pi/2 + i*2*math.pi/5
        tx = cx + 5.0*math.cos(ang); ty = cy + 5.0*math.sin(ang)
        mx = cx + 1.7*math.cos(ang+0.6); my = cy + 1.7*math.sin(ang+0.6)
        mx2 = cx + 1.7*math.cos(ang-0.6); my2 = cy + 1.7*math.sin(ang-0.6)
        for (x0,y0) in [(cx,cy),(mx,my)]:
            steps=6
            for s in range(steps+1):
                t=s/steps
                set(g, int(x0+(tx-x0)*t), int(y0+(ty-y0)*t), 'a')
        for s in range(4):
            t=s/4
            set(g, int(cx+(mx-cx)*t), int(cy+(my-cy)*t), 'a')
            set(g, int(cx+(mx2-cx)*t), int(cy+(my2-cy)*t), 'a')
    ellipse(g, cx, cy, 1.4, 1.4, 'a')
    px(g, [(6,6)], 'b')
    add('Starfish', g, {'a':0xE0762E,'b':0xB55A1E})

def sea_urchin():
    g = grid()
    ellipse(g, 6, 6, 3.0, 3.0, 'a')
    import math
    for i in range(16):
        ang = i * 2*math.pi/16
        x1 = 6+3.0*math.cos(ang); y1=6+3.0*math.sin(ang)
        x2 = 6+5.3*math.cos(ang); y2=6+5.3*math.sin(ang)
        set(g, int(x1), int(y1), 'b'); set(g, int(x2), int(y2), 'b')
    add('Sea Urchin', g, {'a':0x4A2E6E,'b':0x2E1A4A})

def sea_squirt():
    g = grid()
    rect(g, 4, 4, 8, 9, 'a')
    ellipse(g, 6, 9, 2.4, 1.6, 'a')
    hline(g, 4, 5, 3, 'a'); hline(g, 7, 8, 3, 'a')
    px(g, [(5,2),(4,2)], 'b'); px(g, [(8,2),(7,2)], 'b')
    add('Sea Squirt', g, {'a':0xE0C24A,'b':0xB89830})

# ================================================================ Fish ==

def lamprey():
    g = grid()
    pts = [(1,6),(2,6),(3,5),(4,6),(5,6),(6,5),(7,6),(8,6),(9,5),(10,6)]
    for (x,y) in pts:
        set(g,x,y,'a'); set(g,x,y+1,'a')
    ellipse(g, 1.3, 6, 1.1, 1.1, 'b')
    px(g,[(2,5)],'k')
    add('Lamprey', g, {'a':0x8E9A6E,'b':0x5E6E42,'k':0x1E2210})

def shark():
    g = grid()
    ellipse(g, 5.5, 6, 4.6, 1.9, 'a')
    px(g, [(9,4),(10,3),(9,2)], 'a')  # dorsal fin
    for y in [3,4]: set(g,9,y,'a')
    px(g, [(1,6),(0,5),(0,7)], 'a')  # tail
    px(g, [(2,7),(1,8)], 'a')        # pectoral fin
    px(g, [(9,5)], 'k')
    add('Shark', g, {'a':0x6E8CA0,'k':0x1A2428})

def ray_finned_fish():
    g = grid()
    ellipse(g, 6, 6, 3.6, 2.4, 'a')
    px(g, [(1,6),(0,4),(0,8)], 'a')
    px(g, [(6,3),(7,2)], 'a'); px(g, [(6,9),(7,10)], 'a')
    px(g, [(8,5)], 'k'); px(g,[(8,5)],'w')
    add('Ray-finned Fish', g, {'a':0x4A9FD8,'k':0x102030,'w':0xE8F6FF})

def coelacanth():
    g = grid()
    ellipse(g, 6, 6, 3.8, 2.3, 'a')
    px(g, [(2,8),(1,9),(3,9)], 'b')
    px(g, [(1,6),(0,5),(0,6),(0,7)], 'a')
    px(g, [(6,3),(5,9),(4,3)], 'c')
    px(g, [(8,5)], 'k')
    add('Coelacanth', g, {'a':0x4E6E8A,'b':0x3A5468,'c':0xBFD4E0,'k':0x0E1620})

# ============================================================= Tetrapod =

def amphibian():
    g = grid()
    ellipse(g, 6, 6, 4.0, 1.7, 'a')
    px(g,[(9,7),(10,8)],'a'); px(g,[(10,9),(9,9)],'a')
    for (lx,ly) in [(4,7),(3,8),(8,7),(9,8)]:
        set(g, lx, ly, 'a')
    px(g,[(2,4),(1,4)],'k')
    add('Amphibian', g, {'a':0x5FA85A,'k':0x1E3A1C})

def frog():
    g = grid()
    ellipse(g, 5.5, 7, 3.6, 2.6, 'a')
    ellipse(g, 3.5, 4.6, 1.3, 1.3, 'a'); ellipse(g, 6.5, 4.6, 1.3, 1.3, 'a')
    px(g, [(3,4),(7,4)], 'k')
    px(g, [(9,8),(10,7),(10,9)], 'a')
    px(g, [(2,9),(1,8)], 'a')
    add('Frog', g, {'a':0x6FBF4A,'k':0x123008})

# ============================================================== Reptile =

def turtle():
    g = grid()
    ellipse(g, 6, 6.5, 4.4, 3.0, 'a')
    ellipse_ring(g, 6, 6.5, 3.2, 2.0, 'b', 0.9)
    px(g, [(10,6),(11,5)], 'c')
    px(g, [(11,4)], 'k')
    for (lx,ly) in [(2,9),(4,9),(8,9),(9,9)]:
        set(g, lx, ly, 'c')
    add('Turtle', g, {'a':0x3E8E4E,'b':0x2A6636,'c':0xC9A25E,'k':0x102008})

def lizard():
    g = grid()
    ellipse(g, 6, 6, 3.8, 1.6, 'a')
    px(g, [(1,6),(0,5),(0,7)], 'a')
    px(g, [(9,5),(10,4)], 'a')
    for (lx,ly) in [(4,7),(3,9),(8,7),(9,9)]:
        set(g, lx, ly, 'a')
    px(g, [(9,4)], 'k')
    add('Lizard', g, {'a':0x4A9E4A,'k':0x102A10})

def crocodile():
    g = grid()
    ellipse(g, 6, 7, 4.6, 1.6, 'a')
    rect(g, 8, 6, 11, 7, 'a')
    px(g, [(11,6),(10,6)], 'b')
    for (lx,ly) in [(3,8),(2,9),(8,8),(9,9)]:
        set(g, lx, ly, 'a')
    px(g, [(1,6),(0,5),(0,7)], 'a')
    px(g, [(9,6)], 'k')
    add('Crocodile', g, {'a':0x3E6E3A,'b':0x2A4E28,'k':0xE0D060})

def dinosaur():
    g = grid()
    ellipse(g, 5.5, 6.5, 2.4, 2.3, 'a')          # body
    px(g, [(7,4),(8,3),(9,2),(10,1),(10,0)], 'a')  # neck up to head
    for x,y in [(8,4),(9,3),(9,2),(10,2)]:
        set(g, x, y, 'a')
    px(g, [(11,1)], 'b')                          # jaw line
    px(g, [(2,7),(1,9),(0,11),(1,11)], 'a')       # tail sweeping back
    px(g, [(2,8),(1,10)], 'a')
    rect(g, 4, 8, 5, 11, 'a'); rect(g, 6, 8, 7, 11, 'a')  # legs
    px(g, [(3,5),(4,4)], 'b')                      # tiny arm
    px(g, [(10,1)], 'k')
    add('Dinosaur', g, {'a':0x8A5A3A,'b':0x6E4428,'k':0x1E1006})

def bird():
    g = grid()
    ellipse(g, 6, 6.5, 3.2, 2.4, 'a')
    ellipse(g, 8.8, 5, 1.6, 1.4, 'a')
    px(g, [(10.4 if False else 10,5)], 'c')
    px(g, [(10,5)], 'c')
    px(g, [(3,5),(1,4),(3,7)], 'b')
    px(g, [(5,9),(6,10)], 'c')
    px(g, [(9,4)], 'k')
    add('Bird', g, {'a':0x3E6EDE,'b':0x2A4EB0,'c':0xE8A030,'k':0x0A1230})

# ============================================================== Mammal ==

def platypus():
    g = grid()
    ellipse(g, 6, 6.5, 4.0, 2.4, 'a')
    ellipse(g, 10, 6, 1.8, 1.1, 'c')
    px(g, [(1,7),(0,8)], 'a')
    for (lx,ly) in [(4,8),(3,9),(8,8),(9,9)]:
        set(g, lx, ly, 'a')
    px(g, [(8,5)], 'k')
    add('Platypus', g, {'a':0x7A5A34,'c':0xB98A3E,'k':0x1E1206})

def kangaroo():
    g = grid()
    ellipse(g, 6, 5.5, 2.4, 3.0, 'a')
    ellipse(g, 6, 2.6, 1.5, 1.3, 'a')
    px(g, [(5,1),(7,1)], 'a')
    px(g, [(2,8),(1,10),(0,11)], 'a')
    rect(g, 7, 7, 9, 9, 'a')
    px(g, [(9,9),(10,10),(11,10)], 'a')
    rect(g, 4, 8, 5, 10, 'a')
    px(g, [(6,2)], 'k')
    add('Kangaroo', g, {'a':0xC08A4A,'k':0x2A1A08})

def elephant():
    g = grid()
    ellipse(g, 6.5, 6, 4.4, 2.8, 'a')
    ellipse(g, 2.5, 4, 2.2, 2.4, 'b')
    px(g, [(1,3),(2,3)], 'a')
    for y in [6,7,8,9]:
        set(g, 1-0 if False else 1, y, 'a')
    px(g, [(1,6),(1,7),(2,8),(3,9)], 'a')
    for (lx,ly) in [(4,9),(4,10),(8,9),(8,10)]:
        set(g, lx, ly, 'a')
    px(g, [(0,4)], 'k')
    add('Elephant', g, {'a':0x9098A0,'b':0x7A828C,'k':0x181C1E})

def mouse():
    g = grid()
    ellipse(g, 6, 7, 3.4, 2.2, 'a')
    ellipse(g, 3, 4.5, 1.6, 1.6, 'a')
    ellipse(g, 1.6, 3, 1.1, 1.1, 'b')
    px(g, [(9,7),(10,6),(11,5)], 'a')
    px(g, [(2,4)], 'k')
    add('Mouse', g, {'a':0x9A8A78,'b':0xC9B49E,'k':0x14100C})

def bat():
    g = grid()
    ellipse(g, 6, 6.5, 1.2, 1.9, 'a')
    # Wing membrane: a filled triangular fan from the shoulder out to the
    # wingtip, with notches along the trailing edge.
    wing = [(5,5),(3,3),(1,2),(0,3),(1,4),(0,5),(1,6),(0,7),(1,8),(3,8),(5,7)]
    for i in range(len(wing)-1):
        x0,y0 = wing[i]; x1,y1 = wing[i+1]
        steps = max(abs(x1-x0),abs(y1-y0),1)
        for s_ in range(steps+1):
            t=s_/steps
            set(g, round(x0+(x1-x0)*t), round(y0+(y1-y0)*t), 'a')
    rect(g, 1, 4, 4, 7, 'a')
    px(g, [(5,3),(6,2),(7,3)], 'a')
    mirror_right(g, 6)
    px(g, [(5,5),(7,5)], 'k')
    add('Bat', g, {'a':0x3A2E44,'k':0xE04040})

def wolf():
    g = grid()
    ellipse(g, 6, 6, 3.8, 1.9, 'a')
    ellipse(g, 9.5, 4.6, 1.8, 1.5, 'a')
    px(g, [(10,3),(11,2)], 'a')
    px(g, [(1,4),(0,3),(2,7)], 'a')
    for (lx,ly) in [(4,7),(3,9),(7,7),(8,9)]:
        set(g, lx, ly, 'a')
    px(g, [(10,4)], 'k')
    add('Wolf', g, {'a':0x8A8E94,'k':0x14161A})

def whale():
    g = grid()
    ellipse(g, 5.8, 6, 4.6, 2.3, 'a')
    px(g, [(0,5),(0,7),(1,6)], 'b')
    px(g, [(9,4),(8,3)], 'b')
    px(g, [(8,5)], 'k')
    add('Whale', g, {'a':0x3E6E8E,'b':0x2E5470,'k':0x0A1620})

def lemur():
    g = grid()
    ellipse(g, 6, 5.5, 2.0, 2.8, 'a')
    ellipse(g, 6, 2.6, 1.7, 1.5, 'c')
    px(g, [(4,1),(8,1)], 'a')
    px(g, [(8,6),(9,4),(10,3),(11,4),(10,6),(9,7)], 'b')
    for (lx,ly) in [(4,8),(4,10),(7,8),(7,10)]:
        set(g, lx, ly, 'a')
    px(g, [(5,2),(7,2)], 'k')
    add('Lemur', g, {'a':0x8A8478,'c':0xE8E0D0,'b':0x5A5448,'k':0x0E0E0A})

def ape():
    g = grid()
    ellipse(g, 6, 6.5, 2.6, 3.0, 'a')
    ellipse(g, 6, 2.8, 1.9, 1.7, 'a')
    px(g, [(2,5),(1,7)], 'a'); px(g, [(10,5),(11,7)], 'a')
    for (lx,ly) in [(4,9),(4,11),(8,9),(8,11)]:
        set(g, lx, ly, 'a')
    px(g, [(5,2),(7,2)], 'k')
    add('Ape', g, {'a':0x4A3A2E,'k':0x0A0806})

def chimpanzee():
    g = grid()
    ellipse(g, 6, 6.5, 2.2, 2.8, 'a')
    ellipse(g, 6, 2.8, 1.7, 1.6, 'c')
    px(g, [(3,5),(2,7)], 'a'); px(g, [(9,5),(10,7)], 'a')
    for (lx,ly) in [(4,9),(4,11),(8,9),(8,11)]:
        set(g, lx, ly, 'a')
    px(g, [(5,2),(7,2)], 'k')
    add('Chimpanzee', g, {'a':0x6E4E30,'c':0xD9B98A,'k':0x0E0A04})

def human():
    g = grid()
    ellipse(g, 6, 6.5, 1.9, 2.8, 'c')
    ellipse(g, 6, 2.6, 1.5, 1.5, 'b')
    px(g, [(3,5),(2,7)], 'c'); px(g, [(9,5),(10,7)], 'c')
    for (lx,ly) in [(5,9),(5,11),(7,9),(7,11)]:
        set(g, lx, ly, 'a')
    px(g, [(5,2),(7,2)], 'k')
    add('Human', g, {'a':0x3E4E6E,'b':0xC9905E,'c':0xD9A470,'k':0x140E08})

for fn in [sea_sponge, comb_jelly, placozoan, jellyfish, coral, acoel_worm,
           flatworm, snail, octopus, segmented_worm,
           nematode, horseshoe_crab, scorpion, spider, crab, dragonfly, beetle, butterfly, ant,
           starfish, sea_urchin, sea_squirt,
           lamprey, shark, ray_finned_fish, coelacanth,
           amphibian, frog,
           turtle, lizard, crocodile, dinosaur, bird,
           platypus, kangaroo, elephant, mouse, bat, wolf, whale, lemur, ape, chimpanzee, human]:
    fn()

print(f"{len(ANIMALS)} animals defined")
