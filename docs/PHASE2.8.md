# Phase 2.8 — Sekmeli UI + motor ana ekranda

## Motor
- **Başlıkta (ana ekran):** Motor seçici  
- **Varsayılan:** `tesseract-cli` / **Tesseract**
- Ayarlar sekmesinde ikinci seçici; ikisi senkron
- Footer’da kullanılan motor

## Sekmeler
| Sekme | İçerik |
|-------|--------|
| **Yakala** | Ekran OCR · Bölge seç · Önizleme ile seç · Monitör · İptal |
| **Belge** | Dosya yolu · Dosya seç… · Belgeyi OCR'la |
| **Ayarlar** | Dil · Kalite · Motor · Panoya kopyala |

Kaynak | Metin panelleri her sekmede sabit kalır.

## Düzen
```
[M] Mimo OCR          [Motor: Tesseract ▾] [cihazda] [✕]
Yakala | Belge | Ayarlar
[Ekran OCR] [Bölge seç] [Önizleme…]  Monitör  [İptal]
Kaynak görüntü          |  Metin
status · motor · süre
```

## Test
1. Başlıkta **Motor: Tesseract** seçili gelmeli
2. **Yakala / Belge / Ayarlar** sekmeleri
3. Ayarlardan motor değiştir → başlıkta da değişmeli
4. Mock seç → footer `mock`
5. Ekran / bölge / belge OCR aynı çalışmalı
