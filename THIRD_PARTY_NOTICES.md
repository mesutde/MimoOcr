# Üçüncü Taraf Bildirimleri / Third-Party Notices

Mimo OCR (Apache-2.0) aşağıdaki üçüncü taraf yazılımları gömülü olarak dağıtır.

## Tesseract OCR 5.4.0 (+ Leptonica vb. DLL'ler)

- Lisans: **Apache License 2.0**
- Kaynak: https://github.com/tesseract-ocr/tesseract
- Windows derlemesi (UB-Mannheim topluluk derlemesi):
  https://github.com/UB-Mannheim/tesseract/wiki
- `assets/tesseract-runtime/` ve kurulu uygulamada `tesseract-runtime/` altındadır.

## FFmpeg 9.0.2 "essentials" (ffmpeg.exe + ffprobe.exe, static)

- Lisans: **GNU General Public License v3 (GPLv3)** — derleme `--enable-gpl` içerir.
- Kaynak kodu: https://ffmpeg.org/download.html
- Windows derlemesi (gyan.dev): https://www.gyan.dev/ffmpeg/builds/
- `assets/ffmpeg/` ve kurulu uygulamada `ffmpeg/` altındadır.
- Bu program yalnizca Video → CSV/TXT sekmesinde, ekran kaydi videolarindan
  kare cikarimi icin kullanilir; degistirilmemistir.
- GPLv3 metni: https://www.gnu.org/licenses/gpl-3.0.html
- Derleme ZIP SHA-256:
  `60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba`
  (derleme sırasında `src-tauri/build.rs` tarafından doğrulanır).

## Tesseract dil verileri (tessdata: tur/eng/osd + indirilebilir modeller)

- Kaynak: https://github.com/tesseract-ocr/tessdata_fast
- Lisans: Apache-2.0 (eğitim verisi lisansları için kaynağa bakın).

## Python 3.12 (Windows embeddable) + reportlab + pypdf + openpyxl (+ pillow)

- CPython lisansı: **Python Software Foundation License** — https://docs.python.org/3/license.html
- Kaynak: https://www.python.org/downloads/windows/ (embeddable zip)
- `assets/python/` ve kurulu uygulamada `python/` altındadır. Yalnızca Video/Toplu/UDF
  betiklerini çalıştırır; sisteme kurulmaz, PATH'i değiştirmez.
- reportlab (BSD): https://github.com/MrBitBucket/reportlab
- pypdf (BSD-3): https://github.com/py-pdf/pypdf
- openpyxl (MIT): https://github.com/chronossc/openpyxl
- pillow (HPND): https://github.com/python-pillow/Pillow (reportlab bağımlılığı)
