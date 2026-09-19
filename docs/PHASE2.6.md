# Phase 2.6 — Kullanıcı geri bildirimleri

## 1. Kaynak görüntüde mavi kenar yok
- Önizleme `img` üzerinde border / outline / box-shadow kaldırıldı
- Belge OCR sonucunda da kenar çizgisi görünmez

## 2. Önizleme ile seç — Ctrl ile çoklu alan
- **Sürükle**: tek alan (önceki seçimler temizlenir)
- **Ctrl + sürükle**: var olan seçimlere **ekler** (yeşil çerçeveler)
- **Seçimleri temizle** düğmesi
- **Seçili bölgeleri OCR'la**: snapshot üzerinden her alan kırpılır, tek tek OCR, metinler satırla birleşir
- Canlı ekrana tekrar bakılmaz — önizleme kopyası kullanılır

## 3. Motor seçici
Araç çubuğunda **Motor**:
- **Otomatik** (varsayılan) → tesseract-cli varsa o, yoksa mock
- **Tesseract CLI**
- **Mock (test)**

Backend `engine` parametresi alır; `engine_status` ile dropdown dolar.
Footer’da kullanılan motor görünür.

## Komutlar
```powershell
cargo run -p mimo-ocr-app
```

## Test
1. Belge seç / OCR → mavi kenar olmamalı
2. Önizleme ile seç → 1 alan, sonra **Ctrl** ile 2. alan → OCR
3. Motor: Mock seç → footer “mock” yazmalı
4. Motor: Otomatik / Tesseract CLI → gerçek OCR
