//! Toplu is (Batch) + Belge sekmeleri.
//!
//! - Gorseller secili motorla OCR'lanir (tesseract / windows-ocr / mock).
//! - Belgeler (PDF/DOCX/XLSX/PPTX/**UDF (UYAP)**/RTF/TXT/MD/CSV/JSON) icin
//!   `scripts/batch_extract.py` calistirilir (CREATE_NO_WINDOW, UTF-8).
//! - Ilerleme `batch-progress`, durum `ocr-status` olaylariyla one yuze akar.
//! - Cikti: ayri dosyalar veya birlesik TXT/MD.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::commands::{active_engine_id, recognize_with, AppState};
use crate::engine::OcrError;
use crate::video::{hide_console, python_bin, repo_script};

pub fn is_image_ext(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "bmp" | "tif" | "tiff" | "webp" | "gif"
    )
}

pub fn is_doc_ext(ext: &str) -> bool {
    matches!(
        ext,
        "pdf"
            | "docx"
            | "xlsx"
            | "pptx"
            | "udf"
            | "rtf"
            | "txt"
            | "md"
            | "csv"
            | "json"
            | "log"
            | "xml"
            | "html"
            | "htm"
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchItemResult {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub ok: bool,
    pub error: Option<String>,
    pub chars: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResultDto {
    pub items: Vec<BatchItemResult>,
    pub out_dir: String,
    pub ok_count: usize,
    pub fail_count: usize,
    pub combined_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocResultDto {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub plain_text: String,
    pub engine: String,
    pub elapsed_ms: u64,
    pub words: Vec<crate::engine::OcrWord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorDto {
    pub index: usize,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

/// Bagli monitorleri listeler (Yakala sekmesindeki monitor secici icin).
#[tauri::command]
pub fn list_monitors(app: AppHandle) -> Result<Vec<MonitorDto>, String> {
    let mons = app
        .available_monitors()
        .map_err(|e| format!("Monitörler alınamadı: {e}"))?;
    Ok(mons
        .iter()
        .enumerate()
        .map(|(i, m)| MonitorDto {
            index: i,
            x: m.position().x,
            y: m.position().y,
            width: m.size().width,
            height: m.size().height,
            scale: m.scale_factor(),
        })
        .collect())
}

/// Belge sekmesi: tek dosya (gorsel → motor, belge → metin cikarimi + UDF).
#[tauri::command]
pub async fn ocr_path(
    state: State<'_, AppState>,
    path: String,
    engine: Option<String>,
) -> Result<DocResultDto, OcrError> {
    let engine_id = active_engine_id(&state, engine);
    let started = std::time::Instant::now();
    let p = PathBuf::from(&path);
    let name = p
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    if is_image_ext(&ext) {
        let opts = state.options.lock().unwrap().clone();
        let path_cloned = path.clone();
        let png = tauri::async_runtime::spawn_blocking(move || {
            let raw = crate::capture::load_file(&path_cloned)?;
            crate::capture::preprocess(&raw, opts.scale)
        })
        .await
        .map_err(|e| OcrError::Image(e.to_string()))??;
        let doc = recognize_with(&state, &png, &opts, &engine_id).await?;
        return Ok(DocResultDto {
            path,
            name,
            kind: "image".into(),
            plain_text: doc.plain_text,
            engine: doc.engine,
            elapsed_ms: doc.elapsed_ms,
            words: doc.words,
        });
    }
    if is_doc_ext(&ext) {
        let text = extract_doc_text(&path)?;
        let ms = started.elapsed().as_millis() as u64;
        return Ok(DocResultDto {
            path,
            name,
            kind: "document".into(),
            plain_text: text,
            engine: "text-extract".into(),
            elapsed_ms: ms,
            words: vec![],
        });
    }
    Err(OcrError::Image(format!(
        "Desteklenmeyen dosya türü: .{ext} (görsel, PDF/DOCX/XLSX/PPTX/UDF/metin olmalı)"
    )))
}

/// Metni `scripts/text_to_pdf.py` ile PDF'e çevirir (reportlab, Türkçe font).
fn write_pdf_file(dest: &PathBuf, title: &str, text: &str, no_title: bool) -> Result<(), String> {
    let script = repo_script("text_to_pdf.py")
        .ok_or_else(|| "scripts/text_to_pdf.py bulunamadı.".to_string())?;
    let py = python_bin();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = std::env::temp_dir().join(format!("mimo-pdf-{stamp}.txt"));
    std::fs::write(&tmp, text).map_err(|e| format!("geçici yazılamadı: {e}"))?;
    let mut cmd = std::process::Command::new(&py);
    hide_console(&mut cmd);
    cmd.arg(&script)
        .arg("--in")
        .arg(&tmp)
        .arg("--out")
        .arg(dest)
        .arg("--title")
        .arg(title);
    if no_title {
        cmd.arg("--no-title");
    }
    let out = cmd
        .output()
        .map_err(|e| format!("Python çalıştırılamadı ({py}): {e}"))?;
    let _ = std::fs::remove_file(&tmp);
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "PDF üretilemedi (reportlab gerekli)".into()
        } else {
            err.chars().take(300).collect()
        });
    }
    Ok(())
}

/// Gecici PDF'leri `scripts/merge_pdfs.py` ile tek PDF'te birlestirir.
fn merge_pdf_files(parts: &[PathBuf], dest: &PathBuf) -> Result<(), String> {
    let script = repo_script("merge_pdfs.py")
        .ok_or_else(|| "scripts/merge_pdfs.py bulunamadı.".to_string())?;
    let py = python_bin();
    let mut cmd = std::process::Command::new(&py);
    hide_console(&mut cmd);
    cmd.arg(&script).arg("--out").arg(dest);
    for p in parts {
        cmd.arg(p);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("Python çalıştırılamadı ({py}): {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "PDF birleştirilemedi (pypdf gerekli)".into()
        } else {
            err.chars().take(300).collect()
        });
    }
    Ok(())
}

/// `.udf` dosyasini `scripts/udf_to_pdf.py` ile yapisal PDF'e cevirir
/// (paragraf hizalama + tablo korunur; duz-metin akisindan daha sadik).
fn write_udf_pdf(udf_path: &str, dest: &PathBuf, title: &str, no_title: bool) -> Result<(), String> {
    let script = repo_script("udf_to_pdf.py")
        .ok_or_else(|| "scripts/udf_to_pdf.py bulunamadı.".to_string())?;
    let py = python_bin();
    let mut cmd = std::process::Command::new(&py);
    hide_console(&mut cmd);
    cmd.arg(&script)
        .arg("--udf")
        .arg(udf_path)
        .arg("--out")
        .arg(dest)
        .arg("--title")
        .arg(title);
    if no_title {
        cmd.arg("--no-title");
    }
    let out = cmd
        .output()
        .map_err(|e| format!("Python çalıştırılamadı ({py}): {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "UDF→PDF üretilemedi (reportlab gerekli)".into()
        } else {
            err.chars().take(300).collect()
        });
    }
    Ok(())
}

/// `scripts/batch_extract.py <dosya>` calistirir, stdout metni dondurur.
fn extract_doc_text(path: &str) -> Result<String, OcrError> {
    let script =
        repo_script("batch_extract.py").ok_or_else(|| OcrError::Image("scripts/batch_extract.py bulunamadı".into()))?;
    let py = python_bin();
    let mut cmd = std::process::Command::new(&py);
    hide_console(&mut cmd);
    cmd.arg(&script).arg(path);
    // Python betigi UTF-8 yazar; konsol kodu ne olursa olsun dogru okunur.
    let out = cmd
        .output()
        .map_err(|e| OcrError::Image(format!("Python çalıştırılamadı ({py}): {e}")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(OcrError::Image(if err.is_empty() {
            format!("Belge çıkarılamadı (kod {})", out.status.code().unwrap_or(-1))
        } else {
            err.chars().take(400).collect()
        }));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Toplu sekmesi: coklu gorsel + belge → TXT/MD (ayri veya birlesik).
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn batch_process_files(
    app: AppHandle,
    state: State<'_, AppState>,
    files: Vec<String>,
    out_dir: String,
    format: Option<String>,
    save_mode: Option<String>,
    engine: Option<String>,
    file_title: Option<bool>,
) -> Result<BatchResultDto, String> {
    if files.is_empty() {
        return Err("Dosya seçilmedi.".into());
    }
    let format = format.unwrap_or_else(|| "md".into()).to_ascii_lowercase();
    if !["txt", "md", "pdf"].contains(&format.as_str()) {
        return Err("Format txt, md veya pdf olmalı.".into());
    }
    let save_mode = save_mode
        .unwrap_or_else(|| "separate".into())
        .to_ascii_lowercase();
    let engine_id = active_engine_id(&state, engine);
    // PDF basligi: dosya adi yazilsin mi? (varsayilan: evet)
    let no_title = !file_title.unwrap_or(true);
    let app_emit = app.clone();

    let out_dir = PathBuf::from(&out_dir);
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("Çıktı klasörü açılamadı: {e}"))?;
    let total = files.len();
    let mut items = Vec::with_capacity(total);
    // Birlesik kipte parcalar burada birikir.
    let mut combined: Vec<(String, String)> = Vec::new();
    // Birlesik PDF kipinde ara PDF'ler (ayri kipteki yapisal ceviriyle ayni kalite).
    let mut pdf_parts: Vec<PathBuf> = Vec::new();

    let _ = app_emit.emit(
        "ocr-status",
        serde_json::json!({"state": "working", "message": format!("toplu: {total} dosya…")}),
    );

    for (i, file) in files.iter().enumerate() {
        let p = PathBuf::from(file);
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| file.clone());
        let _ = app_emit.emit(
            "batch-progress",
            serde_json::json!({"index": i + 1, "total": total, "name": name, "path": file}),
        );
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        if !p.is_file() {
            items.push(BatchItemResult {
                path: file.clone(),
                name,
                kind: "missing".into(),
                ok: false,
                error: Some("Dosya bulunamadı.".into()),
                chars: 0,
            });
            continue;
        }

        let res: Result<(String, String), String> = if is_image_ext(&ext) {
            // Gorsel → secili motor.
            let opts = state.options.lock().unwrap().clone();
            let file_cloned = file.clone();
            let raw = tauri::async_runtime::spawn_blocking(move || {
                let raw = crate::capture::load_file(&file_cloned)
                    .map_err(|e| e.to_string())
                    .and_then(|raw| {
                        crate::capture::preprocess(&raw, opts.scale).map_err(|e| e.to_string())
                    });
                raw
            })
            .await
            .map_err(|e| format!("yükleme: {e}"))?;
            match raw {
                Err(e) => Err(e),
                Ok(png) => {
                    let opts2 = state.options.lock().unwrap().clone();
                    recognize_with(&state, &png, &opts2, &engine_id)
                        .await
                        .map(|doc| ("image".to_string(), doc.plain_text))
                        .map_err(|e| e.to_string())
                }
            }
        } else if is_doc_ext(&ext) {
            let file_cloned = file.clone();
            tauri::async_runtime::spawn_blocking(move || extract_doc_text(&file_cloned))
                .await
                .map_err(|e| format!("belge: {e}"))?
                .map(|t| ("document".to_string(), t))
                .map_err(|e| e.to_string())
        } else {
            Err(format!("Desteklenmeyen tür: .{ext}"))
        };

            match res {
                Ok((kind, text)) => {
                    let chars = text.chars().count();
                    if save_mode == "combined" && format == "pdf" {
                        // Her belge ayri kipteki yapisal ceviriyle PDF olur, sonra birlesir.
                        let tmp = std::env::temp_dir().join(format!(
                            "mimo-part-{}-{i}.pdf",
                            std::process::id()
                        ));
                        let conv = if ext == "udf" {
                            write_udf_pdf(file, &tmp, &name, no_title)
                        } else {
                            write_pdf_file(&tmp, &name, &text, no_title)
                        };
                        match conv {
                            Ok(()) => {
                                pdf_parts.push(tmp);
                                items.push(BatchItemResult {
                                    path: file.clone(),
                                    name,
                                    kind,
                                    ok: true,
                                    error: None,
                                    chars,
                                });
                            }
                            Err(e) => {
                                let _ = std::fs::remove_file(&tmp);
                                items.push(BatchItemResult {
                                    path: file.clone(),
                                    name,
                                    kind,
                                    ok: false,
                                    error: Some(e),
                                    chars,
                                });
                            }
                        }
                        continue;
                    }
                    if save_mode == "combined" {
                        combined.push((name.clone(), text));
                    } else if format == "pdf" {
                        let stem = p
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "cikti".to_string());
                        let dest = out_dir.join(format!("{stem}.pdf"));
                        // .udf → yapisal cevirici (hiza/tablo korunur); digerleri duz-metin PDF.
                        let conv = if ext == "udf" {
                            write_udf_pdf(file, &dest, &name, no_title)
                        } else {
                            write_pdf_file(&dest, &name, &text, no_title)
                        };
                        if let Err(e) = conv {
                            items.push(BatchItemResult {
                                path: file.clone(),
                                name,
                                kind,
                                ok: false,
                                error: Some(e),
                                chars,
                            });
                            continue;
                        }
                    } else {
                    let stem = p
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "cikti".to_string());
                    let dest = out_dir.join(format!("{stem}.{format}"));
                    let body = if format == "md" {
                        format!("# {name}\n\n```\n{text}\n```\n")
                    } else {
                        text.clone()
                    };
                    if let Err(e) = std::fs::write(&dest, body) {
                        items.push(BatchItemResult {
                            path: file.clone(),
                            name,
                            kind,
                            ok: false,
                            error: Some(format!("Yazılamadı: {e}")),
                            chars,
                        });
                        continue;
                    }
                }
                items.push(BatchItemResult {
                    path: file.clone(),
                    name,
                    kind,
                    ok: true,
                    error: None,
                    chars,
                });
            }
            Err(e) => {
                items.push(BatchItemResult {
                    path: file.clone(),
                    name,
                    kind: "error".into(),
                    ok: false,
                    error: Some(e.chars().take(300).collect()),
                    chars: 0,
                });
            }
        }
    }

        let mut combined_path = None;
        if save_mode == "combined" {
            if format == "pdf" {
                // Ara PDF'ler ayri kip kalitesinde uretildi; tek dosyada birlestir.
                let dest = out_dir.join("mimo-batch.pdf");
                if pdf_parts.is_empty() {
                    return Err("Birleştirilecek PDF üretilemedi.".into());
                }
                merge_pdf_files(&pdf_parts, &dest)?;
                for tmp in &pdf_parts {
                    let _ = std::fs::remove_file(tmp);
                }
                combined_path = Some(dest.display().to_string());
            } else {
                let dest = out_dir.join(format!("mimo-batch.{format}"));
                let mut body = String::new();
                for (name, text) in &combined {
                    if format == "md" {
                        body.push_str(&format!("## {name}\n\n```\n{text}\n```\n\n"));
                    } else {
                        body.push_str(&format!("===== {name} =====\n{text}\n\n"));
                    }
                }
                if let Err(e) = std::fs::write(&dest, body) {
                    return Err(format!("Birleşik dosya yazılamadı: {e}"));
                }
                combined_path = Some(dest.display().to_string());
            }
    }

    let ok_count = items.iter().filter(|x| x.ok).count();
    let dto = BatchResultDto {
        out_dir: out_dir.display().to_string(),
        ok_count,
        fail_count: items.len().saturating_sub(ok_count),
        combined_path,
        items,
    };
    let _ = app_emit.emit(
        "ocr-status",
        serde_json::json!({"state": "ready", "message": format!("{}/{} tamam", dto.ok_count, dto.items.len())}),
    );
    Ok(dto)
}
