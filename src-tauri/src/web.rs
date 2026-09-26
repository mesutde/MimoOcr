//! Web sekmesi genel URL akisi: once algila/kontrol, sonra ture gore isle.
//!
//! - Google Sheets/Docs → ozel uclar (`sheet.rs`).
//! - Direkt dosya baglantilari → bayt indirilir, uzantiya gore mevcut
//!   toplu hatta yonlendirilir (gorsel→OCR, belge→metin cikarimi).
//! - Editor sayfalari (ONLYOFFICE `doceditor`, Office Online…) dosya degildir;
//!   duz indirmeyle alinamaz, net mesaj doner (DocSpace API Faz 2 isidir).
//! Sinirlar: ~100 MB, 60 sn zaman asimi. Ag yalniz kullanici istegiyle.

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::batch::{is_doc_ext, is_image_ext};
use crate::commands::AppState;

const MAX_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDetectDto {
    pub kind: String,
    pub label: String,
    pub formats: Vec<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum WebKind {
    GoogleSheet,
    GoogleDoc,
    DirectFile,
    EditorPage,
    Unknown,
}

fn classify_url(url: &str) -> (WebKind, Option<String>) {
    let u = url.trim().to_lowercase();
    if u.contains("docs.google.com/spreadsheets/d/") {
        (WebKind::GoogleSheet, None)
    } else if u.contains("docs.google.com/document/d/") {
        (WebKind::GoogleDoc, None)
    } else if u.contains("doceditor")
        || u.contains("officeapps.live.com")
        || u.contains("view.officeapps.live.com")
    {
        (
            WebKind::EditorPage,
            Some("Bu bağlantı bir editör sayfası; doğrudan indirilemez.".into()),
        )
    } else {
        // Yolun sonundaki uzantiya bak (query temizlenir).
        let path = u.split(['?', '#']).next().unwrap_or("");
        let ext = path.rsplit('.').next().unwrap_or("");
        let ext = ext.trim_end_matches('/');
        if is_image_ext(ext) || is_doc_ext(ext) || ext == "pdf" || ext == "odt" || ext == "docx" {
            (WebKind::DirectFile, Some(ext.to_string()))
        } else {
            (WebKind::Unknown, None)
        }
    }
}

/// Asama 1: indirmeden algila/kontrol — tur rozeti + format listesi icin.
#[tauri::command]
pub fn detect_web_url(url: String) -> WebDetectDto {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return WebDetectDto {
            kind: "invalid".into(),
            label: "Geçersiz bağlantı".into(),
            formats: vec![],
            detail: Some("http(s) ile başlamalı.".into()),
        };
    }
    match classify_url(url) {
        (WebKind::GoogleSheet, _) => WebDetectDto {
            kind: "sheet".into(),
            label: "Google Tablosu".into(),
            formats: vec!["csv".into(), "xlsx".into(), "md".into()],
            detail: None,
        },
        (WebKind::GoogleDoc, _) => WebDetectDto {
            kind: "doc".into(),
            label: "Google Belgesi".into(),
            formats: vec!["docx".into(), "odt".into(), "txt".into(), "pdf".into(), "md".into()],
            detail: None,
        },
        (WebKind::DirectFile, ext) => WebDetectDto {
            kind: "file".into(),
            label: format!("Dosya{}", ext.map(|e| format!(" (.{e})")).unwrap_or_default()),
            formats: vec!["txt".into(), "md".into(), "pdf".into()],
            detail: None,
        },
        (WebKind::EditorPage, detail) => WebDetectDto {
            kind: "editor".into(),
            label: "Editör sayfası".into(),
            formats: vec![],
            detail,
        },
        (WebKind::Unknown, _) => WebDetectDto {
            kind: "unknown".into(),
            label: "Bilinmeyen tür".into(),
            formats: vec![],
            detail: Some("Dosya uzantısı okunamadı; doğrudan dosya bağlantısı kullanın.".into()),
        },
    }
}

/// HEAD ile erisilebilirlik + boyut ust sinir kontrolu (indirmeden).
fn head_check(url: &str) -> Result<Option<u64>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("İstemci: {e}"))?;
    let resp = client
        .head(url)
        .send()
        .map_err(|e| format!("Erişilemiyor: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err("Erişim engellendi (giriş/izin gerekli).".into());
    }
    if status.as_u16() == 404 {
        return Err("Belge bulunamadı (404).".into());
    }
    if !status.is_success() {
        return Err(format!("HTTP {status}."));
    }
    Ok(resp
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok()))
}

/// Asama 2: direkt dosya indir → toplu hatta tek dosya olarak islet.
#[tauri::command]
pub async fn import_direct_url(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    out_dir: String,
    format: Option<String>,
) -> Result<crate::batch::BatchResultDto, String> {
    let format = format.unwrap_or_else(|| "md".into()).to_ascii_lowercase();
    if !["txt", "md", "pdf"].contains(&format.as_str()) {
        return Err("Format txt, md veya pdf olmalı.".into());
    }
    // Uzantiyi URL yolundan cikar; yoksa icerik koklamaya birak (asagida).
    let path_part = url.split(['?', '#']).next().unwrap_or("").to_lowercase();
    let mut ext = path_part.rsplit('.').next().unwrap_or("").to_string();
    if ext.contains('/') {
        ext.clear();
    }
    if !is_image_ext(&ext) && !is_doc_ext(&ext) && ext != "pdf" && ext != "odt" && ext != "docx" {
        return Err("Dosya türü URL'den okunamadı; doğrudan dosya bağlantısı kullanın.".into());
    }

    let out_dir_p = std::path::PathBuf::from(&out_dir);
    let url_owned = url.clone();
    let (tmp_path, sniffed_ext) =
        tauri::async_runtime::spawn_blocking(move || -> Result<(PathBuf, String), String> {
            if let Ok(len) = head_check(&url_owned) {
                if len.map(|n| n > MAX_BYTES).unwrap_or(false) {
                    return Err("Dosya çok büyük (>100 MB).".into());
                }
            }
            let resp = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .map_err(|e| format!("İstemci: {e}"))?
                .get(&url_owned)
                .send()
                .map_err(|e| format!("İndirilemedi: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("HTTP {} — indirilemedi.", resp.status()));
            }
            let bytes = resp.bytes().map_err(|e| format!("Bayt okunamadı: {e}"))?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err("Dosya çok büyük (>100 MB).".into());
            }
            if bytes.is_empty() {
                return Err("Boş dosya indi.".into());
            }
            // Uzanti yoksa icerikten kokla (PDF sihri / ZIP konteyner).
            let mut ext = ext;
            if ext.is_empty() {
                if bytes.starts_with(b"%PDF") {
                    ext = "pdf".to_string();
                } else if bytes.starts_with(b"PK\x03\x04") {
                    ext = "zip".to_string();
                }
            }
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let tmp = std::env::temp_dir().join(format!("mimo-web-{stamp}.{ext}"));
            std::fs::write(&tmp, &bytes).map_err(|e| format!("Geçici yazılamadı: {e}"))?;
            Ok((tmp, ext))
        })
        .await
        .map_err(|e| format!("Görev yarıda kesildi: {e}"))??;

    if sniffed_ext == "zip" {
        let _ = std::fs::remove_file(&tmp_path);
        return Err("ZIP arşivleri desteklenmiyor; içindeki dosyayı indirin.".into());
    }
    let path_str = tmp_path.display().to_string();
    let dto = crate::batch::batch_process_files(
        app,
        state,
        vec![path_str],
        out_dir_p.display().to_string(),
        Some(format),
        Some("separate".into()),
        None,
        Some(true),
    )
    .await?;
    let _ = std::fs::remove_file(&tmp_path);
    Ok(dto)
}

use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tur_siniflandirma() {
        let (k, _) = classify_url("https://docs.google.com/spreadsheets/d/ABC/edit?gid=2");
        assert_eq!(k, WebKind::GoogleSheet);
        let (k, _) = classify_url("https://docs.google.com/document/d/XYZ/edit?usp=sharing");
        assert_eq!(k, WebKind::GoogleDoc);
        let (k, ext) = classify_url("https://ornek.com/a/belge.PDF?x=1");
        assert_eq!(k, WebKind::DirectFile);
        assert_eq!(ext.as_deref(), Some("pdf"));
        let (k, _) = classify_url("https://x.onlyoffice.com/doceditor?share=A&fileId=1");
        assert_eq!(k, WebKind::EditorPage);
        let (k, _) = classify_url("https://ornek.com/sayfa");
        assert_eq!(k, WebKind::Unknown);
    }
}
