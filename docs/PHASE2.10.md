# Phase 2.10 — Yakala/Belge ayrımı · önizleme crop · buton sırası

## 1. Yakala ≠ Belge
- `store.capture` ve `store.document` ayrı
- Sekme başlığı: **Kaynak · Yakala** / **Kaynak · Belge**
- Sekme değişince o sekmenin son sonucu gösterilir
- Yakala işleri (bölge/ekran/önizleme) capture deposuna
- Belge işleri document deposuna

## 2. Önizleme ile seç — koordinat
- `pointerToImage`: naturalWidth/Height + displayed size oranı
- `clampRegion`: seçim snapshot sınırlarına kırpılır
- Ctrl’suz sürükle → önceki bölgeler **temizlenir**
- Rust tarafında da bölgeler snapshot’a göre clamp edilir
- Status’te `seçilen N bölge OCR… (WxH)` görünür

## 3. Buton sırası
**Yakala** sekmesi:
```
[Bölge seç]  [Ekran OCR]  [Önizleme ile seç]  …
```
Bölge seç birincil; Ekran OCR ikincil (kullanıcı isteği).

## Test
1. Yakala’da OCR → Belge sekmesine geç: önceki belge sonucu (veya boş) görünmeli, yakala metni **taşmamalı**
2. Belge OCR → Yakala’ya geç: belge metni orada **olmamalı**
3. Önizleme ile seç → seçilen alandaki metin gelmeli
4. Butonlar: önce Bölge seç, sonra Ekran OCR
