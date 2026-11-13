#!/usr/bin/env python3
"""Generate src-tauri/icons/icon.png (512x512) — a simple "M" mark.

The PNG is generated (not committed) so the repository stays text-only
and anyone can audit/replace the branding. Run once before
`npx tauri icon` / `npx tauri build`:

    pip install pillow
    python3 scripts/make_icon.py
"""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "icons" / "icon.png"


def load_font(size: int):
    candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "C:\\Windows\\Fonts\\segoeuib.ttf",
        "C:\\Windows\\Fonts\\arialbd.ttf",
    ]
    for c in candidates:
        try:
            return ImageFont.truetype(c, size)
        except OSError:
            continue
    return ImageFont.load_default()


def main() -> None:
    img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle(
        [16, 16, 496, 496], radius=96,
        fill=(16, 20, 24, 255), outline=(63, 185, 107, 255), width=10,
    )
    d.text((256, 268), "M", font=load_font(280), anchor="mm",
           fill=(232, 236, 239, 255))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    img.save(OUT)
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
