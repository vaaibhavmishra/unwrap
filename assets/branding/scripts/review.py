#!/usr/bin/env python3
"""Composite actual exports for offline inspection, without redrawing any artwork."""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
canvas = Image.new("RGB", (1600, 1360), "#0B0D0E")
draw = ImageDraw.Draw(canvas)
font_path = str(ROOT / "source/fonts/Inter-Regular.ttf")
small = ImageFont.truetype(font_path, 18)
heading = ImageFont.truetype(font_path, 30)


def paste(name, x, y, width=None):
    with Image.open(ROOT / name) as image:
        if width:
            image = image.resize((width, round(image.height * width / image.width)), Image.Resampling.LANCZOS)
        canvas.paste(image, (x, y), image)


draw.text((60, 40), "UNWRAP / V7 GLOW", fill="#A7F3D0", font=small)
draw.text((60, 75), "One open form. Every expression.", fill="#F5F5F5", font=heading)
draw.rectangle((800, 140, 1540, 450), fill="#F5F5F5")
paste("unwrap-logo-lockup-dark.png", 65, 180, 670)
paste("unwrap-logo-lockup-light.png", 825, 180, 670)
variants = ["glow", "dark", "light", "flat", "mono", "outline"]
for index, variant in enumerate(variants):
    x = 40 + index * 255
    paste(f"unwrap-app-icon-{variant}.png", x, 475, 235)
    draw.text((x + 80, 727), variant, fill="#8A8F98", font=small)
draw.line((60, 785, 1540, 785), fill="#29302D")
draw.text((60, 817), "ACTUAL PIXELS", fill="#A7F3D0", font=small)
draw.rectangle((800, 870, 1540, 1070), fill="#F5F5F5")
for origin in [70, 850]:
    x = origin
    for size in [128, 64, 32, 16]:
        paste(f"unwrap-icon-{size}.png", x, 1030 - size)
        draw.text((x, 1040), str(size), fill="#8A8F98", font=small)
        x += size + 45
draw.text((60, 1145), "16 PX / 8× NEAREST-NEIGHBOR INSPECTION", fill="#A7F3D0", font=small)
for origin, color in [(930, "#0B0D0E"), (1150, "#F5F5F5")]:
    draw.rectangle((origin, 1120, origin + 180, 1300), fill=color)
    with Image.open(ROOT / "favicon-16.png") as image:
        image = image.resize((128, 128), Image.Resampling.NEAREST)
        canvas.paste(image, (origin + 26, 1146), image)
draw.text((60, 1190), "Open center. Continuous bowl. No glow at favicon scale.", fill="#8A8F98", font=small)
canvas.save(ROOT / "review.jpg", quality=95, subsampling=0)
print(ROOT / "review.jpg")
