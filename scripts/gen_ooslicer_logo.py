#!/usr/bin/env python3
"""
Generate the "OO" monogram (OOSlicer / OptimusOrca by Maison Drabiec) and write
it over the slicer's existing icon files IN PLACE — the filenames are referenced
by the C++/build, so we keep the names and only swap the pixels.

Palette: cyan→indigo gradient + white "OO" (deliberately distinct from the
amber MD optimiser logo). Run: python scripts/gen_ooslicer_logo.py
"""
from __future__ import annotations

from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

REPO = Path(__file__).resolve().parents[1]
IMG = REPO / "resources" / "images"

S = 1024
RADIUS = int(S * 0.22)
TOP = (92, 224, 234, 255)     # #5CE0EA cyan
BOT = (36, 86, 200, 255)      # #2456C8 indigo
FG = (255, 255, 255, 255)
KEYLINE = (255, 255, 255, 46)

FONT_CANDIDATES = [
    r"C:\Windows\Fonts\seguibl.ttf",
    r"C:\Windows\Fonts\bahnschrift.ttf",
    r"C:\Windows\Fonts\arialbd.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
]

def font(px: int) -> ImageFont.FreeTypeFont:
    for p in FONT_CANDIDATES:
        if Path(p).exists():
            return ImageFont.truetype(p, px)
    return ImageFont.load_default()

def gradient(size: int) -> Image.Image:
    g = Image.new("RGBA", (1, size))
    for y in range(size):
        t = y / (size - 1)
        g.putpixel((0, y), tuple(int(TOP[i] + (BOT[i] - TOP[i]) * t) for i in range(4)))
    return g.resize((size, size))

def master() -> Image.Image:
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, S - 1, S - 1], radius=RADIUS, fill=255)
    icon = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    icon.paste(gradient(S), (0, 0), mask)
    d = ImageDraw.Draw(icon)
    inset = int(S * 0.045)
    d.rounded_rectangle([inset, inset, S - 1 - inset, S - 1 - inset],
                        radius=RADIUS - inset, outline=KEYLINE, width=max(3, S // 170))
    text = "OO"
    px = int(S * 0.6)
    target = int(S * 0.82)
    while px > 40:
        f = font(px)
        l, t, r, b = d.textbbox((0, 0), text, font=f)
        if (r - l) <= target:
            break
        px -= 8
    f = font(px)
    l, t, r, b = d.textbbox((0, 0), text, font=f)
    x = (S - (r - l)) / 2 - l
    y = (S - (b - t)) / 2 - t - int(S * 0.01)
    d.text((x + S // 130, y + S // 130), text, font=f, fill=(20, 30, 70, 70))  # shadow
    d.text((x, y), text, font=f, fill=FG)
    return icon

# (filename, size, grayscale?)
PNG_TARGETS = [
    ("Snapmaker_Orca.png", 256, False),
    ("Snapmaker_Orca_32px.png", 32, False),
    ("Snapmaker_Orca_64.png", 64, False),
    ("Snapmaker_Orca_128px.png", 128, False),
    ("Snapmaker_Orca_192px.png", 192, False),
    ("Snapmaker_Orca_192px_transparent.png", 192, False),
    ("Snapmaker_Orca_192px_grayscale.png", 192, True),
    ("Snapmaker_Orca-mac_128px.png", 128, False),
    ("Snapmaker_Orca_154.png", 154, False),
    ("Snapmaker_Orca_154_title.png", 154, False),
    ("Snapmaker_OrcaTitle.png", 256, False),
]
ICO_TARGETS = [
    ("Snapmaker_Orca.ico", [16, 32, 48, 64, 128, 256]),
    ("Snapmaker_Orca-mac_256px.ico", [256]),
    ("Snapmaker_OrcaTitle.ico", [16, 32, 48, 64, 128, 256]),
]

def main() -> None:
    m = master()
    written = []
    for name, sz, gray in PNG_TARGETS:
        p = IMG / name
        if not p.exists():
            continue
        im = m.resize((sz, sz), Image.LANCZOS)
        if gray:
            im = im.convert("LA").convert("RGBA")
        im.save(p)
        written.append(name)
    for name, sizes in ICO_TARGETS:
        p = IMG / name
        if not p.exists():
            continue
        m.save(p, sizes=[(s, s) for s in sizes])
        written.append(name)
    print(f"OOSlicer 'OO' monogram written over {len(written)} slicer icons:")
    print("  " + ", ".join(written))
    print("NOTE: Snapmaker_Orca.icns (macOS) not regenerated here — needs iconutil/png2icns on macOS.")

if __name__ == "__main__":
    main()
