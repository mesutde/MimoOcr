#!/usr/bin/env python3
"""Extract tabular/text data from screen-recording videos (scroll demos).

Usage:
  video_extract.py --video <file> --out <dir> [--lang tur+eng]
                   [--engine auto] [--max-frames 180] [--mode auto|fast|slow]

Strategy:
  1. ffprobe duration
  2. Adaptive frame sampling (fast scroll vs slow scroll)
  3. OCR each frame (tesseract CLI) — sequential, low memory
  4. Dedup overlapping lines from scrolling
  5. Detect table-like columns → CSV; always write TXT

Memory-safe: one frame at a time, long-side scale cap, frame delete after OCR.
"""

from __future__ import annotations

import argparse
import csv
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

# Windows console often uses cp1254 — force UTF-8 so ≈ etc. don't crash prints.
try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass


def _repo_root() -> Path:
    """Depo/kurulum kökünü bul: script konumu → exe konumu → cwd."""
    here = Path(__file__).resolve()
    for parent in [here.parent, *here.parents]:
        if (parent / "tessdata").is_dir() or (parent / "models.json").is_file():
            return parent
    exe = Path(sys.executable).resolve()
    for parent in [exe.parent, *list(exe.parents)[:6]]:
        if (parent / "tessdata").is_dir():
            return parent
    return Path.cwd()


# Kurulumla gomulu ffmpeg/ffprobe varsa PATH'e al (yoksa sistem PATH'i kullanilir).
try:
    _bundled_ff = _repo_root() / "ffmpeg"
    if (_bundled_ff / "ffmpeg.exe").is_file():
        os.environ["PATH"] = str(_bundled_ff) + os.pathsep + os.environ.get("PATH", "")
except Exception:
    pass


def _p(msg: str) -> None:
    """Safe print for Windows consoles."""
    try:
        print(msg, flush=True)
    except UnicodeEncodeError:
        try:
            print(msg.encode("ascii", "replace").decode("ascii"), flush=True)
        except Exception:
            print(repr(msg), flush=True)


def find_tesseract() -> str:
    env = os.environ.get("MIMO_TESSERACT") or os.environ.get("TESSERACT")
    if env and Path(env).exists():
        return env
    root = _repo_root()
    for p in (
        root / "tesseract-runtime" / "tesseract.exe",
        root / "tessdata",
        root / "assets" / "tesseract-runtime" / "tesseract.exe",
        Path(r"C:\Program Files\Tesseract-OCR\tesseract.exe"),
        Path(r"C:\Program Files (x86)\Tesseract-OCR\tesseract.exe"),
    ):
        if p.is_file():
            return str(p)
    for name in ("tesseract", "tesseract.exe"):
        found = shutil.which(name)
        if found:
            return found
    return "tesseract"


def tessdata_dir(langs: str) -> str | None:
    env = os.environ.get("MIMO_TESSDATA") or os.environ.get("TESSDATA_PREFIX")
    if env and Path(env).exists():
        return env
    root = _repo_root()
    for p in (
        root / "tessdata",
        root / "assets" / "models" / "tessdata",
        root / "assets" / "tesseract-runtime" / "tessdata",
        Path(r"C:\Program Files\Tesseract-OCR\tessdata"),
        Path("tessdata"),
    ):
        if p.exists() and (p / "eng.traineddata").exists():
            return str(p)
    return None


def run(cmd: list[str], timeout: int = 120) -> subprocess.CompletedProcess:
    kwargs = dict(
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    if sys.platform == "win32":
        kwargs["creationflags"] = 0x08000000  # CREATE_NO_WINDOW
    return subprocess.run(cmd, **kwargs)


def video_duration_sec(path: Path) -> float:
    fp = shutil.which("ffprobe") or "ffprobe"
    r = run(
        [
            fp,
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            str(path),
        ],
        timeout=30,
    )
    try:
        return float((r.stdout or b"0").decode().strip() or "0")
    except ValueError:
        return 0.0


def extract_frames_adaptive(
    video: Path,
    work: Path,
    mode: str = "auto",
    max_frames: int = 180,
) -> list[Path]:
    """Extract frames; more samples when content changes slowly (slow scroll)."""
    ff = shutil.which("ffmpeg") or "ffmpeg"
    dur = video_duration_sec(video)
    if dur <= 0:
        dur = 30.0
    # Cap memory: never request insane frame counts
    max_frames = max(8, min(max_frames, 400))

    if mode == "fast":
        # Fast scroll: moderate sampling; later OCR merge drops blanks
        fps = max(0.4, min(2.5, max_frames / max(dur, 1)))
    elif mode == "slow":
        # Slow scroll: denser sampling
        fps = max(0.5, min(4.0, max_frames / max(dur, 1)))
    else:
        # auto: blend — scroll speed unknown; start medium
        fps = max(0.45, min(3.0, max_frames / max(dur, 1)))

    # Scale down for OCR speed/memory (tables still readable at 1280)
    vf = f"fps={fps:.3f},scale='min(1280,iw)':-2"
    pattern = work / "frame_%05d.png"
    r = run(
        [
            ff,
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            str(video),
            "-vf",
            vf,
            "-vsync",
            "0",
            str(pattern),
        ],
        timeout=max(60, int(dur) + 60),
    )
    if r.returncode != 0:
        err = (r.stderr or b"").decode(errors="replace")[-400:]
        raise RuntimeError(f"ffmpeg extract failed: {err}")
    frames = sorted(work.glob("frame_*.png"))
    # Enforce max_frames (keep evenly spaced)
    if len(frames) > max_frames:
        step = len(frames) / max_frames
        keep = [frames[int(i * step)] for i in range(max_frames)]
        for f in frames:
            if f not in keep:
                try:
                    f.unlink()
                except OSError:
                    pass
        frames = keep
    return frames


def ocr_frame(
    frame: Path,
    tesseract: str,
    lang: str,
    tessdata: str | None,
    timeout: int = 40,
) -> str:
    out_base = frame.with_suffix("")
    cmd = [tesseract, str(frame), str(out_base), "-l", lang, "--psm", "6"]
    env = os.environ.copy()
    if tessdata:
        env["TESSDATA_PREFIX"] = tessdata
        env["MIMO_TESSDATA"] = tessdata
    kwargs = dict(stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout, env=env, check=False)
    if sys.platform == "win32":
        kwargs["creationflags"] = 0x08000000
    try:
        subprocess.run(cmd, **kwargs)
    except Exception:
        return ""
    txt = out_base.with_suffix(".txt")
    if not txt.exists():
        return ""
    text = txt.read_text(encoding="utf-8", errors="replace")
    try:
        txt.unlink()
    except OSError:
        pass
    return text


def norm_line(s: str) -> str:
    s = s.replace(" ", " ")
    s = re.sub(r"\s+", " ", s).strip()
    return s


def line_key(s: str) -> str:
    s = norm_line(s).lower()
    s = re.sub(r"[^\w\u00c0-\u024f]+", "", s)
    return s


def merge_scrolled_texts(texts: list[str]) -> str:
    """Join frames while dropping lines that appear only because of scroll overlap."""
    if not texts:
        return ""
    # Collect unique non-empty lines in order of first appearance
    seen: set[str] = set()
    merged: list[str] = []
    prev_keys: list[str] = []
    for text in texts:
        lines = [norm_line(x) for x in text.splitlines() if norm_line(x)]
        keys = [line_key(x) for x in lines]
        # Skip if this frame is nearly identical to previous (scroll paused)
        if keys and keys == prev_keys:
            continue
        prev_keys = keys
        # New lines: keep order; if overlap with tail of merged, skip dups
        for line, key in zip(lines, keys):
            if not key or key in seen:
                continue
            seen.add(key)
            merged.append(line)
    return "\n".join(merged)


def looks_tabular(lines: list[str]) -> bool:
    if len(lines) < 3:
        return False
    multi_col = 0
    for ln in lines[:40]:
        # multiple spaces, tabs, or pipe separators
        if "\t" in ln or " | " in ln or re.search(r"\s{2,}\S+\s{2,}", ln):
            multi_col += 1
        # many numbers
        if len(re.findall(r"\d+[.,]\d+|\d{3,}", ln)) >= 3:
            multi_col += 1
    return multi_col >= max(3, len(lines) // 4)


def split_row(line: str) -> list[str]:
    if "\t" in line:
        parts = [p.strip() for p in line.split("\t") if p.strip()]
        if len(parts) >= 2:
            return parts
    if " | " in line:
        parts = [p.strip() for p in line.split(" | ") if p.strip()]
        if len(parts) >= 2:
            return parts
    # 2+ spaces
    parts = re.split(r"\s{2,}", line.strip())
    if len(parts) >= 2:
        return parts
    # comma CSV-like
    if "," in line and line.count(",") >= 1:
        parts = [p.strip() for p in line.split(",")]
        if len(parts) >= 2:
            return parts
    return [line.strip()] if line.strip() else []


def text_to_csv_rows(lines: list[str]) -> list[list[str]]:
    rows = []
    for ln in lines:
        parts = split_row(ln)
        if parts:
            rows.append(parts)
    if not rows:
        return []
    width = max(len(r) for r in rows)
    if width == 1:
        # single column "table" — still export as one col CSV
        return [[r[0]] for r in rows if r]
    return [r + [""] * (width - len(r)) for r in rows]


def main() -> int:
    ap = argparse.ArgumentParser(description="Video → CSV/TXT extract (Mimo OCR)")
    ap.add_argument("--video", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--lang", default="tur+eng")
    ap.add_argument("--mode", default="auto", choices=["auto", "fast", "slow"])
    ap.add_argument("--max-frames", type=int, default=180)
    ap.add_argument("--formats", default="csv,txt", help="comma list: csv,txt,md")
    args = ap.parse_args()

    video = Path(args.video)
    if not video.exists():
        print(f"video not found: {video}", file=sys.stderr)
        return 1
    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)

    tesseract = find_tesseract()
    tessdata = tessdata_dir(args.lang)
    lang = args.lang.replace(",", "+")

    frames_dir = Path(tempfile.mkdtemp(prefix="mimo-vid-"))
    try:
        _p(f"[start] {video.name}")
        _p("PROGRESS 0/1")
        _p("[ffmpeg] extracting frames...")
        frames = extract_frames_adaptive(
            video, frames_dir, mode=args.mode, max_frames=args.max_frames
        )
        _p(
            f"[video] duration~{video_duration_sec(video):.1f}s frames={len(frames)} mode={args.mode}"
        )
        _p(f"PROGRESS 0/{max(1, len(frames))}")

        texts: list[str] = []
        n = len(frames) or 1
        for i, fr in enumerate(frames, 1):
            _p(f"PROGRESS {i}/{n}")
            _p(f"[ocr] {i}/{n} {fr.name}")
            t = ocr_frame(fr, tesseract, lang, tessdata)
            try:
                fr.unlink()
            except OSError:
                pass
            if t.strip():
                texts.append(t)
                _p(f"[ok] {fr.name}: {len(t)} ch")
            else:
                _p(f"[empty] {fr.name}")

        _p("[merge] de-duplicating scroll overlap...")
        _p(f"PROGRESS {n}/{n}")
        merged = merge_scrolled_texts(texts)
        lines = [ln for ln in merged.splitlines() if ln.strip()]
        tabular = looks_tabular(lines)
        _p(f"[merge] frames_ocr={len(texts)} lines={len(lines)} tabular={tabular}")

        stem = video.stem
        written: list[str] = []
        formats = {f.strip().lower() for f in args.formats.split(",") if f.strip()}

        if "txt" in formats or "md" in formats:
            body = merged if merged.strip() else "[no readable text extracted from video]"
            header = (
                f"[Mimo video extract]\nsource={video.name}\n"
                f"frames_ocr={len(texts)} mode={args.mode} tabular={tabular}\n\n"
            )
            if "txt" in formats:
                p = out_dir / f"{stem}.video.txt"
                p.write_text(header + body, encoding="utf-8")
                written.append(str(p))
            if "md" in formats:
                md = f"# {video.name}\n\n> Video OCR extract · tabular={tabular}\n\n```\n{body}\n```\n"
                p = out_dir / f"{stem}.video.md"
                p.write_text(md, encoding="utf-8")
                written.append(str(p))

        if "csv" in formats:
            rows = text_to_csv_rows(lines) if tabular or lines else []
            # Always try CSV if we have any lines — LLM users often want table-ish dump
            if not rows and lines:
                rows = [[ln] for ln in lines]
            p = out_dir / f"{stem}.video.csv"
            with p.open("w", encoding="utf-8-sig", newline="") as f:
                w = csv.writer(f)
                for row in rows:
                    w.writerow(row)
            written.append(str(p))
            _p(f"[csv] rows={len(rows)} tabular={tabular} -> {p}")

        _p("[done] files: " + " | ".join(written))
        return 0
    finally:
        shutil.rmtree(frames_dir, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
