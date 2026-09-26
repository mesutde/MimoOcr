//! Windows.Media.Ocr (WinRT) bagdastiricisi — yeni motor modeline port.
//!
//! Sistem dil paketlerini kullanir (or. Turkce OCR paketi); model dosyasi gerekmez.
//! Kelime guven skoru WinRT tarafindan verilmez → confidence 0.0 birakilir,
//! arayuz kelime sayisini gosterir, yuzdeyi gizler.

use std::time::Instant;

use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::BitmapDecoder;
use windows::Media::Ocr::OcrEngine as WinOcr;
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

use crate::engine::{OcrDocument, OcrError, OcrOptions, OcrWord};

pub const ENGINE_ID: &str = "windows-ocr";
pub const DISPLAY_NAME: &str = "Windows OCR (sistem)";

pub struct WindowsOcrEngine;

impl WindowsOcrEngine {
    /// Bu makinede WinRT OCR acilabiliyorsa true (dil paketi varligi).
    pub fn available() -> bool {
        WinOcr::TryCreateFromUserProfileLanguages().is_ok()
    }

    pub fn available_languages() -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(list) = WinOcr::AvailableRecognizerLanguages() {
            for i in 0..list.Size().unwrap_or(0) {
                if let Ok(lang) = list.GetAt(i) {
                    if let Ok(tag) = lang.LanguageTag() {
                        out.push(tag.to_string());
                    }
                }
            }
        }
        out.sort();
        out
    }

    pub fn recognize_png(
        &self,
        image_png: &[u8],
        options: &OcrOptions,
    ) -> Result<OcrDocument, OcrError> {
        let started = Instant::now();
        let engine = pick_win_engine(&options.languages)?;

        // PNG baytlarini dogrudan WinRT akisina yaz (yeniden kodlama yok).
        let stream = InMemoryRandomAccessStream::new()
            .map_err(|e| OcrError::Image(format!("akis: {e}")))?;
        let writer = DataWriter::CreateDataWriter(&stream)
            .map_err(|e| OcrError::Image(format!("yazici: {e}")))?;
        writer
            .WriteBytes(image_png)
            .map_err(|e| OcrError::Image(format!("yazma: {e}")))?;
        writer
            .StoreAsync()
            .and_then(|op| op.get())
            .map_err(|e| OcrError::Image(format!("store: {e}")))?;
        writer
            .FlushAsync()
            .and_then(|op| op.get())
            .map_err(|e| OcrError::Image(format!("flush: {e}")))?;
        writer
            .DetachStream()
            .map_err(|e| OcrError::Image(format!("detach: {e}")))?;
        stream
            .Seek(0)
            .map_err(|e| OcrError::Image(format!("seek: {e}")))?;

        let decoder = BitmapDecoder::CreateAsync(&stream)
            .and_then(|op| op.get())
            .map_err(|e| OcrError::Image(format!("cozucu: {e}")))?;
        let bitmap = decoder
            .GetSoftwareBitmapAsync()
            .and_then(|op| op.get())
            .map_err(|e| OcrError::Image(format!("bitmap: {e}")))?;

        let result = engine
            .RecognizeAsync(&bitmap)
            .and_then(|op| op.get())
            .map_err(|e| OcrError::Image(format!("tanima: {e}")))?;

        let plain = result.Text().map(|t| t.to_string()).unwrap_or_default();

        let mut words = Vec::new();
        if let Ok(lines) = result.Lines() {
            for i in 0..lines.Size().unwrap_or(0) {
                let Ok(line) = lines.GetAt(i) else { continue };
                let Ok(win_words) = line.Words() else { continue };
                for j in 0..win_words.Size().unwrap_or(0) {
                    let Ok(w) = win_words.GetAt(j) else { continue };
                    let text = w.Text().map(|t| t.to_string()).unwrap_or_default();
                    if text.trim().is_empty() {
                        continue;
                    }
                    let r = w.BoundingRect().unwrap_or(windows::Foundation::Rect {
                        X: 0.0,
                        Y: 0.0,
                        Width: 0.0,
                        Height: 0.0,
                    });
                    words.push(OcrWord {
                        text,
                        confidence: 0.0, // WinRT guven vermez
                        left: r.X.max(0.0).round() as u32,
                        top: r.Y.max(0.0).round() as u32,
                        width: r.Width.max(0.0).round() as u32,
                        height: r.Height.max(0.0).round() as u32,
                    });
                }
            }
        }

        Ok(OcrDocument {
            plain_text: plain.trim_end().to_string(),
            words,
            language: options.languages.clone(),
            engine: ENGINE_ID.to_string(),
            elapsed_ms: started.elapsed().as_millis() as u64,
            image_png_base64: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                image_png,
            ),
        })
    }
}

/// "tur+eng" dizgisini BCP-47 etiketlerine cevirip ilk calisan motoru secer.
fn pick_win_engine(languages: &str) -> Result<WinOcr, OcrError> {
    let mut wanted: Vec<String> = Vec::new();
    for part in languages.split([',', '+']) {
        match part.trim().to_ascii_lowercase().as_str() {
            "" => {}
            "tur" | "tr" | "tr-tr" => wanted.push("tr-TR".into()),
            "eng" | "en" | "en-us" | "en-gb" => wanted.push("en-US".into()),
            "ara" | "ar" | "ar-sa" => wanted.push("ar-SA".into()),
            "jpn" | "ja" | "ja-jp" => wanted.push("ja".into()),
            "chi_sim" | "zh-hans" | "zh-cn" => wanted.push("zh-Hans".into()),
            "chi_tra" | "zh-hant" | "zh-tw" => wanted.push("zh-Hant".into()),
            "kor" | "ko" | "ko-kr" => wanted.push("ko".into()),
            other => wanted.push(other.to_string()),
        }
    }
    if wanted.is_empty() {
        wanted.push("tr-TR".into());
        wanted.push("en-US".into());
    }
    for tag in &wanted {
        if let Ok(lang) = Language::CreateLanguage(&HSTRING::from(tag.as_str())) {
            if let Ok(engine) = WinOcr::TryCreateFromLanguage(&lang) {
                return Ok(engine);
            }
        }
    }
    Err(OcrError::Image(format!(
        "Windows OCR dil paketi yok (istenen {wanted:?}); Ayarlar → Dil paketlerini kurun."
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winocr_smoke() {
        if !WindowsOcrEngine::available() {
            eprintln!("Windows OCR dil paketi yok — test atlandı");
            return;
        }
        let png = std::fs::read("../test-tr.png").expect("test-tr.png depo kökünde olmalı");
        let opts = OcrOptions {
            languages: "tur+eng".into(),
            ..Default::default()
        };
        let doc = WindowsOcrEngine
            .recognize_png(&png, &opts)
            .expect("WinRT OCR başarısız");
        assert!(
            !doc.plain_text.is_empty(),
            "Windows OCR metin üretmedi"
        );
        eprintln!("winocr: {} karakter, {} kelime", doc.plain_text.len(), doc.words.len());
    }
}
