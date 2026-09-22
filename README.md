# Mimo OCR

Açık kaynak, çevrimdışı, Türkçe + İngilizce öncelikli masaüstü OCR aracı.
Tauri 2 (TypeScript ön yüz) + Rust çekirdeği. Varsayılan motor: Tesseract 5 (CLI bağdaştırıcısı).

*Open-source, offline, Turkish + English first desktop OCR tool. Tauri 2 (TypeScript frontend) + Rust core. Default engine: Tesseract 5 (CLI adapter).*

## İndir / Download (v0.3.0)

| Paket | Dosya |
|---|---|
| Kurulum (NSIS) | `Mimo.OCR_0.3.0_x64-setup.exe` |
| Kurulum (MSI) | `Mimo.OCR_0.3.0_x64_en-US.msi` |
| Taşınabilir | `MimoOCR-portable-windows-x64-v0.3.0.zip` |

Hepsi [Releases](https://github.com/mesutde/MimoOcr/releases) sayfasında.

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
- Tamamen çevrimdışı; telemetri yok (yalnızca model indirme ağ kullanır)

## Gereksinimler

- [Tesseract 5](https://github.com/UB-Mannheim/tesseract/wiki) (Windows yükleyicisi)
- Rust (stable), Node.js 20+ (yalnızca geliştirme için)
- Dil verileri: depo kökündeki `tessdata/` klasörü (`tur`, `eng`, `osd` + `configs/`).
  Uygulama tessdata'yı şu sırayla çözer: `MIMO_TESSDATA` → uygulama dizini ve üst dizinlerde `tessdata/` → Tesseract kurulum dizini.

Ortam değişkenleri: `MIMO_TESSERACT` (tesseract.exe yolu), `MIMO_TESSDATA` (tessdata dizini).

**Taşınabilir sürüm:** ZIP'i açın; `mimo-ocr.exe` yanında `tessdata/` ve `models.json` hazır gelir.
OCR için sistemde Tesseract 5 kurulu olmalı veya `MIMO_TESSERACT` ile yol gösterilmelidir.

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
- Aşama 3: PDF, toplu işleme, aranabilir PDF
- Aşama 4: `ort` + PaddleOCR "Yüksek Doğruluk" motoru, akıllı yedekleme
- Aşama 5: tablo/form/düzen analizi, yerel çeviri, eklentiler

Ayrıntılar: `rapor.html`

## Lisans

Apache License 2.0 — bkz. `LICENSE`.

**Geliştirici:** Mesut Demirci · **Yapay zekâ desteği:** Xiaomi MiMo Developers — MiMo-X Pro
