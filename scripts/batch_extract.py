#!/usr/bin/env python3
"""Extract plain text from non-image files for Mimo OCR batch mode.

Usage: batch_extract.py <file>
Prints UTF-8 text to stdout. Exit 0 on success.
Supported: pdf, docx, xlsx, txt, md, csv, json, rtf(simple)
"""

from __future__ import annotations

import json
import sys
import zipfile
import re
from pathlib import Path


def read_plain(path: Path) -> str:
    data = path.read_bytes()
    for enc in ("utf-8", "utf-8-sig", "cp1254", "latin-1"):
        try:
            return data.decode(enc)
        except UnicodeDecodeError:
            continue
    return data.decode("utf-8", errors="replace")


def extract_pdf(path: Path) -> str:
    try:
        from pypdf import PdfReader
    except ImportError:
        try:
            from PyPDF2 import PdfReader  # type: ignore
        except ImportError as e:
            raise RuntimeError(f"pypdf not available: {e}") from e
    reader = PdfReader(str(path))
    parts = []
    for i, page in enumerate(reader.pages):
        try:
            t = page.extract_text() or ""
        except Exception as e:
            t = f"[page {i + 1} error: {e}]"
        if t.strip():
            parts.append(t)
    return "\n\n".join(parts)


def extract_docx(path: Path) -> str:
    try:
        import docx  # python-docx
        d = docx.Document(str(path))
        paras = [p.text for p in d.paragraphs if p.text and p.text.strip()]
        # tables
        for table in d.tables:
            for row in table.rows:
                cells = [c.text.strip() for c in row.cells if c.text and c.text.strip()]
                if cells:
                    paras.append(" | ".join(cells))
        return "\n".join(paras)
    except Exception:
        # Fallback: raw XML text from word/document.xml
        with zipfile.ZipFile(path, "r") as z:
            xml = z.read("word/document.xml").decode("utf-8", errors="replace")
        texts = re.findall(r"<w:t[^>]*>([^<]*)</w:t>", xml)
        return " ".join(texts)


def extract_xlsx(path: Path) -> str:
    try:
        import openpyxl
        wb = openpyxl.load_workbook(str(path), read_only=True, data_only=True)
        lines = []
        for sheet in wb.worksheets:
            lines.append(f"## {sheet.title}")
            for row in sheet.iter_rows(values_only=True):
                cells = ["" if c is None else str(c) for c in row]
                if any(c.strip() for c in cells):
                    lines.append("\t".join(cells))
        wb.close()
        return "\n".join(lines)
    except Exception as e:
        raise RuntimeError(f"xlsx: {e}") from e


def extract_pptx(path: Path) -> str:
    try:
        from pptx import Presentation
        prs = Presentation(str(path))
        parts = []
        for i, slide in enumerate(prs.slides, 1):
            parts.append(f"## Slide {i}")
            for shape in slide.shapes:
                if hasattr(shape, "text") and shape.text and shape.text.strip():
                    parts.append(shape.text)
        return "\n".join(parts)
    except Exception:
        # raw XML fallback
        with zipfile.ZipFile(path, "r") as z:
            slides = [n for n in z.namelist() if n.startswith("ppt/slides/slide") and n.endswith(".xml")]
            parts = []
            for name in sorted(slides):
                xml = z.read(name).decode("utf-8", errors="replace")
                texts = re.findall(r"<a:t>([^<]*)</a:t>", xml)
                if texts:
                    parts.append(" ".join(texts))
        return "\n".join(parts)


def extract_rtf(path: Path) -> str:
    raw = read_plain(path)
    # crude RTF strip
    raw = re.sub(r"\\[a-z]+-?\d* ?", " ", raw)
    raw = re.sub(r"[{}]", "", raw)
    raw = re.sub(r"\s+", " ", raw)
    return raw.strip()


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: batch_extract.py <file>", file=sys.stderr)
        return 2
    path = Path(sys.argv[1])
    if not path.exists():
        print(f"file not found: {path}", file=sys.stderr)
        return 1
    ext = path.suffix.lower()
    try:
        if ext == ".pdf":
            text = extract_pdf(path)
        elif ext == ".docx":
            text = extract_docx(path)
        elif ext == ".xlsx":
            text = extract_xlsx(path)
        elif ext == ".pptx":
            text = extract_pptx(path)
        elif ext == ".rtf":
            text = extract_rtf(path)
        elif ext in {".txt", ".md", ".csv", ".json", ".log", ".xml", ".html", ".htm"}:
            text = read_plain(path)
        else:
            print(f"unsupported type: {ext}", file=sys.stderr)
            return 3
        sys.stdout.write(text)
        return 0
    except Exception as e:
        print(f"extract error: {e}", file=sys.stderr)
        return 4


if __name__ == "__main__":
    raise SystemExit(main())
