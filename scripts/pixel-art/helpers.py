"""Tiny pixel-grid authoring helpers. 12x12 canvas, '.' = transparent."""
N = 12

def grid():
    return [['.' for _ in range(N)] for _ in range(N)]

def set(g, x, y, ch):
    if 0 <= x < N and 0 <= y < N:
        g[y][x] = ch

def hline(g, x0, x1, y, ch):
    for x in range(min(x0,x1), max(x0,x1)+1):
        set(g, x, y, ch)

def vline(g, y0, y1, x, ch):
    for y in range(min(y0,y1), max(y0,y1)+1):
        set(g, x, y, ch)

def rect(g, x0, y0, x1, y1, ch):
    for y in range(min(y0,y1), max(y0,y1)+1):
        for x in range(min(x0,x1), max(x0,x1)+1):
            set(g, x, y, ch)

def ellipse(g, cx, cy, rx, ry, ch):
    for y in range(N):
        for x in range(N):
            dx = (x - cx) / rx
            dy = (y - cy) / ry
            if dx*dx + dy*dy <= 1.0:
                set(g, x, y, ch)

def ellipse_ring(g, cx, cy, rx, ry, ch, thickness=1.0):
    for y in range(N):
        for x in range(N):
            dx = (x - cx) / rx
            dy = (y - cy) / ry
            d = dx*dx + dy*dy
            if 1.0 - (thickness/max(rx,ry))*2 <= d <= 1.0:
                set(g, x, y, ch)

def px(g, pts, ch):
    for (x, y) in pts:
        set(g, x, y, ch)

def mirror_right(g, axis):
    """Mirrors columns [0, axis) onto columns (axis, N) around a centre axis column."""
    for y in range(N):
        for x in range(0, axis):
            src = g[y][x]
            dstx = 2*axis - x
            if src != '.' and 0 <= dstx < N and g[y][dstx] == '.':
                g[y][dstx] = src

def rows(g):
    return [''.join(row) for row in g]

def hexmul(hexcolor, factor):
    r = (hexcolor >> 16) & 0xFF
    gg = (hexcolor >> 8) & 0xFF
    b = hexcolor & 0xFF
    r = max(0, min(255, int(r*factor)))
    gg = max(0, min(255, int(gg*factor)))
    b = max(0, min(255, int(b*factor)))
    return (r << 16) | (gg << 8) | b
