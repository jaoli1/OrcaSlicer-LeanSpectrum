#!/usr/bin/env python3
"""
Generate the "MD" monogram app icon for the Optimisateur de filament et de
profils d'impression by Maison Drabiec.

Brand palette (from site/style.css): dark #0A0C10 + amber accent #FFB454.
The icon is a rounded square with a warm amber gradient and a heavy dark "MD"
monogram. Outputs:
  * tools/sds-importer/icons-source.png        (1024, fed to `cargo tauri icon`)
  * tools/sds-importer/src-tauri/icons/*.png    (32, 128, 256, 1024)
  * tools/sds-importer/src-tauri/icons/icon.ico (multi-size)

Re-run after a brand tweak: python scripts/gen_md_logo.py
"""
from __future__ import annotations

from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

REPO = Path(__file__).resolve().parents[1]
APP = REPO / "tools" / "sds-importer"
ICONS = APP / "src-tauri" / "icons"

S = 1024                      # master canvas
RADIUS = int(S * 0.22)        # iOS-like corner radius
DARK = (10, 12, 16, 255)      # #0A0C10
AMBER_TOP = (255, 203, 122, 255)   # lighter amber
AMBER_BOT = (236, 126, 26, 255)    # deeper amber
KEYLINE = (10, 12, 16, 40)    # subtle inner keyline

FONT_CANDIDATES = [
    r"C:\Windows\Fonts\seguibl.ttf",   # Segoe UI Black
    r"C:\Windows\Fonts\bahnschrift.ttf",
    r"C:\Windows\Fonts\arialbd.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
]

def load_font(px: int) -> ImageFont.FreeTypeFont:
    for path in FONT_CANDIDATES:
        if Path(path).exists():
            return ImageFont.truetype(path, px)
    return ImageFont.load_default()

def vertical_gradient(size: int, top, bot) -> Image.Image:
    grad = Image.new("RGBA", (1, size))
    for y in range(size):
        t = y / (size - 1)
        grad.putpixel((0, y), tuple(int(top[i] + (bot[i] - top[i]) * t) for i in range(4)))
    return grad.resize((size, size))

def make_master() -> Image.Image:
    # Rounded-square mask.
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, S - 1, S - 1], radius=RADIUS, fill=255)

    base = vertical_gradient(S, AMBER_TOP, AMBER_BOT)
    icon = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    icon.paste(base, (0, 0), mask)

    draw = ImageDraw.Draw(icon)
    # Subtle inner keyline for definition.
    inset = int(S * 0.045)
    draw.rounded_rectangle(
        [inset, inset, S - 1 - inset, S - 1 - inset],
        radius=RADIUS - inset, outline=KEYLINE, width=max(3, S // 170),
    )

    # "MD" monogram — find the largest size that fits ~80% width.
    text = "MD"
    target_w = int(S * 0.80)
    px = int(S * 0.6)
    font = load_font(px)
    while px > 40:
        font = load_font(px)
        l, t, r, b = draw.textbbox((0, 0), text, font=font)
        if (r - l) <= target_w:
            break
        px -= 8
    l, t, r, b = draw.textbbox((0, 0), text, font=font)
    tw, th = r - l, b - t
    x = (S - tw) / 2 - l
    y = (S - th) / 2 - t - int(S * 0.01)   # tiny optical lift
    # Soft drop shadow then the mark.
    draw.text((x + S // 110, y + S // 110), text, font=font, fill=(10, 12, 16, 60))
    draw.text((x, y), text, font=font, fill=DARK)
    return icon

def main() -> None:
    ICONS.mkdir(parents=True, exist_ok=True)
    master = make_master()

    master.save(APP / "icons-source.png")
    master.save(ICONS / "icon.png")
    for name, sz in [("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256)]:
        master.resize((sz, sz), Image.LANCZOS).save(ICONS / name)
    master.save(ICONS / "icon.ico",
                sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    print("MD monogram written:",
          "icons-source.png + icons/{icon.png,32x32,128x128,128x128@2x,icon.ico}")

if __name__ == "__main__":
    main()
