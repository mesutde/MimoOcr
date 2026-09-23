#!/usr/bin/env python3
"""UYAP UDF dosyasini duzeni korunmus PDF'e cevirir (tek dosya).

Kullanim:
  udf_to_pdf.py --udf <dosya.udf> [--out <cikti.pdf>] [--title BASLIK]

Yapi analizi:
  Gercek UYAP content.xml iki bicimde gelir:
  A) <content><![CDATA[ham metin]]></content> + <elements> agaci:
     paragraph cocuklari CDATA icine startOffset/length ile bakar;
     Alignment (0=sol,1=orta,2=sag,3=iki yana), Left/Right/FirstLineIndent,
     bold/italic/underline, tab/space; table/row/cell + columnSpans + border.
  B) <elements><paragraph><text Bold Size>satir</text>… (icice duz metin).
  Hicbiri yoksa duz metin akisi kullanilir.

Not: Bu betik, UDF bicimini gozlemleyerek sifirdan yazilmistir
(ornegin hizalama/tablo oznitelikleri); ucuncu taraf kod kopyalanmamistir.
Turkce karakterler icin sistem Arial TTF kullanilir.
"""

from __future__ import annotations

import argparse
import html
import re
import sys
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass

ALIGN = {"0": 0, "1": 1, "2": 2, "3": 4}  # left, center, right, justify


def decode(data: bytes) -> str:
    for enc in ("utf-8", "utf-8-sig", "cp1254", "iso-8859-9", "latin-1"):
        try:
            return data.decode(enc)
        except UnicodeDecodeError:
            continue
    return data.decode("utf-8", errors="replace")


def esc(s: str) -> str:
    return html.escape(s, quote=False)


def nbsp(s: str) -> str:
    s = s.replace("\t", "&nbsp;&nbsp;&nbsp;&nbsp;")
    return re.sub(r" {2,}", lambda m: "&nbsp;" * len(m.group(0)), s)


def register_fonts():
    try:
        from reportlab.pdfbase import pdfmetrics
        from reportlab.pdfbase.ttfonts import TTFont
    except ImportError:
        return ("Helvetica", "Helvetica-Bold")
    regs = [
        Path(r"C:\Windows\Fonts\arial.ttf"),
        Path("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
        Path("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf"),
    ]
    bolds = [
        Path(r"C:\Windows\Fonts\arialbd.ttf"),
        Path("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"),
        Path("/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf"),
    ]
    reg = next((p for p in regs if p.is_file()), None)
    if reg is None:
        return ("Helvetica", "Helvetica-Bold")
    try:
        pdfmetrics.registerFont(TTFont("MimoSans", str(reg)))
        bold = next((p for p in bolds if p.is_file()), None)
        if bold is not None:
            pdfmetrics.registerFont(TTFont("MimoSans-Bold", str(bold)))
            return ("MimoSans", "MimoSans-Bold")
        return ("MimoSans", "MimoSans")
    except Exception:
        return ("Helvetica", "Helvetica-Bold")


def style_for(styles, font, size, node, prefix):
    from reportlab.lib.styles import ParagraphStyle

    align = ALIGN.get(str(node.attrib.get("Alignment", "0")), 0)

    def num(key):
        try:
            return float(node.attrib.get(key, "0"))
        except ValueError:
            return 0.0

    name = f"{prefix}_{align}_{num('LeftIndent')}_{num('RightIndent')}_{num('FirstLineIndent')}_{size}"
    if name not in styles:
        styles.add(
            ParagraphStyle(
                name=name,
                parent=styles["Normal"],
                fontName=font,
                fontSize=size,
                leading=size + 4,
                alignment=align,
                leftIndent=num("LeftIndent"),
                rightIndent=num("RightIndent"),
                firstLineIndent=num("FirstLineIndent"),
            )
        )
    return styles[name]


def style_span(text, node, default_bold=False):
    """bold/italic/underline ozniteliklerini reportlab etiketine cevirir."""
    b = node.attrib.get("bold", node.attrib.get("Bold", "")).lower() == "true" or default_bold
    i = node.attrib.get("italic", node.attrib.get("Italic", "")).lower() == "true"
    u = node.attrib.get("underline", node.attrib.get("Underline", "")).lower() == "true"
    t = nbsp(esc(text))
    if b:
        t = f"<b>{t}</b>"
    if i:
        t = f"<i>{t}</i>"
    if u:
        t = f"<u>{t}</u>"
    return t


def base_size(node) -> float:
    try:
        return max(8.0, min(20.0, float(node.attrib.get("Size", node.attrib.get("size", "10")))))
    except ValueError:
        return 10.0


def parse_paragraph(p_node, cdata, styles, font):
    """A bicimi (offset) once, B bicimi (icice text) sonra denenir."""
    from reportlab.platypus import Paragraph, Spacer

    # --- A bicimi: cocuklar CDATA'ya offset ile bakar ---
    if cdata and any(
        c.attrib.get("startOffset") is not None and c.attrib.get("length") is not None
        for c in list(p_node)
    ):
        parts = []
        for child in list(p_node):
            try:
                s = int(child.attrib.get("startOffset", "0"))
                n = int(child.attrib.get("length", "0"))
            except ValueError:
                continue
            if child.tag == "tab":
                parts.append("&nbsp;&nbsp;&nbsp;&nbsp;")
                continue
            if child.tag == "space":
                parts.append("&nbsp;" * max(n, 1))
                continue
            span = cdata[s : s + n]
            if span:
                parts.append(style_span(span, child))
        text = "".join(parts).replace("\n", "")
        if not text.strip():
            return Spacer(1, 10)
        return Paragraph(
            text, style_for(styles, font, base_size(p_node), p_node, "UDFP")
        )

    # --- B bicimi: icice <text> dugumleri ---
    texts = p_node.findall("text")
    if texts:
        parts = []
        for tnode in texts:
            t = (tnode.text or "")
            tail = (tnode.tail or "")
            if t:
                parts.append(style_span(t, tnode))
            if tail.strip():
                parts.append(nbsp(esc(tail)))
        text = "".join(parts)
        if not text.strip():
            return Spacer(1, 10)
        return Paragraph(
            text, style_for(styles, font, base_size(p_node), p_node, "UDFP")
        )

    # --- duz metin dugumu ---
    direct = "".join(p_node.itertext()).strip()
    if not direct:
        return Spacer(1, 10)
    return Paragraph(
        nbsp(esc(direct)),
        style_for(styles, font, base_size(p_node), p_node, "UDFP"),
    )


def parse_table(t_node, cdata, styles, font, story):
    from reportlab.lib import colors
    from reportlab.platypus import Paragraph, Spacer, Table, TableStyle

    rows = t_node.findall("row")
    data = []
    for row in rows:
        cells = []
        for cell in row.findall("cell"):
            flows = []
            for p in cell.findall("paragraph"):
                flows.append(parse_paragraph(p, cdata, styles, font))
            if not flows:
                txt = "".join(cell.itertext()).strip()
                if txt:
                    flows.append(
                        Paragraph(
                            nbsp(esc(txt)),
                            style_for(styles, font, 10.0, cell, "UDFC"),
                        )
                    )
            if not flows:
                flows.append(Paragraph("", styles["Normal"]))
            cells.append(flows)
        if cells:
            data.append(cells)
    if not data:
        return
    ncols = max(len(r) for r in data)
    avail = 474.0
    widths = None
    if "columnSpans" in t_node.attrib:
        try:
            spans = [float(w) for w in t_node.attrib["columnSpans"].split(",")]
            if len(spans) >= ncols and sum(spans[:ncols]) > 0:
                tot = sum(spans[:ncols])
                widths = [(w / tot) * avail for w in spans[:ncols]]
        except ValueError:
            pass
    if not widths:
        widths = [avail / ncols] * ncols
    bordered = t_node.attrib.get("border", "borderNone") != "borderNone"
    style_cmds = [
        ("VALIGN", (0, 0), (-1, -1), "TOP"),
        ("LEFTPADDING", (0, 0), (-1, -1), 2),
        ("RIGHTPADDING", (0, 0), (-1, -1), 2),
        ("TOPPADDING", (0, 0), (-1, -1), 2),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 2),
    ]
    if bordered:
        style_cmds.append(("GRID", (0, 0), (-1, -1), 0.5, colors.grey))
    story.append(Table(data, colWidths=widths, style=TableStyle(style_cmds)))
    story.append(Spacer(1, 10))


def walk(node, cdata, styles, font, story):
    from reportlab.platypus import Spacer

    tag = node.tag.rsplit("}", 1)[-1] if "}" in node.tag else node.tag
    if tag == "paragraph":
        story.append(parse_paragraph(node, cdata, styles, font))
    elif tag == "table":
        parse_table(node, cdata, styles, font, story)
    elif tag in ("elements", "header", "footer", "cell", "content", "row"):
        for child in list(node):
            walk(child, cdata, styles, font, story)
    # signature/binary/imagedata atlanir


def convert(udf: Path, out: Path, title: str, no_title: bool = False) -> None:
    from reportlab.lib.pagesizes import A4
    from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
    from reportlab.lib.units import mm
    from reportlab.platypus import Paragraph, SimpleDocTemplate, Spacer

    if not zipfile.is_zipfile(udf):
        raise RuntimeError("UDF zip konteyneri değil (bozuk dosya?)")
    with zipfile.ZipFile(udf, "r") as z:
        names = z.namelist()
        xml_name = next(
            (
                c
                for c in (
                    "content.xml",
                    "Content.xml",
                    "CONTENT.XML",
                    "content/Content.xml",
                )
                if c in names
            ),
            None,
        )
        if xml_name is None:
            xml_name = next(
                (n for n in names if n.lower().endswith("content.xml")), None
            )
        if xml_name is None:
            xml_name = next(
                (
                    n
                    for n in names
                    if n.lower().endswith(".xml") and "sign" not in n.lower()
                ),
                None,
            )
        if xml_name is None:
            raise RuntimeError(f"content.xml yok; girdiler={names[:20]}")
        raw = decode(z.read(xml_name))

    try:
        root = ET.fromstring(raw.encode("utf-8"))
    except ET.ParseError:
        root = None

    font, font_bold = register_fonts()
    styles = getSampleStyleSheet()
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
        story.append(
            Paragraph(
                esc(title),
                ParagraphStyle(
                    "UDFTitle",
                    parent=styles["Title"],
                    fontName=font_bold,
                    fontSize=16,
                    leading=20,
                    spaceAfter=6 * mm,
                ),
            )
        )

    structured = False
    if root is not None:
        # A bicimi CDATA metni: <content> cocuk elemani (kokun kendisi degil)
        cdata = ""
        for child in list(root):
            tag = child.tag.rsplit("}", 1)[-1] if "}" in child.tag else child.tag
            if tag.lower() == "content" and child.text and child.text.strip():
                cdata = child.text
                break
        elems = root.find(".//elements")
        if elems is not None and len(list(elems)) > 0:
            walk(elems, cdata, styles, font, story)
            structured = len(story) > 1

    if not structured:
        # Duzyazi akisi (B bicimi de buraya duser ama walk zaten islemistir;
        # burasi yalniz yapi bulunamazsa calisir)
        if root is not None:
            text = "\n".join(
                t.strip() for t in root.itertext() if t and t.strip()
            )
        else:
            chunks = re.findall(r">([^<>]+)<", raw)
            text = "\n".join(
                html.unescape(c).strip() for c in chunks if c.strip()
            )
        body_style = ParagraphStyle(
            "UDFPlain",
            parent=styles["Normal"],
            fontName=font,
            fontSize=10.5,
            leading=15,
            spaceAfter=2 * mm,
        )
        for para in text.split("\n"):
            if para.strip():
                story.append(Paragraph(nbsp(esc(para)), body_style))
            else:
                story.append(Spacer(1, 6))

    # Imza/gomulu notu (metin disi katmanlar)
    with zipfile.ZipFile(udf, "r") as z:
        names = z.namelist()
        notes = []
        if any(n.lower().endswith(".p7s") or "signature" in n.lower() for n in names):
            notes.append("[e-imza mevcut — metne çevrilmez / doğrulanmaz]")
        bins = [n for n in names if n.lower().startswith("binary/")]
        if bins:
            notes.append(f"[gömülü nesne: {len(bins)} — ayrı OCR'a girmez]")
    if notes:
        note_style = ParagraphStyle(
            "UDFNote",
            parent=styles["Normal"],
            fontName=font,
            fontSize=9,
            leading=12,
            textColor="#6B7280",
            spaceBefore=4 * mm,
        )
        for n in notes:
            story.append(Paragraph(esc(n), note_style))

    doc.build(story)


def main() -> int:
    ap = argparse.ArgumentParser(description="UDF → PDF (Mimo OCR)")
    ap.add_argument("--udf", required=True)
    ap.add_argument("--out", required=False)
    ap.add_argument("--title", default=None)
    ap.add_argument("--no-title", action="store_true", help="Baslik paragrafini yazma")
    args = ap.parse_args()

    udf = Path(args.udf)
    if not udf.is_file():
        print(f"dosya yok: {udf}", file=sys.stderr)
        return 1
    out = Path(args.out) if args.out else udf.with_suffix(".pdf")
    title = args.title or udf.stem
    try:
        convert(udf, out, title, args.no_title)
    except ImportError:
        print(
            "reportlab yok: MiMo Python veya 'pip install reportlab' gerekli",
            file=sys.stderr,
        )
        return 2
    except Exception as e:
        print(f"udf→pdf hatası: {e}", file=sys.stderr)
        return 3
    print(f"[udf-pdf] -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
