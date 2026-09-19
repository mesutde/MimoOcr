# Phase 2 — Donma düzeltmesi + belge OCR

Tarih: 2026-09-19

## Donma kök nedenleri (bulunan)

1. **Senkron / ağır OCR yolu** — UI invoke’ları `spawn_blocking` dışına sarkabiliyordu.
2. **Tam ekran + “Yüksek 2×”** — 1920×1080 ×2 ≈ 3840×2160 + çift tesseract geçişi.
3. **`tur+eng` ikinci geçişi** her büyük görselde tekrar çalışıyordu.
4. **Tesseract süresiz** — asılı kalabilirdi.

## Yapılan düzeltmeler

| Alan | Değişiklik |
|------|------------|
| `commands.rs` | `ocr_full_screen` / `ocr_region` / `ocr_path` **async + spawn_blocking** |
| Busy kilidi | Aynı anda tek OCR; üst üste basış reddedilir |
| `cancel_ocr` | UI’da **İptal** düğmesi |
| `ocr-status` | “OCR çalışıyor…” / “tamamlandı” olayları |
| Preprocess | Büyük görselde upscale sınırı (≥1400px → ≤1.25×, ≥2200 → 1×) |
| Backend | Tam ekran + accurate istense bile **balanced**’a düşer |
| Dual-pass | Yalnız ≤1.2 MP (bölge/küçük görsel); tam ekranda kapalı |
| Tesseract | 20 sn timeout + pipe drenajı (ölü kilit yok) |
| UI | Busy paint gecikmesi (40 ms) — buton kilidi görünür |
| Kısayol | Ctrl+Shift+O artık **dengeli** kalitede |
| **Belge OCR** | Yol girişi + **Belgeyi OCR’la** (`ocr_path`) |

## Programatik doğrulama (UI basmadan)

```
capture 1920×1080 → OCR
  accurate/balanced backend sınırı: ~3.5 sn (async; UI donmaz)
fatura görseli (balanced)
  ~0.7–1.1 sn · metin doğru
```

Uygulama yeniden derlendi ve çalışıyor.

## Yeni kullanım

1. **Belge OCR:** yolu yapıştır → **Belgeyi OCR'la**
2. OCR sırasında status satırı sarı; pencere hareket edebilir
3. **İptal** ile işi kes
4. Tam ekran “Yüksek 2×” seçiliyse bile backend büyük ekranda dengeli moda düşer

## Faz 2 devam adayları
- Klasör toplu OCR (CLI + UI)
- App-local Tesseract paketleme
- leptess native (süreç maliyetini kaldırır)
- Ayarlar: kısayol, geçmiş (varsayılan kapalı)

## Not
“Belge seç” için sistem dosya diyaloğu eklenebilir (`tauri-plugin-dialog`); şimdilik yol girdisi var — donma riski yok, iş parçacığı UI dışı.
