"""Renders the BibiCord app icon (brand square + white waveform bars).

Usage: python make_icon.py <out_dir>
Writes icon-1024.png (macOS layout, with margin) and icon.ico (Windows, full bleed).
"""
import sys
from PIL import Image, ImageDraw, ImageFilter

BRAND = (0x5B, 0x6C, 0xFF)
# Bar heights as a fraction of the tile side (a voice waveform).
BARS = [0.20, 0.42, 0.62, 0.36, 0.50, 0.24]


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
    # Subtle vertical gradient on the tile.
    grad = Image.new("RGBA", (1, 256))
    for y in range(256):
        t = y / 255
        top, bottom = (0x6F, 0x7E, 0xFF), (0x4C, 0x5A, 0xF0)
        grad.putpixel((0, y), tuple(int(a + (b - a) * t) for a, b in zip(top, bottom)) + (255,))
    grad = grad.resize((side, side))
    mask = Image.new("L", (side, side), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, side - 1, side - 1), radius=side * radius_frac, fill=255)
    img.paste(grad, (box[0], box[1]), mask)
    d = ImageDraw.Draw(img)
    bar_w, gap = side * 0.075, side * 0.05
    total = len(BARS) * bar_w + (len(BARS) - 1) * gap
    x = S / 2 - total / 2
    for h in BARS:
        hh = side * h
        d.rounded_rectangle((x, S / 2 - hh / 2, x + bar_w, S / 2 + hh / 2), radius=bar_w / 2, fill=(255, 255, 255, 255))
        x += bar_w + gap
    return img.resize((size, size), Image.LANCZOS)


out = sys.argv[1]
# macOS: 824px tile on a 1024 canvas (Apple grid), soft shadow.
tile(1024, 100, 0.225, True).save(f"{out}/icon-1024.png")
# Windows: full-bleed tile, all standard sizes.
ico = tile(256, 8, 0.22, False)
ico.save(f"{out}/icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
ico.save(f"{out}/icon-256.png")
