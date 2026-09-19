# Phase 2.1 — Dosya seçici + app-local Tesseract

Tarih: 2026-09-19

## 1. Sistem dosya seçici (Belge seç)

- `tauri-plugin-dialog` eklendi
- UI: **Belge seç…** → Windows aç iletişim kutusu
- Filtre: png/jpg/jpeg/bmp/tif/tiff/webp
- Seçim sonrası yol alanına yazılır ve **otomatik OCR** başlar (async — donmaz)
- Dialog yoksa: yolu elle girip **Belgeyi OCR'la**

## 2. App-local Tesseract runtime

```
assets/tesseract-runtime/
  tesseract.exe
  libtesseract-5.dll + 50 bağımlı DLL (~160 MB)
  tessdata/tur.traineddata + eng.traineddata
```

- Script: `scripts/copy_tesseract_runtime.ps1`
- Keşif sırası:
  1. `MIMO_TESSERACT` / `MIMO_TESSDATA`
  2. Kaynaklar (`resources/tesseract-runtime/...`)
  3. Exe yanı
  4. Proje assets
  5. Sistem kurulumu (`C:\Program Files\Tesseract-OCR`)
- Setup: resource/runtime klasörünü env’e yazar, PATH’e ekler
- Bundle: `tesseract-runtime` → MSI/NSIS resources

### Doğrulama (app-local)

```
tesseract v5.4.0 (assets\tesseract-runtime)
ocr tr_invoice → FATURA NO: TR-2026-00123 ... (769 ms)
doctor → tesseract OK app-local, tessdata TR+EN OK
```

## 3. Paket notu

MSI/NSIS artık **~170–200 MB** olabilir (libtesseract + ICU dll’leri).  
Bu, kurulum makinesinde sistem Tesseract’u olmadan çalışmak içindir.

İnce ayar istenirse:
- `libicudt75.dll` gibi büyük dll’ler yerine daha dar bir runtime
- Veya NSIS ile “portable zip” + opsiyonel “full offline” paket

## Kullanım

```powershell
cargo run -p mimo-ocr-app
# UI: Belge seç… → sistem dialog → otomatik OCR + pano
# veya yol yapıştır → Belgeyi OCR'la
```

```powershell
# Runtime yeniden kopyala
powershell -File scripts\copy_tesseract_runtime.ps1
# Paket
cmd /c "node_modules\.bin\tauri.cmd build"
```

## Kontrol listesi

- [x] Dialog plugin + Belge seç UI
- [x] App-local tesseract + tessdata
- [x] discover_tesseract / resolve_tessdata keşif zinciri
- [x] App-local OCR doğrulandı
- [x] Bundle resources haritası
- [ ] MSI/NSIS boyutu kabul edilebilir mi? (ölçüm sonrası karar)
- [ ] Portable ZIP (Tesseract’suz veya runtime’lı)

## Sonraki

- Ayarlar paneli (motor, dil, kısayol, geçmiş)
- leptess native (CLI süreç maliyetini kaldırır)
- PaddleOCR kalite motoru
- Aşama 3: PDF / toplu aranabilir çıktı
