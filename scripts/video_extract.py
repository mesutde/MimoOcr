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
XLSX needs openpyxl (bundled); if missing it is skipped with a warning.
"""

from __future__ import annotations

import argparse
import csv
import difflib
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
        stdin=subprocess.DEVNULL,  # GUI'den dogan surecte gecersiz stdin abort yapmasin
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    if sys.platform == "win32":
        kwargs["creationflags"] = 0x08000000  # CREATE_NO_WINDOW
    return subprocess.run(cmd, **kwargs)


def which_or_none(name: str) -> str | None:
    """PATH uzerinden coz; gorele/bozuk eslesmeleri ele (mutlak + dosya sarti)."""
    found = shutil.which(name)
    if not found:
        return None
    abs_path = os.path.abspath(found)
    if not os.path.isfile(abs_path):
        return None
    return abs_path


def bundled_tool(name: str) -> str | None:
    """Kurulumla gomulu ffmpeg/ffprobe: exe-yani -> depo kokleri, mutlak yol."""
    exe = Path(sys.executable).resolve()
    roots = [exe.parent, *list(exe.parents)[:6], _repo_root()]
    try:
        cwd = Path.cwd()
        roots.append(cwd)
    except OSError:
        pass
    seen = set()
    for root in roots:
        if root in seen:
            continue
        seen.add(root)
        for cand in (root / "ffmpeg" / name, root / "assets" / "ffmpeg" / name):
            if cand.is_file():
                return str(cand.resolve())
    return None


def probe_tool(name: str) -> tuple[str, bool, str]:
    """name --version sondasi; (kullanilan_yol, tamam_mi, surum_satiri)."""
    exe = name if name.lower().endswith(".exe") else name + ".exe"
    path = bundled_tool(exe) or which_or_none(name) or name
    try:
        r = run([path, "-version"], timeout=20)
        line = ((r.stdout or b"").decode(errors="replace").splitlines() or ["?"])[0][:120]
        return path, (r.returncode == 0), line
    except FileNotFoundError:
        return path, False, "bulunamadı"
    except Exception as e:
        return path, False, f"sonda hatası: {e}"


def video_duration_sec(path: Path) -> float:
    fp = bundled_tool("ffprobe.exe") or which_or_none("ffprobe") or "ffprobe"
    try:
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
    except FileNotFoundError:
        _p("[warn] ffprobe yok — süre 30sn varsayılacak")
        return 0.0
    try:
        return float((r.stdout or b"0").decode().strip() or "0")
    except ValueError:
        return 0.0


def run_extract(ff: str, video: Path, vf: str, pattern: Path, dur: float):
    """ffmpeg kare cikarimi; warning seviyesinde cikti birakir (tani icin)."""
    # Not: "-vsync 0" ffmpeg 8.0+ tarafindan kaldirildi ("Unrecognized option").
    # fps filtresi kare zamanlamasini zaten yonetir; ek secenek gerekmez.
    try:
        return run(
            [
                ff,
                "-hide_banner",
                "-loglevel",
                "warning",
                "-i",
                str(video),
                "-vf",
                vf,
                str(pattern),
            ],
            timeout=max(60, int(dur) + 60),
        )
    except FileNotFoundError:
        raise RuntimeError(f"ffmpeg bulunamadı: {ff} (kurulumun ffmpeg/ klasörü eksik?)")


def extract_frames_adaptive(
    video: Path,
    work: Path,
    mode: str = "auto",
    max_frames: int = 180,
    mask: str = "",
) -> list[Path]:
    """Extract frames; more samples when content changes slowly (slow scroll)."""
    ff = bundled_tool("ffmpeg.exe") or which_or_none("ffmpeg") or "ffmpeg"
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

    # Scale down for OCR speed/memory (tables still readable at 1280).
    # mask: rec kontrol/cerceve karartma, "x,y,w,h;x,y,w,h" (girdi piksel uzayi).
    pre = ""
    if mask:
        boxes = []
        for part in mask.split(";"):
            try:
                mx, my, mw, mh = (int(float(v)) for v in part.split(",")[:4])
            except ValueError:
                continue
            if mw > 0 and mh > 0:
                boxes.append(
                    f"drawbox=x={mx}:y={my}:w={mw}:h={mh}:color=black:t=fill"
                )
        if boxes:
            pre = ",".join(boxes) + ","
    vf = f"{pre}fps={fps:.3f},scale='min(1280,iw)':-2"
    pattern = work / "frame_%05d.png"
    r = run_extract(ff, video, vf, pattern, dur)
    if r.returncode != 0:
        # Yedek deneme: olceksiz sade filtre (filtre/olcek suphesini eler).
        _p("[warn] ölçekli çıkarma başarısız — sade filtreyle tekrar deneniyor")
        vf_simple = f"{pre}fps={fps:.3f}"
        r = run_extract(ff, video, vf_simple, pattern, dur)
    if r.returncode != 0:
        err = (r.stderr or b"").decode(errors="replace")[-400:].strip()
        hint = ""
        if not err:
            hint = (
                " (stderr boş: süreç sessiz öldü — Windows Güvenliği davranış "
                "engeli/karantina ya da TEMP yazma iznini kontrol edin)"
            )
        raise RuntimeError(
            f"ffmpeg extract failed [kod={r.returncode}] [{ff}]{hint}: {err}"
        )
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
    kwargs = dict(stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout, env=env, check=False)
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


def ocr_frame_words(
    frame: Path,
    tesseract: str,
    lang: str,
    tessdata: str | None,
    timeout: int = 45,
) -> list[dict]:
    """Tek passta kelime kutulari (TSV): text,left,top,width,height,conf.

    Kolonlar x-bosluklarindan cikarilir; duz metin OCR'un tek-bosluk
    birlestirmesi tablo sutunlarini kaybettirir.
    """
    out_base = frame.with_suffix("")
    cmd = [tesseract, str(frame), str(out_base), "-l", lang, "--psm", "6", "tsv"]
    env = os.environ.copy()
    if tessdata:
        env["TESSDATA_PREFIX"] = tessdata
        env["MIMO_TESSDATA"] = tessdata
    kwargs = dict(stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout, env=env, check=False)
    if sys.platform == "win32":
        kwargs["creationflags"] = 0x08000000
    try:
        subprocess.run(cmd, **kwargs)
    except Exception:
        return []
    tsv = out_base.with_suffix(".tsv")
    if not tsv.exists():
        return []
    try:
        raw = tsv.read_text(encoding="utf-8", errors="replace")
    finally:
        try:
            tsv.unlink()
        except OSError:
            pass
    words: list[dict] = []
    reader = csv.DictReader(raw.splitlines(), delimiter="\t")
    for row in reader:
        try:
            if int(row.get("level", 0)) != 5:
                continue
        except (ValueError, TypeError):
            continue
        text = (row.get("text") or "").strip()
        if not text:
            continue
        try:
            words.append({
                "text": text,
                "left": int(row.get("left", 0)),
                "top": int(row.get("top", 0)),
                "width": int(row.get("width", 0)),
                "height": int(row.get("height", 10) or 10),
                "conf": float(row.get("conf", 0) or 0),
            })
        except (ValueError, TypeError):
            continue
    return words


def cluster_word_lines(words: list[dict]) -> list[list[dict]]:
    """Kelimeleri satirlara topla (dikey ortalamaya gore)."""
    if not words:
        return []
    ordered = sorted(words, key=lambda w: (w["top"] + w["height"] / 2, w["left"]))
    lines: list[list[dict]] = []
    cur: list[dict] = []
    for w in ordered:
        if not cur:
            cur = [w]
            continue
        ref = cur[0]
        ref_cy = ref["top"] + ref["height"] / 2
        h = max(ref["height"], w["height"], 1)
        w_cy = w["top"] + w["height"] / 2
        if abs(w_cy - ref_cy) <= 0.55 * h:
            cur.append(w)
        else:
            lines.append(sorted(cur, key=lambda x: x["left"]))
            cur = [w]
    if cur:
        lines.append(sorted(cur, key=lambda x: x["left"]))
    return lines


def words_to_cells(
    lines: list[list[dict]],
    page_width: int = 0,
) -> list[list[str]]:
    """Satir ici x-bosluklarindan hucrelere bol.

    Esik: sayfa genisliginin ~%2'si (en az 15, en cok 60 px). Kelime-arasi
    normal bosluklar (~5-12 px) birlesir, kolon bosluklari ayrilir.
    """
    if page_width <= 0:
        page_width = max(
            (w["left"] + w["width"] for line in lines for w in line),
            default=1280,
        )
    gap_thr = min(60, max(15, int(page_width * 0.02)))
    rows: list[list[str]] = []
    for line in lines:
        cells: list[str] = []
        cur: list[str] = []
        prev_end = None
        for w in line:
            if prev_end is not None and w["left"] - prev_end > gap_thr:
                if cur:
                    cells.append(" ".join(cur))
                cur = []
            cur.append(w["text"])
            prev_end = w["left"] + w["width"]
        if cur:
            cells.append(" ".join(cur))
        if cells:
            rows.append(cells)
    return rows


def norm_line(s: str) -> str:
    s = s.replace(" ", " ")
    s = re.sub(r"\s+", " ", s).strip()
    return s


def line_key(s: str) -> str:
    s = norm_line(s).lower()
    s = re.sub(r"[^\w\u00c0-\u024f]+", "", s)
    return s


def digit_signature(key: str) -> str:
    """4+ haneli sayi gruplari (fatura/VKN ID'leri).

    Ayni satirin farkli OCR okumalarinda harfler bozulsa da sayilar
    genelde sabit kalir. Imza farkliysa FARKLI satirdir (tek hanesi
    farkli iki fatura asla birlesmez).
    """
    return "|".join(re.findall(r"\d{4,}", key))


class FuzzySeen:
    """Geri kaydirmada ayni satirin bozuk tekrarlarini eler.

    - Birebir ayni anahtar → tekrar.
    - Sayisal imzasi ayni + benzerlik ≥0.88 → tekrar (harf hatalari).
    - Imzasiz satirlar (baslik/metin) + benzerlik ≥0.92 → tekrar.
    - Imzasi farkliysa her zaman AYRI satir (muhasebe guvenligi).
    """

    def __init__(self, recent: int = 400):
        self.exact: set[str] = set()
        self.sig_buckets: dict[str, list[str]] = {}
        self.nosig: list[str] = []
        self.recent = recent

    def add(self, key: str) -> bool:
        """Yeni ise True (kaydeder), tekrar ise False."""
        if not key or key in self.exact:
            return False if key else False
        sig = digit_signature(key)
        if sig:
            bucket = self.sig_buckets.setdefault(sig, [])
            for old in bucket[-60:]:
                if difflib.SequenceMatcher(None, key, old).ratio() >= 0.88:
                    self.exact.add(key)
                    return False
            bucket.append(key)
        else:
            for old in self.nosig[-self.recent:]:
                if difflib.SequenceMatcher(None, key, old).ratio() >= 0.92:
                    self.exact.add(key)
                    return False
            self.nosig.append(key)
            if len(self.nosig) > self.recent + 100:
                del self.nosig[:100]
        self.exact.add(key)
        return True


def merge_scrolled_texts(texts: list[str]) -> str:
    """Kareleri birlestir; scroll geri-donuslerinin bozuk tekrarlarini ele."""
    if not texts:
        return ""
    seen = FuzzySeen()
    merged: list[str] = []
    prev_keys: list[str] = []
    for text in texts:
        lines = [norm_line(x) for x in text.splitlines() if norm_line(x)]
        keys = [line_key(x) for x in lines]
        # Kare neredeyse oncekinin aynisiyla (scroll durakladi) atla.
        if keys and keys == prev_keys:
            continue
        prev_keys = keys
        for line, key in zip(lines, keys):
            if not key:
                continue
            if seen.add(key):
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


def write_xlsx(rows: list[list[str]], dest: Path) -> None:
    try:
        from openpyxl import Workbook
    except ImportError as e:
        raise RuntimeError(f"openpyxl missing: {e}") from e
    wb = Workbook()
    ws = wb.active
    ws.title = "OCR"
    for row in rows:
        ws.append(row)
    wb.save(str(dest))
    wb.close()


def main() -> int:
    ap = argparse.ArgumentParser(description="Video → CSV/TXT extract (Mimo OCR)")
    ap.add_argument("--video", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--lang", default="tur+eng")
    ap.add_argument("--mode", default="auto", choices=["auto", "fast", "slow"])
    ap.add_argument("--max-frames", type=int, default=180)
    ap.add_argument("--formats", default="csv,txt", help="comma list: csv,txt,md,xlsx")
    ap.add_argument("--mask", default="", help="x,y,w,h blackout (rec control window)")
    args = ap.parse_args()

    video = Path(args.video)
    if not video.is_file():
        print(f"video not found: {video}", file=sys.stderr)
        return 1
    if video.stat().st_size == 0:
        print(f"video boş (0 bayt): {video}", file=sys.stderr)
        return 1
    out_dir = Path(args.out)
    try:
        out_dir.mkdir(parents=True, exist_ok=True)
    except OSError as e:
        print(f"çıktı klasörü açılamadı: {out_dir}: {e}", file=sys.stderr)
        return 1

    ff_path, ff_ok, ff_ver = probe_tool("ffmpeg")
    fp_path, fp_ok, fp_ver = probe_tool("ffprobe")
    _p(f"[ffmpeg] {ff_path} :: {ff_ver}")
    _p(f"[ffprobe] {fp_path} :: {fp_ver}")
    if not ff_ok:
        print(
            "ffmpeg çalışmıyor (kurulumun ffmpeg/ klasörü eksik ya da "
            "Windows Güvenliği engelliyor olabilir)",
            file=sys.stderr,
        )
        return 1

    tesseract = find_tesseract()
    tessdata = tessdata_dir(args.lang)
    lang = args.lang.replace(",", "+")

    frames_dir = Path(tempfile.mkdtemp(prefix="mimo-vid-"))
    try:
        _p(f"[start] {video.name}")
        _p("PROGRESS 0/1")
        _p("[ffmpeg] extracting frames...")
        frames = extract_frames_adaptive(
            video, frames_dir, mode=args.mode, max_frames=args.max_frames,
            mask=args.mask,
        )
        _p(
            f"[video] duration~{video_duration_sec(video):.1f}s frames={len(frames)} mode={args.mode}"
        )
        _p(f"PROGRESS 0/{max(1, len(frames))}")

        texts: list[str] = []
        cell_rows: list[list[str]] = []
        seen_rows = FuzzySeen()
        n = len(frames) or 1
        for i, fr in enumerate(frames, 1):
            _p(f"PROGRESS {i}/{n}")
            _p(f"[ocr] {i}/{n} {fr.name}")
            words = ocr_frame_words(fr, tesseract, lang, tessdata)
            try:
                fr.unlink()
            except OSError:
                pass
            if words:
                wlines = cluster_word_lines(words)
                rows = words_to_cells(wlines)
                # TXT/MD icin hucreleri tek boslukla birlestir (eski duz-metin bicimi).
                flat = [" ".join(r) for r in rows]
                flat = [ln for ln in flat if norm_line(ln)]
                if flat:
                    texts.append("\n".join(flat))
                    _p(f"[ok] {fr.name}: {len(flat)} ln")
                else:
                    _p(f"[empty] {fr.name}")
                for r in rows:
                    rk = line_key(" ".join(r))
                    if rk and seen_rows.add(rk):
                        cell_rows.append(r)
            else:
                # TSV bossa yedek: duz metin (gorsel agirlikli kareler).
                t = ocr_frame(fr, tesseract, lang, tessdata)
                if t.strip():
                    texts.append(t)
                    _p(f"[ok] {fr.name}: {len(t)} ch (txt)")
                else:
                    _p(f"[empty] {fr.name}")

        _p("[merge] de-duplicating scroll overlap...")
        _p(f"PROGRESS {n}/{n}")
        merged = merge_scrolled_texts(texts)
        lines = [ln for ln in merged.splitlines() if ln.strip()]
        tabular = looks_tabular(lines) or any(len(r) > 1 for r in cell_rows)
        _p(f"[merge] frames_ocr={len(texts)} lines={len(lines)} rows={len(cell_rows)} tabular={tabular}")

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

        if "csv" in formats or "xlsx" in formats:
            # TSV kolonlari: her satir zaten hucrelerine ayrilmis + bulanik deduplu.
            width = 0
            for r in cell_rows:
                width = max(width, len(r))
            padded = [r + [""] * (width - len(r)) for r in cell_rows]
            if not padded and lines:
                padded = [[ln] for ln in lines]
        if "csv" in formats:
            p = out_dir / f"{stem}.video.csv"
            with p.open("w", encoding="utf-8-sig", newline="") as f:
                w = csv.writer(f)
                for row in padded:
                    w.writerow(row)
            written.append(str(p))
            _p(f"[csv] rows={len(padded)} cols={width if cell_rows else 0} tabular={tabular} -> {p}")

        if "xlsx" in formats:
            p = out_dir / f"{stem}.video.xlsx"
            try:
                write_xlsx(padded, p)
                written.append(str(p))
                _p(f"[xlsx] rows={len(padded)} -> {p}")
            except RuntimeError as e:
                _p(f"[warn] xlsx atlandi: {e}")

        _p("[done] files: " + " | ".join(written))
        return 0
    finally:
        shutil.rmtree(frames_dir, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
