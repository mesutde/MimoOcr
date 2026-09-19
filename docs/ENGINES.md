# OCR Motorları — Eklenebilirlik Araştırması

Tarih: 19 Eylül 2026 · Kaynak: crates.io / docs.rs / GitHub (erişim)

Mevcut durum: **tesseract-cli** (varsayılan) + **mock**. `OcrEngine` trait’i hazır; yeni motorlar bağdaştırıcı olarak girer.

---

## Özet tablo

| Motor | Windows | TR/EN | Kurulum yükü | Eklenebilir mi? | Öncelik |
|-------|---------|-------|--------------|-----------------|---------|
| **Windows OCR** (WinRT) | Evet | Dil paketine bağlı | Çok düşük | **Evet — en kolay** | **1** |
| **ocrs** | Evet | Latin/EN ağırlıklı | Orta (model) | Evet (deneysel) | **2** |
| **leptess** | Evet | Tesseract TR/EN | Orta (native lib) | Evet (feature) | **3** |
| **ort + PaddleOCR ONNX** | Evet | TR test edilmeli | Yüksek (runtime+model) | Evet (kalite paketi) | **4** |
| **rusty-tesseract** | Evet | Tesseract | Düşük (CLI) | Kolay ama düşük fark | 5 (opsiyonel) |
| **Apple Vision** | Hayır (macOS) | TR+EN | — | Sadece macOS | Aşama 2 |

---

## 1. leptess — `houqp/leptess`

| | |
|--|--|
| Crate | `leptess` **0.14.0** |
| Güncelleme | Şubat **2023** (bakım yavaş) |
| Lisans | MIT |
| Ne | Tesseract + Leptonica **native** Rust bağlayıcısı |

**Eklenebilir mi?** Evet — `crates/mimo-ocr-engines` feature `leptess`.  
**Zorluk:** Windows’ta `tesseract` + `leptonica` C lib + include path (vcpkg veya app-local dll). Bizde zaten `libtesseract-5.dll` / `libleptonica-6.dll` var; link/ABI ayarı gerekir.  
**Fayda:** CLI süreç açmadan in-process OCR → ~1.2s yerine daha düşük gecikme.  
**Risk:** Eski crate; build zinciri kırılgan. Tesseract 5 uyumu elle doğrulanmalı.  
**Öneri:** Feature flag ile **Aşama 1.5–2**; varsayılan şimdilik CLI.

---

## 2. PaddleOCR + `ort` (ONNX)

| | |
|--|--|
| Runtime crate | `ort` **2.0.0-rc.13** (Tem 2026) — çok aktif, ~7.1M indirme/hafta |
| PaddleOCR | Python repo; Rust’a doğrudan değil |
| Rust yolu | Paddle modelini **ONNX’e** çevir → `ort` ile çalıştır |
| İlgili | `oar-ocr`, `retto` (ort README’de PaddleOCR ONNX örneği) |

**Eklenebilir mi?** Evet, ama “crate ekle” değil; **model + boru hattı** işi.  
**Parçalar:** det (metin kutusu) + rec (karakter tanıma) + dict (Türkçe sözlük) + ön/son işlem.  
**TR:** Latin/Türkçe modeller var; **ç ğ ı İ ö ş ü** için mutlaka gerçek test gerekir (rapordaki uyarı).  
**Lisans:** PaddleOCR/Paddle2ONNX genelde Apache 2.0 — uyumlu.  
**Paket:** ort `download-binaries` + ONNX modeller → onlarca–yüzlerce MB.  
**Öneri:** İsteğe bağlı **“Yüksek doğruluk”** paketi; varsayılan motor değil. UI motor listesine `paddle-onnx` olarak sonra eklenebilir.

---

## 3. Windows OCR (WinRT `Windows.Media.Ocr`)

| | |
|--|--|
| Bağlantı | `windows` crate (`Media_Ocr` / `Graphics_Imaging` features) |
| Repo ipucu | xlzhen-940218/WindowsOCR (üçüncü taraf örnek/wrapper) |
| Model | Sistem dil paketi (TR/EN yüklüyse) |

**Eklenebilir mi?** **Evet — bu listede en pratik ikinci motor.**  
**Zorluk:** Düşük; Bitmap → `OcrEngine` adapter.  
**Artı:** Ek model indirmesi yok, hızlı, yerel.  
**Eksi:** Dil desteği cihazda yüklü paketlere bağlı; Linux/macOS yok; sonuç biçimi Tesseract’a göre farklı (kutu/skor sınırlı).  
**Öneri:** UI’da **“Windows OCR (sistem)”** — özellikle ekran metni için hızlı yol. TR için cihazda Türkçe dil paketi olmalı.

---

## 4. Apple Vision (`vision-rs` / VNRecognizeTextRequest)

| | |
|--|--|
| Platform | **macOS / iOS only** |
| Bu makine | Windows — şimdilik **uygulanamaz** |

**Eklenebilir mi?** Yalnız macOS build’inde (`#[cfg(target_os = "macos")]`).  
**Öneri:** macOS alfasına (Aşama 2) ertelenir. Windows motor listesinde görünmez.

---

## 5. ocrs — `robertknight/ocrs`

| | |
|--|--|
| Crate | `ocrs` **0.13.1** (Eylül **2026** — aktif) |
| Lisans | MIT OR Apache-2.0 |
| Runtime | **RTen** (ONNX feature opsiyonel) |
| Durum | “Early preview” — ticari motorlardan daha çok hata |
| Dil | **Yalnız Latin alfabe (İngilizce ağırlıklı)**; genişleme planlı |
| Model | İlk kullanımda indirilir (`~/.cache/ocrs`) — offline paket için model dosyasını gömmek gerekir |

**Eklenebilir mi?** Evet — adapter kolay; **TR üretmez** (ı/İ/ş/ğ sınırlı).  
**Kullanım:** Deneysel motor / EN metin denemesi / WASM ileride.  
**Öneri:** Motor listesinde **“ocrs (deneysel)”**; varsayılan **yapılmamalı**. Offline için model lisansı + boyutu netleşmeli.

---

## 6. rusty-tesseract

| | |
|--|--|
| Crate | `rusty-tesseract` **1.1.10** (Mart 2024) |
| Ne | Tesseract **CLI** sarmalayıcısı (zaten `tesseract-cli` gibi) |

**Eklenebilir mi?** Evet, düşük maliyet.  
**Fark:** Mevcut CLI adapter’ımızdan az fark — kutu/skor API’si farklı olabilir.  
**Öneri:** Şimdilik **gerek yok**; `tesseract-cli` yeterli.

---

## MimoOCR için önerilen sıralama

```
Şimdi (kolay kazanım)
  1) Windows OCR adapter          → UI: "Windows OCR"
  2) ocrs feature (deneysel)      → UI: "ocrs (deneysel)" — EN/Latin

Orta vade
  3) leptess feature              → in-process Tesseract (paketleme çalışması)
  4) ort + PaddleOCR ONNX         → "Yüksek doğruluk" isteğe bağlı paket

Platforma özel
  5) Apple Vision                 → macOS build
```

### Mimari not
Hepsi mevcut `OcrEngine` trait’ine `OcrDocument` döndürecek şekilde adapter olur:
- `windows_ocr.rs`
- `ocrs_engine.rs` (feature)
- `leptess_engine.rs` (feature)
- `paddle_ort.rs` (feature + model yolu)

UI motor seçicisi `engine_status` listesine otomatik eklenir (şu an tesseract-cli / auto / mock).

### Riskler
| Risk | Not |
|------|-----|
| leptess Windows link | Sistem lib veya vcpkg; CI matrisi gerekir |
| Paddle TR karakter | Mutlaka CER testi; varsayılan yapılmamalı |
| Windows OCR dil | TR yoksa motor “boş/düşük” verir — UI’da durum göster |
| ocrs offline | Model cache gömülmezse internetsiz çalışmaz |
| Paket boyutu | ort + modeller MSI’ı büyütür → ayrı indirilebilir paket |

---

## Kısa cevap

| Soru | Cevap |
|------|-------|
| Hepsini şimdi bağlayabilir miyiz? | **Hayır** — her biri ayrı iş; trait hazır. |
| En mantıklı ilk ek? | **Windows OCR**, sonra **ocrs (deneysel)**. |
| En güçlü kalite yolu? | **ort + PaddleOCR ONNX** (daha sonra). |
| leptess? | Evet, ama native build riski var; CLI yedeği kalsın. |
| Apple Vision? | Bu Windows makinede **hayır**; macOS fazında. |

Uygulamak isterseniz bir sonraki adımda **Windows OCR adapter**’ı feature/registry’ye ekleyip motor listesine koyabiliriz.
