//! Shared OCR domain types for Mimo OCR.
//!
//! All engines adapt their native results into [`OcrDocument`] so the UI,
//! exporters, and benchmarks never depend on a specific OCR backend.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod preprocess;
pub use preprocess::{preprocess, PreprocessOptions};

/// Cooperative cancellation token shared across capture/OCR workers.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    pub fn check(&self) -> Result<(), OcrError> {
        if self.is_cancelled() {
            Err(OcrError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// In-memory image handed to an engine.
#[derive(Debug, Clone)]
pub struct OcrImage {
    pub width: u32,
    pub height: u32,
    /// RGB8 pixels, length = width * height * 3
    pub rgb: Vec<u8>,
    pub source_name: String,
}

impl OcrImage {
    pub fn from_rgb(width: u32, height: u32, rgb: Vec<u8>, source_name: impl Into<String>) -> Self {
        debug_assert_eq!(rgb.len(), (width as usize) * (height as usize) * 3);
        Self {
            width,
            height,
            rgb,
            source_name: source_name.into(),
        }
    }

    pub fn from_image_buffer(img: &image::RgbImage, source_name: impl Into<String>) -> Self {
        Self::from_rgb(img.width(), img.height(), img.as_raw().clone(), source_name)
    }

    pub fn from_path(path: &std::path::Path) -> Result<Self, OcrError> {
        let dyn_img =
            image::open(path).map_err(|e| OcrError::Image(path.display().to_string(), e.to_string()))?;
        let rgb = dyn_img.to_rgb8();
        Ok(Self::from_image_buffer(&rgb, path.display().to_string()))
    }

    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Write RGB buffer as PNG (used by capture prototypes and tests).
    pub fn save_png(&self, path: &std::path::Path) -> Result<(), OcrError> {
        let img = image::RgbImage::from_raw(self.width, self.height, self.rgb.clone())
            .ok_or_else(|| OcrError::InvalidImage("raw buffer size mismatch".into()))?;
        img.save(path)
            .map_err(|e| OcrError::Image(path.display().to_string(), e.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionType {
    Page,
    Paragraph,
    Line,
    Word,
    Block,
    Table,
    Cell,
    Formula,
    Barcode,
}

impl RegionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RegionType::Page => "page",
            RegionType::Paragraph => "paragraph",
            RegionType::Line => "line",
            RegionType::Word => "word",
            RegionType::Block => "block",
            RegionType::Table => "table",
            RegionType::Cell => "cell",
            RegionType::Formula => "formula",
            RegionType::Barcode => "barcode",
        }
    }
}

/// Axis-aligned box in image pixel space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Quad {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

/// Hierarchical OCR region (word / line / paragraph / table… share this shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrRegion {
    pub text: String,
    pub confidence: Option<f32>,
    pub bounds: Quad,
    pub region_type: RegionType,
    pub children: Vec<OcrRegion>,
}

impl OcrRegion {
    pub fn leaf(text: impl Into<String>, confidence: Option<f32>, bounds: Quad, region_type: RegionType) -> Self {
        Self {
            text: text.into(),
            confidence,
            bounds,
            region_type,
            children: Vec::new(),
        }
    }

    fn flatten_into(&self, out: &mut Vec<OcrRegion>, kind: RegionType) {
        if self.region_type == kind {
            out.push(self.clone());
        }
        for child in &self.children {
            child.flatten_into(out, kind);
        }
    }

    pub fn collect_type(&self, kind: RegionType) -> Vec<OcrRegion> {
        let mut out = Vec::new();
        self.flatten_into(&mut out, kind);
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrPage {
    pub index: usize,
    pub width: u32,
    pub height: u32,
    pub text: String,
    pub regions: OcrRegion,
}

impl OcrPage {
    pub fn words(&self) -> Vec<OcrRegion> {
        self.regions.collect_type(RegionType::Word)
    }

    pub fn lines(&self) -> Vec<OcrRegion> {
        self.regions.collect_type(RegionType::Line)
    }

    pub fn mean_word_confidence(&self) -> Option<f32> {
        let words = self.words();
        let scores: Vec<f32> = words.iter().filter_map(|w| w.confidence).collect();
        if scores.is_empty() {
            None
        } else {
            Some(scores.iter().sum::<f32>() / scores.len() as f32)
        }
    }
}

/// Canonical multi-engine OCR result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrDocument {
    pub pages: Vec<OcrPage>,
    pub plain_text: String,
    pub language: Option<String>,
    pub engine: String,
    pub elapsed_ms: u64,
}

impl OcrDocument {
    pub fn empty(engine: impl Into<String>) -> Self {
        Self {
            pages: Vec::new(),
            plain_text: String::new(),
            language: None,
            engine: engine.into(),
            elapsed_ms: 0,
        }
    }

    pub fn from_single_page(
        engine: impl Into<String>,
        page: OcrPage,
        language: Option<String>,
        elapsed: Duration,
    ) -> Self {
        let plain_text = page.text.clone();
        Self {
            pages: vec![page],
            plain_text,
            language,
            engine: engine.into(),
            elapsed_ms: elapsed.as_millis() as u64,
        }
    }

    pub fn mean_confidence(&self) -> Option<f32> {
        let vals: Vec<f32> = self
            .pages
            .iter()
            .filter_map(|p| p.mean_word_confidence())
            .collect();
        if vals.is_empty() {
            None
        } else {
            Some(vals.iter().sum::<f32>() / vals.len() as f32)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    /// ISO-like code used by the engine (`tur`, `eng`, `tur+eng`, …)
    pub code: String,
    pub display_name: String,
}

impl LanguageInfo {
    pub fn new(code: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            display_name: display_name.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityMode {
    Fast,
    Balanced,
    Accurate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineCapabilities {
    pub local: bool,
    pub requires_network: bool,
    pub provides_word_boxes: bool,
    pub provides_line_boxes: bool,
    pub supports_table: bool,
    pub experimental: bool,
    pub max_side_px: Option<u32>,
}

impl Default for EngineCapabilities {
    fn default() -> Self {
        Self {
            local: true,
            requires_network: false,
            provides_word_boxes: true,
            provides_line_boxes: true,
            supports_table: false,
            experimental: false,
            max_side_px: Some(8192),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OcrOptions {
    pub languages: Vec<String>,
    pub quality: QualityMode,
    /// Optional path hint for engine-specific tessdata / model dirs.
    pub model_dir: Option<std::path::PathBuf>,
    pub timeout: Option<Duration>,
}

impl Default for OcrOptions {
    fn default() -> Self {
        Self {
            languages: vec!["tur".into(), "eng".into()],
            quality: QualityMode::Balanced,
            model_dir: None,
            timeout: Some(Duration::from_secs(30)),
        }
    }
}

impl OcrOptions {
    pub fn language_spec(&self) -> String {
        if self.languages.is_empty() {
            "eng".to_string()
        } else {
            self.languages.join("+")
        }
    }

    pub fn tur_eng() -> Self {
        Self {
            languages: vec!["tur".into(), "eng".into()],
            ..Default::default()
        }
    }
}

#[derive(Debug, Error)]
pub enum OcrError {
    #[error("OCR cancelled")]
    Cancelled,
    #[error("image error for {0}: {1}")]
    Image(String, String),
    #[error("invalid image: {0}")]
    InvalidImage(String),
    #[error("engine `{0}` not available: {1}")]
    EngineUnavailable(String, String),
    #[error("model/language missing: {0}")]
    ModelMissing(String),
    #[error("OCR failed: {0}")]
    Failed(String),
    #[error("capture failed: {0}")]
    Capture(String),
}

/// Common interface every OCR backend implements.
pub trait OcrEngine: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn capabilities(&self) -> EngineCapabilities;
    fn supported_languages(&self) -> Vec<LanguageInfo>;

    fn recognize(
        &self,
        image: &OcrImage,
        options: &OcrOptions,
        cancel: &CancellationToken,
    ) -> Result<OcrDocument, OcrError>;

    /// Engines that need a warm-up should override this.
    fn warm_up(&self, _options: &OcrOptions) -> Result<(), OcrError> {
        Ok(())
    }
}

/// Simple character error rate (Levenshtein / reference length).
pub fn cer(reference: &str, hypothesis: &str) -> f32 {
    let r: Vec<char> = reference.chars().collect();
    let h: Vec<char> = hypothesis.chars().collect();
    if r.is_empty() {
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    let mut prev: Vec<usize> = (0..=h.len()).collect();
    let mut cur = vec![0usize; h.len() + 1];
    for (i, rc) in r.iter().enumerate() {
        cur[0] = i + 1;
        for (j, hc) in h.iter().enumerate() {
            let cost = if rc == hc { 0 } else { 1 };
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[h.len()] as f32 / r.len() as f32
}

/// Simple word error rate.
pub fn wer(reference: &str, hypothesis: &str) -> f32 {
    let r: Vec<&str> = reference.split_whitespace().collect();
    let h: Vec<&str> = hypothesis.split_whitespace().collect();
    if r.is_empty() {
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    let mut prev: Vec<usize> = (0..=h.len()).collect();
    let mut cur = vec![0usize; h.len() + 1];
    for (i, rw) in r.iter().enumerate() {
        cur[0] = i + 1;
        for (j, hw) in h.iter().enumerate() {
            let cost = if rw == hw { 0 } else { 1 };
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[h.len()] as f32 / r.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cer_exact_match_is_zero() {
        assert_eq!(cer("Merhaba dünya", "Merhaba dünya"), 0.0);
    }

    #[test]
    fn cer_detects_tr_char_error() {
        let c = cer("ışık", "isik");
        assert!(c > 0.0);
    }

    #[test]
    fn cancellation_token_works() {
        let t = CancellationToken::new();
        assert!(t.check().is_ok());
        t.cancel();
        assert!(matches!(t.check(), Err(OcrError::Cancelled)));
    }
}
