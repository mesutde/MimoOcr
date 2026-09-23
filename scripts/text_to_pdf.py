#!/usr/bin/env python3
"""Duz metni (UDF/toplu ciktisi) duzgun Torkce fontlu PDF'e cevirir.

Kullanim:
  text_to_pdf.py --in <metin.txt> --out <cikti.pdf> [--title BASLIK]

Isaretler:
  "# "       -> baslik (Title)
  "## "      -> alt baslik (Heading 2)
  "===== x =====" -> yeni bolum (sayfa sonu + Heading 1)
  "```"      -> kod cizgisi (yoksayilir, icerik korunur)

Turkce karakterler (g, s, I, c...) icin Windows Arial TTF kullanilir;
bulunamazsa Helvetica'ya dusulur.
"""

from __future__ import annotations

import argparse
import html
import sys
from pathlib import Path

try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass


def register_fonts():
    try:
        from reportlab.pdfbase import pdfmetrics
        from reportlab.pdfbase.ttfonts import TTFont
    except ImportError:
        return ("Helvetica", "Helvetica-Bold")

    candidates = [
        Path(r"C:\Windows\Fonts\arial.ttf"),
        Path(r"C:\Windows\Fonts\Arial.ttf"),
        Path("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
        Path("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf"),
    ]
    bold_candidates = [
        Path(r"C:\Windows\Fonts\arialbd.ttf"),
        Path(r"C:\Windows\Fonts\Arial Bold.ttf"),
        Path("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"),
        Path("/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf"),
    ]
    reg = next((p for p in candidates if p.is_file()), None)
    bold = next((p for p in bold_candidates if p.is_file()), None)
    if reg is None:
        return ("Helvetica", "Helvetica-Bold")
    try:
        pdfmetrics.registerFont(TTFont("MimoSans", str(reg)))
        if bold is not None:
            pdfmetrics.registerFont(TTFont("MimoSans-Bold", str(bold)))
            return ("MimoSans", "MimoSans-Bold")
        return ("MimoSans", "MimoSans")
    except Exception:
        return ("Helvetica", "Helvetica-Bold")


def esc(s: str) -> str:
    return html.escape(s, quote=False)


def build_pdf(text: str, out: Path, title: str, no_title: bool = False) -> None:
    from reportlab.lib.pagesizes import A4
    from reportlab.lib.styles import ParagraphStyle
    from reportlab.lib.units import mm
    from reportlab.platypus import (
        PageBreak,
        Paragraph,
        SimpleDocTemplate,
        Spacer,
    )

    font, font_bold = register_fonts()

    st_title = ParagraphStyle("Title2", fontName=font_bold, fontSize=18, leading=22, spaceAfter=6 * mm)
    st_h1 = ParagraphStyle("H1", fontName=font_bold, fontSize=15, leading=19, spaceBefore=4 * mm, spaceAfter=3 * mm)
    st_h2 = ParagraphStyle("H2", fontName=font_bold, fontSize=13, leading=16, spaceBefore=3 * mm, spaceAfter=2 * mm)
    st_p = ParagraphStyle("P", fontName=font, fontSize=10.5, leading=15, spaceAfter=2 * mm)

    doc = SimpleDocTemplate(
        str(out),
        pagesize=A4,
        leftMargin=18 * mm,
        rightMargin=18 * mm,
        topMargin=16 * mm,
        bottomMargin=16 * mm,
        title=title,
        author="Mimo OCR",
    )
    story = []
    if not no_title:
        story.append(Paragraph(esc(title), st_title))

    in_code = False
    para: list[str] = []

    def flush_para() -> None:
        if para:
            story.append(Paragraph("<br/>".join(esc(x) for x in para), st_p))
            para.clear()

    first_section = True
    for raw in text.splitlines():
        line = raw.rstrip()
        s = line.strip()
        if s == "```":
            in_code = not in_code
            continue
        if not s:
            flush_para()
            continue
        if s.startswith("===== ") and s.endswith(" ====="):
            flush_para()
            name = s[len("===== "):-len(" =====")].strip() or "Belge"
            if not first_section:
                story.append(PageBreak())
            first_section = False
            story.append(Paragraph(esc(name), st_h1))
            continue
        if s.startswith("## "):
            flush_para()
            story.append(Paragraph(esc(s[3:].strip()), st_h2))
            continue
        if s.startswith("# "):
            flush_para()
            story.append(Paragraph(esc(s[2:].strip()), st_h1))
            continue
        para.append(line)
    flush_para()

    doc.build(story)


def main() -> int:
    ap = argparse.ArgumentParser(description="Metin → PDF (Mimo OCR)")
    ap.add_argument("--in", dest="inp", required=True)
    ap.add_argument("--out", dest="out", required=True)
    ap.add_argument("--title", default="Mimo OCR çıktısı")
    ap.add_argument("--no-title", action="store_true", help="Baslik paragrafini yazma")
    args = ap.parse_args()

    inp = Path(args.inp)
    if not inp.is_file():
        print(f"girdi yok: {inp}", file=sys.stderr)
        return 1
    try:
        text = inp.read_bytes().decode("utf-8-sig")
    except UnicodeDecodeError:
        text = inp.read_bytes().decode("cp1254", errors="replace")
    if not text.strip():
        text = "[boş çıktı]"
    try:
        build_pdf(text, Path(args.out), args.title, args.no_title)
    except ImportError:
        print("reportlab yok: MiMo Python veya 'pip install reportlab' gerekli", file=sys.stderr)
        return 2
    except Exception as e:
        print(f"pdf hatası: {e}", file=sys.stderr)
        return 3
    print(f"[pdf] -> {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
