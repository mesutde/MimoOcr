# Phase 2.9 — Bölge seç OCR okumama düzeltmesi

## Sorun
**Bölge seç** → masaüstünde alan seçiliyor ama seçilen bölge **OCR’lanmıyordu**.

## Kök neden
1. Overlay yalnız `region-selected` yayını yapıyordu; gizli ana pencere bu olayı kaçıyordu
2. Sonraki `ocr_region` canlı ekranı **yeniden** yakalıyordu — overlay koordinatları ile uyuşmayabiliyordu
3. Overlay client boyutu ≠ monitör fiziksel boyutu olabilir (ölçek kayması)

## Yeni akış
```
Bölge seç → overlay (masaüstü)
  → sürükle-bırak
  → invoke complete_live_region_pick
       • koordinat ölçekle (client → monitör px)
       • monitörü bir kez yakala
       • seçilen dikdörtgeni kırp
       • OCR (seçili motor/dil)
       • ana pencereyi göster
       • overlay kapat
       • ocr-result + panoya kopyala
```

Açılışta UI ayarları (dil/motor/kalite) `AppState`’e yazılır; overlay bunları kullanır.

## Hata durumları
- Seçim çok küçük → iptal + uyarı
- OCR hatası → status’te kırmızı hata
- Boş metin → “bölge okundu ama metin boş”

## Yedek
**Önizleme ile seç** — uygulama içi kopya + crop (hâlâ çalışıyor).

## Test
1. Uygulamayı yeniden aç (`target\debug\mimo-ocr-app.exe`)
2. **Yakala → Bölge seç**
3. Ekranda metin olan bir yeri sürükle
4. Pencere geri gelmeli; metin alanında OCR sonucu + panoda olmalı
5. Olmazsa alt çubuktaki status satırını not edin
