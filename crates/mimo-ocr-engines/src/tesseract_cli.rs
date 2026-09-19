//! Tesseract CLI adapter — process isolation, easy packaging experiments.
//!
//! Uses the installed `tesseract.exe` (UB Mannheim build on this machine).
//! Output mode: TSV via stdout for word boxes + plain text.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use mimo_ocr_core::{
    preprocess, CancellationToken, EngineCapabilities, LanguageInfo, OcrDocument, OcrEngine,
    OcrError, OcrImage, OcrOptions, OcrPage, OcrRegion, PreprocessOptions, Quad, RegionType,
};

#[derive(Debug, Clone)]
pub struct TesseractCliEngine {
    tesseract_path: PathBuf,
}

impl TesseractCliEngine {
    pub fn new(tesseract_path: impl Into<PathBuf>) -> Self {
        Self {
            tesseract_path: tesseract_path.into(),
        }
    }

    fn tessdata_dir(&self, options: &OcrOptions) -> Option<PathBuf> {
        if let Some(p) = options.model_dir.as_deref() {
            let p = crate::clean_path_for_tesseract(p);
            if p.join("eng.traineddata").exists() || p.join("tur.traineddata").exists() {
                return Some(p);
            }
        }
        crate::resolve_tessdata_for_langs(&options.languages)
            .or_else(|| crate::resolve_tessdata(None))
    }

    fn run_tesseract(
        &self,
        image_path: &Path,
        options: &OcrOptions,
        tessdata: Option<&Path>,
        lang_override: Option<&str>,
    ) -> Result<(String, String), OcrError> {
        let lang = lang_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| options.language_spec());
        let tmp_dir = std::env::temp_dir().join(format!("mimo-ocr-{}", std::process::id()));
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| OcrError::Failed(format!("temp dir: {e}")))?;
        let base = tmp_dir.join("ocr-out");

        let mut cmd = Command::new(&self.tesseract_path);
        cmd.arg(image_path)
            .arg(&base)
            .arg("-l")
            .arg(&lang)
            .arg("--psm")
            .arg("3");
        let tessdata = tessdata.map(|p| crate::clean_path_for_tesseract(p));
        if let Some(td) = tessdata {
            // Must contain *.traineddata directly. No \\?\ prefix — Tesseract fails.
            if !td.join("eng.traineddata").exists() && !td.join("tur.traineddata").exists() {
                return Err(OcrError::ModelMissing(format!(
                    "{lang}: no traineddata in {}",
                    td.display()
                )));
            }
            cmd.env("TESSDATA_PREFIX", &td);
            cmd.env("MIMO_TESSDATA", &td);
        } else {
            return Err(OcrError::ModelMissing(format!(
                "{lang}: tessdata directory not found (tur/eng traineddata)"
            )));
        }
        cmd.arg("txt").arg("tsv");
        cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| OcrError::EngineUnavailable("tesseract-cli".into(), e.to_string()))?;

        let timeout = options
            .timeout
            .unwrap_or(Duration::from_secs(20))
            .max(Duration::from_secs(3));
        let deadline = Instant::now() + timeout;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let out_handle = stdout.map(|mut s| {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let _ = s.read_to_end(&mut buf);
                buf
            })
        });
        let err_handle = stderr.map(|mut s| {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let _ = s.read_to_end(&mut buf);
                buf
            })
        });

        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(OcrError::Failed(format!(
                            "tesseract timeout after {}s",
                            timeout.as_secs()
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(40));
                }
                Err(e) => return Err(OcrError::Failed(format!("tesseract wait: {e}"))),
            }
        };

        let stdout = out_handle.and_then(|h| h.join().ok()).unwrap_or_default();
        let stderr = err_handle.and_then(|h| h.join().ok()).unwrap_or_default();

        if !status.success() {
            let err = String::from_utf8_lossy(&stderr);
            if err.contains("tessdata") || err.contains("Error opening data file") {
                return Err(OcrError::ModelMissing(format!("{lang}: {err}")));
            }
            return Err(OcrError::Failed(err.trim().to_string()));
        }
        let _ = stdout;

        let plain = std::fs::read_to_string(base.with_extension("txt")).unwrap_or_default();
        let tsv = std::fs::read_to_string(base.with_extension("tsv")).unwrap_or_default();
        let _ = std::fs::remove_file(base.with_extension("txt"));
        let _ = std::fs::remove_file(base.with_extension("tsv"));
        let _ = std::fs::remove_dir_all(&tmp_dir);
        Ok((plain, tsv))
    }
}

#[derive(Debug, Clone)]
struct WordBox {
    text: String,
    conf: Option<f32>,
    quad: Quad,
}

fn parse_tsv_words(tsv: &str) -> Vec<WordBox> {
    let mut words = Vec::new();
    for (i, line) in tsv.lines().enumerate() {
        if i == 0 {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 12 {
            continue;
        }
        if cols[0] != "5" {
            continue;
        }
        let text = cols[11].trim();
        if text.is_empty() {
            continue;
        }
        let conf: f32 = cols[10].parse().unwrap_or(-1.0);
        let left: f32 = cols[6].parse().unwrap_or(0.0);
        let top: f32 = cols[7].parse().unwrap_or(0.0);
        let width: f32 = cols[8].parse().unwrap_or(0.0);
        let height: f32 = cols[9].parse().unwrap_or(0.0);
        let conf = if conf < 0.0 { None } else { Some(conf / 100.0) };
        words.push(WordBox {
            text: text.to_string(),
            conf,
            quad: Quad::new(left, top, width, height),
        });
    }
    words
}

fn group_words_into_lines(mut words: Vec<WordBox>) -> Vec<OcrRegion> {
    words.sort_by(|a, b| {
        a.quad
            .y
            .partial_cmp(&b.quad.y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut lines: Vec<Vec<WordBox>> = Vec::new();
    for word in words {
        let mut placed = false;
        for line in lines.iter_mut() {
            if let Some(first) = line.first() {
                if (first.quad.y - word.quad.y).abs() < 12.0 {
                    line.push(word.clone());
                    placed = true;
                    break;
                }
            }
        }
        if !placed {
            lines.push(vec![word]);
        }
    }

    let mut line_regions = Vec::new();
    for mut line in lines {
        line.sort_by(|a, b| {
            a.quad
                .x
                .partial_cmp(&b.quad.x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let text = line
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let confs: Vec<f32> = line.iter().filter_map(|w| w.conf).collect();
        let conf = if confs.is_empty() {
            None
        } else {
            Some(confs.iter().sum::<f32>() / confs.len() as f32)
        };

        let min_x = line
            .iter()
            .map(|w| w.quad.x)
            .fold(f32::INFINITY, f32::min);
        let min_y = line
            .iter()
            .map(|w| w.quad.y)
            .fold(f32::INFINITY, f32::min);
        let max_x = line
            .iter()
            .map(|w| w.quad.x + w.quad.w)
            .fold(f32::NEG_INFINITY, f32::max);
        let max_y = line
            .iter()
            .map(|w| w.quad.y + w.quad.h)
            .fold(f32::NEG_INFINITY, f32::max);

        let children = line
            .into_iter()
            .map(|w| {
                OcrRegion::leaf(w.text, w.conf, w.quad, RegionType::Word)
            })
            .collect();

        line_regions.push(OcrRegion {
            text,
            confidence: conf,
            bounds: Quad::new(
                min_x,
                min_y,
                (max_x - min_x).max(1.0),
                (max_y - min_y).max(1.0),
            ),
            region_type: RegionType::Line,
            children,
        });
    }
    line_regions
}

/// Probe app-local runtime first, then system installs + PATH.
pub fn discover_tesseract() -> Result<PathBuf, OcrError> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(v) = std::env::var("MIMO_TESSERACT") {
        candidates.push(PathBuf::from(v));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("tesseract/tesseract.exe"));
            candidates.push(dir.join("tesseract-runtime/tesseract.exe"));
            candidates.push(dir.join("resources/tesseract/tesseract.exe"));
            candidates.push(dir.join("resources/tesseract-runtime/tesseract.exe"));
            candidates.push(dir.join("tesseract.exe"));
        }
    }

    candidates.push(PathBuf::from("assets/tesseract-runtime/tesseract.exe"));
    candidates.push(PathBuf::from("../assets/tesseract-runtime/tesseract.exe"));
    candidates.push(PathBuf::from("C:/Program Files/Tesseract-OCR/tesseract.exe"));
    candidates.push(PathBuf::from("C:/Program Files (x86)/Tesseract-OCR/tesseract.exe"));
    candidates.push(PathBuf::from(format!(
        "{}/scoop/apps/tesseract/current/tesseract.exe",
        std::env::var("USERPROFILE").unwrap_or_default()
    )));

    for c in candidates {
        let c = crate::clean_path_for_tesseract(&c);
        if c.exists() {
            return Ok(c);
        }
    }
    if let Ok(out) = Command::new("where").arg("tesseract").output() {
        if out.status.success() {
            if let Some(line) = String::from_utf8_lossy(&out.stdout).lines().next() {
                let p = PathBuf::from(line.trim());
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }
    Err(OcrError::EngineUnavailable(
        "tesseract-cli".into(),
        "tesseract.exe not found (app-local runtime or system install)".into(),
    ))
}

/// Count Turkish-specific letters that mixed eng models often flatten.
fn tr_diacritic_score(s: &str) -> usize {
    s.chars()
        .filter(|c| {
            matches!(
                c,
                'ç' | 'ğ' | 'ı' | 'İ' | 'ö' | 'ş' | 'ü' | 'Ç' | 'Ğ' | 'Ö' | 'Ş' | 'Ü'
            )
        })
        .count()
}

fn scale_words(words: &mut [WordBox], scale: f32) {
    if (scale - 1.0).abs() <= 0.01 {
        return;
    }
    for w in words {
        w.quad.x /= scale;
        w.quad.y /= scale;
        w.quad.w /= scale;
        w.quad.h /= scale;
    }
}

impl OcrEngine for TesseractCliEngine {
    fn id(&self) -> &'static str {
        "tesseract-cli"
    }

    fn display_name(&self) -> &'static str {
        "Tesseract 5 (CLI)"
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
        let mut langs = vec![
            LanguageInfo::new("tur", "Turkish"),
            LanguageInfo::new("eng", "English"),
        ];
        if let Some(td) = crate::resolve_tessdata(None) {
            if let Ok(rd) = std::fs::read_dir(&td) {
                for e in rd.flatten() {
                    if let Some(name) = e.file_name().to_str() {
                        if let Some(code) = name.strip_suffix(".traineddata") {
                            if !matches!(code, "osd" | "tur" | "eng") {
                                langs.push(LanguageInfo::new(code, code.to_string()));
                            }
                        }
                    }
                }
            }
        }
        langs
    }

    fn recognize(
        &self,
        image: &OcrImage,
        options: &OcrOptions,
        cancel: &CancellationToken,
    ) -> Result<OcrDocument, OcrError> {
        cancel.check()?;
        let start = Instant::now();

        // Size-aware preprocess: full-screen 2× upscale is a freeze risk.
        let prep_opts =
            PreprocessOptions::for_quality_sized(options.quality, image.width, image.height);
        let scale = prep_opts.upscale.max(0.1);
        let processed = preprocess(image, &prep_opts);

        let safe_name = image
            .source_name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>();
        let tmp_img = std::env::temp_dir().join(format!(
            "mimo-cli-{}-{}.png",
            std::process::id(),
            safe_name
        ));
        processed.save_png(&tmp_img)?;

        let tessdata = self.tessdata_dir(options);
        let primary = self.run_tesseract(&tmp_img, options, tessdata.as_deref(), None);
        let (mut plain, mut tsv) = primary?;
        let mut used_lang = options.language_spec();
        cancel.check()?;

        // Mixed tur+eng can flatten TR capitals. Second pass only on modest
        // images so interactive full-screen OCR stays responsive.
        let langs = options.language_spec();
        let pixels = (image.width as u64) * (image.height as u64);
        if langs.contains("tur") && langs.contains("eng") && pixels <= 1_200_000 {
            if let Ok((plain_tr, tsv_tr)) =
                self.run_tesseract(&tmp_img, options, tessdata.as_deref(), Some("tur"))
            {
                cancel.check()?;
                let score_primary = tr_diacritic_score(&plain);
                let score_tr = tr_diacritic_score(&plain_tr);
                if score_tr > score_primary && !plain_tr.trim().is_empty() {
                    plain = plain_tr;
                    tsv = tsv_tr;
                    used_lang = "tur".to_string();
                }
            }
        }

        let _ = std::fs::remove_file(&tmp_img);

        let mut words = parse_tsv_words(&tsv);
        scale_words(&mut words, scale);
        let line_children = group_words_into_lines(words);

        let root = OcrRegion {
            text: plain.trim().to_string(),
            confidence: None,
            bounds: Quad::new(0.0, 0.0, image.width as f32, image.height as f32),
            region_type: RegionType::Page,
            children: line_children,
        };

        let page = OcrPage {
            index: 0,
            width: image.width,
            height: image.height,
            text: plain.trim().to_string(),
            regions: root,
        };

        Ok(OcrDocument::from_single_page(
            self.id(),
            page,
            Some(used_lang),
            start.elapsed(),
        ))
    }
}
