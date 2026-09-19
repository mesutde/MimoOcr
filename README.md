<img width="925" height="709" alt="1" src="https://github.com/user-attachments/assets/20b19cde-b12d-4311-a51e-263b5f052cd8" />
<img width="924" height="711" alt="2" src="https://github.com/user-attachments/assets/81c3e70f-ad19-4e6a-9edd-867ecd69b642" />
<img width="913" height="707" alt="3" src="https://github.com/user-attachments/assets/5b816af0-c3ce-4cc0-be28-6d1c98cf1218" />
<img width="922" height="709" alt="4" src="https://github.com/user-attachments/assets/2af98744-c6b2-4cf9-928c-c8b3d1ffb8c7" />


# Mimo OCR

**Açık kaynak · Yerel (cihaz üzerinde) · Çok motorlu masaüstü OCR**

Ekrandaki, görsellerdeki ve belgelerdeki metni yakalayıp **TXT / Markdown** olarak dışa aktarır.  
Çıktı özellikle **LLM / yapay zekâ** modellerine analiz girdisi olarak hazırlanmıştır.

> **Geliştirme notu:** Bu proje **Xiaomi MiMo Developers — MiMo-X Pro** yapay zekâ modeli ile geliştirilmiştir.  
> Geliştirici: **[Mesut Demirci](https://www.linkedin.com/in/mesutdemirci/)**

---

## English

**Open-source · On-device · Multi-engine desktop OCR**

Capture text from the screen, images, and documents. Export as **TXT / Markdown**, ready for **LLM / AI** analysis.

> Built with **Xiaomi MiMo Developers — MiMo-X Pro**.  
> Developer: **[Mesut Demirci](https://www.linkedin.com/in/mesutdemirci/)**

---

## Özellikler / Features

| Özellik / Feature | Açıklama / Description |
|-------------------|------------------------|
| **Bölge seç** | Masaüstünde sürükle-bırak ile alan OCR |
| **Ekran OCR** | Tam ekran tek tuş |
| **Önizleme seç** | Uygulama içi kopya; **Ctrl** ile çoklu alan |
| **Belge OCR** | PNG/JPG/BMP/TIFF dosya yolu |
| **Toplu** | Çoklu görsel + PDF/DOCX/XLSX/PPTX metni |
| **Çıktı** | TXT / Markdown · ayrı ayrı veya birleşik |
| **Motorlar** | Tesseract · Windows OCR · Mock (genişletilebilir) |
| **Sağ tık** | Kaynak görsele sağ tık → farklı motorla yeniden OCR |
| **Gizlilik** | Tamamen cihazda; telemetri yok |
| **UI dilleri** | EN, TR, ES, FR, DE, PT, ZH, JA, AR, RU, HI (bayraklı) |
| **Tema** | Koyu / açık |
| **Kısayol** | Ctrl+Shift+O (tam ekran OCR) |
| **Tepsi** | Sistem tepsisi menüsü |

---

## Rakip karşılaştırması / Comparison

| | **Mimo OCR** | **Text Grab** | **NormCap** | **MiniSnip** |
|--|--------------|---------------|-------------|--------------|
| **Lisans / License** | Apache-2.0 | MIT | GPLv3 | GPLv3 |
| **Platform** | Windows (macOS/Linux build notu ↓) | Windows | Win / macOS / Linux | Windows |
| **Motorlar / Engines** | Tesseract, Windows OCR, (Paddle/ort yol haritası) | Tesseract, Windows OCR/AI | Tesseract ağırlıklı | Dahili / sınırlı |
| **TR+EN varsayılan paket** | Evet (tessdata gömülü hazır) | Kısmi | Genelde EN | Dil paketine bağlı |
| **Çoklu alan (Ctrl)** | Evet | Sınırlı | Temel | Hayır |
| **Sağ tık motor değiştirme** | Evet | Hayır | Hayır | Hayır |
| **Toplu + TXT/MD (LLM)** | Evet | Zayıf | Hayır | Hayır |
| **PDF/Office metin** | Evet (toplu) | Sınırlı | Hayır | Hayır |
| **Yerel / offline** | Evet | Evet | Evet | Evet |
| **UI sadeliği** | Sekmeli, motor seçimi header’da | Özellik yoğun | Sade | Çok sade |
| **Hedef kullanıcı** | LLM girdisi + günlük OCR | Windows iş akışı | Hızlı panoya kopyala | Hızlı snip |

**Mimo OCR farkı:** Text Grab kadar motor çeşitliliği + NormCap kadar yerel çalışma + MiniSnip kadar düşük sürtünme; üzerine **toplu LLM çıktısı** ve **çok motorlu sağ tık yeniden işleme**.

---

## Kurulum / Install

### Windows — Setup (MSI / NSIS)

Releases sayfasından indirin:

- `Mimo OCR_0.1.0_x64_en-US.msi`
- `Mimo OCR_0.1.0_x64-setup.exe` (NSIS)

Kurulumdan sonra masaüstü kısayolu / Başlat menüsü.

**Not:** OCR için sistemde Tesseract 5 kurulu olmalıdır (varsayılan `C:\Program Files\Tesseract-OCR`) **veya** app-local `tesseract` + `tessdata` paketini uygulamanın yanına koyun.  
Proje `assets/tesseract-runtime` ve `assets/models/tessdata` (tur+eng) ile gelir; geliştirme kopyasında çalıştırırken yeterlidir.

### Windows — Portable (ZIP)

`MimoOCR-portable-windows-x64.zip` içeriğini çıkarın:

```
mimo-ocr-app.exe
tesseract-runtime\   (opsiyonel — yoksa sistem Tesseract kullanılır)
tessdata\            (tur.traineddata, eng.traineddata)
ui\ (gerekirse)
```

`mimo-ocr-app.exe` çalıştırın. `TESSDATA_PREFIX` gerekirse `tessdata` klasörünü gösterin veya yanına koyun.

```powershell
# Portable örnek
$env:TESSDATA_PREFIX = "C:\MimoOCR\tessdata"
$env:MIMO_TESSERACT   = "C:\MimoOCR\tesseract-runtime\tesseract.exe"
.\mimo-ocr-app.exe
```

---

## Geliştirme / Development

### Ön koşullar

- Rust (stable)
- Node.js 18+ (Tauri CLI için)
- Windows: WebView2 (sistemde genelde vardır)
- Tesseract 5 (önerilen): [UB Mannheim](https://github.com/UB-Mannheim/tesseract/wiki)
- Python 3 + Pillow (test verisi) · pypdf/python-docx (toplu belge)

```powershell
cd MimoOcr
npm install
cargo test --workspace
cargo run -p mimo-ocr-app
```

CLI:

```powershell
cargo run -p mimo-ocr-cli -- doctor
cargo run -p mimo-ocr-cli -- engines
cargo run -p mimo-ocr-cli -- ocr .\datasets\tr-en\synthetic\tr_invoice.png --langs tur,eng
cargo run -p mimo-ocr-cli -- batch .\datasets\tr-en\synthetic
```

---

## Release build (Windows)

```powershell
npm install
# Release binary
cargo build -p mimo-ocr-app --release
# MSI + NSIS (tauri CLI)
npm run build:app
# veya
node_modules\.bin\tauri.cmd build
```

Çıktılar:

```
target\release\mimo-ocr-app.exe
target\release\bundle\msi\Mimo OCR_0.1.0_x64_en-US.msi
target\release\bundle\nsis\Mimo OCR_0.1.0_x64-setup.exe
```

**Portable ZIP** hazırlamak için:

```powershell
$staging = "$env:TEMP\MimoOCR-portable"
Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $staging | Out-Null
Copy-Item target\release\mimo-ocr-app.exe $staging\
Copy-Item assets\models\tessdata $staging\tessdata -Recurse
# İsteğe bağlı app-local Tesseract
Copy-Item assets\tesseract-runtime $staging\tesseract-runtime -Recurse
Copy-Item LICENSE $staging\
Copy-Item README.md $staging\
Compress-Archive -Path "$staging\*" -DestinationPath target\release\MimoOCR-portable-windows-x64.zip -Force
```

---

## Linux build

> Linux masaüstü paketi **henüz resmî release** değildir; kaynaktan derleyebilirsiniz.

### Bağımlılıklar (Debian/Ubuntu)

```bash
sudo apt update
sudo apt install -y \
  build-essential curl wget file \
  libssl-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev \
  libxdo-dev libxcb-randr0-dev \
  tesseract-ocr tesseract-ocr-tur tesseract-ocr-eng \
  python3 python3-pip
pip3 install pillow pypdf python-docx openpyxl
```

### Derleme

```bash
curl -fsSL https://rustup.rs | sh
# Node.js 18+ yükleyin
npm install
# Tauri Linux build
npx @tauri-apps/cli build
# veya npm run build:app
```

**Wayland:** Ekran yakalama için XDG Desktop Portal / PipeWire gerekli olabilir; X11 daha öngörülebilir.  
**Paket:** `target/release/bundle/deb|appimage|rpm` (tauri hedefine göre)  
**Flatpak:** Tauri Flatpak şablonu + `org.freedesktop.Platform` runtime.

### tesseract

```bash
sudo apt install tesseract-ocr tesseract-ocr-tur tesseract-ocr-eng
# veya projenin assets/models/tessdata klasörünü
export TESSDATA_PREFIX=/path/to/MimoOcr/assets/models/tessdata
```

---

## macOS build

```bash
# Xcode CLT + Rust + Node
xcode-select --install
curl -fsSL https://rustup.rs | sh
brew install tesseract tesseract-lang
npm install
npx @tauri-apps/cli build
```

- **İmzalama/noter:** Dağıtım için Apple Developer sertifikası + noter gerekir.
- **Ekran kaydı izni:** Sistem Tercihleri → Güvenlik → Ekran Kaydı (ilgili izin sihirbazı).
- Çıktı: `target/release/bundle/dmg` veya `macos` uygulama paketi.
- Apple Vision motoru macOS’a özeldir; Windows release’te görünmez.

---

## Mimari / Architecture

```
ui/                  WebView arayüzü (vanilla JS + i18n)
src-tauri/           Tauri 2 kabuk: tray, shortcut, commands
crates/mimo-ocr-core     OcrEngine trait, OcrDocument, preprocess
crates/mimo-ocr-engines  tesseract-cli, windows-ocr, mock
crates/mimo-ocr-capture  xcap ekran yakalama
crates/mimo-ocr-cli      mimo CLI (doctor/ocr/bench/batch)
assets/models/tessdata   tur + eng (+ osd)
assets/tesseract-runtime İsteğe bağlı app-local Tesseract
scripts/batch_extract.py PDF/DOCX/XLSX metin çıkarımı
```

Ortak sonuç modeli: tüm motorlar `OcrDocument` / `OcrRegion` üretir; arayüz motora bağlanmaz.

---

## Kullanım özeti

| Eylem | Nasıl |
|-------|--------|
| Tam ekran | **Ekran OCR** veya **Ctrl+Shift+O** |
| Masaüstü bölge | **Bölge seç** → sürükle-bırak |
| Önizleme + çoklu alan | **Önizleme ile seç** → alan, **Ctrl** ile ekle → OCR |
| Belge | **Belge** sekmesi → yol veya **Dosya seç…** |
| Toplu LLM çıktısı | **Toplu** → dosyalar → format MD/TXT → kaydet |
| Motor değiştir | Header **Motor** veya kaynak görsele **sağ tık** |
| Çıkış | ✕ veya tepsi → Çıkış |

---

## Yol haritası (özet)

- [x] Phase 0 — motor arayüzü, tessdata, ölçüm
- [x] Phase 1 — Tauri UI, tepsi, kısayol, bölge OCR
- [x] Windows OCR motoru
- [x] Toplu MD/TXT + LLM çıktısı
- [x] i18n + tema + çok motorlu sağ tık
- [ ] leptess in-process Tesseract
- [ ] ort + PaddleOCR kalite paketi
- [ ] macOS/Linux resmî release
- [ ] ocrs (deneysel, Latin)

---

## Lisans / License

Apache License 2.0 — bkz. [LICENSE](./LICENSE)

Üçüncü taraf: Tesseract (Apache-2.0), Windows OCR (platform), `ort` (MIT), `ocrs` (MIT/Apache-2.0) vb.  
Dağıtımda `THIRD_PARTY_NOTICES` eklenmesi önerilir.

---

## Katkı / Contributing

Issue ve PR açabilirsiniz.  
Motor ekleme: `mimo-ocr-core::OcrEngine` implement edin → `EngineRegistry`’ye kaydedin → UI listesine otomatik düşer.

---

## İletişim / Links

| | |
|--|--|
| Geliştirici / Developer | [Mesut Demirci](https://www.linkedin.com/in/mesutdemirci/) |
| GitHub | [github.com/mesutde/MimoOcr](https://github.com/mesutde/MimoOcr) |
| Yapay zekâ / AI | Xiaomi MiMo Developers — **MiMo-X Pro** |

---

*Mimo OCR — ekrandaki metin, anında sizin; analiz için hazır.*
