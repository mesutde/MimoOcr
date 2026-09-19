# Windows OCR motoru eklendi

Tarih: 19 Eylül 2026  
Araştırma: `docs/ENGINES.md` → 1. öncelik onaylandı.

## Ne eklendi?

| Parça | Yol |
|-------|-----|
| Adapter | `crates/mimo-ocr-engines/src/windows_ocr.rs` |
| Feature | `windows-ocr` (varsayılan açık, Windows) |
| Engine id | `windows-ocr` |
| UI etiketi | **Windows OCR** (başlık + Ayarlar) |
| Registry | `EngineRegistry::phase0_default()` |

## Teknik
- WinRT `Windows.Media.Ocr` + `windows` crate 0.61
- Dil: isteğe göre `tr-TR` / `en-US`; yoksa kullanıcı profili
- Bu makinede sistem paketi **`tr`** mevcut
- PNG → `InMemoryRandomAccessStream` → `BitmapDecoder` → `RecognizeAsync`
- Akış kapanma hatası (`0x80000013`) `DataWriter::DetachStream` ile çözüldü

## Ölçüm (bu makine)

| Görsel | Motor | ms | Not |
|--------|-------|-----|-----|
| tr_invoice | **windows-ocr** | **72** | Fatura metni doğru |
| tr_greeting | **windows-ocr** | **44** | `çĞiöşÜ çğıöşü` — TR karakterler büyük ölçüde yerinde |
| tr_invoice | tesseract-cli | ~1300 | Karşılaştırma |

Windows OCR **çok daha hızlı**; Tesseract varsayılan kalite motoru olarak kalır.

## Kullanım
Motor listesi:
```
Tesseract  |  Windows OCR  |  Otomatik  |  Mock
```

CLI:
```powershell
cargo run -p mimo-ocr-cli -- engines
cargo run -p mimo-ocr-cli -- ocr .\datasets\tr-en\synthetic\tr_invoice.png --engine windows-ocr
```

Uygulamada başlıktaki **Motor** listesinden seçin (varsayılan yine Tesseract).

## Notlar
- TR dil paketi olmayan PC’de `windows-ocr` listede çıkmayabilir veya boş verebilir
- Kutu güven skoru WinRT’de yok (confidence `None`)
- Linux/macOS’ta bu motor derlenmez/çalışmaz
