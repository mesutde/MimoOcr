# Phase 1.5 — Polisaj Notları

Tarih: 2026-09-19

## Yapılanlar

### 1. Görüntü ön işleme (`mimo-ocr-core::preprocess`)
- Gri tonlama, kontrast, keskinleştirme
- Kalite modları: `fast` / `balanced` / `accurate` (2× upscale)
- UI’da **Kalite: Dengeli | Yüksek 2×**
- Kutular orijinal piksel uzayına geri ölçeklenir

### 2. Türkçe karakter (ÇĞİÖŞÜ) akıllı yedekleme
Bulgu: `tur+eng` karışımı büyük TR harflerini düzleştiriyor  
(`ÇGİÖŞÜ` → `CGIOSU`). Yalnızca `tur` ile doğru çıkıyor.

Çözüm (`tesseract-cli`):
1. İstenen dille OCR (örn. tur+eng)
2. Gerekirse `tur` ile ikinci geçiş
3. TR diakritik skoru (`çğıöşüÇĞİÖŞÜ`) yüksek olan sonuç seçilir
4. İngilizce ağırlıklı görsellerde (örn. kod) birincil sonuç korunur

**Bench (tur+eng isteği, akıllı yedek açık):**

| Metrik | Önceki | Şimdi |
|--------|--------|-------|
| `tr_greeting` CER | 0.208 | **0.104** |
| Ortalama CER | 0.076 | **0.063** |
| Ortalama süre | ~430 ms | ~1.2 s (çift geçiş + 2×) |

Örnek çıktı artık: `ÇGİÖŞÜ çğiöşü`

### 3. Çoklu monitör
- UI monitör seçici
- Overlay `capture.html?monitor=N` ile açılır
- Seçili monitörün fiziksel konum/boyutuna yerleştirilir
- `region-selected` payload’unda monitor index taşır

### 4. Dağıtım hazırlığı
- `resolve_tessdata`: `MIMO_TESSDATA` → exe yanı → proje assets → sistem
- App setup: resource `tessdata/` → env
- `tauri.conf.json` bundle.resources → `tessdata`
- `@tauri-apps/cli` projede (`package.json`)
- Kısayol **Ctrl+Shift+O** artık *accurate* kalitede OCR çalıştırır

### 5. Testler
`cargo test --workspace` yeşil (preprocess + core + capture + engines)

## Komutlar

```powershell
# Masaüstü
cargo run -p mimo-ocr-app

# CLI — yüksek kalite TR
cargo run -p mimo-ocr-cli -- ocr .\datasets\tr-en\synthetic\tr_greeting.png --langs tur,eng --quality accurate

# Paket (release + installer)
cmd /c "node_modules\.bin\tauri.cmd build"
```

## Paket durumu — TAMAM

| Dosya | Boyut |
|-------|-------|
| `target\release\mimo-ocr-app.exe` | ~9.9 MB |
| `target\release\bundle\msi\Mimo OCR_0.1.0_x64_en-US.msi` | ~11.6 MB |
| `target\release\bundle\nsis\Mimo OCR_0.1.0_x64-setup.exe` | ~8.4 MB |
| `target\release\tessdata\` (tur+eng+osd) | ~19 MB (release yanına kopyalandı) |

Notlar:
- Sürüm **0.1.0** (MSI pre-release `alpha` etiketini reddediyor)
- tessdata `bundle.resources` ile haritalı; `MIMO_TESSDATA` setup’ta resolve edilir
- App-local **Tesseract ikilisi** hâlâ paketlenmiyor — kurulum makinesinde `C:\Program Files\Tesseract-OCR` gerekli
- Çalışan uygulama doğrulandı (pencere: Mimo OCR)

## Sonraki adaylar
1. App-local tesseract.exe + dll paketleme (bağımsız MSI)
2. `leptess` native binding (CLI süreç maliyetini kaldırır — ~1.2s düşer)
3. PaddleOCR / ort kalite motoru
4. Aşama 2: macOS/Linux
