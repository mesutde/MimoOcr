//! Windows.Media.Ocr (WinRT) adapter — system language-pack OCR.
//!
//! No extra model files; quality/coverage depends on installed Windows
//! language packs (e.g. Turkish OCR pack for `tr-TR`).

use std::io::Cursor;
use std::time::Instant;

use mimo_ocr_core::{
    CancellationToken, EngineCapabilities, LanguageInfo, OcrDocument, OcrEngine, OcrError,
    OcrImage, OcrOptions, OcrPage, OcrRegion, Quad, RegionType,
};
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::BitmapDecoder;
use windows::Media::Ocr::OcrEngine as WinOcr;
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

pub const ENGINE_ID: &str = "windows-ocr";

pub struct WindowsOcrEngine {
    available: Vec<LanguageInfo>,
}

impl WindowsOcrEngine {
    /// Returns Ok only if WinRT OCR engine can be created on this machine.
    pub fn try_new() -> Result<Self, OcrError> {
        let available = list_available_languages();
        if available.is_empty() {
            return Err(OcrError::EngineUnavailable(
                ENGINE_ID.into(),
                "Windows OCR dil paketi bulunamadı".into(),
            ));
        }
        WinOcr::TryCreateFromUserProfileLanguages()
            .map_err(|e| {
                OcrError::EngineUnavailable(ENGINE_ID.into(), format!("create failed: {e}"))
            })?;
        Ok(Self { available })
    }
}

fn list_available_languages() -> Vec<LanguageInfo> {
    let mut out = Vec::new();
    if let Ok(list) = WinOcr::AvailableRecognizerLanguages() {
        for i in 0..list.Size().unwrap_or(0) {
            if let Ok(lang) = list.GetAt(i) {
                if let Ok(tag) = lang.LanguageTag() {
                    let code = tag.to_string();
                    let display = code.clone();
                    out.push(LanguageInfo::new(code, display));
                }
            }
        }
    }
    out
}

fn pick_win_engine(options: &OcrOptions) -> Result<WinOcr, OcrError> {
    // Prefer explicit language packs when requested (tur / eng / tr-TR / …).
    let mut wanted: Vec<String> = Vec::new();
    for l in &options.languages {
        for part in l.split([',', '+']) {
            let t = part.trim().to_ascii_lowercase();
            if t.is_empty() {
                continue;
            }
            match t.as_str() {
                "tur" | "tr" | "tr-tr" => wanted.push("tr-TR".into()),
                "eng" | "en" | "en-us" | "en-gb" => wanted.push("en-US".into()),
                other => wanted.push(other.to_string()),
            }
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

    WinOcr::TryCreateFromUserProfileLanguages().map_err(|e| {
        OcrError::EngineUnavailable(
            ENGINE_ID.into(),
            format!("dil paketi yok (isteğe göre {wanted:?}): {e}"),
        )
    })
}

fn png_stream(img: &OcrImage) -> Result<InMemoryRandomAccessStream, OcrError> {
    let rgb = image::RgbImage::from_raw(img.width, img.height, img.rgb.clone()).ok_or_else(
        || OcrError::InvalidImage("rgb buffer size mismatch".into()),
    )?;
    let mut buf = Vec::new();
    rgb.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| OcrError::Failed(format!("png encode: {e}")))?;

    let stream = InMemoryRandomAccessStream::new()
        .map_err(|e| OcrError::Failed(format!("stream: {e}")))?;
    let writer = DataWriter::CreateDataWriter(&stream)
        .map_err(|e| OcrError::Failed(format!("data writer: {e}")))?;
    writer
        .WriteBytes(&buf)
        .map_err(|e| OcrError::Failed(format!("write bytes: {e}")))?;
    writer
        .StoreAsync()
        .and_then(|op| op.get())
        .map_err(|e| OcrError::Failed(format!("store: {e}")))?;
    writer
        .FlushAsync()
        .and_then(|op| op.get())
        .map_err(|e| OcrError::Failed(format!("flush: {e}")))?;
    // Detach so dropping DataWriter does not close the stream (0x80000013).
    writer
        .DetachStream()
        .map_err(|e| OcrError::Failed(format!("detach: {e}")))?;
    stream
        .Seek(0)
        .map_err(|e| OcrError::Failed(format!("seek: {e}")))?;
    Ok(stream)
}

impl OcrEngine for WindowsOcrEngine {
    fn id(&self) -> &'static str {
        ENGINE_ID
    }

    fn display_name(&self) -> &'static str {
        "Windows OCR (sistem)"
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            local: true,
            requires_network: false,
            provides_word_boxes: true,
            provides_line_boxes: true,
            supports_table: false,
            experimental: false,
            max_side_px: Some(10000),
        }
    }

    fn supported_languages(&self) -> Vec<LanguageInfo> {
        if self.available.is_empty() {
            vec![
                LanguageInfo::new("tr-TR", "Türkçe (sistem)"),
                LanguageInfo::new("en-US", "English (system)"),
            ]
        } else {
            self.available.clone()
        }
    }

    fn recognize(
        &self,
        image: &OcrImage,
        options: &OcrOptions,
        cancel: &CancellationToken,
    ) -> Result<OcrDocument, OcrError> {
        cancel.check()?;
        let start = Instant::now();

        let engine = pick_win_engine(options)?;
        let stream = png_stream(image)?;
        let decoder = BitmapDecoder::CreateAsync(&stream)
            .and_then(|op| op.get())
            .map_err(|e| OcrError::Failed(format!("bitmap decoder: {e}")))?;
        let bitmap = decoder
            .GetSoftwareBitmapAsync()
            .and_then(|op| op.get())
            .map_err(|e| OcrError::Failed(format!("software bitmap: {e}")))?;

        cancel.check()?;

        let result = engine
            .RecognizeAsync(&bitmap)
            .and_then(|op| op.get())
            .map_err(|e| OcrError::Failed(format!("recognize: {e}")))?;

        let plain = result
            .Text()
            .map(|t| t.to_string())
            .unwrap_or_default();

        let mut line_children: Vec<OcrRegion> = Vec::new();
        if let Ok(lines) = result.Lines() {
            for i in 0..lines.Size().unwrap_or(0) {
                let Ok(line) = lines.GetAt(i) else {
                    continue;
                };
                let line_text = line.Text().map(|t| t.to_string()).unwrap_or_default();
                let mut words = Vec::new();
                if let Ok(win_words) = line.Words() {
                    for j in 0..win_words.Size().unwrap_or(0) {
                        let Ok(w) = win_words.GetAt(j) else {
                            continue;
                        };
                        let wtext = w.Text().map(|t| t.to_string()).unwrap_or_default();
                        if wtext.trim().is_empty() {
                            continue;
                        }
                        let r = w.BoundingRect().unwrap_or(windows::Foundation::Rect {
                            X: 0.0,
                            Y: 0.0,
                            Width: 0.0,
                            Height: 0.0,
                        });
                        words.push(OcrRegion::leaf(
                            wtext,
                            None,
                            Quad::new(r.X, r.Y, r.Width, r.Height),
                            RegionType::Word,
                        ));
                    }
                }
                let bounds = if let Some(first) = words.first() {
                    let mut x0 = first.bounds.x;
                    let mut y0 = first.bounds.y;
                    let mut x1 = first.bounds.x + first.bounds.w;
                    let mut y1 = first.bounds.y + first.bounds.h;
                    for w in &words {
                        x0 = x0.min(w.bounds.x);
                        y0 = y0.min(w.bounds.y);
                        x1 = x1.max(w.bounds.x + w.bounds.w);
                        y1 = y1.max(w.bounds.y + w.bounds.h);
                    }
                    Quad::new(x0, y0, (x1 - x0).max(1.0), (y1 - y0).max(1.0))
                } else {
                    Quad::new(0.0, 0.0, image.width as f32, image.height as f32)
                };
                line_children.push(OcrRegion {
                    text: line_text,
                    confidence: None,
                    bounds,
                    region_type: RegionType::Line,
                    children: words,
                });
            }
        }

        let root = OcrRegion {
            text: plain.clone(),
            confidence: None,
            bounds: Quad::new(0.0, 0.0, image.width as f32, image.height as f32),
            region_type: RegionType::Page,
            children: line_children,
        };

        let page = OcrPage {
            index: 0,
            width: image.width,
            height: image.height,
            text: plain.clone(),
            regions: root,
        };

        let lang_tag = options
            .languages
            .first()
            .cloned()
            .or_else(|| self.available.first().map(|l| l.code.clone()));

        Ok(OcrDocument::from_single_page(
            self.id(),
            page,
            lang_tag,
            start.elapsed(),
        ))
    }
}
