//! Ön yüzden çağrılan Tauri komutları.

use std::sync::{Arc, Mutex};

use serde::Deserialize;
use tauri::{AppHandle, Manager, State};

use crate::capture;
use crate::engine::{OcrDocument, OcrEngine, OcrError, OcrOptions, TesseractCli};

pub struct AppState {
    /// Tesseract bulunamazsa `None` olur; uygulama yine de acilir,
    /// OCR istekleri anlasilir bir hata dondurur.
    pub engine: Mutex<Option<Arc<TesseractCli>>>,
    pub options: Mutex<OcrOptions>,
    /// Son başarılı bölge seçimi (overlay yerel mantıksal koordinatları)
    pub last_region: Mutex<Option<[f64; 4]>>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub ok: bool,
    pub path: Option<String>,
    pub tessdata: Option<String>,
    pub error: Option<String>,
}

fn engine_status_of(engine: &Option<Arc<TesseractCli>>) -> EngineStatus {
    match engine {
        Some(e) => EngineStatus {
            ok: true,
            path: Some(e.exe_path().display().to_string()),
            tessdata: e.tessdata_dir.as_ref().map(|p| p.display().to_string()),
            error: None,
        },
        None => EngineStatus {
            ok: false,
            path: None,
            tessdata: None,
            error: Some(
                "Tesseract bulunamadı. Kurulumla gelen tesseract-runtime eksikse \
                 Tesseract 5 kurun ya da exe yolunu secin."
                    .into(),
            ),
        },
    }
}

/// Arayuzdeki uyari bandini besler.
#[tauri::command]
pub fn engine_status(state: State<'_, AppState>) -> EngineStatus {
    engine_status_of(&state.engine.lock().unwrap())
}

/// Motoru yeniden tara (Tesseract sonradan kurulmussa yeniden baslatma gerekmez).
#[tauri::command]
pub fn rescan_engine(state: State<'_, AppState>) -> EngineStatus {
    let found = TesseractCli::detect().ok().map(Arc::new);
    *state.engine.lock().unwrap() = found;
    engine_status_of(&state.engine.lock().unwrap())
}

/// Kullanicinin sectigi tesseract.exe yolunu dogrulayip kalici kaydeder.
#[tauri::command]
pub fn set_engine_path(state: State<'_, AppState>, path: String) -> Result<EngineStatus, OcrError> {
    let p = std::path::PathBuf::from(&path);
    if !p.is_file() {
        return Err(OcrError::Image("Seçilen dosya bulunamadı.".into()));
    }
    // Calistigini dogrula (--version)
    let mut cmd = std::process::Command::new(&p);
    cmd.arg("--version");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let ok = cmd.output().map(|o| o.status.success()).unwrap_or(false);
    if !ok {
        return Err(OcrError::Image("Bu dosya Tesseract olarak çalıştırılamadı.".into()));
    }
    if let Some(f) = TesseractCli::saved_path_file() {
        std::fs::write(f, path.as_bytes()).map_err(OcrError::Spawn)?;
    }
    Ok(rescan_engine(state))
}

fn engine_or_err(state: &State<'_, AppState>) -> Result<Arc<TesseractCli>, OcrError> {
    state
        .engine
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| {
            OcrError::Image(
                "Tesseract bulunamadı. Tesseract 5 kurun ya da ayarladığınız yolu kontrol edin.".into(),
            )
        })
}

fn tessdata_dir(state: &State<'_, AppState>) -> Result<std::path::PathBuf, OcrError> {
    engine_or_err(state)?
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
    let engine = engine_or_err(state)?;
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

