"""Generate the sing-box GUI mark: an isometric box split by a Y-shaped gap.

The three faces of the cube are separate rounded rhombi; the gap between
them reads as one route branching in two — a box that routes traffic.
An amber hub sits where the three routes meet.

    python scripts/gen_logo.py assets/logo.svg
    python scripts/gen_logo.py assets/logo-small.svg small

Rasterise with any SVG renderer (resvg, Inkscape): app-icon.png is logo.svg
at 512px; tray-icon.png is logo-small.svg at 64px; icon.ico holds 16-24px
from logo-small.svg and 32-256px from logo.svg.
"""
import math
import sys

# `small` is the tray/16-24px cut: bigger mark, wider gap, no hub — at that
# size the hub collapses into a smudge.
SMALL = len(sys.argv) > 2 and sys.argv[2] == "small"

CX, CY = 256.0, 256.0
R = 176.0 if SMALL else 150.0  # hexagon circumradius
GAP = 34.0 if SMALL else 18.0  # visual gap between faces
CORNER = 16.0  # rhombus corner radius
DOT = 21.0  # amber hub where the three routes meet
HOLE = DOT + GAP * 0.6
INSET = 0 if SMALL else 16  # tile margin inside the canvas


def hex_pt(deg):
    a = math.radians(deg)
    return (CX + R * math.cos(a), CY + R * math.sin(a))


T = hex_pt(-90)
UR = hex_pt(-30)
LR = hex_pt(30)
B = hex_pt(90)
LL = hex_pt(150)
UL = hex_pt(210)
C = (CX, CY)

faces = {
    "top": [T, UR, C, UL],
    "left": [UL, C, B, LL],
    "right": [C, UR, LR, B],
}


def shrink(poly, d):
    # Rhombi are tangential: every edge sits the same distance from the
    # centroid, so a uniform scale opens an even gap on all inner edges.
    cx = sum(p[0] for p in poly) / len(poly)
    cy = sum(p[1] for p in poly) / len(poly)
    side = math.dist(poly[0], poly[1])
    inr = side * math.sin(math.radians(60)) / 2.0
    s = 1.0 - d / inr
    return [(cx + (x - cx) * s, cy + (y - cy) * s) for x, y in poly]


def rounded(poly, r):
    n = len(poly)
    out = []
    for i in range(n):
        p, v, q = poly[i - 1], poly[i], poly[(i + 1) % n]

        def toward(a, b, dist):
            L = math.dist(a, b)
            return (a[0] + (b[0] - a[0]) * dist / L, a[1] + (b[1] - a[1]) * dist / L)

        a = toward(v, p, r)
        b = toward(v, q, r)
        out.append((a, v, b))
    d = f"M{out[0][2][0]:.2f},{out[0][2][1]:.2f}"
    for i in range(1, n + 1):
        a, v, b = out[i % n]
        d += f" L{a[0]:.2f},{a[1]:.2f} Q{v[0]:.2f},{v[1]:.2f} {b[0]:.2f},{b[1]:.2f}"
    return d + " Z"


paths = {k: rounded(shrink(v, GAP / 2.0), CORNER) for k, v in faces.items()}
TILE_RX = 112 if SMALL else 108
HUB_MASK = "" if SMALL else ' mask="url(#hub)"'
HUB = "" if SMALL else f'  <circle cx="{CX:.0f}" cy="{CY:.0f}" r="{DOT:.1f}" fill="url(#core)"/>\n'

svg = f"""<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
  <title>sing-box GUI</title>
  <defs>
    <linearGradient id="tile" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#1E4D43"/>
      <stop offset="1" stop-color="#0B1C18"/>
    </linearGradient>
    <linearGradient id="rim" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#FFFFFF" stop-opacity="0.16"/>
      <stop offset="0.5" stop-color="#FFFFFF" stop-opacity="0.03"/>
      <stop offset="1" stop-color="#FFFFFF" stop-opacity="0"/>
    </linearGradient>
    <radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#3FBF98" stop-opacity="0.30"/>
      <stop offset="1" stop-color="#3FBF98" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="core" cx="0.38" cy="0.32" r="0.75">
      <stop offset="0" stop-color="#FBE3A6"/>
      <stop offset="0.55" stop-color="#E2AE4F"/>
      <stop offset="1" stop-color="#B9822B"/>
    </radialGradient>
    <mask id="hub" maskUnits="userSpaceOnUse" x="0" y="0" width="512" height="512">
      <rect width="512" height="512" fill="#FFFFFF"/>
      <circle cx="{CX:.0f}" cy="{CY:.0f}" r="{HOLE:.1f}" fill="#000000"/>
    </mask>
    <linearGradient id="top" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#E6F7F0"/>
      <stop offset="1" stop-color="#B3E4D2"/>
    </linearGradient>
    <linearGradient id="left" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#6FD0AF"/>
      <stop offset="1" stop-color="#46B08E"/>
    </linearGradient>
    <linearGradient id="right" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#2B9677"/>
      <stop offset="1" stop-color="#1C735B"/>
    </linearGradient>
  </defs>
  <rect x="{INSET}" y="{INSET}" width="{512 - 2 * INSET}" height="{512 - 2 * INSET}" rx="{TILE_RX}" fill="url(#tile)"/>
  <rect x="{INSET + 1}" y="{INSET + 1}" width="{510 - 2 * INSET}" height="{510 - 2 * INSET}" rx="{TILE_RX - 1}" fill="none" stroke="url(#rim)" stroke-width="2"/>
  <ellipse cx="256" cy="268" rx="200" ry="190" fill="url(#glow)"/>
  <g{HUB_MASK}>
    <path d="{paths['left']}" fill="url(#left)"/>
    <path d="{paths['right']}" fill="url(#right)"/>
    <path d="{paths['top']}" fill="url(#top)"/>
  </g>
{HUB}</svg>
"""

with open(sys.argv[1], "w", encoding="utf-8", newline="\n") as f:
    f.write(svg)
