//! Toplu is (Batch) + Belge sekmeleri.
//!
//! - Gorseller secili motorla OCR'lanir (tesseract / windows-ocr / mock).
//! - Belgeler (PDF/DOCX/XLSX/**XLS**/PPTX/**UDF (UYAP)**/RTF/TXT/MD/CSV/JSON) icin
//!   `scripts/batch_extract.py` calistirilir (CREATE_NO_WINDOW, UTF-8).
//!   Eski Excel `.xls` (BIFF) dosyalari Rust tarafinda calamine ile okunur.
//! - Ilerleme `batch-progress`, durum `ocr-status` olaylariyla one yuze akar.
//! - Cikti: ayri dosyalar veya birlesik TXT/MD/PDF/XLSX/CSV; Excel girdileri
//!   icin istege bagli yapisal JSON.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

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
            | "xls"
            | "pptx"
            | "udf"
            | "rtf"
            | "txt"
            | "md"
            | "markdown"
            | "csv"
            | "tsv"
            | "json"
            | "jsonl"
            | "ndjson"
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

/// Eski Excel (.xls / BIFF) dosyasini calamine ile okur.
/// Sayfa basliklari `## ad` satiri olarak, hucreler TAB ile ayrilarak yazilir.
fn extract_xls_text(path: &str) -> Result<String, String> {
    use calamine::{open_workbook_auto, Data, Reader};

    let mut wb = open_workbook_auto(path).map_err(|e| format!("xls: {e}"))?;
    let mut out = String::new();
    for (name, range) in wb.worksheets() {
        out.push_str(&format!("## {name}\n"));
        for row in range.rows() {
            let cells: Vec<String> = row
                .iter()
                .map(|c| {
                    if matches!(c, Data::Empty) {
                        String::new()
                    } else {
                        c.to_string()
                    }
                })
                .collect();
            if cells.iter().any(|c| !c.trim().is_empty()) {
                out.push_str(&cells.join("\t"));
                out.push('\n');
            }
        }
    }
    if out.trim().is_empty() {
        return Err("xls: bos calisma kitabi".into());
    }
    Ok(out)
}

/// Belge uzantisina gore metin cikarir. `.xls` icin calamine, `.xlsx` icin
/// hizli akis tabanli okuyucu (`crate::xlsx`), digerleri icin Python betigi.
fn extract_doc_text(path: &str, lang: &str) -> Result<String, OcrError> {
    let ext = PathBuf::from(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    if ext == "xls" {
        return extract_xls_text(path).map_err(OcrError::Image);
    }
    if ext == "xlsx" {
        // openpyxl bazi dosyalarda (dimension = tum sayfa) saatlerce
        // dolasabiliyor; hizli akis tabanli okuyucu kullanilir. Okuyucu
        // desteklemeyen bir yapi olursa Python yedegine dusulur.
        match crate::xlsx::extract_xlsx_text(path) {
            Ok(text) => return Ok(text),
            Err(err) => {
                if let Ok(text) = run_extract_script(path, lang) {
                    return Ok(text);
                }
                return Err(OcrError::Image(err));
            }
        }
    }
    run_extract_script(path, lang)
}

/// `scripts/batch_extract.py <dosya>` calistirir, stdout metnini dondurur.
fn run_extract_script(path: &str, lang: &str) -> Result<String, OcrError> {
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

/// Metin cikarimini sayfa/satir yapisina cevirir.
///
/// `.xls`/`.xlsx` icin `## Sayfa` basliklari sayfa ayiricidir; diger
/// belgelerde tek sayfa kabul edilir ve satirlar TAB ile hucrelere bolunur.
fn text_rows(text: &str, ext: &str, fallback_name: &str) -> Vec<(String, Vec<Vec<String>>)> {
    let mut sheets: Vec<(String, Vec<Vec<String>>)> = Vec::new();
    if matches!(ext, "xls" | "xlsx") {
        let mut current: Option<(String, Vec<Vec<String>>)> = None;
        for line in text.lines() {
            if let Some(name) = line.strip_prefix("## ") {
                if let Some(s) = current.take() {
                    sheets.push(s);
                }
                current = Some((name.trim().to_string(), Vec::new()));
            } else if !line.trim().is_empty() {
                if current.is_none() {
                    current = Some((fallback_name.to_string(), Vec::new()));
                }
                if let Some((_, rows)) = current.as_mut() {
                    rows.push(line.split('\t').map(|s| s.to_string()).collect());
                }
            }
        }
        if let Some(s) = current.take() {
            sheets.push(s);
        }
    } else {
        let rows: Vec<Vec<String>> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.split('\t').map(|s| s.to_string()).collect())
            .collect();
        sheets.push((fallback_name.to_string(), rows));
    }
    if sheets.is_empty() {
        sheets.push((fallback_name.to_string(), Vec::new()));
    }
    sheets
}

fn sanitize_sheet_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, '[' | ']' | ':' | '*' | '?' | '/' | '\\'))
        .collect();
    let trimmed = cleaned.trim().trim_matches('\'').trim();
    let base = if trimmed.is_empty() { "Sheet" } else { trimmed };
    base.chars().take(31).collect()
}

/// Sayfa adlarini Excel sinirlarina gore benzersizlestirir.
fn unique_sheet_name(base: &str, used: &mut HashSet<String>) -> String {
    let clean = sanitize_sheet_name(base);
    let mut candidate = clean.clone();
    let mut n = 2;
    while used.contains(&candidate.to_ascii_lowercase()) {
        let suffix = format!("~{n}");
        let keep = 31usize.saturating_sub(suffix.chars().count());
        candidate = format!("{}{}", clean.chars().take(keep).collect::<String>(), suffix);
        n += 1;
    }
    used.insert(candidate.to_ascii_lowercase());
    candidate
}

fn write_xlsx_sheets(sheets: &[(String, Vec<Vec<String>>)], dest: &Path) -> Result<(), String> {
    use rust_xlsxwriter::Workbook;

    let mut wb = Workbook::new();
    let mut used: HashSet<String> = HashSet::new();
    for (name, rows) in sheets {
        let sheet_name = unique_sheet_name(name, &mut used);
        let ws = wb
            .add_worksheet()
            .set_name(&sheet_name)
            .map_err(|e| format!("sheet: {e}"))?;
        for (r, row) in rows.iter().enumerate() {
            if r as u32 >= 1_048_576 {
                break;
            }
            for (c, cell) in row.iter().enumerate() {
                if c as u16 >= 16_383 {
                    break;
                }
                if cell.is_empty() {
                    continue;
                }
                // Excel hucre siniri 32.767 karakter.
                if cell.chars().count() > 32_767 {
                    let truncated: String = cell.chars().take(32_767).collect();
                    ws.write_string(r as u32, c as u16, &truncated)
                        .map_err(|e| format!("cell ({r},{c}): {e}"))?;
                } else {
                    ws.write_string(r as u32, c as u16, cell)
                        .map_err(|e| format!("cell ({r},{c}): {e}"))?;
                }
            }
        }
    }
    wb.save(dest).map_err(|e| format!("xlsx: {e}"))
}

fn write_csv_rows(rows: &[Vec<String>], dest: &Path) -> Result<(), String> {
    let mut wtr = csv::WriterBuilder::new()
        .flexible(true)
        .from_path(dest)
        .map_err(|e| format!("csv: {e}"))?;
    for row in rows {
        wtr.write_record(row).map_err(|e| format!("csv: {e}"))?;
    }
    wtr.flush().map_err(|e| format!("csv: {e}"))?;
    Ok(())
}

fn write_csv_text(text: &str, ext: &str, stem: &str, dest: &Path) -> Result<(), String> {
    let sheets = text_rows(text, ext, stem);
    let rows: Vec<Vec<String>> = sheets.into_iter().flat_map(|(_, rows)| rows).collect();
    write_csv_rows(&rows, dest)
}

fn write_xlsx_text(text: &str, ext: &str, stem: &str, dest: &Path) -> Result<(), String> {
    let sheets = text_rows(text, ext, stem);
    write_xlsx_sheets(&sheets, dest)
}

/// Excel girdisini LLM'in kolay okudugu yapisal JSON'a cevirir.
fn write_excel_json(
    text: &str,
    ext: &str,
    source: &str,
    stem: &str,
    dest: &Path,
) -> Result<(), String> {
    let sheets = text_rows(text, ext, stem);
    let sheet_values: Vec<serde_json::Value> = sheets
        .iter()
        .map(|(name, rows)| serde_json::json!({ "name": name, "rows": rows }))
        .collect();
    let value = serde_json::json!({ "source": source, "sheets": sheet_values });
    let body = serde_json::to_string_pretty(&value).map_err(|e| format!("json: {e}"))?;
    std::fs::write(dest, body).map_err(|e| format!("json: {e}"))
}

fn split_ranges(
    sizes: &[u64],
    split_mode: &str,
    split_count: usize,
    split_mb: f64,
) -> Vec<(usize, usize)> {
    if split_mode == "count" {
        chunk_indices_count(sizes.len(), split_count)
    } else if split_mode == "size" {
        chunk_indices_size(sizes, (split_mb * 1024.0 * 1024.0) as u64)
    } else {
        vec![(0, sizes.len())]
    }
}

/// Birlesik CSV icin her satirin basina kaynak dosya adini ekler.
fn combined_csv_rows(entries: &[(String, String, String)]) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    for (name, text, ext) in entries {
        let stem = Path::new(name)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| name.clone());
        for (_, rows) in text_rows(text, ext, &stem) {
            for row in rows {
                let mut with_file = Vec::with_capacity(row.len() + 1);
                with_file.push(name.clone());
                with_file.extend(row);
                out.push(with_file);
            }
        }
    }
    out
}

/// Birlesik XLSX icin her kaynak dosyayi ayri sayfa olarak hazirlar.
fn combined_xlsx_sheets(entries: &[(String, String, String)]) -> Vec<(String, Vec<Vec<String>>)> {
    let mut out: Vec<(String, Vec<Vec<String>>)> = Vec::new();
    for (name, text, ext) in entries {
        let stem = Path::new(name)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| name.clone());
        for (sheet, rows) in text_rows(text, ext, &stem) {
            let label = if sheet.eq_ignore_ascii_case(&stem) {
                stem.clone()
            } else {
                format!("{stem}-{sheet}")
            };
            out.push((label, rows));
        }
    }
    out
}

/// Bu boyuttan buyuk dosyalar icin "islem uzun surebilir" bilgisi gonderilir.
const LARGE_FILE_BYTES: u64 = 20 * 1024 * 1024;

fn emit_batch_log(app: &AppHandle, payload: serde_json::Value) {
    let _ = app.emit("batch-log", payload);
}

/// Dosya bitti: ilerleme cubugunu tamamla + log satiri yayinla.
fn finish_batch_file(
    app: &AppHandle,
    index: usize,
    total: usize,
    name: &str,
    ok: bool,
    chars: usize,
    error: Option<&str>,
    ms: u128,
) {
    let _ = app.emit(
        "batch-progress",
        serde_json::json!({"index": index, "total": total, "name": name, "done": true}),
    );
    let mut payload = serde_json::json!({
        "stage": if ok { "done" } else { "error" },
        "level": if ok { "ok" } else { "err" },
        "index": index, "total": total, "name": name,
        "chars": chars, "ms": ms,
    });
    if let Some(e) = error {
        payload["error"] = serde_json::Value::String(e.to_string());
    }
    emit_batch_log(app, payload);
}

/// Toplu sekmesi: coklu gorsel + belge → TXT/MD/PDF/XLSX/CSV (ayri veya birlesik).
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn batch_process_files(
    app: AppHandle,
    state: State<'_, AppState>,
    files: Vec<String>,
    out_dir: String,
    format: Option<String>,
    formats: Option<Vec<String>>,
    save_mode: Option<String>,
    engine: Option<String>,
    file_title: Option<bool>,
    split_mode: Option<String>,
    split_count: Option<usize>,
    split_mb: Option<f64>,
    group_by_ext: Option<bool>,
    excel_json: Option<bool>,
) -> Result<BatchResultDto, String> {
    if files.is_empty() {
        return Err(crate::i18n::msg(
            &crate::commands::lang_of(&state),
            "no_files",
        ));
    }
    // Coklu format: yeni `formats` alani; `format` eski tek-format cagrilar
    // icin geriye uyumlu yedek olarak korunur.
    let mut formats: Vec<String> = formats
        .unwrap_or_default()
        .into_iter()
        .map(|f| f.trim().to_ascii_lowercase())
        .filter(|f| ["txt", "md", "pdf", "xlsx", "csv"].contains(&f.as_str()))
        .collect();
    if formats.is_empty() {
        if let Some(single) = format {
            let single = single.trim().to_ascii_lowercase();
            if ["txt", "md", "pdf", "xlsx", "csv"].contains(&single.as_str()) {
                formats.push(single);
            }
        }
    }
    formats.sort();
    formats.dedup();
    if formats.is_empty() {
        return Err(crate::i18n::msg(
            &crate::commands::lang_of(&state),
            "bad_batch_format",
        ));
    }
    let has_pdf = formats.iter().any(|f| f.as_str() == "pdf");
    let excel_json = excel_json.unwrap_or(false);
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
    // Kaydetme: kaynak dosyanin uzantisina gore alt klasor ac (pdf/, docx/, png/ ...).
    let group_by_ext = group_by_ext.unwrap_or(false);
    let lang = crate::commands::lang_of(&state);
    let app_emit = app.clone();

    let out_dir = PathBuf::from(&out_dir);
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("{}: {e}", crate::i18n::msg(&lang, "out_dir_fail")))?;
    let total = files.len();
    let mut items = Vec::with_capacity(total);
    // Birlesik kipte parcalar burada birikir: (dosya adi, metin, uzanti).
    let mut combined: Vec<(String, String, String)> = Vec::new();
    // Birlesik PDF kipinde ara PDF'ler (ayri kipteki yapisal ceviriyle ayni kalite).
    let mut pdf_parts: Vec<PathBuf> = Vec::new();

    let _ = app_emit.emit(
        "ocr-status",
        serde_json::json!({"state": "working", "message": format!("batch: {total}")}),
    );

    for (i, file) in files.iter().enumerate() {
        let file_started = std::time::Instant::now();
        let p = PathBuf::from(file);
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| file.clone());
        let size = p.metadata().map(|m| m.len()).unwrap_or(0);
        let _ = app_emit.emit(
            "batch-progress",
            serde_json::json!({
                "index": i + 1, "total": total, "name": name, "path": file, "done": false
            }),
        );
        emit_batch_log(
            &app_emit,
            serde_json::json!({
                "stage": "start", "level": "info", "index": i + 1, "total": total,
                "name": name, "path": file, "size": size,
            }),
        );
        if size >= LARGE_FILE_BYTES {
            emit_batch_log(
                &app_emit,
                serde_json::json!({
                    "stage": "large", "level": "info", "index": i + 1, "total": total,
                    "name": name, "size": size,
                }),
            );
        }
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        if !p.is_file() {
            let err = crate::i18n::msg(&lang, "file_missing");
            items.push(BatchItemResult {
                path: file.clone(),
                name: name.clone(),
                kind: "missing".into(),
                ok: false,
                error: Some(err.clone()),
                chars: 0,
            });
            finish_batch_file(
                &app_emit,
                i + 1,
                total,
                &name,
                false,
                0,
                Some(&err),
                file_started.elapsed().as_millis(),
            );
            continue;
        }

        let res: Result<(String, String), String> = if is_image_ext(&ext) {
            // Gorsel → secili motor.
            emit_batch_log(
                &app_emit,
                serde_json::json!({
                    "stage": "prepare", "level": "info", "index": i + 1, "total": total,
                    "name": name,
                }),
            );
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
                    emit_batch_log(
                        &app_emit,
                        serde_json::json!({
                            "stage": "ocr", "level": "info", "index": i + 1, "total": total,
                            "name": name,
                        }),
                    );
                    let opts2 = state.options.lock().unwrap().clone();
                    recognize_with(&state, &png, &opts2, &engine_id)
                        .await
                        .map(|doc| ("image".to_string(), doc.plain_text))
                        .map_err(|e| e.to_string())
                }
            }
        } else if is_doc_ext(&ext) {
            emit_batch_log(
                &app_emit,
                serde_json::json!({
                    "stage": "extract", "level": "info", "index": i + 1, "total": total,
                    "name": name,
                }),
            );
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
                emit_batch_log(
                    &app_emit,
                    serde_json::json!({
                        "stage": "save", "level": "info", "index": i + 1, "total": total,
                        "name": name,
                    }),
                );
                // Ayri kipte istenirse kaynak uzantisina gore alt klasore yaz.
                let dest_dir = if group_by_ext && !ext.is_empty() {
                    let d = out_dir.join(&ext);
                    match std::fs::create_dir_all(&d) {
                        Ok(()) => d,
                        Err(_) => out_dir.clone(),
                    }
                } else {
                    out_dir.clone()
                };
                let stem = p
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "cikti".to_string());
                let mut save_errors: Vec<String> = Vec::new();

                // Excel girdisi → ek JSON (birlesik kipte de dosya basina).
                if excel_json && matches!(ext.as_str(), "xls" | "xlsx") {
                    let dest = dest_dir.join(format!("{stem}.json"));
                    if let Err(e) = write_excel_json(&text, &ext, &name, &stem, &dest) {
                        save_errors.push(e);
                    }
                }

                if save_mode == "combined" {
                    combined.push((name.clone(), text.clone(), ext.clone()));
                    if has_pdf {
                        // Her belge ayri kipteki yapisal ceviriyle PDF olur, sonra birlesir.
                        let tmp = std::env::temp_dir()
                            .join(format!("mimo-part-{}-{i}.pdf", std::process::id()));
                        let conv = if ext == "udf" {
                            write_udf_pdf(file, &tmp, &name, no_title, &lang)
                        } else {
                            write_pdf_file(&tmp, &name, &text, no_title, &lang)
                        };
                        match conv {
                            Ok(()) => pdf_parts.push(tmp),
                            Err(e) => {
                                let _ = std::fs::remove_file(&tmp);
                                save_errors.push(format!("pdf: {e}"));
                            }
                        }
                    }
                } else {
                    for fmt in &formats {
                        let dest = dest_dir.join(format!("{stem}.{fmt}"));
                        let written = match fmt.as_str() {
                            "pdf" => {
                                // .udf → yapisal cevirici (hiza/tablo korunur);
                                // digerleri duz-metin PDF.
                                if ext == "udf" {
                                    write_udf_pdf(file, &dest, &name, no_title, &lang)
                                } else {
                                    write_pdf_file(&dest, &name, &text, no_title, &lang)
                                }
                            }
                            "md" => {
                                std::fs::write(&dest, format!("# {name}\n\n```\n{text}\n```\n"))
                                    .map_err(|e| e.to_string())
                            }
                            "txt" => std::fs::write(&dest, &text).map_err(|e| e.to_string()),
                            "csv" => write_csv_text(&text, &ext, &stem, &dest),
                            "xlsx" => write_xlsx_text(&text, &ext, &stem, &dest),
                            _ => Ok(()),
                        };
                        if let Err(e) = written {
                            save_errors.push(format!("{fmt}: {e}"));
                        }
                    }
                }

                let ok = save_errors.is_empty();
                let err = if ok {
                    None
                } else {
                    Some(save_errors.join("; ").chars().take(300).collect::<String>())
                };
                finish_batch_file(
                    &app_emit,
                    i + 1,
                    total,
                    &name,
                    ok,
                    chars,
                    err.as_deref(),
                    file_started.elapsed().as_millis(),
                );
                items.push(BatchItemResult {
                    path: file.clone(),
                    name,
                    kind,
                    ok,
                    error: err,
                    chars,
                });
            }
            Err(e) => {
                finish_batch_file(
                    &app_emit,
                    i + 1,
                    total,
                    &name,
                    false,
                    0,
                    Some(&e),
                    file_started.elapsed().as_millis(),
                );
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
        // Metin formatlari (md/txt): her format icin ayri birlesik dosya(lar).
        for fmt in formats
            .iter()
            .filter(|f| f.as_str() == "md" || f.as_str() == "txt")
        {
            let entries: Vec<String> = combined
                .iter()
                .map(|(name, text, _)| {
                    if fmt == "md" {
                        format!("## {name}\n\n```\n{text}\n```\n\n")
                    } else {
                        format!("===== {name} =====\n{text}\n\n")
                    }
                })
                .collect();
            let sizes: Vec<u64> = entries.iter().map(|b| b.len() as u64).collect();
            let mut ranges = split_ranges(&sizes, &split_mode, split_count, split_mb);
            // Hic basarili belge yoksa eski davranis: tek bos dosya yaz.
            if ranges.is_empty() {
                ranges = vec![(0, 0)];
            }
            for (pi, (s, e)) in ranges.iter().enumerate() {
                let dest = if ranges.len() > 1 {
                    out_dir.join(format!("mimo-batch-part{}.{fmt}", pi + 1))
                } else {
                    out_dir.join(format!("mimo-batch.{fmt}"))
                };
                let body: String = entries[*s..(*e).min(entries.len())].concat();
                if let Err(e) = std::fs::write(&dest, body) {
                    return Err(format!("{}: {e}", crate::i18n::msg(&lang, "combined_fail")));
                }
                combined_paths.push(dest.display().to_string());
            }
        }

        // CSV: her satirin basina kaynak dosya adini ekleyerek birlestir.
        if formats.iter().any(|f| f.as_str() == "csv") {
            let sizes: Vec<u64> = combined.iter().map(|(_, t, _)| t.len() as u64).collect();
            let mut ranges = split_ranges(&sizes, &split_mode, split_count, split_mb);
            if ranges.is_empty() {
                ranges = vec![(0, 0)];
            }
            for (pi, (s, e)) in ranges.iter().enumerate() {
                let chunk = &combined[*s..(*e).min(combined.len())];
                let rows = combined_csv_rows(chunk);
                let dest = if ranges.len() > 1 {
                    out_dir.join(format!("mimo-batch-part{}.csv", pi + 1))
                } else {
                    out_dir.join("mimo-batch.csv")
                };
                write_csv_rows(&rows, &dest)
                    .map_err(|e| format!("{}: {e}", crate::i18n::msg(&lang, "combined_fail")))?;
                combined_paths.push(dest.display().to_string());
            }
        }

        // XLSX: her kaynak dosya ayri sayfa.
        if formats.iter().any(|f| f.as_str() == "xlsx") {
            let sizes: Vec<u64> = combined.iter().map(|(_, t, _)| t.len() as u64).collect();
            let mut ranges = split_ranges(&sizes, &split_mode, split_count, split_mb);
            if ranges.is_empty() {
                ranges = vec![(0, 0)];
            }
            for (pi, (s, e)) in ranges.iter().enumerate() {
                let chunk = &combined[*s..(*e).min(combined.len())];
                let mut sheets = combined_xlsx_sheets(chunk);
                if sheets.is_empty() {
                    sheets.push(("mimo-batch".to_string(), Vec::new()));
                }
                let dest = if ranges.len() > 1 {
                    out_dir.join(format!("mimo-batch-part{}.xlsx", pi + 1))
                } else {
                    out_dir.join("mimo-batch.xlsx")
                };
                write_xlsx_sheets(&sheets, &dest)
                    .map_err(|e| format!("{}: {e}", crate::i18n::msg(&lang, "combined_fail")))?;
                combined_paths.push(dest.display().to_string());
            }
        }

        // PDF: ara PDF'ler ayri kip kalitesinde uretildi; parcalara bolunup birlestirilir.
        if has_pdf {
            if pdf_parts.is_empty() {
                return Err(crate::i18n::msg(&lang, "combined_fail"));
            }
            let sizes: Vec<u64> = pdf_parts
                .iter()
                .map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
                .collect();
            let mut ranges = split_ranges(&sizes, &split_mode, split_count, split_mb);
            if ranges.is_empty() {
                ranges = vec![(0, pdf_parts.len())];
            }
            for (pi, (s, e)) in ranges.iter().enumerate() {
                let chunk = &pdf_parts[*s..(*e).min(pdf_parts.len())];
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
    emit_batch_log(
        &app_emit,
        serde_json::json!({
            "stage": "summary", "level": "info",
            "ok": dto.ok_count, "fail": dto.fail_count, "total": dto.items.len(),
        }),
    );
    let _ = app_emit.emit(
        "ocr-status",
        serde_json::json!({"state": "ready", "ok": dto.ok_count, "total": dto.items.len()}),
    );
    Ok(dto)
}

#[cfg(test)]
mod tests {
    use super::{
        chunk_indices_count, chunk_indices_size, expand_batch_dirs, is_doc_ext, text_rows,
        write_csv_rows, write_excel_json, write_xlsx_sheets,
    };
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("mimo-batch-{tag}-{}-{n}", std::process::id()))
    }

    #[test]
    fn doc_ext_accepts_legacy_xls() {
        assert!(is_doc_ext("xls"));
        assert!(is_doc_ext("xlsx"));
        assert!(!is_doc_ext("png"));
    }

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

    #[test]
    fn expand_dirs_includes_md_and_tsv() {
        let root =
            std::env::temp_dir().join(format!("mimo-batch-test-ext-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for (name, body) in [
            ("a.md", "# baslik"),
            ("b.tsv", "a\tb\n1\t2"),
            ("c.csv", "a,b"),
            ("d.pdf", "%PDF"),
        ] {
            std::fs::write(root.join(name), body).unwrap();
        }

        let out = expand_batch_dirs(vec![root.display().to_string()], false).unwrap();
        let names: Vec<String> = out
            .iter()
            .map(|p| {
                std::path::Path::new(p)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert!(names.contains(&"a.md".to_string()), "md yok: {names:?}");
        assert!(names.contains(&"b.tsv".to_string()), "tsv yok: {names:?}");
        assert!(names.contains(&"c.csv".to_string()));
        assert!(names.contains(&"d.pdf".to_string()));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn text_rows_reads_excel_sheets() {
        let text = "## Sayfa1\nAd\tTutar\nAli\t100\n\n## Sayfa2\nX\tY";
        let sheets = text_rows(text, "xlsx", "kitap");
        assert_eq!(sheets.len(), 2);
        assert_eq!(sheets[0].0, "Sayfa1");
        assert_eq!(sheets[0].1[1], vec!["Ali", "100"]);
        assert_eq!(sheets[1].0, "Sayfa2");
        assert_eq!(sheets[1].1[0], vec!["X", "Y"]);
    }

    #[test]
    fn text_rows_treats_plain_docs_as_one_sheet() {
        let sheets = text_rows("satir 1\nsatir 2", "pdf", "belge");
        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].0, "belge");
        assert_eq!(sheets[0].1.len(), 2);
    }

    #[test]
    fn excel_json_has_sheets_and_rows() {
        let dir = temp_dir("json");
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("a.json");
        write_excel_json("## S\nA\tB\n1\t2", "xlsx", "a.xlsx", "a", &dest).unwrap();
        let body = std::fs::read_to_string(&dest).unwrap();
        let value: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(value["source"], "a.xlsx");
        assert_eq!(value["sheets"][0]["name"], "S");
        assert_eq!(value["sheets"][0]["rows"][1][0], "1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn csv_and_xlsx_writers_create_files() {
        let dir = temp_dir("writers");
        std::fs::create_dir_all(&dir).unwrap();
        let csv_path = dir.join("out.csv");
        write_csv_rows(
            &[vec!["a".into(), "b,c".into()], vec!["1".into(), "2".into()]],
            &csv_path,
        )
        .unwrap();
        let csv_text = std::fs::read_to_string(&csv_path).unwrap();
        assert!(csv_text.contains("\"b,c\""), "csv: {csv_text}");

        let xlsx_path = dir.join("out.xlsx");
        write_xlsx_sheets(
            &[(
                "Sayfa1".into(),
                vec![
                    vec!["Ad".into(), "Tutar".into()],
                    vec!["Ali".into(), "100".into()],
                ],
            )],
            &xlsx_path,
        )
        .unwrap();
        let bytes = std::fs::read(&xlsx_path).unwrap();
        assert!(bytes.starts_with(b"PK"), "xlsx zip imzasi yok");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
