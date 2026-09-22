# Video sekmesi — kaydırılan ekran → CSV/TXT

## Amaç
Bazı kişiler Excel/CSV yerine **ekran videosu** atıyor; aşağı kaydırıyor.  
Mimo OCR bu videolardan okunabilir metni çıkarır; **tablo gibi görünüyorsa CSV**, değilse **TXT/MD** üretir. Düzensiz metni sonradan **LLM** ile düzenleyebilirsiniz.

## Nasıl çalışır?
1. **ffmpeg** ile adaptif kare örneklemesi  
   - **Otomatik / Hızlı scroll / Yavaş scroll**  
   - Hızlı kaydırmada orta FPS; yavaşta daha sık kare  
2. Her kare **Tesseract** ile OCR (tek tek; kare silinir → **bellek dostu**)  
3. Scroll nedeniyle **tekrar eden satırlar** elenir  
4. Sütun/spasi/çift boşluk varsa **CSV**; her koşulda **TXT**  
5. Uzun kenar en fazla **1280px**; kare sayısı sınırı (varsayılan ~180)

## Kullanım
1. **Video** sekmesi  
2. **Video ekle…** (mp4/mov/avi/mkv/webm…)  
3. **📁** ile çıktı klasörü  
4. Scroll: Otomatik / Hızlı / Yavaş  
5. Format: CSV+TXT (öneri)  
6. Kalite/Hız: Hızlı (~120 kare) · Dengeli (~180) · Yüksek (~280)  
7. **Videodan çıkar**

Çıktı: `<videoadi>.video.csv` · `.video.txt` (ve istenirse `.md`)

## CLI
```powershell
& $env:MIMO_PYTHON scripts\video_extract.py `
  --video C:\path\table-scroll.mp4 `
  --out C:\path\out `
  --lang tur+eng `
  --mode auto `
  --max-frames 180 `
  --formats csv,txt
```

Gereksinimler: **ffmpeg/ffprobe**, **tesseract**, Python.

## Sınırlar
- Ekranda **okunmayan** çok küçük yazı zayıf çıkar  
- Görsel grafik/sütun çizgileri yalnız metin OCR’a girer  
- Tamamen döngü/boş kare videolarda metin az olabilir  
- Yargı değeri yok; çıktı ham / LLM için girdidir

## Bellek
Kareler tek tek OCR’dan sonra silinir; 400 kareden fazlası örnekleme sınırında tutulur. Sistem yorgunluğunda **Video kalite**’yi **Hızlı** yapın.
