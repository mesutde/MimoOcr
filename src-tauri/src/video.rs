//! Video sekmesi: kaydirilan ekran kayitlarindan (mp4/mov/avi/mkv/webm…)
//! tablo/metin cikarimi. Agir is `scripts/video_extract.py` tarafindan yapilir
//! (ffmpeg adaptif kare ornekleme + Tesseract kare OCR + scroll dedup +
//! CSV/TXT/MD yazimi). Bu modul Python yorumlayicisini bulur, betigi
//! `CREATE_NO_WINDOW` ile calistirir ve `PROGRESS n/m` satirlarini
//! `video-progress` / `video-log` olaylari olarak one yuze akitir.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[cfg(windows)]
pub(crate) fn hide_console(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    // 0x08000000 = CREATE_NO_WINDOW (konsol penceresi acarip kapatmaz)
    cmd.creation_flags(0x08000000);
    // Pencereli surecten dogan konsol cocuklari gecersiz stdin'de takilmasin
    cmd.stdin(std::process::Stdio::null());
}

#[cfg(not(windows))]
pub(crate) fn hide_console(cmd: &mut std::process::Command) {
    cmd.stdin(std::process::Stdio::null());
}

/// Yorumlayici secimi: `MIMO_PYTHON` → kurulumla gomulu `python/`
/// → MiMo Desktop → sistem yollari → PATH (Store sahtesi elenir).
/// Gecersiz aday yerine her zaman gercek calisan bir yorumlayici doner;
/// hicbiri yoksa son care "python" (hata mesaji uretir).
pub(crate) fn python_bin() -> String {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("MIMO_PYTHON") {
        if !p.trim().is_empty() {
            candidates.push(PathBuf::from(p));
        }
    }
    // Kurulumla gomulu Python (exe yaninda veya gelistirme agacinda).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("python/python.exe"));
            candidates.push(dir.join("../python/python.exe"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("assets/python/python.exe"));
    }
    candidates.push(PathBuf::from("assets/python/python.exe"));
    if let Ok(base) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(base).join(
            "Programs/Xiaomi MiMo AI/resources/runtimes/win32-x64/python/python.exe",
        ));
    }
    #[cfg(windows)]
    candidates.push(PathBuf::from(
        r"C:\Program Files\Python313\python.exe",
    ));
    for c in &candidates {
        if c.is_file() && probe_python(&c.display().to_string()) {
            return c.display().to_string();
        }
    }
    // PATH uzerinde ilk GERCEK yorumlayici (Store sahtesi atlanir).
    for c in ["python", "py", "python3"] {
        if probe_python(c) {
            return c.to_string();
        }
    }
    "python".to_string()
}

/// Aday yorumlayiciyi `--version` ile dogrular.
/// Microsoft Store sahtesi (`WindowsApps`, "install from the Microsoft Store")
/// bilerek elenir — yoksa Store penceresi acar.
fn probe_python(bin: &str) -> bool {
    if bin.to_ascii_lowercase().contains("windowsapps") {
        return false;
    }
    // Store sahtesi genelde `...\WindowsApps\python.exe` yolundadir; cozumlenmis
    // yolu da kontrol et (PATH'teki `python` oraya baglanabilir).
    if let Ok(resolved) = which_resolve(bin) {
        if resolved.to_ascii_lowercase().contains("windowsapps") {
            return false;
        }
    }
    let mut probe = std::process::Command::new(bin);
    probe.arg("--version");
    hide_console(&mut probe);
    match probe.output() {
        Ok(o) => {
            if !o.status.success() {
                return false;
            }
            let txt = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            !txt.to_ascii_lowercase().contains("microsoft store")
        }
        Err(_) => false,
    }
}

/// PATH uzerinden calistirilabilir dosyanin tam yolunu bulur (probe icin).
fn which_resolve(bin: &str) -> Result<String, ()> {
    if bin.contains('/') || bin.contains('\\') {
        return Ok(bin.to_string());
    }
    let path = std::env::var_os("PATH").ok_or(())?;
    for dir in std::env::split_paths(&path) {
        for name in [format!("{bin}.exe"), bin.to_string()] {
            let p = dir.join(&name);
            if p.is_file() {
                return Ok(p.display().to_string());
            }
        }
    }
    Err(())
}

/// `scripts/<name>` konumunu cozer (toplu is ayni cozumu kullanir).
pub(crate) fn repo_script(name: &str) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("scripts").join(name));
            candidates.push(dir.join("../scripts").join(name));
            candidates.push(dir.join("../../scripts").join(name));
            candidates.push(dir.join("resources/scripts").join(name));
            candidates.push(dir.join("../resources/scripts").join(name));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("scripts").join(name));
    }
    candidates.push(PathBuf::from("scripts").join(name));
    if let Ok(mut extra) = std::env::current_exe() {
        extra.pop();
        candidates.push(extra.join("resources").join("scripts").join(name));
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// `scripts/video_extract.py` konumunu cozer:
/// exe yakinindaki `scripts/` ve `_up_/scripts` (kurulu app + tasinabilir) →
/// gelistirme agacindaki `scripts/` → kaynak yanindaki `resources/`.
fn video_script_path() -> Option<PathBuf> {
    repo_script("video_extract.py")
}

pub fn is_video_ext(ext: &str) -> bool {
    matches!(
        ext,
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "m4v" | "wmv" | "flv"
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoItemResult {
    pub path: String,
    pub name: String,
    pub ok: bool,
    pub error: Option<String>,
    pub output_files: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoBatchDto {
    pub items: Vec<VideoItemResult>,
    pub out_dir: String,
    pub ok_count: usize,
    pub fail_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSupportInfo {
    pub python: String,
    pub python_ok: bool,
    pub script: Option<String>,
    pub script_ok: bool,
    pub ffmpeg_ok: bool,
    pub tesseract_ok: bool,
}

/// On yuzun "Gereksinim" satirini besler; eksik bilesen erken gorunur.
#[tauri::command]
pub fn video_support_info() -> VideoSupportInfo {
    let py = python_bin();
    let python_ok = probe_python(&py);

    let script = video_script_path().map(|p| p.display().to_string());
    // ffmpeg: kurulumla gomulu ffmpeg/ dizini → PATH
    let bundled_ff = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
        .map(|d| d.join("ffmpeg/ffmpeg.exe").is_file())
        .unwrap_or(false);
    let mut ff = std::process::Command::new("ffmpeg");
    ff.arg("-version");
    hide_console(&mut ff);
    let ffmpeg_ok = bundled_ff || ff.output().map(|o| o.status.success()).unwrap_or(false);

    // Tesseract: env → kurulumla gomulu runtime → bilinen kurulum yollari → PATH
    let bundled_ok = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
        .map(|d| d.join("tesseract-runtime/tesseract.exe").is_file())
        .unwrap_or(false);
    let tess_ok = bundled_ok
        || std::env::var("MIMO_TESSERACT")
        .map(|p| PathBuf::from(&p).is_file())
        .unwrap_or(false)
        || PathBuf::from(r"C:\Program Files\Tesseract-OCR\tesseract.exe").is_file()
        || {
            let mut t = std::process::Command::new("tesseract");
            t.arg("--version");
            hide_console(&mut t);
            t.output().map(|o| o.status.success()).unwrap_or(false)
        };

    VideoSupportInfo {
        python: py,
        python_ok,
        script_ok: script.is_some(),
        script,
        ffmpeg_ok,
        tesseract_ok: tess_ok,
    }
}

/// Secilen videolardan CSV/TXT(/MD) cikarir. Her video icin `video-log` ve
/// `video-progress` olaylari yayinlanir; toplu ilerleme `batch-progress` ile.
#[tauri::command]
pub async fn video_extract_batch(
    app: AppHandle,
    files: Vec<String>,
    out_dir: String,
    mode: Option<String>,
    langs: Option<String>,
    max_frames: Option<u32>,
    formats: Option<String>,
) -> Result<VideoBatchDto, String> {
    if files.is_empty() {
        return Err("Video seçilmedi.".into());
    }
    let mode = mode.unwrap_or_else(|| "auto".into());
    if !["auto", "fast", "slow"].contains(&mode.as_str()) {
        return Err("Geçersiz scroll modu (auto/fast/slow).".into());
    }
    let langs = langs.unwrap_or_else(|| "tur+eng".into());
    let max_frames = max_frames.unwrap_or(180).clamp(8, 400);
    let formats = formats.unwrap_or_else(|| "csv,txt".into());

    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(
        move || -> Result<VideoBatchDto, String> {
            let out_dir = PathBuf::from(&out_dir);
            std::fs::create_dir_all(&out_dir)
                .map_err(|e| format!("Çıktı klasörü açılamadı: {e}"))?;
            let script =
                video_script_path().ok_or_else(|| "scripts/video_extract.py bulunamadı.".to_string())?;
            let py = python_bin();
            let total = files.len();
            let mut items = Vec::with_capacity(total);

            let _ = handle.emit(
                "video-log",
                format!("[start] {} video → {}", total, out_dir.display()),
            );
            let _ = handle.emit("video-log", format!("[python] {py}"));
            let _ = handle.emit(
                "video-log",
                format!("[script] {}", script.display()),
            );

            for (i, file) in files.iter().enumerate() {
                let path = PathBuf::from(file);
                let name = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| file.clone());
                let _ = handle.emit(
                    "batch-progress",
                    serde_json::json!({ "index": i + 1, "total": total, "name": name, "path": file }),
                );
                let _ = handle.emit("video-log", format!("=== {name} ({}/{total}) ===", i + 1));

                if !path.is_file() {
                    items.push(VideoItemResult {
                        path: file.clone(),
                        name,
                        ok: false,
                        error: Some("Dosya bulunamadı.".into()),
                        output_files: vec![],
                        message: String::new(),
                    });
                    continue;
                }
                if let Some(ext) = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_ascii_lowercase())
                {
                    if !is_video_ext(&ext) {
                        items.push(VideoItemResult {
                            path: file.clone(),
                            name,
                            ok: false,
                            error: Some(format!("Desteklenmeyen uzantı: .{ext}")),
                            output_files: vec![],
                            message: String::new(),
                        });
                        continue;
                    }
                }

                let mut cmd = std::process::Command::new(&py);
                hide_console(&mut cmd);
                // -u: stdout tamponlamasiz → PROGRESS satirlari anlik akar
                cmd.arg("-u")
                    .arg(&script)
                    .arg("--video")
                    .arg(&path)
                    .arg("--out")
                    .arg(&out_dir)
                    .arg("--lang")
                    .arg(langs.replace(',', "+"))
                    .arg("--mode")
                    .arg(&mode)
                    .arg("--max-frames")
                    .arg(max_frames.to_string())
                    .arg("--formats")
                    .arg(&formats)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());

                let mut child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        items.push(VideoItemResult {
                            path: file.clone(),
                            name,
                            ok: false,
                            error: Some(format!("Python başlatılamadı ({py}): {e}")),
                            output_files: vec![],
                            message: String::new(),
                        });
                        continue;
                    }
                };

                let stdout = child.stdout.take();
                let stderr = child.stderr.take();
                let log_handle = handle.clone();
                let err_task = std::thread::spawn(move || -> String {
                    use std::io::Read;
                    let mut buf = Vec::new();
                    if let Some(mut e) = stderr {
                        let _ = e.read_to_end(&mut buf);
                    }
                    String::from_utf8_lossy(&buf).trim().to_string()
                });

                let mut last_line = String::new();
                if let Some(out) = stdout {
                    use std::io::{BufRead, BufReader};
                    for line in BufReader::new(out).lines().map_while(Result::ok) {
                        last_line = line.clone();
                        if let Some(rest) = line.strip_prefix("PROGRESS ") {
                            let mut it = rest.split('/');
                            let fr: f64 =
                                it.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
                            let ft: f64 =
                                it.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
                            let pct = if ft > 0.0 { (fr / ft * 100.0).min(100.0) } else { 0.0 };
                            let _ = log_handle.emit(
                                "video-progress",
                                serde_json::json!({
                                    "videoIndex": i + 1, "videoTotal": total,
                                    "frame": fr, "frameTotal": ft, "percent": pct,
                                    "name": name, "phase": "ocr",
                                }),
                            );
                        }
                        let _ = log_handle.emit("video-log", line.clone());
                    }
                }

                let status = child.wait().map_err(|e| format!("Video işi beklenemedi: {e}"))?;
                let err_out = err_task.join().unwrap_or_default();

                if status.success() {
                    let stem = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "video".to_string());
                    let mut outputs = Vec::new();
                    for ext in ["csv", "txt", "md"] {
                        let p = out_dir.join(format!("{stem}.video.{ext}"));
                        if p.is_file() {
                            outputs.push(p.display().to_string());
                        }
                    }
                    let _ = handle.emit(
                        "video-progress",
                        serde_json::json!({
                            "videoIndex": i + 1, "videoTotal": total,
                            "frame": 100.0, "frameTotal": 100.0, "percent": 100.0,
                            "name": name, "phase": "done",
                        }),
                    );
                    let _ = handle.emit("video-log", format!("[done] {name}"));
                    items.push(VideoItemResult {
                        path: file.clone(),
                        name,
                        ok: true,
                        error: None,
                        output_files: outputs,
                        message: last_line,
                    });
                } else {
                    let detail = if err_out.is_empty() {
                        format!("Çıkış kodu: {}", status.code().unwrap_or(-1))
                    } else {
                        err_out.chars().take(600).collect()
                    };
                    let _ = handle.emit("video-log", format!("[error] {name}: {detail}"));
                    items.push(VideoItemResult {
                        path: file.clone(),
                        name,
                        ok: false,
                        error: Some(detail),
                        output_files: vec![],
                        message: last_line,
                    });
                }
            }

            let ok_count = items.iter().filter(|x| x.ok).count();
            let dto = VideoBatchDto {
                out_dir: out_dir.display().to_string(),
                ok_count,
                fail_count: items.len().saturating_sub(ok_count),
                items,
            };
            let _ = handle.emit(
                "video-log",
                format!("[finish] {}/{} başarılı", dto.ok_count, dto.items.len()),
            );
            Ok(dto)
        },
    )
    .await
    .map_err(|e| format!("Video görevi yarıda kesildi: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_sahtesi_elenir() {
        // Calistirmadan elenmeli (Store penceresi acilmamali).
        assert!(!probe_python(r"C:\WindowsApps\python.exe"));
        assert!(!probe_python("WindowsApps/python3.exe"));
        assert!(!probe_python("kesinlikle-yok-boyle-bir-python-xyz"));
    }

    #[test]
    fn gomulu_python_calisir() {
        let exe = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../assets/python/python.exe");
        if !exe.is_file() {
            eprintln!("assets/python yok — test atlandı (temiz klon + çevrimdışı derleme?)");
            return;
        }
        let mut v = std::process::Command::new(&exe);
        v.arg("--version");
        hide_console(&mut v);
        let out = v.output().expect("gömülü python çalışmalı");
        assert!(out.status.success());
        let txt = String::from_utf8_lossy(&out.stdout);
        assert!(txt.contains("Python 3."), "çıktı: {txt}");
        let mut libs = std::process::Command::new(&exe);
        libs.args(["-c", "import reportlab, pypdf, openpyxl"]);
        hide_console(&mut libs);
        assert!(
            libs.output().map(|o| o.status.success()).unwrap_or(false),
            "reportlab/pypdf/openpyxl gömülü python'da olmalı"
        );
    }
}
