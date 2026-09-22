# Mimo OCR

Açık kaynak, çevrimdışı, Türkçe + İngilizce öncelikli masaüstü OCR aracı.
Tauri 2 (TypeScript ön yüz) + Rust çekirdeği. Varsayılan motor: Tesseract 5 (CLI bağdaştırıcısı).

*Open-source, offline, Turkish + English first desktop OCR tool. Tauri 2 (TypeScript frontend) + Rust core. Default engine: Tesseract 5 (CLI adapter).*

## İndir / Download (v0.4.1)

| Paket | Dosya |
|---|---|
| Kurulum (NSIS) | `Mimo.OCR_0.4.1_x64-setup.exe` |
| Kurulum (MSI) | `Mimo.OCR_0.4.1_x64_en-US.msi` |
| Taşınabilir | `MimoOCR-portable-windows-x64-v0.4.1.zip` |

Hepsi [Releases](https://github.com/mesutde/MimoOcr/releases) sayfasında.

## Sürüm 0.4.1 — Yenilikler / What's New

- **Tesseract artık kurulumla gömülü geliyor:** `tesseract-runtime/` (Tesseract 5.4.0 + tüm DLL'ler)
  MSI/NSIS/portable içinde. Başka bir PC'ye kurup açtığınızda ayrıca Tesseract kurmanıza
  **gerek yok** — uygulama kutudan çıktığı gibi OCR yapar. Öncelik sırası: `MIMO_TESSERACT` →
  kullanıcı yolu → gömülü runtime → sistem kurulumu → PATH.
- **Sessiz-ölüm düzeltmesi:** Tesseract bulunamasa bile uygulama artık **her zaman açılır**;
  eksikse uyarı bandı çıkar (indir bağlantısı + "Yolu Seç…" + "Tekrar Tara"). Eskiden motor
  yoksa pencere hiç oluşmuyordu.
- **Özel Tesseract yolu:** Dosya seçiciyle gösterilen `tesseract.exe` doğrulanıp
  `%APPDATA%/mimo-ocr/tesseract-path.txt` içinde kalıcı saklanır.
- Video betiği ve gereksinim denetimi de gömülü runtime'ı tanır.

*New in 0.4.1: embedded Tesseract 5.4.0 runtime in all packages (no separate install needed),
graceful startup with engine warning banner + rescan + custom path picker.*

## Sürüm 0.4.0 — Yenilikler / What's New

- **Video → CSV/TXT sekmesi:** Kaydırılan ekran kayıtlarından (Excel/CSV yerine video atanlar için)
  metin çıkarır. `ffmpeg` ile adaptif kare örnekleme (Otomatik / Hızlı scroll / Yavaş scroll) →
  kare kare Tesseract OCR (işlenen kare silinir, bellek dostu) → scroll tekrarı eleme →
  tablo gibiyse CSV, her koşulda TXT (istenirse MD). Kalite/Hız: ~120 / ~180 / ~280 kare.
  Canlı ilerleme çubuğu + log akışı; gereksinimler (`python`, `ffmpeg`, `tesseract`, betik)
  sekmede otomatik denetlenir (`video_support_info`).
- **Konsol penceresi titremesi giderildi:** Windows'ta Tesseract/Python alt süreçleri artık
  `CREATE_NO_WINDOW` ile çalışıyor; OCR ve video işlemede siyah konsol penceresi açılıp kapanmıyor.
- **Türkçe UDF/XML kodlaması düzeltmesi:** `batch_extract` XML çözümleme artık UTF-8 dışı
  (cp1254 / latin-1) içerikleri de doğru okuyor.
- **Kurulum paketleri kaynakları içeriyor:** MSI/NSIS/portable artık `tessdata/` (tur+eng+osd),
  `models.json` ve `scripts/video_extract.py` ile geliyor.
- **Tauri güncelleme altyapısı eşitlendi:** `tauri-plugin-updater` Rust tarafı 2.12'ye yükseltildi
  (npm ile sürüm uyumsuzluğu giderildi).

*New in 0.4.0: Video → CSV/TXT tab (adaptive ffmpeg frame sampling + per-frame Tesseract OCR
with scroll dedup, live progress/log, self-checked requirements), no more console flashing on
Windows (CREATE_NO_WINDOW), bundled tessdata/models.json/video script in all packages,
updater plugin version alignment.*

## Sürüm 0.3.0 — Yenilikler / What's New

- **Dil modeli yöneticisi:** 19 dil (Türkçe, İngilizce, Almanca, Fransızca, İspanyolca, İtalyanca,
  Portekizce, Azerbaycanca, Hollandaca, Lehçe, Rusça, Ukraynaca, Arapça, Farsça, Yunanca,
  Japonca, Korece, Basit/Geleneksel Çince) uygulama içinden indirilebilir.
  İndirmeler SHA-256 doğrulamalıdır, yarım dosyalar `.part` olarak yazılıp doğrulama sonrası
  atomik şekilde yerine taşınır. Temel modeller (`tur`/`eng`/`osd`) silmeye karşı korunur.
- **Çoklu monitör + DPI desteği:** Bölge seçim katmanı (overlay) artık tüm monitörlerin
  birleşimini (sanal masaüstü) kaplar; monitör başına DPI ölçeği hesaba katılır ve farklı
  ölçekli monitörler arasında kalan seçimler Lanczos3 yeniden örneklemeyle doğru birleştirilir.
- **Otomatik pano kopyalama:** OCR tamamlanınca sonuç otomatik olarak panoya yazılır (ayardan kapatılabilir).
- **Son bölgeyi yeniden OCR:** Son yakalanan bölge tek komutla tekrar işlenebilir.
- **Kelime düzeyi güven skoru:** Tesseract TSV çıktısından kelime bazlı güven ve koordinatlar;
  sonuç panelinde ortalama güven yüzdesi, kelime sayısı ve süre (ms) gösterilir.

*New in 0.3.0: in-app language model manager (19 languages, SHA-256 verified atomic downloads),
multi-monitor overlay with per-monitor DPI handling, auto-copy to clipboard, re-capture of the
last region, word-level confidence scores (TSV).*

## Özellikler

- Küresel kısayol: **Ctrl+Shift+X** → bölge seçim katmanı → OCR → sonuç otomatik panoda
- Sistem tepsisi: Bölge Yakala / Aç / Çıkış
- Bölge seçimi: sürükle-bırak, canlı boyut göstergesi, Esc ile iptal
- Görsel dosyası açma (PNG/JPG/BMP/WebP/GIF) ve panodaki görseli OCR'leme
- Dil: `tur`, `eng`, `tur+eng` ve model yöneticisiyle indirilen diller · PSM: 3/6/7/11 · Ön işleme: 1×/2×/3× büyütme
- Sonuç düzenleme, TXT/MD/JSON olarak kaydetme, kelime düzeyi güven skoru (TSV)
- Video → CSV/TXT: ekran kaydı videosundan tablo/metin çıkarımı (ffmpeg + Tesseract)
- Tamamen çevrimdışı; telemetri yok (yalnızca model indirme ağ kullanır)

## Gereksinimler

- [Tesseract 5](https://github.com/UB-Mannheim/tesseract/wiki) (Windows yükleyicisi)
- Rust (stable), Node.js 20+ (yalnızca geliştirme için)
- Dil verileri: depo kökündeki `tessdata/` klasörü (`tur`, `eng`, `osd` + `configs/`).
  Uygulama tessdata'yı şu sırayla çözer: `MIMO_TESSDATA` → uygulama dizini ve üst dizinlerde `tessdata/` → Tesseract kurulum dizini.

Ortam değişkenleri: `MIMO_TESSERACT` (tesseract.exe yolu), `MIMO_TESSDATA` (tessdata dizini).

**Taşınabilir sürüm:** ZIP'i açın; `mimo-ocr.exe` yanında `tessdata/`, `tesseract-runtime/`,
`models.json` ve `scripts/video_extract.py` hazır gelir — ayrıca kurulum gerekmez.
Video sekmesi için `ffmpeg`/`ffprobe` ve Python 3 gerekir
(`MIMO_PYTHON` ile yorumlayıcı yolu verilebilir).

## Geliştirme

```powershell
npm install
npm run tauri dev
```

## Test

```powershell
cd src-tauri
cargo test    # TSV ayrıştırma + uçtan uca Türkçe OCR (test-tr.png kullanır)
```

## Yol haritası (özet)

- ~~Aşama 2: model yöneticisi~~ → **v0.3.0'da tamamlandı** (otomatik güncelleme altyapısı eklendi, macOS/Linux bekliyor)
- ~~Video → CSV/TXT~~ → **v0.4.0'da tamamlandı** (adaptif kare örnekleme + scroll dedup + canlı ilerleme)
- Aşama 3: PDF, toplu işleme, aranabilir PDF
- Aşama 4: `ort` + PaddleOCR "Yüksek Doğruluk" motoru, akıllı yedekleme
- Aşama 5: tablo/form/düzen analizi, yerel çeviri, eklentiler

Ayrıntılar: `rapor.html`

## Lisans

Apache License 2.0 — bkz. `LICENSE`.

**Geliştirici:** Mesut Demirci · **Yapay zekâ desteği:** Xiaomi MiMo Developers — MiMo-X Pro
