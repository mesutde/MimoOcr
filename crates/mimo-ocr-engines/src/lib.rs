//! Engine adapters that normalize backends into [`mimo_ocr_core::OcrEngine`].

mod mock;
#[cfg(feature = "cli-tesseract")]
mod tesseract_cli;
#[cfg(all(feature = "windows-ocr", target_os = "windows"))]
mod windows_ocr;

pub use mock::MockEngine;
#[cfg(feature = "cli-tesseract")]
pub use tesseract_cli::{discover_tesseract, TesseractCliEngine};
#[cfg(all(feature = "windows-ocr", target_os = "windows"))]
pub use windows_ocr::WindowsOcrEngine;

use mimo_ocr_core::{CancellationToken, OcrDocument, OcrEngine, OcrError, OcrImage, OcrOptions};

/// Registry used by CLI / UI to pick engines by id.
pub struct EngineRegistry {
    engines: Vec<Box<dyn OcrEngine>>,
}

impl EngineRegistry {
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
        }
    }

    pub fn register(&mut self, engine: Box<dyn OcrEngine>) {
        self.engines.push(engine);
    }

    pub fn ids(&self) -> Vec<&'static str> {
        self.engines.iter().map(|e| e.id()).collect()
    }

    pub fn get(&self, id: &str) -> Result<&dyn OcrEngine, OcrError> {
        self.engines
            .iter()
            .find(|e| e.id() == id)
            .map(|e| e.as_ref())
            .ok_or_else(|| OcrError::EngineUnavailable(id.to_string(), "not registered".into()))
    }

    pub fn recognize(
        &self,
        id: &str,
        image: &OcrImage,
        options: &OcrOptions,
        cancel: &CancellationToken,
    ) -> Result<OcrDocument, OcrError> {
        self.get(id)?.recognize(image, options, cancel)
    }

    /// All engines available on this machine (mock + tesseract + windows-ocr…).
    pub fn phase0_default() -> Self {
        let mut reg = Self::new();
        reg.register(Box::new(MockEngine::new()));
        #[cfg(feature = "cli-tesseract")]
        {
            if let Ok(path) = discover_tesseract() {
                reg.register(Box::new(TesseractCliEngine::new(path)));
            }
        }
        #[cfg(all(feature = "windows-ocr", target_os = "windows"))]
        {
            if let Ok(win) = WindowsOcrEngine::try_new() {
                reg.register(Box::new(win));
            }
        }
        reg
    }
}

impl Default for EngineRegistry {
    fn default() -> Self {
        Self::phase0_default()
    }
}

/// Locate tessdata for Tesseract (`TESSDATA_PREFIX` style directory).
///
/// Search order:
/// 1. explicit preferred path
/// 2. `MIMO_TESSDATA` then `TESSDATA_PREFIX` env
/// 3. next to the current executable (`tessdata/`, `tesseract/tessdata/`, …)
/// 4. project-relative assets
/// 5. system Tesseract install
/// Strip Windows extended-length prefixes (`\\?\`, `//?/`) that Tesseract cannot open.
pub fn clean_path_for_tesseract(p: &std::path::Path) -> std::path::PathBuf {
    let s = p.to_string_lossy();
    let cleaned = s
        .strip_prefix(r"\\?\UNC\")
        .map(|r| format!(r"\\{r}"))
        .or_else(|| s.strip_prefix(r"\\?\").map(|r| r.to_string()))
        .or_else(|| s.strip_prefix("//?/").map(|r| r.to_string()))
        .unwrap_or_else(|| s.to_string());
    // Also normalize forward slashes Tesseract sometimes prints
    let cleaned = cleaned.replace('/', "\\");
    std::path::PathBuf::from(cleaned)
}

/// True if directory contains usable traineddata (eng or tur).
fn has_tessdata(p: &std::path::Path) -> bool {
    let p = clean_path_for_tesseract(p);
    if !p.is_dir() {
        return false;
    }
    p.join("eng.traineddata").exists() || p.join("tur.traineddata").exists()
}

/// Does this tessdata dir contain every language code (e.g. tur, eng)?
pub fn tessdata_has_langs(p: &std::path::Path, langs: &[String]) -> bool {
    let p = clean_path_for_tesseract(p);
    if !p.is_dir() {
        return false;
    }
    if langs.is_empty() {
        return has_tessdata(&p);
    }
    langs.iter().all(|code| {
        code.split('+')
            .filter(|c| !c.is_empty())
            .all(|c| p.join(format!("{c}.traineddata")).exists())
    })
}

/// All plausible tessdata directories, best-first.
pub fn tessdata_candidates() -> Vec<std::path::PathBuf> {
    let mut out: Vec<std::path::PathBuf> = Vec::new();

    for key in ["MIMO_TESSDATA", "TESSDATA_PREFIX"] {
        if let Ok(v) = std::env::var(key) {
            if !v.trim().is_empty() {
                out.push(std::path::PathBuf::from(v));
            }
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // Packaged / next-to-exe layouts first (after env).
            out.push(dir.join("tesseract-runtime/tessdata"));
            out.push(dir.join("tesseract/tessdata"));
            out.push(dir.join("resources/tesseract-runtime/tessdata"));
            out.push(dir.join("resources/tesseract/tessdata"));
            out.push(dir.join("resources/tessdata"));
            out.push(dir.join("tessdata"));
        }
        // Walk up toward workspace root (target/debug → project).
        let mut anc = dir_ancestors(exe.parent().map(|p| p.to_path_buf()));
        for dir in anc.drain(..) {
            out.push(dir.join("assets/tesseract-runtime/tessdata"));
            out.push(dir.join("assets/models/tessdata"));
            out.push(dir.join("tessdata"));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("assets/tesseract-runtime/tessdata"));
        out.push(cwd.join("assets/models/tessdata"));
        out.push(cwd.join("tessdata"));
        out.push(cwd.join("target/debug/tessdata"));
    }

    out.push(std::path::PathBuf::from(
        "assets/tesseract-runtime/tessdata",
    ));
    out.push(std::path::PathBuf::from("assets/models/tessdata"));
    out.push(std::path::PathBuf::from(
        "C:/Program Files/Tesseract-OCR/tessdata",
    ));

    // Dedup while keeping order
    let mut seen = std::collections::HashSet::new();
    out.retain(|p| {
        let k = p.to_string_lossy().to_string().to_lowercase();
        seen.insert(k)
    });
    out
}

fn dir_ancestors(start: Option<std::path::PathBuf>) -> Vec<std::path::PathBuf> {
    let mut v = Vec::new();
    if let Some(mut p) = start {
        for _ in 0..6 {
            v.push(p.clone());
            if !p.pop() {
                break;
            }
        }
    }
    v
}

/// Locate tessdata directory that actually contains model files.
/// Returned path is cleaned for Tesseract (no `\\?\` prefix).
pub fn resolve_tessdata(preferred: Option<&std::path::Path>) -> Option<std::path::PathBuf> {
    if let Some(p) = preferred {
        let p = clean_path_for_tesseract(p);
        if has_tessdata(&p) {
            return Some(p);
        }
    }
    tessdata_candidates()
        .into_iter()
        .map(|p| clean_path_for_tesseract(&p))
        .find(|p| has_tessdata(p))
}

/// Resolve tessdata that contains every requested language code.
pub fn resolve_tessdata_for_langs(langs: &[String]) -> Option<std::path::PathBuf> {
    // Flatten "tur,eng" style entries if any
    let mut codes: Vec<String> = Vec::new();
    for l in langs {
        for part in l.split([',', '+']) {
            let t = part.trim();
            if !t.is_empty() {
                codes.push(t.to_string());
            }
        }
    }
    if codes.is_empty() {
        return resolve_tessdata(None);
    }
    tessdata_candidates()
        .into_iter()
        .map(|p| clean_path_for_tesseract(&p))
        .find(|p| tessdata_has_langs(p, &codes))
        .or_else(|| resolve_tessdata(None))
}
