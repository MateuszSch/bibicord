"""Renders the BibiCord app icon: logo-mark.png (the "b" on a chat bubble)
centred on a dark rounded tile.

Usage: python make_icon.py <out_dir>
Writes icon-1024.png and BibiCord.icns (macOS layout, with margin) plus
icon.ico and icon-256.png (Windows and the window icon, full bleed).
"""
import os
import sys
from PIL import Image, ImageDraw, ImageFilter

MARK = Image.open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "logo-mark.png")).convert("RGBA")
# Tile: bg_base from DESIGN.md, a touch lighter at the top so it reads on dark docks.
TOP, BOTTOM = (0x1C, 0x1D, 0x22), (0x11, 0x12, 0x14)
MARK_FRAC = 0.66  # mark's longer side relative to the tile side


def tile(size, inset, radius_frac, shadow):
    S = size * 4  # supersample
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    box = (inset * 4, inset * 4, S - inset * 4, S - inset * 4)
    side = box[2] - box[0]
    if shadow:
        sh = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        ImageDraw.Draw(sh).rounded_rectangle(
            (box[0], box[1] + side * 0.02, box[2], box[3] + side * 0.02),
            radius=side * radius_frac, fill=(0, 0, 0, 90))
        img = Image.alpha_composite(img, sh.filter(ImageFilter.GaussianBlur(side * 0.025)))
    grad = Image.new("RGBA", (1, 256))
    for y in range(256):
        t = y / 255
        grad.putpixel((0, y), tuple(int(a + (b - a) * t) for a, b in zip(TOP, BOTTOM)) + (255,))
    grad = grad.resize((side, side))
    mask = Image.new("L", (side, side), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, side - 1, side - 1), radius=side * radius_frac, fill=255)
    img.paste(grad, (box[0], box[1]), mask)
    scale = side * MARK_FRAC / max(MARK.size)
    mark = MARK.resize((round(MARK.width * scale), round(MARK.height * scale)), Image.LANCZOS)
    img.alpha_composite(mark, (S // 2 - mark.width // 2, S // 2 - mark.height // 2))
    return img.resize((size, size), Image.LANCZOS)


out = sys.argv[1]
# macOS: 824px tile on a 1024 canvas (Apple grid), soft shadow.
mac = tile(1024, 100, 0.225, True)
mac.save(f"{out}/icon-1024.png")
mac.save(f"{out}/BibiCord.icns")
# Windows: full-bleed tile, all standard sizes.
ico = tile(256, 8, 0.22, False)
ico.save(f"{out}/icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
ico.save(f"{out}/icon-256.png")
