"""Renders the DMG window background (640x400 @1x and @2x).

Usage: python make_background.py <Inter-Medium.ttf> <out_dir>
"""
import sys
from PIL import Image, ImageDraw, ImageFont

W, H = 640, 400
BG = (0x10, 0x12, 0x16)
TEXT = (0xEE, 0xF1, 0xF5)
MUTED = (0x99, 0xA1, 0xAD)
BRAND = (0x5C, 0xFF, 0x3A)


def render(scale):
    img = Image.new("RGB", (W * scale, H * scale), BG)
    d = ImageDraw.Draw(img)
    s = lambda v: int(v * scale)
    title = ImageFont.truetype(sys.argv[1], s(18))
    small = ImageFont.truetype(sys.argv[1], s(13))
    d.text((s(W / 2), s(52)), "Przeciągnij BibiCord do folderu Aplikacje", font=title, fill=TEXT, anchor="mm")
    # Arrow between the two icons (icons sit at x=170 and x=470, y=200).
    y = s(200)
    d.line((s(262), y, s(372), y), fill=BRAND, width=s(4))
    d.polygon([(s(384), y), (s(368), y - s(10)), (s(368), y + s(10))], fill=BRAND)
    d.text((s(W / 2), s(338)), "Przy pierwszym uruchomieniu: Ustawienia systemowe →", font=small, fill=MUTED, anchor="mm")
    d.text((s(W / 2), s(358)), "Prywatność i ochrona → „Otwórz mimo to”.", font=small, fill=MUTED, anchor="mm")
    return img


out = sys.argv[2]
render(1).save(f"{out}/dmg-background.png")
render(2).save(f"{out}/dmg-background@2x.png")
