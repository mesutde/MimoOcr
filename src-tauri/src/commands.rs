//! Ön yüzden çağrılan Tauri komutları.

use std::sync::{Arc, Mutex};

use serde::Deserialize;
use tauri::{AppHandle, Manager, State};

use crate::capture;
use crate::engine::{OcrDocument, OcrEngine, OcrError, OcrOptions, TesseractCli};

pub struct AppState {
    pub engine: Arc<TesseractCli>,
    pub options: Mutex<OcrOptions>,
    /// Son başarılı bölge seçimi (overlay yerel mantıksal koordinatları)
    pub last_region: Mutex<Option<[f64; 4]>>,
}

fn tessdata_dir(state: &State<'_, AppState>) -> Result<std::path::PathBuf, OcrError> {
    state
        .engine
        .tessdata_dir
        .clone()
        .ok_or_else(|| OcrError::Image("tessdata dizini çözümlenemedi".into()))
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum OcrSource {
    File { path: String },
    Clipboard,
}

fn overlay(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    app.get_webview_window("overlay")
}

#[tauri::command]
pub fn begin_capture(app: AppHandle, state: State<'_, AppState>) -> Result<Option<[f64; 4]>, OcrError> {
    if let Some(ov) = overlay(&app) {
        ov.show().map_err(|e| OcrError::Image(e.to_string()))?;
        ov.set_focus().map_err(|e| OcrError::Image(e.to_string()))?;
    }
    Ok(*state.last_region.lock().unwrap())
}

/// Son bölgeyi yeniden OCR'lar (overlay'deki Enter kısayolu için).
#[tauri::command]
pub async fn re_capture_last(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<OcrDocument, OcrError> {
    let region = state
        .last_region
        .lock()
        .unwrap()
        .ok_or_else(|| OcrError::Image("Henüz yakalanmış bölge yok".into()))?;
    complete_capture(app, state, region[0], region[1], region[2], region[3]).await
}

#[tauri::command]
pub fn cancel_capture(app: AppHandle) -> Result<(), OcrError> {
    if let Some(ov) = overlay(&app) {
        let _ = ov.hide();
    }
    Ok(())
}

/// Overlay seçimini alır, bölgeyi yakalar, OCR çalıştırır ve sonucu döner.
#[tauri::command]
pub async fn complete_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<OcrDocument, OcrError> {
    if let Some(ov) = overlay(&app) {
        let _ = ov.hide();
    }
    *state.last_region.lock().unwrap() = Some([x, y, width, height]);

    let opts = state.options.lock().unwrap().clone();
    let png = tauri::async_runtime::spawn_blocking(move || {
        let raw = capture::capture_region(x, y, width, height)?;
        capture::preprocess(&raw, opts.scale)
    })
    .await
    .map_err(|e| OcrError::Image(e.to_string()))??;

    run_ocr(&state, png).await
}

/// Dosya veya pano kaynağından OCR çalıştırır.
#[tauri::command]
pub async fn ocr_run(
    state: State<'_, AppState>,
    source: OcrSource,
) -> Result<OcrDocument, OcrError> {
    let opts = state.options.lock().unwrap().clone();
    let png = tauri::async_runtime::spawn_blocking(move || {
        let raw = match source {
            OcrSource::File { path } => capture::load_file(&path)?,
            OcrSource::Clipboard => capture::load_clipboard()?,
        };
        capture::preprocess(&raw, opts.scale)
    })
    .await
    .map_err(|e| OcrError::Image(e.to_string()))??;

    run_ocr(&state, png).await
}

async fn run_ocr(state: &State<'_, AppState>, png: Vec<u8>) -> Result<OcrDocument, OcrError> {
    let opts = state.options.lock().unwrap().clone();
    let engine = Arc::clone(&state.engine);
    let doc = tauri::async_runtime::spawn_blocking(move || engine.recognize(&png, &opts))
        .await
        .map_err(|e| OcrError::Image(e.to_string()))??;

    // Varsayılan davranış: sonucu otomatik panoya kopyala (ayardan kapatılabilir)
    let auto_copy = state.options.lock().unwrap().auto_copy;
    if auto_copy && !doc.plain_text.is_empty() {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(doc.plain_text.clone());
        }
    }
    Ok(doc)
}

#[tauri::command]
pub fn copy_text(text: String) -> Result<(), OcrError> {
    let mut cb = arboard::Clipboard::new().map_err(|e| OcrError::Image(e.to_string()))?;
    cb.set_text(text).map_err(|e| OcrError::Image(e.to_string()))
}

#[tauri::command]
pub fn save_text(path: String, text: String) -> Result<(), OcrError> {
    std::fs::write(path, text).map_err(OcrError::Spawn)
}

#[tauri::command]
pub fn set_options(state: State<'_, AppState>, options: OcrOptions) {
    *state.options.lock().unwrap() = options;
}

#[tauri::command]
pub fn get_options(state: State<'_, AppState>) -> OcrOptions {
    state.options.lock().unwrap().clone()
}
// ---------------------------------------------------------------------------
// Model yöneticisi
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_models(state: State<'_, AppState>) -> Result<Vec<crate::models::ModelStatus>, OcrError> {
    crate::models::list_models(&tessdata_dir(&state)?)
}

#[tauri::command]
pub async fn install_model(
    state: State<'_, AppState>,
    code: String,
) -> Result<crate::models::ModelStatus, OcrError> {
    let dir = tessdata_dir(&state)?;
    tauri::async_runtime::spawn_blocking(move || crate::models::install_model(&code, &dir))
        .await
        .map_err(|e| OcrError::Image(e.to_string()))?
}

#[tauri::command]
pub fn remove_model(state: State<'_, AppState>, code: String) -> Result<(), OcrError> {
    crate::models::remove_model(&code, &tessdata_dir(&state)?)
}

/// Tesseract'ın o an kullanabildiği dilleri döner (dil seçim listesini besler).
#[tauri::command]
pub fn available_languages(state: State<'_, AppState>) -> Result<Vec<String>, OcrError> {
    let dir = tessdata_dir(&state)?;
    let mut langs: Vec<String> = std::fs::read_dir(&dir)
        .map_err(OcrError::Spawn)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".traineddata")
                .map(|s| s.to_string())
                .filter(|s| s != "osd")
        })
        .collect();
    langs.sort();
    Ok(langs)
}

