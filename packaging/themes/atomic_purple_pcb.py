# Regenerate assets/themes/circuit.png: python3 packaging/themes/atomic_purple_pcb.py assets/themes/circuit.png  (needs Pillow)
"""Seamless PCB tile for the Atomic Purple theme: board, traces, chips,
passives, button contacts, screws and copper flex ribbon cables."""
import math, random, sys
from PIL import Image, ImageDraw, ImageFilter, ImageFont

W, H = 960, 640
S = 2  # supersample
random.seed(64)
img = Image.new("RGBA", (W * S, H * S), (18, 58, 44, 255))
d = ImageDraw.Draw(img, "RGBA")

def wrap(fn):
    """Draw fn at the 9 tile offsets so the result tiles seamlessly."""
    for ox in (-W, 0, W):
        for oy in (-H, 0, H):
            fn(ox * S, oy * S)

def P(x, y, ox, oy):
    return (x * S + ox, y * S + oy)

def rect(x0, y0, x1, y1, fill, outline=None, w=1, r=0):
    def f(ox, oy):
        box = [P(x0, y0, ox, oy), P(x1, y1, ox, oy)]
        if r:
            d.rounded_rectangle(box, r * S, fill=fill, outline=outline, width=w * S)
        else:
            d.rectangle(box, fill=fill, outline=outline, width=w * S)
    wrap(f)

def ellipse(cx, cy, rx, ry, fill=None, outline=None, w=1):
    wrap(lambda ox, oy: d.ellipse([P(cx - rx, cy - ry, ox, oy), P(cx + rx, cy + ry, ox, oy)],
                                  fill=fill, outline=outline, width=w * S))

def line(pts, fill, w):
    wrap(lambda ox, oy: d.line([P(x, y, ox, oy) for x, y in pts], fill=fill, width=int(w * S), joint="curve"))

def poly(pts, fill):
    wrap(lambda ox, oy: d.polygon([P(x, y, ox, oy) for x, y in pts], fill=fill))

# --- solder mask texture
noise = Image.effect_noise((W * S, H * S), 18).convert("L")
tex = Image.merge("RGBA", (noise.point(lambda v: 18 + v // 14), noise.point(lambda v: 56 + v // 10),
                           noise.point(lambda v: 42 + v // 12), Image.new("L", noise.size, 255)))
img.alpha_composite(tex)
d = ImageDraw.Draw(img, "RGBA")

# --- background trace network (45-degree routing, low contrast)
TRACE = (40, 112, 78, 255)
for _ in range(70):
    x, y = random.randrange(W), random.randrange(H)
    pts = [(x, y)]
    for _ in range(random.randint(2, 5)):
        L = random.randint(30, 160)
        dx, dy = random.choice([(1, 0), (0, 1), (1, 1), (1, -1), (-1, 0), (0, -1)])
        x, y = x + dx * L, y + dy * L
        pts.append((x, y))
    n = random.choice([1, 1, 2, 3])
    for k in range(n):
        off = k * 4
        line([(px + off, py + off) for px, py in pts], TRACE, 1.6)
    for px, py in (pts[0], pts[-1]):
        ellipse(px, py, 2.6, 2.6, fill=(196, 160, 72, 255), outline=(30, 70, 50, 255))

# vias scattered
for _ in range(260):
    x, y = random.randrange(W), random.randrange(H)
    ellipse(x, y, 1.8, 1.8, fill=(176, 146, 70, 255))
    ellipse(x, y, 0.7, 0.7, fill=(20, 40, 30, 255))

# --- silkscreen labels
try:
    font = ImageFont.load_default(size=9 * S)
except TypeError:
    font = ImageFont.load_default()
def label(x, y, text):
    wrap(lambda ox, oy: d.text(P(x, y, ox, oy), text, fill=(214, 222, 210, 200), font=font))

# --- passives: little SMD resistors/caps in clusters
def passive(x, y, vertical=False):
    w, h = (5, 10) if vertical else (10, 5)
    body = random.choice([(32, 30, 28, 255), (150, 120, 86, 255), (120, 96, 70, 255)])
    rect(x, y, x + w, y + h, body)
    if vertical:
        rect(x, y, x + w, y + 2, (200, 200, 196, 255)); rect(x, y + h - 2, x + w, y + h, (200, 200, 196, 255))
    else:
        rect(x, y, x + 2, y + h, (200, 200, 196, 255)); rect(x + w - 2, y, x + w, y + h, (200, 200, 196, 255))

for _ in range(26):
    cx, cy = random.randrange(W), random.randrange(H)
    for _ in range(random.randint(4, 10)):
        passive(cx + random.randint(-30, 30), cy + random.randint(-20, 20), random.random() < 0.4)
    label(cx + 34, cy - 6, random.choice(["C", "R", "L", "D"]) + str(random.randint(1, 99)))

# --- chips
def qfp(x, y, s, name):
    pin = (196, 198, 200, 255)
    for i in range(int(s / 6)):
        p = x + 5 + i * 6
        rect(p, y - 5, p + 2, y, pin); rect(p, y + s, p + 2, y + s + 5, pin)
        q = y + 5 + i * 6
        rect(x - 5, q, x, q + 2, pin); rect(x + s, q, x + s + 5, q + 2, pin)
    rect(x, y, x + s, y + s, (26, 26, 30, 255), (52, 52, 58, 255))
    ellipse(x + 6, y + 6, 2, 2, fill=(60, 60, 66, 255))
    label(x + 4, y + s + 8, name)

def soc(x, y, s):
    rect(x - 6, y - 6, x + s + 6, y + s + 6, (34, 70, 56, 255))
    rect(x, y, x + s, y + s, (150, 152, 158, 255), (110, 112, 118, 255), r=4)
    rect(x + 6, y + 6, x + s - 6, y + s - 6, (172, 174, 180, 255), r=3)
    line([(x + 10, y + s - 14), (x + s - 10, y + s - 14)], (120, 122, 128, 255), 1)
    label(x, y + s + 10, "U1")
    for i in range(18):  # decoupling caps around
        passive(x - 16 + (i % 9) * ((s + 22) / 8), y - 18 if i < 9 else y + s + 20, False)

def ram(x, y, w, h, name):
    rect(x, y, x + w, y + h, (22, 22, 26, 255), (44, 44, 50, 255))
    label(x + 6, y + h / 2 - 6, name)

soc(150, 150, 86)
ram(280, 150, 64, 40, "LPDDR")
ram(280, 200, 64, 40, "LPDDR")
qfp(560, 380, 48, "U7")
qfp(760, 120, 36, "U4")
ram(620, 120, 70, 30, "eMMC")
qfp(90, 430, 30, "U9")

# crystal and electrolytic caps
def crystal(x, y):
    rect(x, y, x + 26, y + 12, (196, 198, 202, 255), (150, 150, 156, 255), r=6)
    label(x, y + 15, "Y1")
def can(x, y, r):
    ellipse(x, y, r, r, fill=(176, 178, 184, 255), outline=(120, 120, 126, 255), w=2)
    line([(x - r * .6, y), (x + r * .6, y)], (110, 110, 116, 255), 2)
    line([(x, y - r * .6), (x, y + r * .6)], (110, 110, 116, 255), 2)
    wrap(lambda ox, oy: d.pieslice([P(x - r, y - r, ox, oy), P(x + r, y + r, ox, oy)], 200, 250, fill=(40, 40, 46, 255)))
crystal(400, 300); crystal(700, 520)
can(470, 90, 13); can(860, 300, 11); can(60, 300, 12)

# button contact pads (GBC style: concentric rings with comb)
def button_pad(cx, cy, r):
    ellipse(cx, cy, r + 3, r + 3, fill=(30, 74, 54, 255))
    ellipse(cx, cy, r, r, fill=(200, 168, 84, 255))
    ellipse(cx, cy, r - 3, r - 3, fill=(24, 60, 44, 255))
    for k in range(-int(r) + 5, int(r) - 4, 4):
        half = math.sqrt(max(0, (r - 5) ** 2 - k ** 2))
        line([(cx - half, cy + k), (cx + half, cy + k)], (200, 168, 84, 255) if (k // 4) % 2 else (24, 60, 44, 255), 1.4)

def dpad(cx, cy):
    for dx, dy in ((0, -20), (0, 20), (-20, 0), (20, 0)):
        button_pad(cx + dx, cy + dy, 9)
    label(cx - 14, cy + 34, "DPAD")
dpad(80, 110)
button_pad(860, 470, 13); button_pad(900, 440, 13)
label(846, 492, "A    B")
button_pad(500, 560, 7); button_pad(530, 560, 7)
label(490, 574, "SEL  STA")

# screws / standoffs
def screw(cx, cy):
    ellipse(cx, cy, 12, 12, fill=(36, 80, 60, 255), outline=(196, 160, 72, 255), w=2)
    ellipse(cx, cy, 7, 7, fill=(158, 160, 166, 255), outline=(110, 110, 116, 255))
    line([(cx - 4, cy), (cx + 4, cy)], (90, 90, 96, 255), 1.6)
    line([(cx, cy - 4), (cx, cy + 4)], (90, 90, 96, 255), 1.6)
for x, y in ((20, 20), (480, 20), (20, 330), (930, 600), (640, 300)):
    screw(x, y)

# battery contacts
rect(380, 470, 420, 486, (200, 168, 84, 255), r=2); rect(380, 494, 420, 510, (200, 168, 84, 255), r=2)
label(426, 476, "BAT+"); label(426, 498, "BAT-")

# --- copper flex ribbon cables (translucent amber, fine traces, ZIF ends)
def flex(path, width, n_traces):
    amber = (204, 118, 36, 215)
    def strip(ox, oy):
        pts = [P(x, y, ox, oy) for x, y in path]
        d.line(pts, fill=amber, width=int(width * S), joint="curve")
    wrap(strip)
    # parallel traces: offset path along normals per segment
    for k in range(n_traces):
        t = (k + 0.5) / n_traces - 0.5
        off = t * (width - 4)
        pts = []
        for i, (x, y) in enumerate(path):
            if i == 0:
                nx, ny = path[1][0] - x, path[1][1] - y
            elif i == len(path) - 1:
                nx, ny = x - path[i - 1][0], y - path[i - 1][1]
            else:
                nx, ny = path[i + 1][0] - path[i - 1][0], path[i + 1][1] - path[i - 1][1]
            L = math.hypot(nx, ny) or 1
            pts.append((x - ny / L * off, y + nx / L * off))
        line(pts, (240, 170, 80, 200), 0.9)
    # sheen along one edge
    line([(x + 1, y - width / 2 + 2) for x, y in path], (255, 214, 150, 70), 1.2)
    for (x, y), (x2, y2) in ((path[0], path[1]), (path[-1], path[-2])):
        ang = math.atan2(y2 - y, x2 - x)
        cx, cy = x - math.cos(ang) * 8, y - math.sin(ang) * 8
        hw, hl = width / 2 + 6, 9
        c, s_ = math.cos(ang), math.sin(ang)
        corners = [(cx + c * a - s_ * b, cy + s_ * a + c * b) for a, b in ((-hl, -hw), (hl, -hw), (hl, hw), (-hl, hw))]
        poly(corners, (30, 30, 32, 255))
        inner = [(cx + c * a - s_ * b, cy + s_ * a + c * b) for a, b in ((-hl + 3, -hw + 3), (hl - 3, -hw + 3), (hl - 3, hw - 3), (-hl + 3, hw - 3))]
        poly(inner, (214, 196, 150, 255))
        for k in range(n_traces):
            b = -hw + 5 + k * ((2 * hw - 10) / max(1, n_traces - 1))
            line([(cx + c * -4 - s_ * b, cy + s_ * -4 + c * b), (cx + c * 4 - s_ * b, cy + s_ * 4 + c * b)], (214, 172, 70, 255), 1.4)

flex([(370, 260), (450, 260), (520, 210), (620, 210), (700, 250), (790, 250)], 26, 14)
flex([(240, 560), (240, 480), (300, 420), (330, 360)], 18, 9)
flex([(900, 60), (900, 160), (940, 200), (1010, 200)], 20, 10)  # wraps across the edge
flex([(600, 470), (660, 600), (660, 700)], 14, 7)

out = img.resize((W, H), Image.LANCZOS).convert("RGB")
out.save(sys.argv[1], optimize=True)
