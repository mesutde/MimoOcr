# Phase 2.3 — tessdata + canlı masaüstü bölge seçimi

Tarih: 2026-09-19

## 1. tessdata hatası düzeltildi

**Hata:**
```
Error opening data file ...\target\debug\tessdata/tur.traineddata
Please make sure the TESSDATA_PREFIX environment variable is set...
```

**Neden:** Tesseract `TESSDATA_PREFIX` olarak `target\debug\tessdata` yolunu
alıyordu; orada model yoktu (veya boş klasör / yanlış öncelik).

**Düzeltme:**
- `target\debug\tessdata` içine `tur.traineddata` + `eng.traineddata` kopyalandı
- `target\debug\tesseract\tesseract.exe` + DLL’ler (app-local)
- `resolve_tessdata` / `tessdata_candidates`: boş olmayan, model dosyası olan
  klasörleri arar (env → exe yanı → yukarı doğru assets → sistem)
- `resolve_tessdata_for_langs`: `tur+eng` için **her iki** `.traineddata`’yı arar
- `TESSDATA_PREFIX` yalnızca model içeren dizine set edilir
- Setup (`setup_tessdata_env`) aynı doğrulamayı yapar

Doğrulama: `target/debug` altında `tur+eng` OCR fatura metnini doğru okur.

## 2. Canlı masaüstü bölge seçimi

**Şikayet:** “masaüstü ekranı uygulamanın içine gömülüyor” — snapshot modalı
gerçek ekran seçimine benzemiyordu.

**Yeni akış (Bölge seç → masaüstü):**
1. Ana pencere **gizlenir**
2. Seçili monitör üzerine **şeffaf, çerçevesiz** overlay açılır
3. **`fullscreen(true)` YOK** — o API PC’yi kilitliyordu
4. Yalnız `position` + `inner_size` = monitör fiziksel boyutu
5. Artı imleç + çizgiler; masaüstü seçilir
6. Sürükle-bırak → `region-selected` → pencere geri gelir → OCR
7. **Esc** / sağ tık = iptal; 90 sn failsafe

**Güvenli yedek:** **Önizleme ile seç** — uygulama içi kopya (eski modal).

**Çıkış:** Çıkış butonu / X / tepsi → uygulama kapanır.

## Kullanım

| Buton | Ne yapar |
|-------|----------|
| **Bölge seç (masaüstü)** | Gerçek ekranda sürükle |
| **Önizleme ile seç** | Uygulama içinde kopya + crop |
| **Çıkış** | Uygulamayı kapatır |

## Komutlar

```powershell
# tessdata debug’e kopyala (script yerine elle de yapılabilir)
Copy-Item assets\models\tessdata\*.traineddata target\debug\tessdata\ -Force
Copy-Item assets\tesseract-runtime\* target\debug\tesseract\ -Force -Recurse

# Uygulama
cargo run -p mimo-ocr-app
```

## Hâlâ hata alırsanız
1. Uygulamayı kapatın
2. `target\debug\tessdata` içinde `tur.traineddata` ve `eng.traineddata` olduğunu doğrulayın
3. Yeniden başlatın
4. Hâlâ olursa: `C:\Program Files\Tesseract-OCR\tessdata` sistem kurulumuna bakın

## Kontrol
- [x] target/debug tessdata TR+EN
- [x] resolve zinciri model dosyası doğrular
- [x] Live overlay (fullscreen yok)
- [x] Önizleme yedeği
- [x] Çıkış butonu
- [ ] Kullanıcı testi: masaüstü seçim + OCR + PC donmasın
