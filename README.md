# Mimo OCR

**Açık kaynak · Tamamen çevrimdışı · Çok motorlu masaüstü OCR**
**Open-source · Fully offline · Multi-engine desktop OCR**

Ekrandaki, dosyalardaki ve videolardaki metni yakalayıp **TXT / Markdown / PDF** olarak dışa aktarır.
Çıktı özellikle **LLM / yapay zekâ** analizine hazırdır. Telemetri yoktur, hesap gerekmez, internet gerekmez.

Capture text from screen, files and videos. Export as **TXT / Markdown / PDF**, ready for **LLM / AI**
analysis. No telemetry, no account, no internet required.

> Geliştirici / Developer: **[Mesut Demirci](https://www.linkedin.com/in/mesutdemirci/)** ·
> Yapay zekâ desteği / AI assistance: Xiaomi MiMo Developers — **MiMo-X Pro**

---

## İndir / Download (v0.6.0)

| Paket / Package | Dosya / File |
|---|---|
| Kurulum (NSIS) | `Mimo.OCR_0.6.0_x64-setup.exe` |
| Kurulum (MSI) | `Mimo.OCR_0.6.0_x64_en-US.msi` |
| Taşınabilir / Portable | `MimoOCR-portable-windows-x64-v0.6.0.zip` |

Hepsi [Releases](https://github.com/mesutde/MimoOcr/releases) sayfasında. / All on [Releases](https://github.com/mesutde/MimoOcr/releases).

## Yenilikler (v0.6.0)

- **Web sekmesi (yeni):** Bağlantıyı yapıştırın, tür otomatik saptanır —
  Google Tablosu → CSV / Excel (XLSX) / Markdown; Google Belgesi (Docs) →
  DOCX / ODT / TXT / PDF / Markdown; direkt dosya bağlantıları türe göre
  OCR/metin hattına girer. Editör sayfaları (ONLYOFFICE vb.) için net mesaj.
- **Dinamik OCR dili:** Dil listesi seçili motora göre dizilir —
  Tesseract'ta tur/eng/ara/chi_sim/chi_tra/jpn/kor (+tur+eng), Windows OCR'da sistem paketleri.
- Kalan her şey v0.5.5 ile aynı.

## What's New (v0.6.0)

- **New Web tab:** paste a link, type auto-detected — Google Sheets → CSV / Excel (XLSX) / Markdown;
  Google Docs → DOCX / ODT / TXT / PDF / Markdown; direct file links route into OCR/text pipelines.
- **Dynamic OCR languages:** the list follows the active engine.
- Everything else as in v0.5.5.

## Yenilikler (v0.5.5)

- **Çift monitör / HiDPI isabeti:** Overlay sanal ekranı artık her yorumda tam kaplar
  (konum minimumu, boyut maksimumu — sağda boşluk yok); fare koordinatı pencerenin
  gerçek konumu + ölçeğinden fiziğe çevrilir, kesişim saf fiziksel uzayda yapılır.
  Tek monitör davranışı değişmez.
- **Tepsi tek tık:** Sol tık uygulamayı öne getirip doğrudan bölge yakalamayı başlatır.
- Kalan her şey v0.5.4 ile aynı.

## What's New (v0.5.5)

- **Dual-monitor / HiDPI accuracy:** overlay covers the virtual screen under every
  interpretation (no right-side gap); cursor maps via live window geometry in pure
  physical space. Single-monitor behavior unchanged.
- **Tray single-click** opens the app straight into region capture.
- Everything else as in v0.5.4.

## Yenilikler (v0.5.4)

- **Klasik arayüz:** Yatay Yakala / Belge / Toplu / UDF / Video sekmeleri, motor seçici (Tesseract + Windows OCR),
  TR/EN, koyu + açık tema, başlıkta sürüm.
- **Sürükle-bırak:** Dosyaları sekmelere bırakın — Yakala'ya resim direkt OCR'lanır.
- **Tepsi:** `✕` tepsiye gizler, tek-örnek koruması, "Tümünü Temizle".
- **Belge:** Büyük belgelerde donma yok (görünüm sınırlı, tamamı kayda hazır) + Temizle.
- **Sağ-tık:** İmleçte menü — motorla yeniden OCR + resmi panoya kopyalama.
- **Temiz yakalama:** Seçim kutusu kareye sızmaz (gizlenme onayı + bekleme payı).
- **Önizleme zoom:** Fare tekerleğiyle yakınlaştırma, çift tık sıfırlama.
- **UDF → PDF:** Yapısal dönüştürücü + başlık kutucuğu + birleşik kipte aynı kalite.
- **Tamamen çevrimdışı:** Telemetri/güncelleyici yok; Tesseract + ffmpeg + Python gömülü.

## What's New (v0.5.4)

- Classic tabbed UI, multi-engine, drag-drop, tray with hide + single instance + clear-all,
  artifact-free capture, preview zoom, structured UDF→PDF, fully offline with bundled runtimes.

## Yenilikler (v0.5.3)

- **Video ffmpeg seçimi düzeltildi:** Betik, gömülü `ffmpeg/ffprobe`'u dosya olarak doğrulanmış
  **mutlak yolla** seçer (önce kurulum dizini, sonra PATH); göreli/bozuk eşleşmeler elenir.
  `[ffmpeg]` log satırı artık gerçek yolu gösterir.
- Kalan her şey v0.5.2 ile aynı.

## What's New (v0.5.3)

- **Video ffmpeg resolution fixed:** bundled `ffmpeg`/`ffprobe` are picked by verified absolute path
  (install dir first, then PATH); relative/broken matches are rejected.
- Everything else as in v0.5.2.

## Yenilikler (v0.5.2)

- **Video sağlamlığı:** Betik başlangıçta çözümlenen `ffmpeg`/`ffprobe` yolunu ve sürümünü loglar;
  video yok/0-bayt ve çıktı klasörü sorunları net mesajla erken biter.
- **Zengin video hatası:** Başarısızlıkta `returncode` + ffmpeg yolu + stderr gösterilir;
  stderr boşsa Defender/TEMP ipucu eklenir. Filtre seviyesi `warning` yapıldı, sade filtreyle yedek deneme var.
- **stdin düzeltmesi:** Pencereli uygulamadan doğan tüm alt süreçlerin (Python/Tesseract/ffmpeg/reg)
  stdin'i kapalı — takılma/abort ihtimali kalktı. Video-log'a `[python]`/`[script]` satırları eklendi.
- Kalan her şey v0.5.1 ile aynı.

## What's New (v0.5.2)

- **Video robustness:** resolved `ffmpeg`/`ffprobe` paths + versions are logged; clear early errors.
- **Rich video errors:** returncode + ffmpeg path + stderr (Defender/TEMP hint when empty); warning-level
  ffmpeg output; one simplified-filter retry.
- **stdin fix:** all child processes get null stdin. Everything else as in v0.5.1.

## Yenilikler (v0.5.1)

- **Gömülü Python 3.12:** Video/Toplu/UDF/PDF akışları artık kurulumla gelen Python ile çalışır
  (reportlab + pypdf + openpyxl dahil). **Başka PC'de Python kurmanıza gerek yok.**
- **Store-sahte koruması:** `WindowsApps` Python sahtesi bilerek atlanır; ne seçimde ne denetimde Store penceresi açılmaz.
- Kalan her şey v0.5.0 ile aynı (klasik arayüz, çok motor, yapısal UDF→PDF, gömülü Tesseract + ffmpeg, tam offline).

## What's New (v0.5.1)

- **Embedded Python 3.12** (reportlab + pypdf + openpyxl): Video/Batch/UDF/PDF flows work out of the box.
  **No Python install needed on other PCs.**
- **Store-stub guard:** the `WindowsApps` fake Python is skipped everywhere.
- Everything else as in v0.5.0.

---

## Yenilikler (v0.5.0)

- **Klasik arayüz geri döndü:** Yatay sekmeler **Yakala / Belge / Toplu / UDF / Video**, üstte motor seçici, 🇹🇷/🇺🇸 dil düğmeleri, koyu + açık tema.
- **Çok motorlu:** Gömülü **Tesseract 5.4.0** + **Windows OCR** (sistem) — önizlemeye sağ tıkla başka motorla yeniden OCR.
- **Yakala:** Bölge seç (seçimde ana pencere gizlenir, bitince/iptalde geri gelir) · Tam ekran OCR · Önizlemede **Ctrl ile çoklu alan**.
- **Belge:** Görsel + PDF/DOCX/XLSX/PPTX/**UDF (UYAP)**/metin → tek görünümde metin.
- **Toplu + UDF:** Çoklu dosya (× ile tek tek silme), çıktı klasörü, TXT/Markdown/**PDF** (ayrı veya birleşik). UDF sekmesi `.udf` filtreli toplu dönüştürür; PDF çıktısı dosya adını başlık yapar (kapatılabilir kutucuk).
- **UDF → yapısal PDF:** Paragraf hizalama, kalın/italik/altı çizili ve **tablolar** korunur; birleşik PDF, ayrı PDF'lerin aynı kalitesinde birleştirilir.
- **Video → CSV/TXT:** Kaydırılan ekran kayıtlarından tablo/metin çıkarımı, canlı ilerleme.
- **Kutudan çıkar:** Tesseract runtime + tessdata + ffmpeg/ffprobe kurulumda gömülü. **Ayrıca kurulum gerekmez.**
- **Sağlamlık:** Açılış `%APPDATA%/mimo-ocr/startup.log` dosyasına yazılır; ölümcül hata mesaj kutusuyla gösterilir.

## What's New (v0.5.0)

- **Classic UI is back:** horizontal tabs **Capture / Document / Batch / UDF / Video**, engine picker in the header, 🇹🇷/🇺🇸 buttons, dark + light themes.
- **Multi-engine:** embedded **Tesseract 5.4.0** + **Windows OCR** — right-click the preview to re-OCR with another engine.
- **Capture:** region select (main window hides during selection) · fullscreen OCR · **Ctrl multi-area** in overlay.
- **Document:** images + PDF/DOCX/XLSX/PPTX/**UDF (UYAP)**/text → single text view.
- **Batch + UDF:** multi-file (× per row), output folder, TXT/Markdown/**PDF** (separate or combined). The UDF tab batch-converts `.udf`; PDF output uses the file name as title (toggleable).
- **Structured UDF → PDF:** paragraph alignment, bold/italic/underline and **tables** preserved; combined PDFs are merged from the same quality as separate ones.
- **Video → CSV/TXT** with live progress.
- **Batteries included:** Tesseract runtime + tessdata + ffmpeg/ffprobe ship in the installer. **No extra installs.**
- **Robustness:** startup goes to `%APPDATA%/mimo-ocr/startup.log`; fatal errors show a message box.

---

## Özellikler / Features

- Kısayol / Shortcut: **Ctrl+Shift+X** → bölge seç → OCR → sonuç otomatik panoda
- Sistem tepsisi / Tray: Bölge Yakala / Aç / Çıkış
- Monitör seçici (çoklu monitör), PSM 3/6/7/11, 1×/2×/3× ön işleme
- Dil: `tur`, `eng`, `tur+eng` (+ indirilebilir modeller); arayüz: TR/EN
- Kaydet: TXT/MD/JSON (yakala), TXT/MD/PDF (toplu/UDF), CSV/TXT(/MD) (video)
- Gizlilik / Privacy: 100% cihazda / on-device. Tek ağ kullanımı: **isteğe bağlı** dil modeli indirme (19 dil, SHA-256 doğrulamalı). / The only network use: **optional** language-model downloads.

---

## Rakip karşılaştırması / Comparison (Eylül 2026 itibarıyla / as of Sep 2026)

| | **Mimo OCR** | **Text Grab** | **NormCap** | **Umi-OCR** | **Capture2Text** |
|--|---|---|---|---|---|
| Lisans / License | Apache-2.0 | MIT | GPLv3 | GPLv3 | GPL |
| Platform | Windows (kurulumlu); Linux/mac kaynaktan | Windows | Win / macOS / Linux | Windows | Windows |
| Motorlar / Engines | Tesseract (gömülü) + Windows OCR | Tesseract + Windows OCR | Tesseract | PaddleOCR (+rapid) | Tesseract |
| Kurulumla gelir / Bundled | Tesseract + ffmpeg + tessdata | Kısmen (sisteme dayanır) | Genelde sistem Tesseract'ı | Evet (ağır paket) | Evet |
| Tamamen çevrimdışı / Offline | Evet / Yes | Evet | Evet | Evet | Evet |
| Türkçe önceliği / TR-first | Evet (tur+eng gömülü) | Kısmi | Dil paketine bağlı | Kısmi (Çince ağırlıklı) | Kısmi |
| Çoklu alan (Ctrl) | Evet | Sınırlı | Temel | Evet | Hayır |
| Sağ-tık motor değiştirme | Evet | Hayır | Hayır | Hayır | Hayır |
| Toplu + LLM çıktısı (TXT/MD) | Evet | Zayıf | Hayır | Evet | Hayır |
| PDF çıktısı | Evet | Hayır | Hayır | Evet | Hayır |
| UDF (UYAP) + yapısal PDF | Evet | Hayır | Hayır | Hayır | Hayır |
| Video → CSV/TXT | Evet | Hayır | Hayır | Hayır | Hayır |
| Taşınabilir / Portable | Evet (ZIP) | Evet | Kısmi | Evet | Evet |

**Mimo OCR farkı:** rakiplerin güçlü yanlarını (Text Grab motor çeşitliliği, NormCap yerelliği, Umi-OCR toplu çıktı) tek çatıda toplar; üzerine **UYAP UDF desteği**, **video → tablo** ve **LLM'e hazır toplu çıktı** ekler.
**In short:** the strengths of the alternatives in one app, plus UYAP UDF support, video → table, and LLM-ready batch output.

---

## Kurulum / Install (Windows)

**NSIS:** `Mimo.OCR_0.5.0_x64-setup.exe` — çift tıkla, kur, aç. / Double-click, install, open.
**MSI:** `Mimo.OCR_0.5.0_x64_en-US.msi` — kurumsal dağıtım / enterprise deployment.
**Portable:** ZIP'i aç, `mimo-ocr.exe` çalıştır. Yanında `tessdata/`, `tesseract-runtime/`, `ffmpeg/`, `models.json`, `scripts/` gelir. / Unzip, run `mimo-ocr.exe`.

> Not / Note: Video sekmesi **kurulumla gelen Python'u** kullanır (gerekirse `MIMO_PYTHON` ile başka yol verilebilir). Toplu Office/PDF için ek kuruluma gerek yoktur (reportlab/pypdf/openpyxl gömülü). / The Video tab uses the **bundled Python** (`MIMO_PYTHON` overrides). No extra installs needed.

---

## Geliştirme / Development

```powershell
npm install
npm run tauri dev      # arayüz + debug exe (devUrl: localhost:1420)
```

Test / Tests:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib   # TSV + TR OCR + Windows OCR + tessdata
python -m py_compile scripts/*.py
```

Release:

```powershell
npm run tauri build     # MSI + NSIS → src-tauri/target/release/bundle
```

---

## Linux'ta derleme / Building on Linux

> Resmî Linux paketi henüz yok; kaynaktan derleyin. / No official Linux package yet; build from source.
> Windows OCR motoru Windows'a özeldir (Linux'ta gizlenir); gömülü runtime/ffmpeg Windows içindir —
> Linux'ta sistem `tesseract` + `ffmpeg` + `python3` kullanılır. / Windows OCR is Windows-only;
> on Linux the system `tesseract` + `ffmpeg` + `python3` are used.

```bash
sudo apt update
sudo apt install -y build-essential curl wget file pkg-config \
  libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
  libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev \
  tesseract-ocr tesseract-ocr-tur tesseract-ocr-eng \
  ffmpeg python3 python3-pip
pip3 install --user pypdf python-docx openpyxl reportlab pillow
curl -fsSL https://rustup.rs | sh   # Rust stable
# Node.js 20+: https://nodejs.org
export MIMO_TESSDATA=/usr/share/tesseract-ocr/5/tessdata   # dağıtımınıza göre ayarlayın
npm install
npm run tauri build
# Çıktı: src-tauri/target/release/bundle/deb|appimage
```

Notlar / Notes:
- **Wayland:** ekran yakalama için XDG Desktop Portal gerekir; X11 daha öngörülebilir. / Screen capture needs XDG Desktop Portal; X11 is more predictable.
- Ctrl+Shift+X kısayolu masaüstüne göre değişir. / The global shortcut varies by desktop.
- `tessdata` bulunamazsa `MIMO_TESSDATA` ile yol gösterin. / Point `MIMO_TESSDATA` at your tessdata.

## macOS'te derleme / Building on macOS

```bash
xcode-select --install
curl -fsSL https://rustup.rs | sh   # Rust stable
brew install node tesseract tesseract-lang ffmpeg python
pip3 install pypdf python-docx openpyxl reportlab pillow
export MIMO_TESSDATA=$(brew --prefix)/share/tessdata
npm install
npm run tauri build
# Çıktı: src-tauri/target/release/bundle/dmg
```

Notlar / Notes:
- Dağıtım için Apple Developer imza + noter gerekir (bu derleme imzasızdır). / Distribution needs Apple signing + notarization.
- İlk çalıştırmada **Ekran Kaydı** izni verin: Sistem Ayarları → Gizlilik → Ekran Kaydı. / Grant Screen Recording permission on first run.
- Apple Silicon (M1+): native derlenir; Intel: Rosetta 2 ile çalışır. / Native on Apple Silicon.

---

## Mimari / Architecture

```
src/                 Vite + TS ön yüz (sekmeler, i18n TR/EN, tema)
src-tauri/src/       Tauri 2 kabuk: lib.rs, commands.rs, batch.rs, video.rs,
                     engine.rs (Tesseract), engine_winocr.rs, capture.rs,
                     models.rs (19 dil), startup_log.rs
scripts/             batch_extract.py (PDF/DOCX/XLSX/PPTX/UDF/metin)
                     text_to_pdf.py · udf_to_pdf.py (yapısal) · merge_pdfs.py
                     video_extract.py (ffmpeg + Tesseract)
assets/tesseract-runtime  Tesseract 5.4.0 (Windows, kuruluma gömülü)
assets/ffmpeg             ffmpeg 9.0.2 + ffprobe (kuruluma gömülü)
assets/python             Python 3.12 + reportlab/pypdf/openpyxl (kuruluma gömülü)
tessdata/            tur + eng + osd (+ configs)
```

Ortak sonuç modeli: tüm motorlar `OcrDocument` üretir; arayüz motora bağlanmaz.
All engines produce `OcrDocument`; the UI never depends on an engine.

---

## Yol haritası / Roadmap

- [x] v0.3.0 — model yöneticisi, çoklu monitör, güven skoru
- [x] v0.4.0 — Video → CSV/TXT
- [x] v0.4.1 — gömülü Tesseract, dayanıklı açılış
- [x] v0.4.2 — açılış crash düzeltmesi, gömülü ffmpeg, startup.log
- [x] **v0.5.0 — klasik arayüz, 3 yakalama modu, çok motor, Belge/Toplu/UDF, PDF çıktısı**
- [x] **v0.6.0 — Web sekmesi (Google Sheets → CSV/XLSX/MD), dinamik motor dilleri**
- [ ] macOS/Linux resmî paketleri
- [ ] `ort` + PaddleOCR "Yüksek Doğruluk" motoru
- [ ] Aranabilir PDF (görüntü + gizli metin katmanı)

---

## Lisans / License

Apache License 2.0 — bkz. / see [LICENSE](./LICENSE).
Üçüncü taraf bildirimleri / Third-party notices: [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)
(Tesseract Apache-2.0 · FFmpeg GPLv3 · dil verileri / language data).

---

## İletişim / Links

| | |
|--|--|
| Geliştirici / Developer | [Mesut Demirci](https://www.linkedin.com/in/mesutdemirci/) |
| GitHub | [github.com/mesutde/MimoOcr](https://github.com/mesutde/MimoOcr) |
| Yapay zekâ / AI | Xiaomi MiMo Developers — **MiMo-X Pro** |

*Mimo OCR — ekrandaki metin, anında sizin; analiz için hazır. / Text on screen, instantly yours — ready to analyze.*
