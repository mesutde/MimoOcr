# Phase 0 Bench — 18 Eylül 2026

Makine: Windows x64, 1920×1080 @ scale 1.00, Tesseract 5.4.0  
Veri: `datasets/tr-en/synthetic` (10 metin × 3 varyant = 30 PNG)  
Motorlar: `mock`, `tesseract-cli` (`tur+eng`, tessdata_fast)

## Özet

| Motor | n | ok | avg ms | min | max | avg CER |
|-------|---|----|--------|-----|-----|---------|
| mock | 30 | 30 | 1.0 | 1 | 2 | 0.843 (sabit metin — beklenen) |
| tesseract-cli | 30 | 30 | **429.7** | 332 | 694 | **0.076** |

## Tesseract-cli CER (görsel bazlı, tekrarlar aynı)

| Görsel | CER | WER | Not |
|--------|-----|-----|-----|
| tr_gazete | 0.033 | 0.000 | çok iyi |
| en_small | 0.045 | 0.059 | iyi |
| tr_en_mixed | 0.046 | 0.000 | karma TR+EN stabil |
| en_tech | 0.056 | 0.000 | URL/e-posta |
| tr_invoice | 0.057 | 0.000 | TL / tarih / vergi no |
| tr_tech | 0.059 | 0.000 | |
| tr_numbers | 0.060 | 0.062 | IBAN/telefon |
| tr_ui_dark | 0.082 | 0.000 | koyu tema |
| en_code | 0.113 | 0.000 | girinti/monospace |
| **tr_greeting** | **0.208** | 0.143 | **ÇĞİÖŞÜ hattı** |

## Kritik bulgu: Türkçe özel karakterler

`tr_greeting` beklenen:

```
ÇĞİÖŞÜ çğıöşü
```

Tesseract çıktısı:

```
CGIOSU çğıöşü
```

- Büyük harf: `ÇĞİÖŞÜ` → `CGIOSU` (İ→I, Ğ→G, Ş→S, Ü→U benzeri düzleşme)
- Küçük harf `çğıöşü` daha iyi korunuyor
- Bu, rapordaki uyarı ile uyumlu: dil listesinde `tur` yazması tek başına yeterli değil; **karakter bazlı regresyon şart**

### Sonraki denemeler

1. `--psm 6` (tek blok) vs `--psm 3`
2. 2× upscale ön işleme
3. `tessdata_best` tur modeli karşılaştırması
4. Karışık satır için ayrı `tur` / `eng` koşuları

## Ekran yakalama e2e

```
mimo capture --out captures/screen.png
→ 1920x1080 monitör-0

mimo capture --out captures/region.png --region 0,0,800,400
mimo ocr captures/region.png --langs eng
→ ~295 ms, gerçek tarayıcı içeriği okundu (Türkçe sayfa başlıkları seçilebildi)
```

## Paket boyutu (Phase 0 ölçümü)

| Öğe | Boyut |
|-----|-------|
| Proje tessdata (tur+eng+osd, tessdata_fast) | ~19.2 MB |
| Sistem Tesseract-OCR kurulumu (tam klasör) | ~234 MB |
| **Dağıtım hedefi** | yalnız gerekli dll + tessdata_fast tur+eng ≈ **~15–25 MB** (tahmin, Aşama 1 MSI ölçümü) |

## Sonuç

| Kriter | Durum |
|--------|-------|
| Ortak `OcrEngine` arayüzü | Tamam |
| Mock + Tesseract motorları | Tamam |
| TR/EN tessdata proje paketi | Tamam |
| Windows ekran/bölge yakalama | Tamam |
| CLI OCR + bench (CER/WER) | Tamam |
| TR karakter regresyonu | Kısmi — ÇĞİÖŞÜ sorunu tespitli |
| leptess native | Erteleme (feature-gated) |
| cargo-tauri / UI iskeleti | Aşama 1 |

**Phase 0 geçti.** Aşama 1'e (Windows alfa: tepsisi + kısayol + bölge OCR + pano) geçilebilir; ön işleme ve TR karakter testleri alfa boyunca sıkı tutulmalı.
