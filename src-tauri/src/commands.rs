use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mimo_ocr_capture::{capture_monitor, capture_region, list_monitors as capture_list_monitors};
use mimo_ocr_core::{CancellationToken, OcrDocument, OcrImage, OcrOptions, QualityMode};
use mimo_ocr_engines::{discover_tesseract, resolve_tessdata, EngineRegistry};
use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::state::AppState;

#[derive(Serialize, Clone)]
pub struct MonitorDto {
    pub index: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub is_primary: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OcrResultDto {
    pub engine: String,
    pub language: Option<String>,
    pub elapsed_ms: u64,
    pub mean_confidence: Option<f32>,
    pub plain_text: String,
    pub width: u32,
    pub height: u32,
    pub image_png_base64: String,
    pub tessdata: Option<String>,
    pub offline: bool,
}

#[derive(Serialize, Clone)]
pub struct EngineStatusDto {
    pub engines: Vec<String>,
    pub tesseract_path: Option<String>,
    pub tessdata: Option<String>,
    pub tur_model: bool,
    pub eng_model: bool,
    /// OCR language codes available on this machine (tessdata + system engines).
    pub languages: Vec<String>,
}

#[derive(Serialize, Clone)]
struct StatusPayload {
    state: String,
    message: String,
}

fn emit_status(app: &tauri::AppHandle, state: &str, message: impl AsRef<str>) {
    let _ = app.emit(
        "ocr-status",
        StatusPayload {
            state: state.to_string(),
            message: message.as_ref().to_string(),
        },
    );
}

fn registry() -> EngineRegistry {
    // Shared builder: mock + tesseract-cli + windows-ocr (when available).
    EngineRegistry::phase0_default()
}

fn preferred_engine(reg: &EngineRegistry) -> &'static str {
    if reg.ids().contains(&"tesseract-cli") {
        "tesseract-cli"
    } else {
        "mock"
    }
}

fn pick_engine(reg: &EngineRegistry, requested: Option<&str>) -> &'static str {
    match requested.map(|s| s.trim()).unwrap_or("") {
        "" | "auto" => preferred_engine(reg),
        id => reg.get(id).map(|e| e.id()).unwrap_or_else(|_| preferred_engine(reg)),
    }
}

fn options_from_langs(langs: Option<String>, accurate: bool, width: u32, height: u32) -> OcrOptions {
    let langs = langs.unwrap_or_else(|| "tur,eng".to_string());
    let languages = langs
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    // Full-screen captures must not run in "accurate 2×" — that froze the UI.
    let mut quality = if accurate {
        QualityMode::Accurate
    } else {
        QualityMode::Balanced
    };
    let max_side = width.max(height);
    if max_side >= 1400 && quality == QualityMode::Accurate {
        quality = QualityMode::Balanced;
    }
    OcrOptions {
        languages,
        quality,
        model_dir: resolve_tessdata(None),
        ..Default::default()
    }
}

fn png_base64(img: &OcrImage) -> String {
    let mut buf = Vec::new();
    if let Some(rgb) = image::RgbImage::from_raw(img.width, img.height, img.rgb.clone()) {
        let _ = rgb.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png);
    }
    STANDARD.encode(buf)
}

fn dto_from_doc(doc: &OcrDocument, img: &OcrImage) -> OcrResultDto {
    OcrResultDto {
        engine: doc.engine.clone(),
        language: doc.language.clone(),
        elapsed_ms: doc.elapsed_ms,
        mean_confidence: doc.mean_confidence(),
        plain_text: doc.plain_text.clone(),
        width: img.width,
        height: img.height,
        image_png_base64: png_base64(img),
        tessdata: resolve_tessdata(None).map(|p| p.display().to_string()),
        offline: true,
    }
}

fn run_ocr(
    img: &OcrImage,
    langs: Option<String>,
    accurate: bool,
    engine: Option<String>,
    cancel: &CancellationToken,
) -> Result<(OcrResultDto, OcrDocument), String> {
    let options = options_from_langs(langs, accurate, img.width, img.height);
    let reg = registry();
    let engine_id = pick_engine(&reg, engine.as_deref());
    let doc = reg
        .recognize(engine_id, img, &options, cancel)
        .map_err(|e| e.to_string())?;
    let dto = dto_from_doc(&doc, img);
    Ok((dto, doc))
}

fn crop_rgb(img: &OcrImage, x: u32, y: u32, w: u32, h: u32) -> Result<OcrImage, String> {
    if w == 0 || h == 0 {
        return Err("bölge boyutu 0 olamaz".into());
    }
    if x + w > img.width || y + h > img.height {
        return Err(format!(
            "bölge görüntü dışında: ({x},{y},{w}x{h}) vs {}x{}",
            img.width, img.height
        ));
    }
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for row in y..y + h {
        let start = ((row * img.width + x) * 3) as usize;
        let end = start + (w * 3) as usize;
        rgb.extend_from_slice(&img.rgb[start..end]);
    }
    Ok(OcrImage::from_rgb(w, h, rgb, format!("crop_{x}_{y}")))
}

fn store_and_emit(app: &tauri::AppHandle, dto: &OcrResultDto, doc: OcrDocument) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut last) = state.last_document.lock() {
            *last = Some(doc);
        }
        if let Ok(mut langs) = state.langs.lock() {
            if let Some(l) = &dto.language {
                *langs = l.clone();
            }
        }
    }
    let _ = app.emit("ocr-result", dto);
}

fn busy_flag(app: &tauri::AppHandle) -> Option<Arc<AtomicBool>> {
    app.try_state::<AppState>().map(|s| s.busy.clone())
}

fn begin_busy(app: &tauri::AppHandle) -> bool {
    if let Some(flag) = busy_flag(app) {
        if flag.swap(true, Ordering::SeqCst) {
            return false; // already running
        }
    }
    true
}

fn end_busy(app: &tauri::AppHandle) {
    if let Some(flag) = busy_flag(app) {
        flag.store(false, Ordering::SeqCst);
    }
}

fn cancel_token(app: &tauri::AppHandle) -> CancellationToken {
    app.try_state::<AppState>()
        .map(|s| s.cancel.lock().map(|g| g.clone()).unwrap_or_default())
        .unwrap_or_default()
}

#[tauri::command]
pub fn list_monitors() -> Result<Vec<MonitorDto>, String> {
    capture_list_monitors()
        .map(|ms| {
            ms.into_iter()
                .map(|m| MonitorDto {
                    index: m.index,
                    name: m.name,
                    x: m.x,
                    y: m.y,
                    width: m.width,
                    height: m.height,
                    scale_factor: m.scale_factor,
                    is_primary: m.is_primary,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn engine_status() -> Result<EngineStatusDto, String> {
    let reg = registry();
    let tessdata = resolve_tessdata(None);
    let tur_model = tessdata
        .as_ref()
        .map(|p| p.join("tur.traineddata").exists())
        .unwrap_or(false);
    let eng_model = tessdata
        .as_ref()
        .map(|p| p.join("eng.traineddata").exists())
        .unwrap_or(false);

    // Collect language codes engines can actually serve.
    let mut langs: Vec<String> = Vec::new();
    if let Some(td) = &tessdata {
        if let Ok(rd) = std::fs::read_dir(td) {
            for e in rd.flatten() {
                if let Some(name) = e.file_name().to_str() {
                    if let Some(code) = name.strip_suffix(".traineddata") {
                        if code != "osd" {
                            langs.push(code.to_string());
                        }
                    }
                }
            }
        }
    }
    // From registered engines (e.g. windows-ocr tr/en from language packs)
    for id in reg.ids() {
        if let Ok(engine) = reg.get(id) {
            for l in engine.supported_languages() {
                let code = l.code.split('-').next().unwrap_or(&l.code).to_lowercase();
                if !code.is_empty() && code != "osd" && !langs.iter().any(|x| x == &code) {
                    langs.push(code);
                }
                if !l.code.is_empty() && !langs.iter().any(|x| x == &l.code) {
                    // keep full tag if unique and useful
                    if l.code.contains('-') {
                        langs.push(l.code.clone());
                    }
                }
            }
        }
    }
    langs.sort();
    langs.dedup();

    Ok(EngineStatusDto {
        engines: reg.ids().iter().map(|s| s.to_string()).collect(),
        tesseract_path: discover_tesseract()
            .ok()
            .map(|p| p.display().to_string()),
        tessdata: tessdata.map(|p| p.display().to_string()),
        tur_model,
        eng_model,
        languages: langs,
    })
}

#[tauri::command]
pub fn cancel_ocr(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(guard) = state.cancel.lock() {
            guard.cancel();
        }
        // fresh token for next run
        if let Ok(mut guard) = state.cancel.lock() {
            *guard = CancellationToken::new();
        }
    }
    end_busy(&app);
    emit_status(&app, "idle", "iptal edildi");
    Ok(())
}

#[tauri::command]
pub async fn ocr_full_screen(
    app: tauri::AppHandle,
    monitor: Option<usize>,
    langs: Option<String>,
    accurate: Option<bool>,
    engine: Option<String>,
) -> Result<OcrResultDto, String> {
    ocr_full_screen_inner_async(
        app,
        monitor.unwrap_or(0),
        langs,
        accurate.unwrap_or(false),
        engine,
    )
    .await
}

pub async fn ocr_full_screen_inner_async(
    app: tauri::AppHandle,
    monitor: usize,
    langs: Option<String>,
    accurate: bool,
    engine: Option<String>,
) -> Result<OcrResultDto, String> {
    if !begin_busy(&app) {
        return Err("OCR zaten çalışıyor — önce iptal edin veya bitmesini bekleyin".into());
    }
    emit_status(&app, "working", "tam ekran yakalanıyor…");
    let cancel = cancel_token(&app);

    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        ocr_full_screen_inner_blocking(&handle, monitor, langs, accurate, engine, cancel)
    })
    .await
    .map_err(|e| e.to_string())?;
    result
}

fn ocr_full_screen_inner_blocking(
    app: &tauri::AppHandle,
    monitor: usize,
    langs: Option<String>,
    accurate: bool,
    engine: Option<String>,
    cancel: CancellationToken,
) -> Result<OcrResultDto, String> {
    let result: Result<OcrResultDto, String> = (|| {
        let img = capture_monitor(monitor).map_err(|e| e.to_string())?;
        emit_status(app, "working", format!("OCR {}×{}…", img.width, img.height));
        let (dto, doc) = run_ocr(&img, langs, accurate, engine, &cancel)?;
        store_and_emit(app, &dto, doc);
        Ok(dto)
    })();
    end_busy(app);
    match &result {
        Ok(_) => emit_status(app, "done", "tamamlandı"),
        Err(e) => emit_status(app, "error", e.as_str()),
    }
    result
}

/// Sync helper for tray/shortcut workers (already on a blocking thread).
pub fn ocr_full_screen_inner(
    app: &tauri::AppHandle,
    monitor: usize,
    langs: Option<String>,
    accurate: bool,
) -> Result<OcrResultDto, String> {
    if !begin_busy(app) {
        return Err("OCR zaten çalışıyor".into());
    }
    emit_status(app, "working", "tam ekran yakalanıyor…");
    let cancel = cancel_token(app);
    let result: Result<OcrResultDto, String> = (|| {
        let img = capture_monitor(monitor).map_err(|e| e.to_string())?;
        emit_status(app, "working", format!("OCR {}×{}…", img.width, img.height));
        let (dto, doc) = run_ocr(&img, langs, accurate, None, &cancel)?;
        store_and_emit(app, &dto, doc);
        Ok(dto)
    })();

    end_busy(app);
    match &result {
        Ok(_) => emit_status(app, "done", "tamamlandı"),
        Err(e) => emit_status(app, "error", e.as_str()),
    }
    result
}

#[tauri::command]
pub async fn ocr_region(
    app: tauri::AppHandle,
    monitor: usize,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    langs: Option<String>,
    accurate: Option<bool>,
    engine: Option<String>,
) -> Result<OcrResultDto, String> {
    if !begin_busy(&app) {
        return Err("OCR zaten çalışıyor".into());
    }
    emit_status(&app, "working", format!("bölge OCR {width}×{height}…"));
    let cancel = cancel_token(&app);
    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<OcrResultDto, String> {
        let img = capture_region(monitor, x, y, width, height).map_err(|e| e.to_string())?;
        let (dto, doc) = run_ocr(&img, langs, accurate.unwrap_or(false), engine, &cancel)?;
        store_and_emit(&handle, &dto, doc);
        Ok(dto)
    })
    .await
    .map_err(|e| e.to_string())?;

    end_busy(&app);
    match &result {
        Ok(_) => emit_status(&app, "done", "tamamlandı"),
        Err(e) => emit_status(&app, "error", e.as_str()),
    }
    result
}

/// OCR one or more rectangles cut from a snapshot (preview multi-select).
/// Does not re-capture the live screen — crops the provided image.
#[derive(Serialize, Clone, serde::Deserialize)]
pub struct CropRectDto {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub async fn ocr_snapshot_regions(
    app: tauri::AppHandle,
    image_png_base64: String,
    regions: Vec<CropRectDto>,
    langs: Option<String>,
    accurate: Option<bool>,
    engine: Option<String>,
    snap_width: Option<u32>,
    snap_height: Option<u32>,
) -> Result<OcrResultDto, String> {
    if regions.is_empty() {
        return Err("en az bir bölge seçin".into());
    }
    if !begin_busy(&app) {
        return Err("OCR zaten çalışıyor".into());
    }
    let _ = (snap_width, snap_height);
    emit_status(
        &app,
        "working",
        format!("{} bölge OCR…", regions.len()),
    );
    let cancel = cancel_token(&app);
    let handle = app.clone();
    let result =
        tauri::async_runtime::spawn_blocking(move || -> Result<OcrResultDto, String> {
            use base64::Engine as _;
            let raw = STANDARD
                .decode(image_png_base64.as_bytes())
                .map_err(|e| format!("base64: {e}"))?;
            let dyn_img = image::load_from_memory(&raw)
                .map_err(|e| format!("görüntü: {e}"))?
                .to_rgb8();
            let snap = OcrImage::from_image_buffer(&dyn_img, "snapshot");
            // Clamp regions to the actual snapshot — wrong coords must not
            // OCR a different part of the image.
            let regions: Vec<CropRectDto> = regions
                .into_iter()
                .map(|r| CropRectDto {
                    x: r.x.min(snap.width.saturating_sub(1)),
                    y: r.y.min(snap.height.saturating_sub(1)),
                    width: r.width.min(snap.width.saturating_sub(r.x.min(snap.width - 1))).max(1),
                    height: r
                        .height
                        .min(snap.height.saturating_sub(r.y.min(snap.height - 1)))
                        .max(1),
                })
                .collect();

            let mut texts: Vec<String> = Vec::new();
            let mut elapsed = 0u64;
            let mut engine_name = String::from("auto");
            let mut lang_used: Option<String> = None;
            let mut conf_sum = 0f32;
            let mut conf_n = 0usize;
            let mut preview_img: Option<OcrImage> = None;
            let mut last_doc: Option<OcrDocument> = None;

            for (i, r) in regions.iter().enumerate() {
                cancel.check().map_err(|e| e.to_string())?;
                let crop = crop_rgb(&snap, r.x, r.y, r.width, r.height)?;
                if preview_img.is_none() {
                    preview_img = Some(crop.clone());
                }
                emit_status(
                    &handle,
                    "working",
                    format!("bölge {}/{} OCR…", i + 1, regions.len()),
                );
                let (dto, doc) = run_ocr(
                    &crop,
                    langs.clone(),
                    accurate.unwrap_or(false),
                    engine.clone(),
                    &cancel,
                )?;
                if !dto.plain_text.trim().is_empty() {
                    texts.push(dto.plain_text.trim().to_string());
                }
                elapsed += dto.elapsed_ms;
                engine_name = dto.engine.clone();
                lang_used = dto.language.clone();
                if let Some(c) = dto.mean_confidence {
                    conf_sum += c;
                    conf_n += 1;
                }
                last_doc = Some(doc);
            }

            let preview = preview_img.unwrap_or(snap.clone());
            let combined = texts.join("\n");
            let mean_conf = if conf_n > 0 {
                Some(conf_sum / conf_n as f32)
            } else {
                None
            };
            let dto = OcrResultDto {
                engine: engine_name,
                language: lang_used,
                elapsed_ms: elapsed,
                mean_confidence: mean_conf,
                plain_text: combined,
                width: preview.width,
                height: preview.height,
                image_png_base64: png_base64(&preview),
                tessdata: resolve_tessdata(None).map(|p| p.display().to_string()),
                offline: true,
            };
            if let Some(doc) = last_doc {
                store_and_emit(&handle, &dto, doc);
            } else {
                let _ = handle.emit("ocr-result", &dto);
            }
            Ok(dto)
        })
        .await
        .map_err(|e| e.to_string())?;

    end_busy(&app);
    match &result {
        Ok(_) => emit_status(&app, "done", "tamamlandı"),
        Err(e) => emit_status(&app, "error", e.as_str()),
    }
    result
}

#[tauri::command]
pub async fn ocr_path(
    app: tauri::AppHandle,
    path: String,
    langs: Option<String>,
    accurate: Option<bool>,
    engine: Option<String>,
) -> Result<OcrResultDto, String> {
    if !begin_busy(&app) {
        return Err("OCR zaten çalışıyor".into());
    }
    emit_status(&app, "working", "belge okunuyor…");
    let cancel = cancel_token(&app);
    let path2 = path.clone();
    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<OcrResultDto, String> {
        let img = OcrImage::from_path(&PathBuf::from(&path2)).map_err(|e| e.to_string())?;
        emit_status(&handle, "working", format!("OCR {}…", img.source_name));
        let (dto, doc) = run_ocr(&img, langs, accurate.unwrap_or(false), engine, &cancel)?;
        if let Ok(mut last) = handle.state::<AppState>().last_image_path.lock() {
            *last = Some(path2);
        }
        store_and_emit(&handle, &dto, doc);
        Ok(dto)
    })
    .await
    .map_err(|e| e.to_string())?;

    end_busy(&app);
    match &result {
        Ok(_) => emit_status(&app, "done", "tamamlandı"),
        Err(e) => emit_status(&app, "error", e.as_str()),
    }
    result
}

#[tauri::command]
pub fn copy_text(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    crate::state::allow_exit_now();
    end_busy(&app);
    // Close capture if open, then exit.
    if let Some(win) = app.get_webview_window("capture") {
        let _ = win.close();
    }
    app.exit(0);
}

/// Always bring the main window back (after overlay / hidden pick flow).
#[tauri::command]
pub fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    let _ = main.show();
    let _ = main.unminimize();
    let _ = main.set_focus();
    Ok(())
}

/// Overlay finished: show main first, then destroy overlay so the app
/// never looks like it vanished.
#[tauri::command]
pub fn finish_region_pick(app: tauri::AppHandle) -> Result<(), String> {
    let _ = show_main_window(app.clone());
    if let Some(win) = app.get_webview_window("capture") {
        let _ = win.close();
    }
    emit_status(&app, "idle", "bölge seçimi hazır");
    Ok(())
}

/// Cancel live pick without OCR.
#[tauri::command]
pub fn cancel_region_pick(app: tauri::AppHandle) -> Result<(), String> {
    let _ = show_main_window(app.clone());
    if let Some(win) = app.get_webview_window("capture") {
        let _ = win.close();
    }
    emit_status(&app, "idle", "bölge seçimi iptal edildi");
    Ok(())
}

/// Capture the monitor and return a snapshot for in-window region picking.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RegionPickDto {
    pub monitor: usize,
    pub width: u32,
    pub height: u32,
    pub image_png_base64: String,
}

/// Safe in-window snapshot picker (no OS overlay).
#[tauri::command]
pub async fn start_region_pick(
    app: tauri::AppHandle,
    monitor: Option<usize>,
) -> Result<RegionPickDto, String> {
    let monitor = monitor.unwrap_or(0);
    emit_status(&app, "working", "ekran kopyalanıyor…");
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<RegionPickDto, String> {
        let img = capture_monitor(monitor).map_err(|e| e.to_string())?;
        Ok(RegionPickDto {
            monitor,
            width: img.width,
            height: img.height,
            image_png_base64: png_base64(&img),
        })
    })
    .await
    .map_err(|e| e.to_string())?;

    match &result {
        Ok(_) => emit_status(&app, "idle", "görsel üzerinde sürükleyerek alan seçin"),
        Err(e) => emit_status(&app, "error", e.as_str()),
    }
    result
}

/// LIVE desktop region pick: hide main window, show a borderless transparent
/// overlay on the monitor. Does NOT call WebView fullscreen() (that froze the PC).
#[tauri::command]
pub async fn open_live_region_pick(
    app: tauri::AppHandle,
    monitor: Option<usize>,
    langs: Option<String>,
    engine: Option<String>,
    accurate: Option<bool>,
) -> Result<(), String> {
    let monitor = monitor.unwrap_or(0);

    if let Some(state) = app.try_state::<AppState>() {
        state.store_pick_opts(
            langs.as_deref().unwrap_or("tur,eng"),
            engine.as_deref().unwrap_or("tesseract-cli"),
            accurate.unwrap_or(false),
        );
    }

    // Never stack overlays.
    if let Some(win) = app.get_webview_window("capture") {
        let _ = win.close();
    }

    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }

    let mon = capture_list_monitors()
        .ok()
        .and_then(|ms| ms.into_iter().find(|m| m.index == monitor));

    let url = tauri::WebviewUrl::App(format!("capture.html?mode=live&monitor={monitor}").into());

    let mut builder = tauri::WebviewWindowBuilder::new(&app, "capture", url)
        .title("Mimo OCR — Bölge seç")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .resizable(false)
        .skip_taskbar(true)
        .focused(true)
        .shadow(false)
        .visible(true);

    if let Some(m) = &mon {
        builder = builder
            .position(m.x as f64, m.y as f64)
            .inner_size(m.width as f64, m.height as f64);
    } else {
        builder = builder.inner_size(800.0, 600.0);
    }

    // Intentionally no .fullscreen(true) — that path froze the whole PC.
    let win = builder.build().map_err(|e| e.to_string())?;

    if let Some(m) = &mon {
        let _ = win.set_position(tauri::PhysicalPosition::new(m.x, m.y));
        let _ = win.set_size(tauri::PhysicalSize::new(m.width, m.height));
    }

    emit_status(
        &app,
        "working",
        "masaüstünde sürükle-bırak ile alan seçin (Esc = iptal)",
    );
    Ok(())
}

/// Called from the live overlay when the user finishes a drag selection.
/// Scales overlay client coords → monitor pixels, OCRs that crop, shows main.
#[tauri::command]
pub async fn complete_live_region_pick(
    app: tauri::AppHandle,
    monitor: usize,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    client_w: u32,
    client_h: u32,
) -> Result<OcrResultDto, String> {
    if width < 4 || height < 4 {
        let _ = cancel_region_pick(app.clone());
        return Err("seçim çok küçük".into());
    }

    let (langs, engine, accurate) = app
        .try_state::<AppState>()
        .map(|s| s.load_pick_opts())
        .unwrap_or((None, None, false));

    if !begin_busy(&app) {
        return Err("OCR zaten çalışıyor".into());
    }
    emit_status(&app, "working", "seçilen bölge OCR…");

    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<OcrResultDto, String> {
        // Map overlay client pixels → monitor physical pixels.
        let mons = capture_list_monitors().map_err(|e| e.to_string())?;
        let mon = mons
            .into_iter()
            .find(|m| m.index == monitor)
            .ok_or_else(|| format!("monitör {monitor} yok"))?;

        let (sx, sy) = if client_w > 0 && client_h > 0 {
            (
                mon.width as f32 / client_w as f32,
                mon.height as f32 / client_h as f32,
            )
        } else {
            (1.0, 1.0)
        };
        let rx = ((x as f32) * sx).round().max(0.0) as u32;
        let ry = ((y as f32) * sy).round().max(0.0) as u32;
        let rw = ((width as f32) * sx).round().max(4.0) as u32;
        let rh = ((height as f32) * sy).round().max(4.0) as u32;
        let rx = rx.min(mon.width.saturating_sub(1));
        let ry = ry.min(mon.height.saturating_sub(1));
        let rw = rw.min(mon.width.saturating_sub(rx)).max(4);
        let rh = rh.min(mon.height.saturating_sub(ry)).max(4);

        emit_status(
            &handle,
            "working",
            format!("bölge {rw}×{rh} @({rx},{ry}) OCR…"),
        );

        // Capture monitor once, crop the selected rectangle — more reliable
        // than a second live capture after the overlay closes.
        let full = capture_monitor(monitor).map_err(|e| e.to_string())?;
        let crop = crop_rgb(&full, rx, ry, rw, rh)?;
        let cancel = cancel_token(&handle);
        let (dto, doc) = run_ocr(&crop, langs, accurate, engine, &cancel)?;
        store_and_emit(&handle, &dto, doc);
        Ok(dto)
    })
    .await
    .map_err(|e| e.to_string())?;

    // Always bring main back and drop overlay.
    let _ = show_main_window(app.clone());
    if let Some(win) = app.get_webview_window("capture") {
        let _ = win.close();
    }

    end_busy(&app);
    match &result {
        Ok(dto) => {
            emit_status(&app, "done", format!("bölge OCR · {} ms", dto.elapsed_ms));
        }
        Err(e) => emit_status(&app, "error", e.as_str()),
    }
    result
}

/// Legacy hook: prefer live desktop pick (what users expect from screen OCR).
#[tauri::command]
pub async fn open_capture_overlay(
    app: tauri::AppHandle,
    monitor: Option<usize>,
) -> Result<(), String> {
    open_live_region_pick(app, monitor, None, None, None).await
}

/* ---------- Toplu (batch) — LLM için TXT / Markdown çıktısı ---------- */

#[derive(Serialize, Clone)]
pub struct BatchItemResult {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub ok: bool,
    pub engine: String,
    pub chars: usize,
    pub elapsed_ms: u64,
    pub error: Option<String>,
    pub text: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatchResultDto {
    pub items: Vec<BatchItemResult>,
    pub format: String,
    pub mode: String,
    pub out_dir: String,
    pub output_files: Vec<String>,
    pub combined_path: Option<String>,
    pub ok_count: usize,
    pub fail_count: usize,
}

fn python_bin() -> String {
    for key in ["MIMO_PYTHON"] {
        if let Ok(p) = std::env::var(key) {
            if !p.trim().is_empty() && std::path::Path::new(&p).exists() {
                return p;
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for rel in [
                "scripts/batch_extract.py",
                "../scripts/batch_extract.py",
                "../../scripts/batch_extract.py",
            ] {
                let _ = rel;
            }
        }
    }
    // Common Windows fallbacks
    for c in ["python", "py", "python3"] {
        if let Ok(out) = std::process::Command::new(c).arg("--version").output() {
            if out.status.success() {
                return c.to_string();
            }
        }
    }
    "python".into()
}

fn extract_script_path() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("../../scripts/batch_extract.py"));
            candidates.push(dir.join("../scripts/batch_extract.py"));
            candidates.push(dir.join("scripts/batch_extract.py"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("scripts/batch_extract.py"));
    }
    candidates.push(PathBuf::from("scripts/batch_extract.py"));
    candidates.into_iter().find(|p| p.exists())
}

fn is_image_ext(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "bmp" | "tif" | "tiff" | "webp" | "gif"
    )
}

fn is_extractable_ext(ext: &str) -> bool {
    matches!(
        ext,
        "pdf" | "docx" | "xlsx" | "pptx" | "rtf" | "txt" | "md" | "csv" | "json" | "log" | "xml" | "html" | "htm"
    )
}

fn extract_via_python(path: &Path) -> Result<String, String> {
    let script = extract_script_path().ok_or_else(|| {
        "batch_extract.py bulunamadı (scripts/batch_extract.py)".to_string()
    })?;
    let py = python_bin();
    let out = std::process::Command::new(&py)
        .arg(&script)
        .arg(path)
        .output()
        .map_err(|e| format!("python başlatılamadı ({py}): {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(err.trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn format_item_md(item: &BatchItemResult) -> String {
    let mut s = String::new();
    s.push_str(&format!("## {}\n\n", item.name));
    s.push_str(&format!(
        "- **Kaynak:** `{}`\n- **Tür:** {}\n- **Motor:** {}\n- **Karakter:** {}\n",
        item.path,
        item.kind,
        if item.engine.is_empty() { "—" } else { &item.engine },
        item.chars
    ));
    if let Some(e) = &item.error {
        s.push_str(&format!("- **Hata:** {}\n", e));
    }
    s.push('\n');
    if item.ok {
        s.push_str("```\n");
        s.push_str(item.text.trim());
        s.push_str("\n```\n\n");
    }
    s
}

fn format_item_txt(item: &BatchItemResult) -> String {
    let mut s = String::new();
    s.push_str("==== ");
    s.push_str(&item.name);
    s.push_str(" ====\n");
    s.push_str(&format!("path: {}\nkind: {}\nengine: {}\n", item.path, item.kind, item.engine));
    if let Some(e) = &item.error {
        s.push_str(&format!("error: {e}\n"));
    }
    s.push_str(&item.text);
    s.push_str("\n\n");
    s
}

#[tauri::command]
pub async fn batch_process_files(
    app: tauri::AppHandle,
    files: Vec<String>,
    out_dir: String,
    format: Option<String>,
    mode: Option<String>,
    engine: Option<String>,
    langs: Option<String>,
    accurate: Option<bool>,
) -> Result<BatchResultDto, String> {
    let format = format.unwrap_or_else(|| "md".into()).to_ascii_lowercase();
    let mode = mode.unwrap_or_else(|| "combined".into()).to_ascii_lowercase();
    let format = if format == "txt" { "txt" } else { "md" }.to_string();
    let mode = if mode == "separate" { "separate" } else { "combined" }.to_string();

    if files.is_empty() {
        return Err("dosya seçilmedi".into());
    }
    if !begin_busy(&app) {
        return Err("OCR zaten çalışıyor".into());
    }
    emit_status(&app, "working", format!("toplu: {} dosya…", files.len()));

    let handle = app.clone();
    let engine2 = engine.clone();
    let langs2 = langs.clone();
    let accurate = accurate.unwrap_or(false);

    let result = tauri::async_runtime::spawn_blocking(move || -> Result<BatchResultDto, String> {
        let out_dir = PathBuf::from(&out_dir);
        std::fs::create_dir_all(&out_dir).map_err(|e| format!("çıktı klasörü: {e}"))?;

        let mut items: Vec<BatchItemResult> = Vec::new();
        let total = files.len();

        for (i, file) in files.iter().enumerate() {
            let path = PathBuf::from(file);
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| file.clone());
            let ext = path
                .extension()
                .map(|s| s.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();

            emit_status(
                &handle,
                "working",
                format!("{}/{total} · {name}", i + 1),
            );
            let _ = handle.emit(
                "batch-progress",
                serde_json::json!({
                    "index": i + 1,
                    "total": total,
                    "name": name,
                    "path": file,
                }),
            );

            if !path.exists() {
                items.push(BatchItemResult {
                    path: file.clone(),
                    name,
                    kind: ext.clone(),
                    ok: false,
                    engine: String::new(),
                    chars: 0,
                    elapsed_ms: 0,
                    error: Some("dosya yok".into()),
                    text: String::new(),
                });
                continue;
            }

            if is_image_ext(&ext) {
                match OcrImage::from_path(&path) {
                    Ok(img) => {
                        match run_ocr(&img, langs2.clone(), accurate, engine2.clone(), &CancellationToken::new())
                        {
                            Ok((dto, _)) => {
                                items.push(BatchItemResult {
                                    path: file.clone(),
                                    name,
                                    kind: format!("image/{ext}"),
                                    ok: true,
                                    engine: dto.engine,
                                    chars: dto.plain_text.chars().count(),
                                    elapsed_ms: dto.elapsed_ms,
                                    error: None,
                                    text: dto.plain_text,
                                });
                            }
                            Err(e) => items.push(BatchItemResult {
                                path: file.clone(),
                                name,
                                kind: format!("image/{ext}"),
                                ok: false,
                                engine: engine2.clone().unwrap_or_default(),
                                chars: 0,
                                elapsed_ms: 0,
                                error: Some(e),
                                text: String::new(),
                            }),
                        }
                    }
                    Err(e) => items.push(BatchItemResult {
                        path: file.clone(),
                        name,
                        kind: format!("image/{ext}"),
                        ok: false,
                        engine: String::new(),
                        chars: 0,
                        elapsed_ms: 0,
                        error: Some(e.to_string()),
                        text: String::new(),
                    }),
                }
            } else if is_extractable_ext(&ext) {
                match extract_via_python(&path) {
                    Ok(text) => {
                        let chars = text.chars().count();
                        items.push(BatchItemResult {
                            path: file.clone(),
                            name,
                            kind: format!("doc/{ext}"),
                            ok: true,
                            engine: "extract".into(),
                            chars,
                            elapsed_ms: 0,
                            error: None,
                            text,
                        });
                    }
                    Err(e) => items.push(BatchItemResult {
                        path: file.clone(),
                        name,
                        kind: format!("doc/{ext}"),
                        ok: false,
                        engine: "extract".into(),
                        chars: 0,
                        elapsed_ms: 0,
                        error: Some(e),
                        text: String::new(),
                    }),
                }
            } else {
                items.push(BatchItemResult {
                    path: file.clone(),
                    name,
                    kind: ext.clone(),
                    ok: false,
                    engine: String::new(),
                    chars: 0,
                    elapsed_ms: 0,
                    error: Some(format!("desteklenmeyen tür: {ext}")),
                    text: String::new(),
                });
            }
        }

        // Write outputs
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let mut output_files: Vec<String> = Vec::new();
        let mut combined_path: Option<String> = None;

        if mode == "separate" {
            for item in &items {
                let stem = Path::new(&item.name)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "out".into());
                let fname = format!("{stem}.ocr.{format}");
                let out_path = out_dir.join(&fname);
                let body = if format == "md" {
                    format_item_md(item)
                } else {
                    format_item_txt(item)
                };
                let head = if format == "md" {
                    format!("# {}\n\n> LLM analizi için Mimo OCR çıktısı\n\n", item.name)
                } else {
                    String::new()
                };
                std::fs::write(&out_path, format!("{head}{body}"))
                    .map_err(|e| format!("{fname}: {e}"))?;
                output_files.push(out_path.display().to_string());
            }
        } else {
            // combined — LLM'e tek dosya
            let mut body = String::new();
            if format == "md" {
                body.push_str("# Mimo OCR — Toplu Çıktı\n\n");
                body.push_str(&format!(
                    "- Tarih: {}\n- Dosya: {}\n- Motor: {}\n- Amaç: LLM / yapay zekâ analiz girdisi\n\n",
                    stamp,
                    items.len(),
                    engine2.clone().unwrap_or_else(|| "seçili".into())
                ));
                body.push_str("---\n\n");
                for item in &items {
                    body.push_str(&format_item_md(item));
                }
            } else {
                body.push_str(&format!(
                    "Mimo OCR — Toplu Çıktı\nTarih: {stamp}\nDosya: {}\nAmaç: LLM analiz girdisi\n\n",
                    items.len()
                ));
                for item in &items {
                    body.push_str(&format_item_txt(item));
                }
            }
            let combined_name = format!("mimo-ocr-batch-{stamp}.{format}");
            let out_path = out_dir.join(&combined_name);
            std::fs::write(&out_path, &body).map_err(|e| e.to_string())?;
            combined_path = Some(out_path.display().to_string());
            output_files.push(out_path.display().to_string());
        }

        let ok_count = items.iter().filter(|i| i.ok).count();
        let fail_count = items.len() - ok_count;

        Ok(BatchResultDto {
            items,
            format: format.clone(),
            mode: mode.clone(),
            out_dir: out_dir.display().to_string(),
            output_files,
            combined_path,
            ok_count,
            fail_count,
        })
    })
    .await
    .map_err(|e| e.to_string())?;

    end_busy(&app);
    match &result {
        Ok(dto) => emit_status(
            &app,
            "done",
            format!(
                "toplu tamam · {}/{} · {}",
                dto.ok_count,
                dto.ok_count + dto.fail_count,
                dto.out_dir
            ),
        ),
        Err(e) => emit_status(&app, "error", e.as_str()),
    }
    result
}
