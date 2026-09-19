# Phase 2.5 — tessdata `\\?\` yol hatası

## Hata
```
Error opening data file \\?\C:\...\target\debug\tessdata/tur.traineddata
Please make sure the TESSDATA_PREFIX ...
```

## Kök neden
Tesseract Windows’ta **uzun yol öneki** `\\?\` ile gelen
`TESSDATA_PREFIX` değerini açamıyor. Dosyalar diskte vardı; yol biçimi yanlıştı.

Kanıt:
- `TESSDATA_PREFIX=target\debug\tessdata` → **çalışır**
- `TESSDATA_PREFIX=\\?\C:\...\target\debug\tessdata` → **birebir aynı hata**

## Düzeltme
`clean_path_for_tesseract()`:
- `\\?\` ve `//?/` öneklerini siler
- `TESSDATA_PREFIX` / `MIMO_TESSDATA` temiz yolla set edilir
- `run_tesseract` çağırmadan önce path temizlenir ve `*.traineddata` doğrulanır
- `resolve_tessdata*` yalnız model içeren, temiz yol döner

## Doğrulama
```
ocr tr_invoice tur+eng → FATURA NO: TR-2026-00123 (589 ms)
doctor → tessdata OK, tur+eng present
```

Uygulama yeniden derlendi ve açıldı.

## Sizde hâlâ çıkarsa
1. Uygulamayı tamamen kapatın (Çıkış)
2. `target\debug\mimo-ocr-app.exe` yeni derlemeyi çalıştırın
3. Eski süreç Görev Yöneticisi’nde kalmasın
