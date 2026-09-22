//! Dil modeli yöneticisi: kurulu/yüklenebilir dilleri listeler, indirir
//! (SHA-256 doğrulamalı), siler. Manifest: depo kökünde `models.json`.

use std::io::Read;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::engine::OcrError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub code: String,
    pub name: String,
    pub tier: u8,
    pub sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub models: Vec<ModelEntry>,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    pub code: String,
    pub name: String,
    pub tier: u8,
    pub installed: bool,
    pub size_bytes: Option<u64>,
}

fn manifest_path() -> Option<PathBuf> {
    let mut d = std::env::current_exe().ok()?.parent()?.to_path_buf();
    for _ in 0..8 {
        let m = d.join("models.json");
        if m.is_file() {
            return Some(m);
        }
        if !d.pop() {
            break;
        }
    }
    None
}

pub fn load_manifest() -> Result<Manifest, OcrError> {
    let path = manifest_path()
        .ok_or_else(|| OcrError::Image("models.json bulunamadı".into()))?;
    let text = std::fs::read_to_string(&path)
        .map_err(|e| OcrError::Image(format!("models.json okunamadı: {e}")))?;
    serde_json::from_str(&text)
        .map_err(|e| OcrError::Image(format!("models.json geçersiz: {e}")))
}

/// Kurulu modelleri manifest ile birleştirir.
pub fn list_models(tessdata_dir: &std::path::Path) -> Result<Vec<ModelStatus>, OcrError> {
    let manifest = load_manifest()?;
    let mut out = Vec::new();
    for m in manifest.models {
        let file = tessdata_dir.join(format!("{}.traineddata", m.code));
        let (installed, size_bytes) = match std::fs::metadata(&file) {
            Ok(md) => (true, Some(md.len())),
            Err(_) => (false, None),
        };
        out.push(ModelStatus {
            code: m.code,
            name: m.name,
            tier: m.tier,
            installed,
            size_bytes,
        });
    }
    Ok(out)
}

/// Modeli indirir; manifestte SHA-256 varsa doğrular. Yarım indirmeler
/// `.part` uzantısıyla yazılır, doğrulama sonrası yeniden adlandırılır.
pub fn install_model(
    code: &str,
    tessdata_dir: &std::path::Path,
) -> Result<ModelStatus, OcrError> {
    let manifest = load_manifest()?;
    let entry = manifest
        .models
        .iter()
        .find(|m| m.code == code)
        .ok_or_else(|| OcrError::Image(format!("Bilinmeyen model: {code}")))?
        .clone();

    let url = manifest.base_url.replace("{code}", &entry.code);
    let final_path = tessdata_dir.join(format!("{}.traineddata", entry.code));
    let part_path = tessdata_dir.join(format!("{}.traineddata.part", entry.code));

    let mut resp = reqwest::blocking::get(&url)
        .map_err(|e| OcrError::Image(format!("İndirme başarısız: {e}")))?;
    if !resp.status().is_success() {
        return Err(OcrError::Image(format!(
            "İndirme HTTP {} döndü",
            resp.status()
        )));
    }

    let mut hasher = sha2::Sha256::new();
    let mut buf = Vec::with_capacity(8 * 1024 * 1024);
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let n = resp
            .read(&mut chunk)
            .map_err(|e| OcrError::Image(format!("İndirme okunamadı: {e}")))?;
        if n == 0 {
            break;
        }
        hasher.update(&chunk[..n]);
        buf.extend_from_slice(&chunk[..n]);
    }

    if let Some(expected) = &entry.sha256 {
        let got = format!("{:x}", hasher.finalize());
        if !got.eq_ignore_ascii_case(expected) {
            return Err(OcrError::Image(format!(
                "SHA-256 uyuşmazlığı: beklenen {expected}, gelen {got}"
            )));
        }
    }

    std::fs::write(&part_path, &buf)?;
    std::fs::rename(&part_path, &final_path)?;

    let size = std::fs::metadata(&final_path).ok().map(|m| m.len());
    Ok(ModelStatus {
        code: entry.code,
        name: entry.name,
        tier: entry.tier,
        installed: true,
        size_bytes: size,
    })
}

pub fn remove_model(code: &str, tessdata_dir: &std::path::Path) -> Result<(), OcrError> {
    if matches!(code, "tur" | "eng" | "osd") {
        return Err(OcrError::Image(
            "Temel modeller (tur/eng/osd) silinemez".into(),
        ));
    }
    let path = tessdata_dir.join(format!("{code}.traineddata"));
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
