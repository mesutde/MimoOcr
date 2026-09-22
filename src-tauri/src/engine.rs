//! OCR motor soyutlaması.
//!
//! Bütün motorlar (Tesseract, PaddleOCR/ONNX, sistem OCR'leri) `OcrEngine`
//! arayüzünün arkasında çalışır; sonuçlar ortak `OcrDocument` modeline indirgenir.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OcrError {
    #[error("Tesseract çalıştırılabilir dosyası bulunamadı. MIMO_TESSERACT ortam değişkeniyle yol belirtin.")]
    EngineNotFound,
    #[error("Motor çalıştırılamadı: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("Motor hata ile çıktı (kod {code}): {stderr}")]
    EngineFailed { code: i32, stderr: String },
    #[error("Görüntü işlenemedi: {0}")]
    Image(String),
    #[error("Panoda görsel yok")]
    NoClipboardImage,
}

impl Serialize for OcrError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrOptions {
    /// Tesseract dil dizgisi, örn. "tur", "eng", "tur+eng"
    pub languages: String,
    /// Sayfa segmentasyon modu (3, 6, 7, 11…)
    pub psm: String,
    /// Ön işleme büyütme katsayısı (1 = yok)
    pub scale: u32,
    /// OCR sonrası sonucu otomatik panoya kopyala
    pub auto_copy: bool,
}

impl Default for OcrOptions {
    fn default() -> Self {
        Self {
            languages: "tur+eng".into(),
            psm: "3".into(),
            scale: 1,
            auto_copy: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrWord {
    pub text: String,
    pub confidence: f32,
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrDocument {
    pub plain_text: String,
    pub words: Vec<OcrWord>,
    pub language: String,
    pub engine: String,
    pub elapsed_ms: u64,
    /// Önizleme için işlenen görselin PNG'si (base64)
    pub image_png_base64: String,
}

/// Bütün OCR motorlarının uyguladığı ortak arayüz.
pub trait OcrEngine: Send + Sync {
    fn id(&self) -> &'static str;
    fn recognize(&self, image_png: &[u8], options: &OcrOptions) -> Result<OcrDocument, OcrError>;
}

/// Tesseract TSV çıktısından kelime düzeyi (level=5) kayıtları ayıklar.
pub fn parse_tsv_words(tsv: &str) -> Vec<OcrWord> {
    let mut words = Vec::new();
    for (i, line) in tsv.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue; // başlık satırı
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 12 {
            continue;
        }
        // level page block par line word left top width height conf text
        if cols[0].trim() != "5" {
            continue;
        }
        let text = cols[11].trim();
        if text.is_empty() {
            continue;
        }
        let conf: f32 = cols[10].parse().unwrap_or(-1.0);
        if conf < 0.0 {
            continue;
        }
        words.push(OcrWord {
            text: text.to_string(),
            confidence: conf,
            left: cols[6].parse().unwrap_or(0),
            top: cols[7].parse().unwrap_or(0),
            width: cols[8].parse().unwrap_or(0),
            height: cols[9].parse().unwrap_or(0),
        });
    }
    words
}

pub fn which_on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                dir.join(format!("{name}.exe")).is_file() || dir.join(name).is_file()
            })
        })
        .unwrap_or(false)
}

/// Benzersiz geçici dosya adı üretmek için zaman tabanlı kimlik.
pub fn unique_stamp() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("mimo_ocr_{}_{}", std::process::id(), nanos)
}

// ---------------------------------------------------------------------------
// Tesseract CLI bağdaştırıcısı (rusty-tesseract yaklaşımı: alt süreç)
// ---------------------------------------------------------------------------

pub struct TesseractCli {
    exe_path: PathBuf,
    pub tessdata_dir: Option<PathBuf>,
}

impl TesseractCli {
    pub fn detect() -> Result<Self, OcrError> {
        if let Ok(p) = std::env::var("MIMO_TESSERACT") {
            let path = PathBuf::from(p);
            if path.is_file() {
                return Ok(Self::with_exe(path));
            }
        }
        for cand in [
            r"C:\Program Files\Tesseract-OCR\tesseract.exe",
            r"C:\Program Files (x86)\Tesseract-OCR\tesseract.exe",
        ] {
            let path = PathBuf::from(cand);
            if path.is_file() {
                return Ok(Self::with_exe(path));
            }
        }
        if which_on_path("tesseract") {
            return Ok(Self {
                exe_path: PathBuf::from("tesseract"),
                tessdata_dir: std::env::var("MIMO_TESSDATA").ok().map(PathBuf::from),
            });
        }
        Err(OcrError::EngineNotFound)
    }

    fn with_exe(exe_path: PathBuf) -> Self {
        Self {
            tessdata_dir: resolve_tessdata_dir(&exe_path),
            exe_path,
        }
    }

    fn run_tesseract(
        &self,
        image_png: &[u8],
        options: &OcrOptions,
    ) -> Result<(String, String), OcrError> {
        let stamp = unique_stamp();
        let dir = std::env::temp_dir();
        let in_path = dir.join(format!("{stamp}.png"));
        let out_base = dir.join(&stamp);
        let txt_path = dir.join(format!("{stamp}.txt"));
        let tsv_path = dir.join(format!("{stamp}.tsv"));

        std::fs::File::create(&in_path)?.write_all(image_png)?;

        let mut cmd = Command::new(&self.exe_path);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Tesseract her karede/OCR'de konsol penceresi açıp kapatmasın
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        cmd.arg(&in_path)
            .arg(&out_base)
            .arg("-l")
            .arg(&options.languages)
            .arg("--psm")
            .arg(&options.psm)
            .arg("txt")
            .arg("tsv");
        if let Some(td) = &self.tessdata_dir {
            cmd.env("TESSDATA_PREFIX", td);
        }

        let output = cmd.output()?;

        let cleanup = |paths: [&PathBuf; 3]| {
            for p in paths {
                let _ = std::fs::remove_file(p);
            }
        };

        if !output.status.success() {
            cleanup([&in_path, &txt_path, &tsv_path]);
            return Err(OcrError::EngineFailed {
                code: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let txt = std::fs::read_to_string(&txt_path).unwrap_or_default();
        let tsv = std::fs::read_to_string(&tsv_path).unwrap_or_default();
        cleanup([&in_path, &txt_path, &tsv_path]);
        Ok((txt, tsv))
    }
}

impl OcrEngine for TesseractCli {
    fn id(&self) -> &'static str {
        "tesseract-cli"
    }

    fn recognize(&self, image_png: &[u8], options: &OcrOptions) -> Result<OcrDocument, OcrError> {
        let started = Instant::now();
        let (txt, tsv) = self.run_tesseract(image_png, options)?;
        let words = parse_tsv_words(&tsv);
        Ok(OcrDocument {
            plain_text: txt.trim_end().to_string(),
            words,
            language: options.languages.clone(),
            engine: self.id().to_string(),
            elapsed_ms: started.elapsed().as_millis() as u64,
            image_png_base64: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                image_png,
            ),
        })
    }
}

/// Öncelik: `MIMO_TESSDATA` → depo kökündeki kullanıcı düzeyi tessdata
/// (tur+eng burada tutulur) → exe yanındaki sistem tessdata'sı.
/// Seçilen dizinin `configs/txt` içermesi gerekir; yoksa exe yanındaki
/// configs/tessconfigs oraya kopyalanmaya çalışılır (yazılabilirse).
pub fn resolve_tessdata_dir(exe_path: &std::path::Path) -> Option<PathBuf> {
    let side = exe_path.parent().map(|p| p.join("tessdata"));

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("MIMO_TESSDATA") {
        candidates.push(PathBuf::from(p));
    }
    if let Some(repo) = find_repo_tessdata() {
        candidates.push(repo);
    }
    if let Some(side) = side.clone() {
        candidates.push(side.clone());
    }

    let mut chosen: Option<PathBuf> = None;
    for c in candidates {
        if !c.is_dir() {
            continue;
        }
        // configs eksikse exe yanındakinden kopyalamayı dene
        if !c.join("configs").join("txt").exists() {
            if let Some(side) = &side {
                let src_cfg = side.join("configs");
                if src_cfg.is_dir() {
                    copy_dir_all(&src_cfg, &c.join("configs")).ok();
                }
                let src_tcfg = side.join("tessconfigs");
                if src_tcfg.is_dir() {
                    copy_dir_all(&src_tcfg, &c.join("tessconfigs")).ok();
                }
            }
        }
        // Türkçe destekli ilk tam dizin tercih edilir
        if c.join("configs").join("txt").exists() && c.join("tur.traineddata").exists() {
            return Some(c);
        }
        if chosen.is_none() && c.join("configs").join("txt").exists() {
            chosen = Some(c);
        }
    }
    chosen
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &to)?;
        } else if !to.exists() {
            std::fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

/// Geliştirme ağacında yukarı doğru `tessdata` klasörü arar
/// (kullanıcı düzeyinde indirilen dil dosyaları için).
fn find_repo_tessdata() -> Option<PathBuf> {
    let mut d = std::env::current_exe().ok()?.parent()?.to_path_buf();
    for _ in 0..8 {
        let t = d.join("tessdata");
        if t.is_dir() {
            return Some(t);
        }
        if !d.pop() {
            break;
        }
    }
    None
}


#[cfg(test)]
mod tests {
    use super::*;

    const TSV: &str = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
        1\t1\t0\t0\t0\t0\t0\t0\t900\t240\t-1\t\n\
        5\t1\t1\t1\t1\t1\t20\t20\t85\t36\t96.5\tMimo\n\
        5\t1\t1\t1\t1\t2\t110\t20\t64\t36\t91.2\tOCR\n\
        5\t1\t1\t1\t1\t3\t180\t20\t120\t36\t-1\t\n\
        4\t1\t1\t1\t1\t0\t20\t20\t120\t36\t-1\t\n\
        5\t1\t1\t1\t2\t1\t20\t100\t95\t32\t88.0\tFiyat:\n";

    #[test]
    fn tsv_yalnizca_kelime_duzyeyini_doner() {
        let words = parse_tsv_words(TSV);
        assert_eq!(words.len(), 3, "level 5 + boş olmayan + conf>=0 kayıtlar");
        assert_eq!(words[0].text, "Mimo");
        assert!((words[0].confidence - 96.5).abs() < f32::EPSILON);
        assert_eq!(
            (words[0].left, words[0].top, words[0].width, words[0].height),
            (20, 20, 85, 36)
        );
        assert_eq!(words[2].text, "Fiyat:");
    }

    #[test]
    fn bos_cikti_kelime_uretmez() {
        assert!(parse_tsv_words("").is_empty());
        assert!(parse_tsv_words("level\tconf\ttext\n").is_empty());
    }

    #[test]
    fn tesseract_bulunur() {
        let cli = TesseractCli::detect().expect("Tesseract kurulu olmalı");
        assert_eq!(cli.id(), "tesseract-cli");
    }

    #[test]
    fn tessdata_klasoru_configs_icerir() {
        let cli = TesseractCli::detect().unwrap();
        let dir = cli
            .tessdata_dir
            .as_ref()
            .expect("tessdata dizini çözümlenmeli");
        assert!(dir.join("tur.traineddata").exists(), "{dir:?} içinde tur yok");
        assert!(dir.join("eng.traineddata").exists(), "{dir:?} içinde eng yok");
        assert!(dir.join("configs").join("txt").exists(), "{dir:?} içinde configs/txt yok");
    }

    #[test]
    fn uctan_uca_turkce_ocr() {
        let png = std::fs::read("../test-tr.png").expect("test-tr.png depo kökünde olmalı");
        let cli = TesseractCli::detect().unwrap();
        let doc = cli
            .recognize(&png, &OcrOptions::default())
            .expect("OCR başarısız");
        assert!(doc.plain_text.contains("Türkçe"), "metin: {}", doc.plain_text);
        assert!(doc.plain_text.contains("İstanbul"), "metin: {}", doc.plain_text);
        assert!(doc.plain_text.contains("1.234,56 TL"), "metin: {}", doc.plain_text);
        assert!(doc.plain_text.contains("Error 0x80070005"), "metin: {}", doc.plain_text);
        assert!(doc.words.len() >= 12, "{} kelime", doc.words.len());
        assert!(!doc.image_png_base64.is_empty());
    }
}
