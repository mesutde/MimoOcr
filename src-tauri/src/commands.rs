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
    /// Secili motor kimligi: "tesseract" | "windows-ocr"
    pub active_engine: Mutex<String>,
    /// Son başarılı bölge seçimi (overlay yerel mantıksal koordinatları)
    pub last_region: Mutex<Option<[f64; 4]>>,
}

// ---------------------------------------------------------------------------
// Motor kaydi (tesseract / windows-ocr)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub detail: Option<String>,
    pub active: bool,
}

fn engine_list(state: &State<'_, AppState>) -> Vec<EngineInfo> {
    let active = state.active_engine.lock().unwrap().clone();
    let tess = state.engine.lock().unwrap().clone();
    let (tess_ok, tess_detail) = match &tess {
        Some(e) => (true, Some(e.exe_path().display().to_string())),
        None => (false, Some("bulunamadı".into())),
    };
    #[cfg(windows)]
    let (win_ok, win_detail) = {
        let langs = crate::engine_winocr::WindowsOcrEngine::available_languages();
        if crate::engine_winocr::WindowsOcrEngine::available() {
            (true, Some(format!("dil: {}", langs.join(", "))))
        } else {
            (false, Some("dil paketi yok".into()))
        }
    };
    #[cfg(not(windows))]
    let (win_ok, win_detail) = (false, Some("yalnız Windows".into()));
    #[cfg(windows)]
    let win_name = crate::engine_winocr::DISPLAY_NAME;
    #[cfg(not(windows))]
    let win_name = "Windows OCR (sistem)";
    vec![
        EngineInfo {
            id: "tesseract".into(),
            name: "Tesseract (gömülü)".into(),
            available: tess_ok,
            detail: tess_detail,
            active: active == "tesseract",
        },
        EngineInfo {
            id: "windows-ocr".into(),
            name: win_name.into(),
            available: win_ok,
            detail: win_detail,
            active: active == "windows-ocr",
        },
    ]
}

/// Header'daki motor seciciyi besler.
#[tauri::command]
pub fn list_engines(state: State<'_, AppState>) -> Vec<EngineInfo> {
    engine_list(&state)
}

/// Motoru degistirir (kullanilamayan motora gecise izin vermez).
#[tauri::command]
pub fn set_engine(state: State<'_, AppState>, id: String) -> Result<Vec<EngineInfo>, OcrError> {
    let ok = engine_list(&state)
        .iter()
        .any(|e| e.id == id && e.available);
    if !ok {
        return Err(OcrError::Image(format!("Motor kullanılamıyor: {id}")));
    }
    *state.active_engine.lock().unwrap() = id;
    Ok(engine_list(&state))
}

pub(crate) fn active_engine_id(state: &State<'_, AppState>, override_id: Option<String>) -> String {
    if let Some(id) = override_id {
        if matches!(id.as_str(), "tesseract" | "windows-ocr") {
            return id;
        }
    }
    let cur = state.active_engine.lock().unwrap().clone();
    if cur == "windows-ocr" {
        cur
    } else {
        // Bilinmeyen/kaldirilmis motor kimligi tesseract'a duser.
        "tesseract".to_string()
    }
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
    cmd.stdin(std::process::Stdio::null());
    crate::video::hide_console(&mut cmd);
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

/// Overlay penceresinin GERCEK geometrisi (fiziksel konum + olcek).
/// Fare CSS pikseli buradan fiziksele cevrilir; varsayim yok.
fn overlay_geometry(app: &AppHandle) -> Option<(f64, f64, f64)> {
    let ov = overlay(app)?;
    let pos = ov.outer_position().ok()?;
    let sf = ov.scale_factor().ok()?;
    Some((pos.x as f64, pos.y as f64, sf))
}

/// Secim bitince/iptal olunca ana pencereyi tekrar gosterir.
fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Overlay'i gizler ve GERCEKTEN gizlenmesini bekler.
/// `hide()` eszamansizdir; beklemeden yakalanirsa secim kutusu
/// (mavi #rect / yesil coklu kutular) karenin icine islenir.
/// Bestecinin yeniden cizmesi icin kisa pay birakilir.
fn settle_overlay_hidden(app: &AppHandle) {
    if let Some(ov) = overlay(app) {
        let _ = ov.hide();
        for _ in 0..66 {
            match ov.is_visible() {
                Ok(false) => break,
                _ => std::thread::sleep(std::time::Duration::from_millis(15)),
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
}

#[tauri::command]
pub fn begin_capture(app: AppHandle, state: State<'_, AppState>) -> Result<Option<[f64; 4]>, OcrError> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
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
    show_main(&app);
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
    // Once overlay tam gizlensin (secim kutusu kareye sizmasin), yakala,
    // sonra ana pencereyi gosterip OCR'la.
    settle_overlay_hidden(&app);
    *state.last_region.lock().unwrap() = Some([x, y, width, height]);

    let opts = state.options.lock().unwrap().clone();
    // Overlay GERCEK geometrisi (canli): fare CSS pikseli buradan fiziksele
    // cevrilir; koken/yerlesim varsayimi yok, cift monitor ofseti tutar.
    let ov_geo = overlay_geometry(&app);
    let png = tauri::async_runtime::spawn_blocking(move || {
        let raw = match ov_geo {
            Some((ox, oy, sf)) => capture::capture_region(x, y, width, height, ox, oy, sf)?,
            None => capture::capture_region(x, y, width, height, 0.0, 0.0, 1.0)?,
        };
        capture::preprocess(&raw, opts.scale)
    })
    .await
    .map_err(|e| OcrError::Image(e.to_string()))??;

    show_main(&app);
    let engine_id = active_engine_id(&state, None);
    run_ocr(&state, png, engine_id).await
}

/// Tam ekran OCR (Yakala sekmesindeki tek tuş + monitör seçici).
#[tauri::command]
pub async fn ocr_fullscreen(
    app: AppHandle,
    state: State<'_, AppState>,
    monitor: Option<usize>,
) -> Result<OcrDocument, OcrError> {
    // Uygulamanin kendisi kareye girmesin diye ana pencereyi de gizle.
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    settle_overlay_hidden(&app);
    let engine_id = active_engine_id(&state, None);
    let opts = state.options.lock().unwrap().clone();
    let idx = monitor.unwrap_or(0);
    let png = tauri::async_runtime::spawn_blocking(move || {
        let raw = capture::capture_monitor(idx)?;
        capture::preprocess(&raw, opts.scale)
    })
    .await
    .map_err(|e| OcrError::Image(e.to_string()))??;

    show_main(&app);
    run_ocr(&state, png, engine_id).await
}

/// Önizleme-içi çoklu alan: overlay'de Ctrl ile biriktirilen bölgeler tek
/// seferde OCR'lanır, her bölgenin belgesi ayrı döner.
#[tauri::command]
pub async fn ocr_preview_regions(
    app: AppHandle,
    state: State<'_, AppState>,
    regions: Vec<[f64; 4]>,
) -> Result<Vec<OcrDocument>, OcrError> {
    if regions.is_empty() {
        return Err(OcrError::Image("Bölge seçilmedi".into()));
    }
    if regions.len() > 12 {
        return Err(OcrError::Image("En fazla 12 bölge".into()));
    }
    // Once overlay tam gizlensin (yesil kutular kareye sizmasin), tum
    // bolgeleri yakala, sonra pencereyi gosterip OCR'la.
    settle_overlay_hidden(&app);
    let engine_id = active_engine_id(&state, None);
    let ov_geo = overlay_geometry(&app);
    let mut pngs = Vec::with_capacity(regions.len());
    for r in &regions {
        let opts = state.options.lock().unwrap().clone();
        let (rx, ry, rw, rh) = (r[0], r[1], r[2], r[3]);
        let png = tauri::async_runtime::spawn_blocking(move || {
            let raw = match ov_geo {
                Some((ox, oy, sf)) => capture::capture_region(rx, ry, rw, rh, ox, oy, sf)?,
                None => capture::capture_region(rx, ry, rw, rh, 0.0, 0.0, 1.0)?,
            };
            capture::preprocess(&raw, opts.scale)
        })
        .await
        .map_err(|e| OcrError::Image(e.to_string()))??;
        pngs.push(png);
    }
    show_main(&app);
    let mut docs = Vec::with_capacity(pngs.len());
    for png in pngs {
        docs.push(run_ocr(&state, png, engine_id.clone()).await?);
    }
    Ok(docs)
}

/// Dosya veya pano kaynağından OCR çalıştırır.
/// `engine` verilirse (sağ-tık yeniden OCR) aktif motor yerine o kullanılır.
#[tauri::command]
pub async fn ocr_run(
    state: State<'_, AppState>,
    source: OcrSource,
    engine: Option<String>,
) -> Result<OcrDocument, OcrError> {
    let engine_id = active_engine_id(&state, engine);
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

    run_ocr(&state, png, engine_id).await
}

async fn run_ocr(
    state: &State<'_, AppState>,
    png: Vec<u8>,
    engine_id: String,
) -> Result<OcrDocument, OcrError> {
    let opts = state.options.lock().unwrap().clone();
    let doc = recognize_with(state, &png, &opts, &engine_id).await?;

    // Varsayılan davranış: sonucu otomatik panoya kopyala (ayardan kapatılabilir)
    let auto_copy = state.options.lock().unwrap().auto_copy;
    if auto_copy && !doc.plain_text.is_empty() {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(doc.plain_text.clone());
        }
    }
    Ok(doc)
}

/// Secili motorla (veya degistirilmis motorla) PNG uzerinden tanima.
/// Toplu is de ayni dagitimi kullanir.
pub(crate) async fn recognize_with(
    state: &State<'_, AppState>,
    png: &[u8],
    opts: &OcrOptions,
    engine_id: &str,
) -> Result<OcrDocument, OcrError> {
    let png = png.to_vec();
    let opts = opts.clone();
    let engine_id = engine_id.to_string();
    match engine_id.as_str() {
        #[cfg(windows)]
        "windows-ocr" => {
            tauri::async_runtime::spawn_blocking(move || {
                crate::engine_winocr::WindowsOcrEngine.recognize_png(&png, &opts)
            })
            .await
            .map_err(|e| OcrError::Image(e.to_string()))?
        }
        #[cfg(not(windows))]
        "windows-ocr" => Err(OcrError::Image("Windows OCR yalnız Windows'ta".into())),
        _ => {
            let engine = engine_or_err(state)?;
            tauri::async_runtime::spawn_blocking(move || engine.recognize(&png, &opts))
                .await
                .map_err(|e| OcrError::Image(e.to_string()))?
        }
    }
}

/// Önizlemedeki son görseli farklı motorla yeniden OCR'lar (sağ-tık menüsü).
/// Bölge yakalamalarının dosya yolu olmadığı için base64 taşınır.
#[tauri::command]
pub async fn ocr_bytes(
    state: State<'_, AppState>,
    image_base64: String,
    engine: Option<String>,
) -> Result<OcrDocument, OcrError> {
    use base64::Engine as _;
    let engine_id = active_engine_id(&state, engine);
    let png = base64::engine::general_purpose::STANDARD
        .decode(image_base64.trim())
        .map_err(|e| OcrError::Image(format!("Görsel çözülemedi: {e}")))?;
    run_ocr(&state, png, engine_id).await
}

#[tauri::command]
pub fn copy_text(text: String) -> Result<(), OcrError> {
    let mut cb = arboard::Clipboard::new().map_err(|e| OcrError::Image(e.to_string()))?;
    cb.set_text(text).map_err(|e| OcrError::Image(e.to_string()))
}

/// Önizlemedeki resmi panoya kopyalar (sağ-tık menüdeki Kopyala).
#[tauri::command]
pub fn copy_image(image_base64: String) -> Result<(), OcrError> {
    use base64::Engine as _;
    let png = base64::engine::general_purpose::STANDARD
        .decode(image_base64.trim())
        .map_err(|e| OcrError::Image(format!("Görsel çözülemedi: {e}")))?;
    let img = image::load_from_memory(&png).map_err(|e| OcrError::Image(e.to_string()))?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let clip = arboard::ImageData {
        width: w,
        height: h,
        bytes: rgba.into_raw().into(),
    };
    let mut cb = arboard::Clipboard::new().map_err(|e| OcrError::Image(e.to_string()))?;
    cb.set_image(clip).map_err(|e| OcrError::Image(e.to_string()))
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

/// Secili motorun destekledigi OCR dilleri (on yuzdeki dil listesini besler).
/// Tesseract: kurulu tessdata'dan; Windows OCR: sistem dil paketlerinden.
#[tauri::command]
pub fn engine_languages(
    state: State<'_, AppState>,
    engine: Option<String>,
) -> Vec<EngineLang> {
    match active_engine_id(&state, engine).as_str() {
        #[cfg(windows)]
        "windows-ocr" => {
            let mut codes: Vec<String> =
                crate::engine_winocr::WindowsOcrEngine::available_languages()
                    .into_iter()
                    .map(|t| win_tag_to_code(&t))
                    .collect();
            codes.sort();
            codes.dedup();
            // Windows motoru tur+eng yazimini da anlar; liste bossa temel ikili.
            if codes.is_empty() {
                codes = vec!["tur".into(), "eng".into()];
            }
            codes.into_iter().map(|code| EngineLang { code }).collect()
        }
        _ => {
            let dir = state
                .engine
                .lock()
                .unwrap()
                .clone()
                .and_then(|e| e.tessdata_dir.clone());
            match dir {
                Some(d) => {
                    let mut codes = list_tessdata_langs(&d);
                    // tur+eng her zaman onerilir (gömülü varsayılan).
                    if codes.contains(&"tur".to_string())
                        && codes.contains(&"eng".to_string())
                        && !codes.contains(&"tur+eng".to_string())
                    {
                        codes.insert(0, "tur+eng".to_string());
                    }
                    codes.into_iter().map(|code| EngineLang { code }).collect()
                }
                None => vec![],
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EngineLang {
    pub code: String,
}

/// WinRT dil etiketini tesseract koduna cevirir (bilinmeyeni oldugu gibi birakir).
fn win_tag_to_code(tag: &str) -> String {
    match tag.to_ascii_lowercase().as_str() {
        "tr" | "tr-tr" => "tur".into(),
        "en" | "en-us" | "en-gb" => "eng".into(),
        "ar" | "ar-sa" => "ara".into(),
        "ja" | "ja-jp" => "jpn".into(),
        "zh-hans" | "zh-cn" | "zh-sg" => "chi_sim".into(),
        "zh-hant" | "zh-tw" | "zh-hk" => "chi_tra".into(),
        "ko" | "ko-kr" => "kor".into(),
        _ => tag.to_string(),
    }
}

/// tessdata dizinindeki *.traineddata dosyalarindan dil kodlari (osd haric).
fn list_tessdata_langs(dir: &std::path::Path) -> Vec<String> {
    let mut codes: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    name.strip_suffix(".traineddata")
                        .map(|s| s.to_string())
                        .filter(|s| s != "osd")
                })
                .collect()
        })
        .unwrap_or_default();
    codes.sort();
    codes
}

