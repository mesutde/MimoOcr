#!/usr/bin/env python3
"""Birden fazla PDF'i tek PDF'te birlestirir (birlesik kip).

Kullanim:
  merge_pdfs.py --out <birlesik.pdf> <girdi1.pdf> [girdi2.pdf ...]

Her belge kendi yapisal duzeniyle (ayri ceviri) uretilir, sonra uclari
birlestirilir; boylece birlesik PDF, ayri PDF'lerle ayni kalitede olur.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass


def main() -> int:
    ap = argparse.ArgumentParser(description="PDF birlestir (Mimo OCR)")
    ap.add_argument("--out", required=True)
    ap.add_argument("inputs", nargs="+")
    args = ap.parse_args()

    try:
        from pypdf import PdfWriter
    except ImportError:
        try:
            from PyPDF2 import PdfWriter  # type: ignore
        except ImportError:
            print(
                "pypdf yok: MiMo Python veya 'pip install pypdf' gerekli",
                file=sys.stderr,
            )
            return 2

    writer = PdfWriter()
    n = 0
    for inp in args.inputs:
        p = Path(inp)
        if not p.is_file():
            print(f"atlandi (yok): {inp}", file=sys.stderr)
            continue
        try:
            writer.append(str(p))
            n += 1
        except Exception as e:
            print(f"atlandi (bozuk?): {inp}: {e}", file=sys.stderr)
    if n == 0:
        print("birlestirilecek PDF yok", file=sys.stderr)
        return 3
    with open(args.out, "wb") as f:
        writer.write(f)
    print(f"[merge] {n} pdf -> {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
