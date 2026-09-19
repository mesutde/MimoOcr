# Phase 2.7 — Minimalist UI

## Değişiklik
Üst araç çubuğundaki buton kalabalığı kaldırıldı.

### Ana ekranda yalnız
| Kontrol | Görev |
|---------|--------|
| **Ekran OCR** | Tam ekran yakala + OCR + pano |
| **Bölge** | Masaüstünde alan seç |
| **Belge** | Yoldaki dosyayı OCR’la (yol boşsa ⋯ açılır) |
| **⋯** | Diğer seçenekler paneli |
| **✕** | Çıkış |

### Motor
Üst barda **yok**. ⋯ panelinde **Motor** (Otomatik / Tesseract / Mock).  
Alt bilgi çubuğunda **kullanılan motor** görünür.

### ⋯ paneli
Dil · Kalite · Motor · Monitör · Belge yolu · Dosya seç ·  
Önizleme ile seç · Panoya kopyala · İptal

## Konsept
```
[M] Mimo OCR          [cihazda] [✕]
[Ekran OCR] [Bölge] [Belge]      [⋯]
  Kaynak  |  Metin
  status  |  Motor tesseract-cli · tur+eng
```

## Test
1. Üst barda 3 buton + ⋯ + ✕
2. ⋯ → Motor seçilebilir
3. Belge boşken **Belge** → panel açılır
4. Fonksiyonlar aynı: ekran / bölge / belge / OCR
