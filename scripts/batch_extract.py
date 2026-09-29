#!/usr/bin/env python3
"""Extract plain text from non-image files for Mimo OCR batch mode.

Usage: batch_extract.py <file>
Prints UTF-8 text to stdout. Exit 0 on success.
Supported: pdf, docx, xlsx, pptx, udf (UYAP), odt/ods/odp, epub,
rtf(simple), + plain text (txt md csv json log xml html htm sql srt vtt
ini cfg yaml yml toml ps1 bat cmd sh)
"""

from __future__ import annotations

import html
import sys
import zipfile
import re
from pathlib import Path
from xml.etree import ElementTree as ET

# Windows konsolu (cp1254) Türkçe karakterlerde patlamasin.
try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass


def _decode_xml_bytes(data: bytes) -> str:
    for enc in ("utf-8", "utf-8-sig", "cp1254", "iso-8859-9", "latin-1"):
        try:
            return data.decode(enc)
        except UnicodeDecodeError:
            continue
    return data.decode("utf-8", errors="replace")


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


def _strip_rtf(rtf: str) -> str:
    rtf = re.sub(r"\\'[0-9a-fA-F]{2}", lambda m: chr(int(m.group(0)[2:], 16)), rtf)
    rtf = re.sub(r"\\[a-zA-Z]+-?\d* ?", " ", rtf)
    rtf = re.sub(r"[{}]", "", rtf)
    rtf = re.sub(r"\s+", " ", rtf)
    return rtf.strip()


def extract_rtf(path: Path) -> str:
    return _strip_rtf(read_plain(path))


PLAIN_EXTS = {
    "txt", "md", "csv", "json", "log", "xml", "html", "htm",
    "sql", "srt", "vtt", "ini", "cfg", "yaml", "yml", "toml",
    "ps1", "bat", "cmd", "sh",
}


def extract_odf(path: Path) -> str:
    """LibreOffice odt/ods/odp: ZIP icindeki content.xml'den <text:p> metinleri.

    ODS'te sayfa (table:table) basliklari korunur.
    """
    if not zipfile.is_zipfile(path):
        raise RuntimeError("not an ODF zip container (or file is corrupt)")
    with zipfile.ZipFile(path, "r") as z:
        try:
            xml = z.read("content.xml").decode("utf-8", errors="replace")
        except KeyError as e:
            raise RuntimeError("ODF content.xml not found") from e
    tables = re.findall(
        r'<table:table[^>]*table:name="([^"]*)"[^>]*>(.*?)</table:table>',
        xml,
        re.DOTALL,
    )
    parts: list[str] = []
    if tables:
        for name, body in tables:
            paras = re.findall(r"<text:p[^>]*>(.*?)</text:p>", body, re.DOTALL)
            clean = [_clean_odf_p(p) for p in paras]
            clean = [c for c in clean if c]
            if clean:
                parts.append(f"## {html.unescape(name)}")
                parts.extend(clean)
    else:
        paras = re.findall(r"<text:p[^>]*>(.*?)</text:p>", xml, re.DOTALL)
        parts = [_clean_odf_p(p) for p in paras]
        parts = [c for c in parts if c]
    return "\n".join(parts)


def _clean_odf_p(fragment: str) -> str:
    # <text:span>, <text:s c="n"/> (bosluk), <text:line-break/> cozulur.
    fragment = re.sub(r"<text:s[^/]*c=\"(\d+)\"[^/]*/>", lambda m: " " * int(m.group(1)), fragment)
    fragment = re.sub(r"<text:s[^/]*/>", " ", fragment)
    fragment = re.sub(r"<text:line-break[^/]*/>", "\n", fragment)
    fragment = re.sub(r"<[^>]+>", "", fragment)
    return html.unescape(fragment).strip()


def extract_epub(path: Path) -> str:
    """EPUB: ZIP icindeki xhtml/html belgelerinden duz metin (dosya adi sirasi)."""
    if not zipfile.is_zipfile(path):
        raise RuntimeError("not an EPUB zip container (or file is corrupt)")
    with zipfile.ZipFile(path, "r") as z:
        names = sorted(
            n for n in z.namelist()
            if n.lower().endswith((".xhtml", ".xht", ".html", ".htm"))
            and not n.startswith("META-INF/")
        )
        if not names:
            raise RuntimeError("EPUB has no readable pages")
        parts = []
        for name in names:
            raw = z.read(name).decode("utf-8", errors="replace")
            body = re.search(r"<body[^>]*>(.*)</body>", raw, re.DOTALL | re.IGNORECASE)
            frag = body.group(1) if body else raw
            frag = re.sub(r"<script.*?</script>", " ", frag, flags=re.DOTALL | re.IGNORECASE)
            frag = re.sub(r"<style.*?</style>", " ", frag, flags=re.DOTALL | re.IGNORECASE)
            frag = re.sub(r"<(p|h\d|li|tr|div|br)[^>]*>", "\n", frag, flags=re.IGNORECASE)
            frag = re.sub(r"<[^>]+>", "", frag)
            text = html.unescape(frag)
            text = re.sub(r"[ \t]+\n", "\n", text)
            text = re.sub(r"\n{3,}", "\n\n", text).strip()
            if text:
                parts.append(text)
    return "\n\n".join(parts)


def _xml_local(tag: str) -> str:
    return tag.rsplit("}", 1)[-1] if "}" in tag else tag


def _collect_text_xml(xml_bytes: bytes) -> str:
    raw = _decode_xml_bytes(xml_bytes)
    try:
        root = ET.fromstring(raw.encode("utf-8"))
    except ET.ParseError:
        chunks = re.findall(r">([^<>]+)<", raw)
        return "\n".join(html.unescape(c).strip() for c in chunks if c.strip())

    para_tags = {"paragraph", "p", "textparagraph", "par", "block", "content"}
    line_tags = {"line", "br", "tab"}

    def walk(node, buf: list[str]) -> None:
        name = _xml_local(node.tag).lower()
        if node.text:
            t = node.text.strip()
            if t:
                buf.append(t)
        for child in list(node):
            cname = _xml_local(child.tag).lower()
            if cname in {"signature", "binary", "imagedata", "image"}:
                continue
            if name in para_tags:
                walk(child, buf)
                buf.append("\n")
            elif cname in line_tags:
                walk(child, buf)
                buf.append("\n")
            else:
                walk(child, buf)
            if child.tail:
                tt = child.tail.strip()
                if tt:
                    buf.append(tt)

    buf: list[str] = []
    walk(root, buf)
    text = "".join(buf)
    for frag in re.findall(r"\{\\rtf1(?:[^{}]|\{[^{}]*\})*\}", raw):
        st = _strip_rtf(frag)
        if st:
            text += "\n" + st
    text = re.sub(r"[ \t]+\n", "\n", text)
    text = re.sub(r"\n{3,}", "\n\n", text)
    return text.strip()


def extract_udf(path: Path) -> str:
    """UYAP Doküman Formatı (.udf): ZIP + content.xml (+ signature.p7s, binary/).

    Justice Ministry UYAP container (not optical-disc UDF).
    """
    if not zipfile.is_zipfile(path):
        raise RuntimeError("not a UYAP UDF zip container (or file is corrupt)")
    with zipfile.ZipFile(path, "r") as z:
        names = z.namelist()
        candidates = ["content.xml", "Content.xml", "CONTENT.XML", "content/Content.xml"]
        xml_name = next((c for c in candidates if c in names), None)
        if xml_name is None:
            xml_name = next((n for n in names if n.lower().endswith("content.xml")), None)
        if xml_name is None:
            xml_name = next(
                (n for n in names if n.lower().endswith(".xml") and "sign" not in n.lower()),
                None,
            )
        if xml_name is None:
            raise RuntimeError(f"UDF content.xml not found; entries={names[:20]}")

        text = _collect_text_xml(z.read(xml_name))
        has_sig = any(n.lower().endswith(".p7s") or "signature" in n.lower() for n in names)
        binaries = [n for n in names if n.lower().startswith("binary/")]
        meta = [f"[UYAP UDF] content={xml_name}"]
        if has_sig:
            meta.append("[e-signature present — not exported as text]")
        if binaries:
            meta.append(f"[embedded objects: {len(binaries)}]")
        body = text if text.strip() else "[content.xml has no extractable text — image-only?]"
        return "\n".join(meta) + "\n\n" + body


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: batch_extract.py <file>", file=sys.stderr)
        return 2
    path = Path(sys.argv[1])
    if not path.exists():
        print(f"file not found: {path}", file=sys.stderr)
        return 1
    ext = path.suffix.lower().lstrip(".")
    try:
        if ext == "pdf":
            text = extract_pdf(path)
        elif ext == "docx":
            text = extract_docx(path)
        elif ext == "xlsx":
            text = extract_xlsx(path)
        elif ext == "pptx":
            text = extract_pptx(path)
        elif ext == "udf":
            text = extract_udf(path)
        elif ext == "rtf":
            text = extract_rtf(path)
        elif ext in {"odt", "ods", "odp"}:
            text = extract_odf(path)
        elif ext == "epub":
            text = extract_epub(path)
        elif ext in PLAIN_EXTS:
            text = read_plain(path)
        else:
            print(f"unsupported type: .{ext}", file=sys.stderr)
            return 3
        sys.stdout.write(text)
        return 0
    except Exception as e:
        print(f"extract error: {e}", file=sys.stderr)
        return 4


if __name__ == "__main__":
    raise SystemExit(main())
