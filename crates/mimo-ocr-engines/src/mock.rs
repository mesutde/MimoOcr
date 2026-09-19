//! Deterministic mock engine for tests, UI wiring, and benchmark harnesses.

use std::time::{Duration, Instant};

use mimo_ocr_core::{
    CancellationToken, EngineCapabilities, LanguageInfo, OcrDocument, OcrEngine, OcrError,
    OcrImage, OcrOptions, OcrPage, OcrRegion, Quad, RegionType,
};

pub struct MockEngine {
    canned_text: String,
    confidence: f32,
}

impl MockEngine {
    pub fn new() -> Self {
        Self {
            canned_text: "Merhaba dünya\nHello world\nÇĞİÖŞÜ çğıöşü 0123456789".into(),
            confidence: 0.92,
        }
    }

    pub fn with_text(text: impl Into<String>, confidence: f32) -> Self {
        Self {
            canned_text: text.into(),
            confidence,
        }
    }
}

impl Default for MockEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrEngine for MockEngine {
    fn id(&self) -> &'static str {
        "mock"
    }

    fn display_name(&self) -> &'static str {
        "Mock (Phase 0)"
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            local: true,
            requires_network: false,
            provides_word_boxes: true,
            provides_line_boxes: true,
            supports_table: false,
            experimental: false,
            max_side_px: None,
        }
    }

    fn supported_languages(&self) -> Vec<LanguageInfo> {
        vec![
            LanguageInfo::new("tur", "Turkish"),
            LanguageInfo::new("eng", "English"),
        ]
    }

    fn recognize(
        &self,
        image: &OcrImage,
        options: &OcrOptions,
        cancel: &CancellationToken,
    ) -> Result<OcrDocument, OcrError> {
        cancel.check()?;
        let start = Instant::now();
        let mut children = Vec::new();
        let mut y = 8.0f32;
        for (i, line) in self.canned_text.lines().enumerate() {
            let mut words = Vec::new();
            let mut x = 8.0f32;
            for word in line.split_whitespace() {
                let w = (word.chars().count() as f32) * 12.0;
                words.push(OcrRegion::leaf(
                    word,
                    Some(self.confidence),
                    Quad::new(x, y, w, 20.0),
                    RegionType::Word,
                ));
                x += w + 6.0;
            }
            let line_text = line.to_string();
            children.push(OcrRegion {
                text: line_text,
                confidence: Some(self.confidence),
                bounds: Quad::new(8.0, y, (x - 8.0).max(1.0), 22.0),
                region_type: RegionType::Line,
                children: words,
            });
            y += 28.0;
            let _ = i;
        }

        let root = OcrRegion {
            text: self.canned_text.clone(),
            confidence: Some(self.confidence),
            bounds: Quad::new(0.0, 0.0, image.width as f32, image.height as f32),
            region_type: RegionType::Page,
            children,
        };

        let page = OcrPage {
            index: 0,
            width: image.width,
            height: image.height,
            text: self.canned_text.clone(),
            regions: root,
        };

        // Simulate a tiny amount of work so elapsed_ms is non-zero in benches.
        std::thread::sleep(Duration::from_millis(1));
        cancel.check()?;

        Ok(OcrDocument::from_single_page(
            self.id(),
            page,
            Some(options.language_spec()),
            start.elapsed(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_produces_words_and_tr_chars() {
        let engine = MockEngine::new();
        let img = OcrImage::from_rgb(200, 80, vec![255u8; 200 * 80 * 3], "test");
        let doc = engine
            .recognize(&img, &OcrOptions::tur_eng(), &CancellationToken::new())
            .unwrap();
        assert!(doc.plain_text.contains("Merhaba"));
        assert!(doc.plain_text.contains('ş') || doc.plain_text.contains('Ş') || doc.plain_text.contains('ü'));
        assert!(!doc.pages[0].words().is_empty());
        assert!(doc.mean_confidence().unwrap() > 0.5);
    }
}
