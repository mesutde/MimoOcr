# Toplu sekmesi — LLM analiz çıktısı

## Ne yapar?
**Yakala | Belge | Toplu | Ayarlar** sekmelerine ek olarak **Toplu**:
- Birden fazla **görsel / PDF / Office** dosyası seç
- Metin çıkar (görsel → OCR; PDF/DOCX/XLSX/PPTX/TXT → metin çıkarımı)
- **TXT** veya **Markdown** kaydet
- **Ayrı ayrı** veya **Birleştir (tek dosya)** seçenekleri
- Amaç: çıktıyı **LLM / yapay zekâ** modellerine girdi yapmak

## Kullanım
1. **Toplu** sekmesi
2. **Dosyalar seç…** (çoklu seçim)
3. **Çıktı klasörü**
4. Format: **Markdown** (önerilen LLM için) veya **TXT**
5. Kayıt: **Birleştir** (tek `.md`) veya **Ayrı ayrı**
6. **Toplu başlat**

## Destek
| Tür | Yöntem |
|-----|--------|
| png jpg bmp tif webp | OCR (seçili motor) |
| pdf | pypdf metin çıkarımı |
| docx | python-docx / XML |
| xlsx | openpyxl |
| pptx | python-pptx / XML |
| txt md csv json | doğrudan okuma |

Python: `MIMO_PYTHON` + `scripts/batch_extract.py`

## LLM’e uygun Markdown örnek
```markdown
# Mimo OCR — Toplu Çıktı
- Tarih: 20260919-...
- Dosya: 5
- Amaç: LLM / yapay zekâ analiz girdisi

## fatura.png
- **Kaynak:** `C:\...\fatura.png`
- **Tür:** image/png
- **Motor:** tesseract-cli

```
FATURA NO: ...
```
```

Tek dosya: `mimo-ocr-batch-<zaman>.md`

## Komut
Uygulama UI’ı veya Rust: `batch_process_files`

## Not
- OCR motoru başlıktaki **Motor** seçicisinden gelir (varsayılan Tesseract)
- Görsel olmayan dosyalarda motor “extract” yazar
- Taramalı **PDF**’de metin yoksa sayfaları PNG’ye çevirip görsel olarak seçmek gerekir (şu an metin katmanı okunur)
