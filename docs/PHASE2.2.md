# Phase 2.2 — PC donma düzeltmesi (Bölge seç)

Tarih: 2026-09-19  
Öncelik: **kritik**

## Sorun
**Bölge seç** → tam ekran, `always_on_top`, çerçevesiz OS overlay penceresi
açılıyordu. Bu Windows’ta tüm ekranı kilitleyebiliyor; PC yeniden başlatmak
gerekiyordu.

## Kök neden
`open_capture_overlay` → `WebviewWindowBuilder` + `fullscreen(true)` +
`always_on_top` + `decorations(false)`. Canlı masaüstü üzerine bindirilen
tam ekran WebView overlay’i sistem düzeyinde kilitlenmeye yol açabiliyor.

## Çözüm (güvenli akış)
OS overlay **tamamen kaldırıldı**.

Yeni akış:
1. **Bölge seç** → monitörün PNG kopyası alınır (`start_region_pick`)
2. Kopya **ana pencerede** modal içinde gösterilir
3. **Artı imleç** (crosshair) + satır kılavuzları
4. Sol tık **bas–sürükle–bırak** ile alan seç
5. **Seçili bölgeyi OCR’la** → fiziksel piksel koordinatlarıyla `ocr_region`
6. **Esc / İptal** modalı kapatır

Teknik:
- Modal `#cropModal` yalnızca ana WebView içinde (`position:absolute`)
- Hiçbir `fullscreen` / `always_on_top` ek pencere açılmaz
- Eski `capture` penceresi varsa **kapatılır**, yeniden üretilmez
- OCR hâlâ `spawn_blocking` + timeout ile arka planda

## Çıkış
- Header’da **Çıkış** (kırmızı) → `quit_app` → `app.exit(0)`
- Pencere **X** artık gizlemez, **uygulamayı kapatır**
- Tepsi menüsü: **Çıkış (uygulamayı kapat)**

## Bölge seç nasıl kullanılır
1. **Bölge seç**’e bas
2. Ekran kopyası ekrana gelir (+ imleç)
3. Fareyle istediğin alanı sürükle
4. **Seçili bölgeyi OCR’la**
5. Sonuç metin + panoya kopyalanır
6. Vazgeç: **Esc** veya **İptal**

## Doğrulama
- Uygulama yeniden derlendi ve çalışıyor
- CLI bölge OCR: `capture --region` + `ocr` — sorun yok
- Overlay kod yolu kaldırıldı; `open_capture_overlay` artık güvenli (snapshot döner)

## Uyarı
Eski bir `mimo-ocr-app` süreci hâlâ açıksa kapatın:
```powershell
Get-Process mimo-ocr-app -ErrorAction SilentlyContinue | Stop-Process -Force
```
Yeni ikili: `target\debug\mimo-ocr-app.exe`

## Kontrol
- [x] Fullscreen overlay kaldırıldı
- [x] Crosshair + sürükle-bırak crop UI
- [x] Çıkış butonu + X ile kapanış
- [x] Tepsi menüsü net çıkış
- [ ] Kullanıcı onayı: bölge seç PC’yi kilitlememeli
