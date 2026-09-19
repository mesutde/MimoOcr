# Phase 2.4 — Uygulama kaybolma düzeltmesi

Semptom: **Bölge seç** / seçim sonrası **uygulama kayboluyor**.

## Kök neden
Canlı seçimde ana pencere `hide()` ediliyordu. Overlay kapanınca görünür
pencere kalmıyordu → Tauri `ExitRequested` ile **uygulamayı kapatıyordu**.
JS’teki `show_main` da hatalı API kullanıyordu (`getByLabel` yok).

## Çözüm
1. **`ALLOW_EXIT` kilidi** — yalnız Çıkış / X / tepsi çıkışına izin
2. `RunEvent::ExitRequested` → otomatik kapanış **engellenir**, main geri gelir
3. Overlay `CloseRequested` / `Destroyed` → **main show + focus**
4. Yeni komutlar:
   - `show_main_window`
   - `finish_region_pick` (önce main, sonra overlay kapat)
   - `cancel_region_pick`
5. Overlay JS bu komutları çağırır
6. `region-selected` dinleyicisi de `show_main_window` çağırır

## Akış
```
Bölge seç → main gizlen → overlay
  → sürükle-bırak / Esc
  → finish_region_pick / cancel_region_pick
  → main görünür → OCR (gerekirse)
  → uygulama KAYBOLMAZ
```

## Doğrulama
- Uygulama yeniden derlendi
- Süreç çalışıyor: Mimo OCR (responding)

## Test
1. **Bölge seç (masaüstü)** → alan sürükle → pencere geri gelmeli
2. **Esc** iptal → pencere geri gelmeli
3. **Çıkış** gerçekten kapatmalı
4. **X** kapatmalı
