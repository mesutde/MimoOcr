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
            // Duz-metin paketi (betik read_plain ile okur).
            | "sql"
            | "srt"
            | "vtt"
            | "ini"
            | "cfg"
            | "yaml"
            | "yml"
            | "toml"
            | "ps1"
            | "bat"
            | "cmd"
            | "sh"
            // LibreOffice + EPUB (betikte zip+XML cozum).
            | "odt"
            | "ods"
            | "odp"
            | "epub"
    )
}

/// Uzanti destekleniyor mu (gorsel veya belge)?
pub fn is_supported_ext(ext: &str) -> bool {
    is_image_ext(ext) || is_doc_ext(ext)
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
    /// Ilk birlesik parca (geriye uyumluluk: UDF/Web sekmeleri bunu okur).
    pub combined_path: Option<String>,
    /// Tum birlesik parcalar (bolme kapaliyken tek oge).
    pub combined_paths: Vec<String>,
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
pub fn list_monitors(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<MonitorDto>, String> {
    let lang = crate::commands::lang_of(&state);
    let mons = app
        .available_monitors()
        .map_err(|e| format!("{}: {e}", crate::i18n::msg(&lang, "monitors_fail")))?;
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

/// Klasor Ekle: verilen klasorlerdeki desteklenen dosyalari listeler.
/// `recursive` aciksa alt klasorler de taranir. Sonuc sirali + tekrarsizdir.
#[tauri::command]
pub fn expand_batch_dirs(dirs: Vec<String>, recursive: bool) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for dir in &dirs {
        let root = PathBuf::from(dir);
        if !root.is_dir() {
            continue;
        }
        // Yiginla derinlik-oncelikli yuruyus (ozyinelemeli fonksiyon yerine).
        let mut stack = vec![root];
        while let Some(d) = stack.pop() {
            let entries = match std::fs::read_dir(&d) {
                Ok(e) => e,
                Err(_) => continue,
            };
            let mut subdirs: Vec<PathBuf> = Vec::new();
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if recursive {
                        // Gizli/sistem klasorlerini atla (baslangic noktasi degilse).
                        let hidden = p
                            .file_name()
                            .map(|n| n.to_string_lossy().starts_with('.'))
                            .unwrap_or(false);
                        if !hidden {
                            subdirs.push(p);
                        }
                    }
                } else if p.is_file() {
                    let ext = p
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_ascii_lowercase())
                        .unwrap_or_default();
                    if is_supported_ext(&ext) {
                        out.push(p.display().to_string());
                    }
                }
            }
            // Sirali gezinti icin ters sirada yigina koy.
            subdirs.sort();
            for s in subdirs.into_iter().rev() {
                stack.push(s);
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Parca sayisina gore araliklar: n ogeyi `parts` gruba sirali dagitir.
/// Donus: her grubun (baslangic, bitis) indeksleri [start, end).
pub fn chunk_indices_count(n: usize, parts: usize) -> Vec<(usize, usize)> {
    if n == 0 || parts == 0 {
        return Vec::new();
    }
    let parts = parts.clamp(1, n.max(1));
    let base = n / parts;
    let extra = n % parts;
    let mut ranges = Vec::with_capacity(parts);
    let mut start = 0;
    for i in 0..parts {
        let len = base + usize::from(i < extra);
        ranges.push((start, start + len));
        start += len;
    }
    ranges
}

/// Bayt esigine gore araliklar: biriken boyut `limit`i asmadan gruplar.
/// Her grup en az 1 oge alir; tek oge limiti asarsa tek basina parca olur.
pub fn chunk_indices_size(sizes: &[u64], limit: u64) -> Vec<(usize, usize)> {
    let limit = limit.max(1);
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut start = 0;
    let mut acc: u64 = 0;
    for (i, &s) in sizes.iter().enumerate() {
        if acc > 0 && acc.saturating_add(s) > limit {
            ranges.push((start, i));
            start = i;
            acc = 0;
        }
        acc = acc.saturating_add(s);
    }
    if start < sizes.len() || ranges.is_empty() {
        ranges.push((start, sizes.len()));
    }
    ranges
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
        let text = extract_doc_text(&path, &crate::commands::lang_of(&state))?;
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
        "{}: .{ext}",
        crate::i18n::msg(&crate::commands::lang_of(&state), "doc_bad_ext")
    )))
}

/// Metni `scripts/text_to_pdf.py` ile PDF'e çevirir (reportlab, Türkçe font).
fn write_pdf_file(
    dest: &PathBuf,
    title: &str,
    text: &str,
    no_title: bool,
    lang: &str,
) -> Result<(), String> {
    let script = repo_script("text_to_pdf.py").ok_or_else(|| crate::i18n::msg(lang, "script_missing"))?;
    let py = python_bin();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = std::env::temp_dir().join(format!("mimo-pdf-{stamp}.txt"));
    std::fs::write(&tmp, text)
        .map_err(|e| format!("{}: {e}", crate::i18n::msg(lang, "write_fail")))?;
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
        .map_err(|e| format!("{} ({py}): {e}", crate::i18n::msg(lang, "spawn_fail")))?;
    let _ = std::fs::remove_file(&tmp);
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            crate::i18n::msg(lang, "pdf_fail")
        } else {
            err.chars().take(300).collect()
        });
    }
    Ok(())
}

/// Gecici PDF'leri `scripts/merge_pdfs.py` ile tek PDF'te birlestirir.
fn merge_pdf_files(parts: &[PathBuf], dest: &PathBuf, lang: &str) -> Result<(), String> {
    let script = repo_script("merge_pdfs.py").ok_or_else(|| crate::i18n::msg(lang, "script_missing"))?;
    let py = python_bin();
    let mut cmd = std::process::Command::new(&py);
    hide_console(&mut cmd);
    cmd.arg(&script).arg("--out").arg(dest);
    for p in parts {
        cmd.arg(p);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("{} ({py}): {e}", crate::i18n::msg(lang, "spawn_fail")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            crate::i18n::msg(lang, "merge_fail")
        } else {
            err.chars().take(300).collect()
        });
    }
    Ok(())
}

/// `.udf` dosyasini `scripts/udf_to_pdf.py` ile yapisal PDF'e cevirir
/// (paragraf hizalama + tablo korunur; duz-metin akisindan daha sadik).
fn write_udf_pdf(
    udf_path: &str,
    dest: &PathBuf,
    title: &str,
    no_title: bool,
    lang: &str,
) -> Result<(), String> {
    let script = repo_script("udf_to_pdf.py").ok_or_else(|| crate::i18n::msg(lang, "script_missing"))?;
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
        .map_err(|e| format!("{} ({py}): {e}", crate::i18n::msg(lang, "spawn_fail")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            crate::i18n::msg(lang, "udf_pdf_fail")
        } else {
            err.chars().take(300).collect()
        });
    }
    Ok(())
}

/// `scripts/batch_extract.py <dosya>` calistirir, stdout metni dondurur.
fn extract_doc_text(path: &str, lang: &str) -> Result<String, OcrError> {
    let script = repo_script("batch_extract.py")
        .ok_or_else(|| OcrError::Image(crate::i18n::msg(lang, "script_missing")))?;
    let py = python_bin();
    let mut cmd = std::process::Command::new(&py);
    hide_console(&mut cmd);
    cmd.arg(&script).arg(path);
    // Python betigi UTF-8 yazar; konsol kodu ne olursa olsun dogru okunur.
    let out = cmd
        .output()
        .map_err(|e| OcrError::Image(format!("{} ({py}): {e}", crate::i18n::msg(lang, "spawn_fail"))))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(OcrError::Image(if err.is_empty() {
            format!(
                "{} {})",
                crate::i18n::msg(lang, "doc_exit_code"),
                out.status.code().unwrap_or(-1)
            )
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
    split_mode: Option<String>,
    split_count: Option<usize>,
    split_mb: Option<f64>,
) -> Result<BatchResultDto, String> {
    if files.is_empty() {
        return Err(crate::i18n::msg(
            &crate::commands::lang_of(&state),
            "no_files",
        ));
    }
    let format = format.unwrap_or_else(|| "md".into()).to_ascii_lowercase();
    if !["txt", "md", "pdf"].contains(&format.as_str()) {
        return Err(crate::i18n::msg(
            &crate::commands::lang_of(&state),
            "bad_batch_format",
        ));
    }
    let save_mode = save_mode
        .unwrap_or_else(|| "separate".into())
        .to_ascii_lowercase();
    let engine_id = active_engine_id(&state, engine);
    // PDF basligi: dosya adi yazilsin mi? (varsayilan: evet)
    let no_title = !file_title.unwrap_or(true);
    // Bolme yalnizca birlesik kipte gecerli: "none" | "count" | "size".
    let split_mode = split_mode.unwrap_or_else(|| "none".into()).to_ascii_lowercase();
    let split_mode = if save_mode == "combined" {
        split_mode
    } else {
        "none".to_string()
    };
    let split_count = split_count.unwrap_or(4).clamp(2, 50);
    let split_mb = split_mb.unwrap_or(15.0).clamp(1.0, 4096.0);
    let lang = crate::commands::lang_of(&state);
    let app_emit = app.clone();

    let out_dir = PathBuf::from(&out_dir);
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("{}: {e}", crate::i18n::msg(&lang, "out_dir_fail")))?;
    let total = files.len();
    let mut items = Vec::with_capacity(total);
    // Birlesik kipte parcalar burada birikir.
    let mut combined: Vec<(String, String)> = Vec::new();
    // Birlesik PDF kipinde ara PDF'ler (ayri kipteki yapisal ceviriyle ayni kalite).
    let mut pdf_parts: Vec<PathBuf> = Vec::new();

    let _ = app_emit.emit(
        "ocr-status",
        serde_json::json!({"state": "working", "message": format!("batch: {total}")}),
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
                error: Some(crate::i18n::msg(&lang, "file_missing")),
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
            .map_err(|e| format!("load: {e}"))?;
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
            let lang2 = lang.clone();
            tauri::async_runtime::spawn_blocking(move || extract_doc_text(&file_cloned, &lang2))
                .await
                .map_err(|e| format!("doc: {e}"))?
                .map(|t| ("document".to_string(), t))
                .map_err(|e| e.to_string())
        } else {
            Err(format!(
                "{}: .{ext}",
                crate::i18n::msg(&lang, "ext_unsupported")
            ))
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
                            write_udf_pdf(file, &tmp, &name, no_title, &lang)
                        } else {
                            write_pdf_file(&tmp, &name, &text, no_title, &lang)
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
                            write_udf_pdf(file, &dest, &name, no_title, &lang)
                        } else {
                            write_pdf_file(&dest, &name, &text, no_title, &lang)
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
                            error: Some(format!("{}: {e}", crate::i18n::msg(&lang, "write_fail"))),
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

        let mut combined_paths: Vec<String> = Vec::new();
        if save_mode == "combined" {
            if format == "pdf" {
                // Ara PDF'ler ayri kip kalitesinde uretildi; parcalara bolunup birlestirilir.
                if pdf_parts.is_empty() {
                    return Err(crate::i18n::msg(&lang, "combined_fail"));
                }
                let ranges = if split_mode == "count" {
                    chunk_indices_count(pdf_parts.len(), split_count)
                } else if split_mode == "size" {
                    let sizes: Vec<u64> = pdf_parts
                        .iter()
                        .map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
                        .collect();
                    chunk_indices_size(&sizes, (split_mb * 1024.0 * 1024.0) as u64)
                } else {
                    vec![(0, pdf_parts.len())]
                };
                for (pi, (s, e)) in ranges.iter().enumerate() {
                    let chunk = &pdf_parts[*s..*e];
                    if chunk.is_empty() {
                        continue;
                    }
                    let dest = if ranges.len() > 1 {
                        out_dir.join(format!("mimo-batch-part{}.pdf", pi + 1))
                    } else {
                        out_dir.join("mimo-batch.pdf")
                    };
                    merge_pdf_files(chunk, &dest, &lang)?;
                    combined_paths.push(dest.display().to_string());
                }
                for tmp in &pdf_parts {
                    let _ = std::fs::remove_file(tmp);
                }
            } else {
                // Once her belgeyi metne dok (boyut olcumu icin), sonra parcalara ayir.
                let entries: Vec<String> = combined
                    .iter()
                    .map(|(name, text)| {
                        if format == "md" {
                            format!("## {name}\n\n```\n{text}\n```\n\n")
                        } else {
                            format!("===== {name} =====\n{text}\n\n")
                        }
                    })
                    .collect();
                let ranges = if split_mode == "count" {
                    chunk_indices_count(entries.len(), split_count)
                } else if split_mode == "size" {
                    let sizes: Vec<u64> =
                        entries.iter().map(|b| b.len() as u64).collect();
                    chunk_indices_size(&sizes, (split_mb * 1024.0 * 1024.0) as u64)
                } else {
                    vec![(0, entries.len())]
                };
                // Hic basarili belge yoksa eski davranis: tek bos dosya yaz.
                let ranges = if ranges.is_empty() {
                    vec![(0, 0)]
                } else {
                    ranges
                };
                for (pi, (s, e)) in ranges.iter().enumerate() {
                    let dest = if ranges.len() > 1 {
                        out_dir.join(format!("mimo-batch-part{}.{format}", pi + 1))
                    } else {
                        out_dir.join(format!("mimo-batch.{format}"))
                    };
                    let body: String = entries[*s..(*e).min(entries.len())].concat();
                    if let Err(e) = std::fs::write(&dest, body) {
                        return Err(format!("{}: {e}", crate::i18n::msg(&lang, "combined_fail")));
                    }
                    combined_paths.push(dest.display().to_string());
                }
            }
    }

    let ok_count = items.iter().filter(|x| x.ok).count();
    let dto = BatchResultDto {
        out_dir: out_dir.display().to_string(),
        ok_count,
        fail_count: items.len().saturating_sub(ok_count),
        combined_path: combined_paths.first().cloned(),
        combined_paths,
        items,
    };
    let _ = app_emit.emit(
        "ocr-status",
        serde_json::json!({"state": "ready", "ok": dto.ok_count, "total": dto.items.len()}),
    );
    Ok(dto)
}

#[cfg(test)]
mod tests {
    use super::{chunk_indices_count, chunk_indices_size, expand_batch_dirs};

    #[test]
    fn count_split_even() {
        assert_eq!(
            chunk_indices_count(10, 5),
            vec![(0, 2), (2, 4), (4, 6), (6, 8), (8, 10)]
        );
    }

    #[test]
    fn count_split_uneven_and_clamped() {
        assert_eq!(chunk_indices_count(5, 2), vec![(0, 3), (3, 5)]);
        // Parca sayisi oge sayisindan buyukse oge basina bir parca.
        assert_eq!(chunk_indices_count(3, 9), vec![(0, 1), (1, 2), (2, 3)]);
        assert!(chunk_indices_count(0, 4).is_empty());
    }

    #[test]
    fn size_split_threshold() {
        // 10+10+30 bayt, limit 25 → [0..2), [2..3)
        assert_eq!(chunk_indices_size(&[10, 10, 30], 25), vec![(0, 2), (2, 3)]);
        // Tek oge limiti asarsa yalniz parca olur.
        assert_eq!(chunk_indices_size(&[100], 25), vec![(0, 1)]);
        assert_eq!(chunk_indices_size(&[], 25), vec![(0, 0)]);
    }

    #[test]
    fn expand_dirs_filters_and_recurses() {
        let root = std::env::temp_dir().join(format!("mimo-batch-test-{}", std::process::id()));
        let sub = root.join("alt");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(root.join("a.txt"), "a").unwrap();
        std::fs::write(root.join("b.exe"), "x").unwrap();
        std::fs::write(sub.join("c.sql"), "select 1").unwrap();
        std::fs::write(sub.join("d.odt"), "zip?").unwrap();

        let flat =
            expand_batch_dirs(vec![root.display().to_string()], false).unwrap();
        assert_eq!(flat.len(), 1, "flat: {flat:?}");
        assert!(flat[0].ends_with("a.txt"));

        let rec = expand_batch_dirs(vec![root.display().to_string()], true).unwrap();
        assert_eq!(rec.len(), 3, "rec: {rec:?}");

        std::fs::remove_dir_all(&root).unwrap();
    }
}
