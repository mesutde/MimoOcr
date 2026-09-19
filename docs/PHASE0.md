# Phase 0 — Teknik Doğrulama Notları

Tarih: 2026-09-18  
Kaynak: `MimoOcr-Proje-Analiz-Raporu.html` Aşama 0

## Ortam

| Bileşen | Durum |
|---------|-------|
| Rust | 1.98.1 stable, `x86_64-pc-windows-msvc` |
| Node / npm | v22.20.0 / 10.9.3 (Tauri UI için hazır) |
| Tesseract | 5.4.0 @ `C:\Program Files\Tesseract-OCR` (PATH'te değil) |
| tessdata (sistem) | eng + osd (tur **yok**) |
| tessdata (proje) | `assets/models/tessdata`: tur + eng + osd (tessdata_fast) |
| cargo-tauri | kurulu değil — Aşama 1'e bırakıldı |

## Mimari kararlar (Phase 0)

1. **Ortak sonuç modeli şart** — motorlar `OcrDocument` / `OcrRegion` üretir; UI motora bağlanmaz.
2. **İlk motor yolu: Tesseract CLI** — bu makinede çalışan `tesseract.exe` ile en hızlı doğrulama. Paketleme riskini ölçmek için ideal.
3. **`leptess` feature-gated** — Windows'ta Leptonica/Tesseract yerel lib gerekir; CI'da ayrı iş.
4. **Mock motor zorunlu** — arayüz/bench tesisatı gerçek OCR olmadan test edilir.
5. **`xcap` yakalama** — monitör listesi, tam ekran, fiziksel bölge, mantıksal (DPI) bölge.

## Kapsam kontrol listesi

- [x] Workspace + core trait/types
- [x] Mock engine
- [x] Tesseract CLI adapter (discover + TSV word boxes)
- [x] tessdata TR/EN indirildi
- [x] Capture prototype (xcap)
- [x] CLI: doctor / engines / ocr / capture / bench
- [x] Sentetik TR/EN veri seti iskeleti (30 görsel)
- [x] `cargo test --workspace` yeşil (5 test)
- [x] Gerçek ekran yakalama + OCR e2e (1920×1080 bölge)
- [x] Bench sonuçları — `docs/phase0-bench.md`
- [ ] `leptess` derlemesi (opsiyonel feature — ertelendi)
- [ ] Tauri 2 iskeleti (Aşama 1 kapısı)

## Doğrulama özeti (2026-09-18)

| Sonuç | Değer |
|-------|-------|
| Tesseract avg latency | ~430 ms / sentetik görsel (tur+eng) |
| Tesseract avg CER | **0.076** |
| En zayıf TR hattı | `ÇĞİÖŞÜ` → `CGIOSU` (CER 0.208) |
| Ekran yakalama | OK — monitör 1920×1080 scale 1.00 |
| Proje tessdata | ~19.2 MB (tur+eng+osd) |

Detay: [`phase0-bench.md`](./phase0-bench.md)

## Riskler

| Risk | Phase 0 notu |
|------|----------------|
| Tesseract PATH'te değil | Adapter sabit yollara bakıyor; paketlemede app-local binary şart |
| Sistem tessdata'da TR yok | Proje `assets/models/tessdata` + `TESSDATA_PREFIX` |
| leptess Windows | Şimdilik CLI yol; native binding ayrı iş |
| Wayland / macOS | Kapsam dışı (Windows doğrulama) |

## Sonraki adım (Aşama 1)

Windows alfa: sistem tepsisi, küresel kısayol, bölge yakalama → Tesseract TR/EN → pano, basit önizleme, MSI+ZIP.
