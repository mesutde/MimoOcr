# Phase 1 — Windows Alfa Notları

Tarih: 2026-09-18  
Durum: **derlendi ve çalıştırılabilir**

## Teslim edilenler

| Öğe | Durum |
|-----|-------|
| Tauri 2 masaüstü kabuğu | `src-tauri` → `mimo-ocr-app` |
| Ana pencere UI | `ui/index.html` (önizleme + metin + dil/monitör) |
| Bölge seçimi overlay | `ui/capture.html` |
| Tepsi menüsü | Tam ekran OCR · Bölge seç · Göster · Çıkış |
| Küresel kısayol | **Ctrl+Shift+O** → tam ekran OCR + pano |
| OCR komutları | `ocr_full_screen`, `ocr_region`, `ocr_path`, `copy_text` |
| Motor | `tesseract-cli` (TR/EN tessdata_fast proje içinde) |
| Gizlilik rozeti | UI + `offline: true` |

## Mimari

```
ui/ (WebView2)
  index.html      → sonuç + kontroller
  capture.html    → sürükle-bırak bölge overlay
        │ invoke / event
src-tauri/ (Rust)
  commands.rs     → capture + OcrEngine + clipboard
  lib.rs          → tray + global shortcut + pencere kapatma → gizle
crates/*          → Phase 0 motorları aynen kullanılıyor
```

## Çalıştırma

```powershell
cd C:\Users\Mesut\Desktop\MimoOcr
cargo run -p mimo-ocr-app
```

veya doğrudan:

```powershell
.\target\debug\mimo-ocr-app.exe
```

Not: tessdata çözümü `cwd` üzerinden `assets/models/tessdata` arar — projeden çalıştırın.

## Kullanım (alfa)

1. **Ctrl+Shift+O** veya araç çubuğunda **Tam ekran OCR**
2. Sonuç metni ekrana gelir ve **panoya** kopyalanır
3. **Bölge seç** → overlay’de sürükleyip bırakın
4. **Manuel bölge** → x,y,w,h fiziksel piksel
5. Tepsi simgesi: hızlı erişim / gizle / çıkış
6. Pencere X’e basınca **kapanmaz**, tepsiye iner (Çıkış ile kapanır)

## Aşama 1 kontrol listesi

- [x] Tauri 2 iskeleti + ikonlar
- [x] Sistem tepsisi + menü
- [x] Küresel kısayol Ctrl+Shift+O
- [x] Tam ekran OCR → pano
- [x] Bölge seçimi UI (overlay + manuel)
- [x] TR/EN dil seçimi
- [x] Motor/tessdata durumu footer’da
- [ ] MSI / NSIS paketi (`cargo tauri build` — CLI gerekli)
- [ ] App-local Tesseract ikilisi (dağıtım için)
- [ ] İlk kullanımda kısayol çakışma doğrulaması
- [ ] Çoklu monitör overlay (şimdilik monitör 0)
- [ ] TR karakter ön işleme (ÇĞİÖŞÜ regresyonu — Phase 0 bulgusu)

## Bilinen sınırlamalar

- Overlay şeffaflık Windows’ta WM/DPİ’ye göre kayabilir; gerekirse **Manuel bölge** kullanın
- `leptess` / PaddleOCR henüz bağlı değil — `tesseract-cli` yolu
- `cargo-tauri` kurulu değil; bundle için:
  ```powershell
  cargo install tauri-cli --locked
  cargo tauri build
  ```

## Sonraki (Aşama 2)

Çoklu platform beta, model yöneticisi, otomatik güncelleme; veya alfa polisajı (paket + app-local tesseract + kısayol ayarları).
