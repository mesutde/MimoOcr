#!/usr/bin/env python3
"""Generate synthetic TR/EN OCR test images + expected text for Phase 0."""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

# Turkish special characters are intentional — CER regressions live here.
SAMPLES: list[tuple[str, str, str]] = [
    (
        "tr_greeting",
        "Merhaba dünya\nTürkçe OCR testi\nÇĞİÖŞÜ çğıöşü",
        "dark",
    ),
    (
        "tr_invoice",
        "FATURA NO: TR-2026-00123\nTutar: 1.234,56 TL\nTarih: 18.09.2026\nVergi No: 1234567890",
        "light",
    ),
    (
        "tr_tech",
        "işletim sistemi: Windows\nışık sensörü arızalı\ngüncelleme gerekli",
        "dark",
    ),
    (
        "en_tech",
        "Error: connection refused\nlocalhost:8080\nuser@example.com\nhttps://github.com/mimo-ocr",
        "dark",
    ),
    (
        "tr_en_mixed",
        "Mimo OCR local engine\nTürkçe + English karışık satır\nconfidence: 0.92 / güven skoru",
        "light",
    ),
    (
        "tr_numbers",
        "IBAN: TR33 0006 1005 1978 6457 8413 26\nTel: +90 532 123 45 67\nSipariş: A-998877",
        "light",
    ),
    (
        "en_code",
    "def recognize(image):\n    return engine.run(image)\n# pipeline ready",
        "dark",
    ),
    (
        "tr_ui_dark",
        "Ayarlar\nMotor: Otomatik\nDil: Türkçe\nPanoya kopyala: Açık",
        "dark",
    ),
    (
        "tr_gazete",
        "İstanbul'da yeni bir dönem başlıyor.\nÖğrenciler için yerel OCR araçları önem kazanıyor.",
        "light",
    ),
    (
        "en_small",
        "The quick brown fox jumps over the lazy dog.\nPack my box with five dozen liquor jugs.",
        "light",
    ),
]


def pick_font(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = [
        "C:/Windows/Fonts/segoeui.ttf",
        "C:/Windows/Fonts/arial.ttf",
        "C:/Windows/Fonts/tahoma.ttf",
        "C:/Windows/Fonts/calibri.ttf",
        "C:/Windows/Fonts/consola.ttf",
    ]
    for path in candidates:
        if Path(path).exists():
            try:
                return ImageFont.truetype(path, size=size)
            except OSError:
                continue
    return ImageFont.load_default()


def render(text: str, theme: str, scale: int = 2) -> Image.Image:
    font = pick_font(22 * scale)
    pad = 24 * scale
    line_h = 34 * scale
    lines = text.splitlines() or [text]
    # Measure
    tmp = Image.new("RGB", (8, 8))
    draw = ImageDraw.Draw(tmp)
    max_w = 0
    for line in lines:
        bbox = draw.textbbox((0, 0), line, font=font)
        max_w = max(max_w, bbox[2] - bbox[0])
    width = max_w + pad * 2
    height = line_h * len(lines) + pad * 2

    if theme == "dark":
        bg = (18, 20, 26)
        fg = (232, 234, 239)
    else:
        bg = (248, 249, 251)
        fg = (20, 24, 32)

    img = Image.new("RGB", (width, height), bg)
    draw = ImageDraw.Draw(img)
    y = pad
    for line in lines:
        draw.text((pad, y), line, font=font, fill=fg)
        y += line_h
    return img


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--out",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "datasets" / "tr-en" / "synthetic",
    )
    parser.add_argument("--repeat", type=int, default=3, help="variants per sample (noise-free clones for timing)")
    args = parser.parse_args()
    out: Path = args.out
    out.mkdir(parents=True, exist_ok=True)
    expected = out.parent / "expected"
    expected.mkdir(parents=True, exist_ok=True)

    count = 0
    for name, text, theme in SAMPLES:
        img = render(text, theme)
        for i in range(max(1, args.repeat)):
            suffix = "" if i == 0 else f"_v{i}"
            stem = f"{name}{suffix}"
            png = out / f"{stem}.png"
            img.save(png)
            (expected / f"{stem}.txt").write_text(text + "\n", encoding="utf-8")
            # CLI bench looks for sidecar .txt next to image
            (out / f"{stem}.txt").write_text(text + "\n", encoding="utf-8")
            count += 1
    print(f"wrote {count} images to {out}")
    print(f"expected texts in {expected}")


if __name__ == "__main__":
    main()
