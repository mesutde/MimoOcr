//! Canli OCR sekmesi: ekran bolgesi / pencere / tam ekran kaydi (ffmpeg gdigrab)
//! + mevcut video hatti (kare OCR + scroll dedup + CSV/TXT/MD/XLSX).
//!
//! Akis: rec_start (ffmpeg arka planda kayit) → rec_stop ("q" ile kapat,
//! oynatilabilir mp4) → on yuz video_extract_batch ile metne cevirir.
//! Pencere listesi EnumWindows ile, ses cihazlari dshow listesinden gelir.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, Position, State, WebviewUrl,
    WebviewWindowBuilder,
};

use crate::commands::AppState;
use crate::video::hide_console;

// ---------------------------------------------------------------------------
// Paylasilan kayit durumu
// ---------------------------------------------------------------------------

struct RecSession {
    child: std::process::Child,
    started: std::time::Instant,
    tmp_path: PathBuf,
    log_path: PathBuf,
    kind: String,
    /// Bolge kaydiysa fiziksel [x,y,w,h] (maske hesabı icin).
    region_phys: Option<[i32; 4]>,
    /// Mini Durdur penceresinin fiziksel konumu (OCR maskesi icin).
    recctl_rect: Option<[i32; 4]>,
}

struct RecGlobals {
    armed: bool,
    region_phys: Option<[i32; 4]>,
    region_css: Option<[f64; 4]>,
    session: Option<RecSession>,
}

fn globals() -> &'static Mutex<RecGlobals> {
    static REC: OnceLock<Mutex<RecGlobals>> = OnceLock::new();
    REC.get_or_init(|| {
        Mutex::new(RecGlobals {
            armed: false,
            region_phys: None,
            region_css: None,
            session: None,
        })
    })
}

// ---------------------------------------------------------------------------
// Yardimcilar
// ---------------------------------------------------------------------------

/// Gomulu ffmpeg (exe yani) → PATH `ffmpeg`.
fn bundled_ffmpeg() -> String {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let c = dir.join("ffmpeg/ffmpeg.exe");
            if c.is_file() {
                return c.display().to_string();
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let c = cwd.join("assets/ffmpeg/ffmpeg.exe");
        if c.is_file() {
            return c.display().to_string();
        }
    }
    "ffmpeg".to_string()
}

fn ffmpeg_ok(path: &str) -> bool {
    let mut cmd = std::process::Command::new(path);
    hide_console(&mut cmd);
    cmd.arg("-version");
    cmd.output().map(|o| o.status.success()).unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Pencere + ses listesi
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecWindow {
    pub id: String,
    pub title: String,
    /// Tam baslik (hwnd olmazsa `title=` yedegi icin; gosterimde `title` kullanilir).
    pub full: String,
    pub w: i32,
    pub h: i32,
}

#[cfg(windows)]
fn enum_windows_native() -> Vec<RecWindow> {
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::core::BOOL;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowTextW, IsWindowVisible,
    };

    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let out = unsafe { &mut *(lparam.0 as *mut Vec<isize>) };
        out.push(hwnd.0 as isize);
        true.into()
    }

    let mut raw: Vec<isize> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(cb),
            LPARAM(&mut raw as *mut Vec<isize> as isize),
        );
    }
    let mut wins = Vec::new();
    for h in raw {
        let hwnd = HWND(h as *mut std::ffi::c_void);
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                continue;
            }
            let mut buf = [0u16; 512];
            let n = GetWindowTextW(hwnd, &mut buf);
            if n <= 0 {
                continue;
            }
            let title = String::from_utf16_lossy(&buf[..n as usize]).trim().to_string();
            if title.is_empty() || title.starts_with("Mimo OCR") {
                continue;
            }
            let mut rc = RECT::default();
            let (w, h) = if GetWindowRect(hwnd, &mut rc).is_ok() {
                (rc.right - rc.left, rc.bottom - rc.top)
            } else {
                (0, 0)
            };
            if w < 60 || h < 60 {
                continue;
            }
            wins.push(RecWindow {
                id: format!("0x{h:X}"),
                title: if title.chars().count() > 70 {
                    title.chars().take(70).collect::<String>() + "…"
                } else {
                    title.clone()
                },
                full: title,
                w,
                h,
            });
        }
    }
    // Basliga gore sirali, tekrarli basliklari ele.
    wins.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    wins.dedup_by(|a, b| a.title == b.title);
    wins
}

/// Acik pencereler (kendi pencerelerimiz haric).
#[tauri::command]
pub fn list_open_windows() -> Vec<RecWindow> {
    #[cfg(windows)]
    return enum_windows_native();
    #[cfg(not(windows))]
    return Vec::new();
}

/// dshow ses girisleri (`ffmpeg -list_devices true -f dshow -i dummy`).
pub fn parse_dshow_audio(output: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        // ... ] "Ad" (audio)
        if !t.ends_with("(audio)") {
            continue;
        }
        if let Some(q1) = t.find('"') {
            if let Some(q2) = t[q1 + 1..].find('"') {
                let name = t[q1 + 1..q1 + 1 + q2].trim().to_string();
                if !name.is_empty() && !names.contains(&name) {
                    names.push(name);
                }
            }
        }
    }
    names
}

#[tauri::command]
pub fn list_audio_inputs() -> Vec<String> {
    let ff = bundled_ffmpeg();
    let mut cmd = std::process::Command::new(&ff);
    hide_console(&mut cmd);
    cmd.args(["-hide_banner", "-list_devices", "true", "-f", "dshow", "-i", "dummy"]);
    let out = cmd.output();
    match out {
        Ok(o) => {
            let txt = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            parse_dshow_audio(&txt)
        }
        Err(_) => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Bolge secimi (overlay kayit modu)
// ---------------------------------------------------------------------------

/// Overlay'i kayit-bolge secimi icin kurar (secimde OCR calismaz).
#[tauri::command]
pub fn rec_arm(armed: bool) {
    globals().lock().unwrap().armed = armed;
}

/// Overlay secimi: kuruluysa bolgeyi saklar, overlay'i kapatir, true doner.
/// Koordinatlar complete_capture ile ayni uzaydadir (overlay CSS pikseli).
#[tauri::command]
pub fn rec_take_region(
    app: AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<bool, String> {
    if !globals().lock().unwrap().armed {
        return Ok(false);
    }
    // Fiziksele cevir (cift monitor ofseti dahil) — overlay henuz gorunur.
    let geo = crate::commands::overlay_geometry(&app);
    let (ox, oy, sf) = geo.unwrap_or((0.0, 0.0, 1.0));
    let phys = [
        (ox + x * sf).round() as i32,
        (oy + y * sf).round() as i32,
        (width * sf).round().max(8.0) as i32,
        (height * sf).round().max(8.0) as i32,
    ];
    {
        let mut g = globals().lock().unwrap();
        g.armed = false;
        g.region_phys = Some(phys);
        g.region_css = Some([x, y, width, height]);
    }
    if let Some(ov) = crate::commands::overlay(&app) {
        let _ = ov.hide();
    }
    crate::commands::show_main(&app);
    let _ = app.emit("rec-region", [x, y, width, height]);
    Ok(true)
}

/// Secili kayit bolgesi (CSS piksel, etiket icin).
#[tauri::command]
pub fn rec_get_region() -> Option<[f64; 4]> {
    globals().lock().unwrap().region_css
}

// ---------------------------------------------------------------------------
// Kayit oturumu
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecStartResult {
    pub started_at_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecStopResult {
    pub path: String,
    pub elapsed_sec: u64,
    pub kept_video: bool,
    /// Mini Durdur penceresini OCR'dan cikaran "x,y,w,h" (yoksa None).
    pub mask: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecStatus {
    pub recording: bool,
    pub elapsed_sec: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Tutamac gecerli bir pencere mi? (Liste bayatladiysa hayir.)
#[cfg(windows)]
fn hwnd_valid(id: &str) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::IsWindow;
    let hex = id.trim().trim_start_matches("0x").trim_start_matches("0X");
    let Ok(n) = usize::from_str_radix(hex, 16) else {
        return false;
    };
    let hwnd = windows::Win32::Foundation::HWND(n as *mut std::ffi::c_void);
    unsafe { IsWindow(Some(hwnd)).as_bool() }
}

#[cfg(not(windows))]
fn hwnd_valid(_id: &str) -> bool {
    false
}

/// ffmpeg'i baslatip 2 sn yasarlik kontrolu yapar. Erken olumde gunluk
/// kuyruguyla hata doner (bos kayit yapilmaz).
fn spawn_checked(
    ff: &str,
    args: &[String],
    tmp: &PathBuf,
    log_path: &PathBuf,
    lang: &str,
) -> Result<std::process::Child, String> {
    let log_file = std::fs::File::create(log_path).ok();
    let mut cmd = std::process::Command::new(ff);
    hide_console(&mut cmd);
    cmd.args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null());
    if let Some(f) = log_file {
        cmd.stderr(f);
    } else {
        cmd.stderr(std::process::Stdio::null());
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("{}: {e}", crate::i18n::msg(lang, "rec_spawn")))?;
    std::thread::sleep(std::time::Duration::from_millis(2000));
    if let Ok(Some(st)) = child.try_wait() {
        let tail = read_tail(log_path, 600);
        let _ = std::fs::remove_file(tmp);
        return Err(format!(
            "{} (kod {}): {}",
            crate::i18n::msg(lang, "rec_spawn"),
            st.code().unwrap_or(-1),
            tail
        ));
    }
    Ok(child)
}

/// Kaydi baslatir: kind = "region" | "window" | "screen".
/// - region: onceden rec_take_region ile secilmis olmali.
/// - window: ident = "0x..." pencere tutamaci (gdigrab `hwnd=`).
/// - screen: tum sanal masaustu.
/// audio: dshow cihaz adi (None = sessiz). 2fps sabit.
#[tauri::command]
pub fn rec_start(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
    ident: Option<String>,
    ident_title: Option<String>,
    audio: Option<String>,
) -> Result<RecStartResult, String> {
    let lang = crate::commands::lang_of(&state);
    if globals().lock().unwrap().session.is_some() {
        return Err(crate::i18n::msg(&lang, "rec_busy"));
    }
    let ff = bundled_ffmpeg();
    if !ffmpeg_ok(&ff) {
        return Err(crate::i18n::msg(&lang, "rec_ffmpeg"));
    }
    let mut args: Vec<String> = vec!["-hide_banner".into(), "-loglevel".into(), "warning".into(), "-y".into()];
    let mut region_phys: Option<[i32; 4]> = None;
    match kind.as_str() {
        "region" => {
            let r = globals()
                .lock()
                .unwrap()
                .region_phys
                .ok_or_else(|| crate::i18n::msg(&lang, "rec_no_region"))?;
            // libx264 tek sayi boyutta aninda olur: cift sayiya yuvarla.
            let w = (r[2].max(16) / 2) * 2;
            let h = (r[3].max(16) / 2) * 2;
            region_phys = Some([r[0], r[1], w, h]);
            args.extend([
                "-f".into(), "gdigrab".into(),
                "-framerate".into(), "2".into(),
                "-offset_x".into(), r[0].to_string(),
                "-offset_y".into(), r[1].to_string(),
                "-video_size".into(), format!("{w}x{h}"),
                "-i".into(), "desktop".into(),
            ]);
        }
        "window" => {
            let id = ident.clone().unwrap_or_default();
            if id.is_empty() {
                return Err(crate::i18n::msg(&lang, "rec_no_window"));
            }
            args.extend([
                "-f".into(), "gdigrab".into(),
                "-framerate".into(), "2".into(),
                "-i".into(), format!("hwnd={id}"),
            ]);
        }
        _ => {
            args.extend([
                "-f".into(), "gdigrab".into(),
                "-framerate".into(), "2".into(),
                "-i".into(), "desktop".into(),
            ]);
        }
    }
    if let Some(dev) = audio.as_ref() {
        let dev = dev.trim().to_string();
        if !dev.is_empty() {
            args.extend(["-f".into(), "dshow".into(), "-i".into(), format!("audio={dev}")]);
        }
    }
    let tmp = std::env::temp_dir().join(format!("mimo-rec-{}.mp4", now_ms()));
    let log_path = std::env::temp_dir().join(format!("mimo-rec-{}.log", now_ms()));
    // Cikti blogu ayri tutulur (pencere yedegi ayni ciktiyla yeniden dener).
    let out_args: Vec<String> = vec![
        // Pencere/tam-ekran yerli boyutu da tek sayi olabilir: 1px kirp, olcekleme yok.
        "-vf".into(), "crop=trunc(iw/2)*2:trunc(ih/2)*2".into(),
        "-pix_fmt".into(), "yuv420p".into(),
        "-c:v".into(), "libx264".into(),
        "-preset".into(), "ultrafast".into(),
        "-movflags".into(), "frag_keyframe+empty_moov".into(),
        tmp.display().to_string(),
    ];
    args.extend(out_args.clone());
    // ONCE gizle: ana pencere ilk karelere girmesin (yoksa kendi arayuz
    // yazilarimiz OCR ciktisina siziyor). Bestecinin yeniden cizmesi icin pay.
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    // Pencere kaydi: once tutamac, olmazsa tam baslikla yedek deneme.
    // Tutamac bayatlamis/kapali pencere error 1400 verir; kullaniciya hata
    // gostermek yerine sessizce basliga dus (pencere aciksa yakalanir).
    let mut child = if kind.as_str() == "window" {
        let id = ident.clone().unwrap_or_default();
        let title = ident_title.clone().unwrap_or_default().trim().to_string();
        if id.is_empty() && title.is_empty() {
            crate::commands::show_main(&app);
            return Err(crate::i18n::msg(&lang, "rec_no_window"));
        }
        let hwnd_ok = hwnd_valid(&id);
        let first: Result<std::process::Child, String> = if hwnd_ok {
            spawn_checked(&ff, &args, &tmp, &log_path, &lang)
        } else {
            Err("tutamac bayat".to_string())
        };
        match first {
            Ok(c) => c,
            Err(e) => {
                if title.is_empty() {
                    crate::commands::show_main(&app);
                    return Err(e);
                }
                // Yedek: baslikla yakala.
                let mut args2: Vec<String> =
                    vec!["-hide_banner".into(), "-loglevel".into(), "warning".into(), "-y".into()];
                args2.extend([
                    "-f".into(), "gdigrab".into(),
                    "-framerate".into(), "2".into(),
                    "-i".into(), format!("title={title}"),
                ]);
                if let Some(dev) = audio.clone() {
                    let dev = dev.trim().to_string();
                    if !dev.is_empty() {
                        args2.extend(["-f".into(), "dshow".into(), "-i".into(), format!("audio={dev}")]);
                    }
                }
                args2.extend(out_args.clone());
                match spawn_checked(&ff, &args2, &tmp, &log_path, &lang) {
                    Ok(c) => c,
                    Err(e2) => {
                        crate::commands::show_main(&app);
                        return Err(format!("{e}\n{e2}"));
                    }
                }
            }
        }
    } else {
        match spawn_checked(&ff, &args, &tmp, &log_path, &lang) {
            Ok(c) => c,
            Err(e) => {
                crate::commands::show_main(&app);
                return Err(e);
            }
        }
    };
    // Mini Durdur penceresi (acilista kuruldu; sadece goster).
    let recctl_rect = show_recctl(&app);
    globals().lock().unwrap().session = Some(RecSession {
        child,
        started: std::time::Instant::now(),
        tmp_path: tmp,
        log_path,
        kind: kind.clone(),
        region_phys,
        recctl_rect,
    });
    let _ = app.emit("rec-status", "started");
    // Bolge kaydi: cerceveyi goster (sadece goz; tiklama gecirgen).
    // Cizgi bolgenin 4px DISINDA cizilir (overlay) → videoya girmez;
    // OCR yedegi icin kenar seritleri maskeye eklenir.
    if kind.as_str() == "region" {
        if let Some(r) = globals().lock().unwrap().region_css {
            if let Some(ov) = crate::commands::overlay(&app) {
                let _ = ov.set_ignore_cursor_events(true);
                let _ = ov.show();
            }
            let _ = app.emit("rec-frame", [r[0], r[1], r[2], r[3]]);
        }
    }
    Ok(RecStartResult { started_at_ms: now_ms() })
}

/// Kaydi durdurur ("q" → zarif kapatma; olmazsa sonlandir).
/// keep_video=true ise mp4 cikti klasorune tasinir.
#[tauri::command]
pub fn rec_stop(
    app: AppHandle,
    state: State<'_, AppState>,
    out_dir: String,
    keep_video: bool,
) -> Result<RecStopResult, String> {
    let lang = crate::commands::lang_of(&state);
    let mut g = globals().lock().unwrap();
    let mut sess = g.session.take().ok_or_else(|| crate::i18n::msg(&lang, "rec_idle"))?;
    let elapsed = sess.started.elapsed().as_secs();
    drop(g);
    // Zarif kapatma: stdin'e "q" (mp4 oynatilabilir kalir).
    {
        use std::io::Write;
        if let Some(stdin) = sess.child.stdin.as_mut() {
            let _ = stdin.write_all(b"q");
            let _ = stdin.flush();
        }
    }
    // 15 sn bekle, kapanmazsa oldur.
    let mut waited = 0;
    let finished = loop {
        match sess.child.try_wait() {
            Ok(Some(_)) => break true,
            Ok(None) => {
                if waited >= 150 {
                    let _ = sess.child.kill();
                    let _ = sess.child.wait();
                    break false;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
                waited += 1;
            }
            Err(_) => break false,
        }
    };
    let _ = finished;
    let tmp_path = sess.tmp_path.clone();
    let log_tail = read_tail(&sess.log_path, 600);
    let _ = std::fs::remove_file(&sess.log_path);
    // Mini Durdur penceresini gizle (sonraki kayitta yeniden kullanilir).
    if let Some(w) = app.get_webview_window("recctl") {
        let _ = w.hide();
    }
    // Kayit cercevesini kaldir, overlay'i secime hazirla.
    if let Some(ov) = crate::commands::overlay(&app) {
        let _ = ov.set_ignore_cursor_events(false);
        let _ = ov.hide();
    }
    let size_ok = std::fs::metadata(&tmp_path).map(|m| m.len()).unwrap_or(0) > 1024;
    if !size_ok {
        crate::commands::show_main(&app);
        return Err(format!("{}: {log_tail}", crate::i18n::msg(&lang, "rec_nofile")));
    }
    // OCR maskesi: Durdur penceresi kayit alanindaysa karart.
    let mask = make_mask(&sess);
    let dest = if keep_video {
        let dir = PathBuf::from(&out_dir);
        let _ = std::fs::create_dir_all(&dir);
        let name = format!(
            "ekran-{}.mp4",
            chrono_stamp()
        );
        let d = dir.join(name);
        match std::fs::rename(&tmp_path, &d) {
            Ok(()) => d,
            Err(_) => {
                let _ = std::fs::copy(&tmp_path, &d);
                let _ = std::fs::remove_file(&tmp_path);
                d
            }
        }
    } else {
        tmp_path
    };
    crate::commands::show_main(&app);
    let _ = app.emit("rec-status", "stopped");
    Ok(RecStopResult {
        path: dest.display().to_string(),
        elapsed_sec: elapsed,
        kept_video: keep_video,
        mask,
    })
}

/// Kayit durumu (on yuz zamanlayici + kilit icin).
#[tauri::command]
pub fn rec_status() -> RecStatus {
    match globals().lock().unwrap().session.as_ref() {
        Some(s) => RecStatus {
            recording: true,
            elapsed_sec: s.started.elapsed().as_secs(),
        },
        None => RecStatus {
            recording: false,
            elapsed_sec: 0,
        },
    }
}

/// Saklanmayan ara mp4'u siler (yalnizca gecici dizindeyse).
#[tauri::command]
pub fn rec_cleanup(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let tmp = std::env::temp_dir();
    if p.starts_with(&tmp)
        && p
            .file_name()
            .map(|n| n.to_string_lossy().starts_with("mimo-rec-"))
            .unwrap_or(false)
    {
        let _ = std::fs::remove_file(&p);
    }
    Ok(())
}

fn chrono_stamp() -> String {
    // Dis bagimlilik yok: sistem saatinden dosya-adi guvenli damga.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 20251001-123456 bicimi icin basit ceviri (UTC degil, ham sayi da olur;
    // okunabilirlik icin gun-saat ayrimi yapilmaz — benzersizlik yeterli).
    format!("{secs}")
}

fn read_tail(path: &PathBuf, max_chars: usize) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(max_chars)
        .collect()
}

/// Mini Durdur penceresi: birincil monitorun sag-alt kosesinde 232x78,
/// cercevesiz, hep ustte, gorev cubugunda yok. Fiziksel konum doner.
/// Pencere ACILISTA kurulur (build_recctl); burada sadece konumlanip
/// gosterilir — komut icinden pencere OLUSTURMAK ana-donguyle kilitlenir.
fn show_recctl(app: &AppHandle) -> Option<[i32; 4]> {
    const W: i32 = 232;
    const H: i32 = 78;
    const MGN: i32 = 14;
    let w = app.get_webview_window("recctl")?;
    let mons = app.available_monitors().unwrap_or_default();
    let m = mons
        .iter()
        .find(|m| m.position().x == 0 && m.position().y == 0)
        .or(mons.first())?;
    let (mx, my) = (m.position().x, m.position().y);
    let (mw, mh) = (m.size().width as i32, m.size().height as i32);
    let x = mx + mw - W - MGN;
    let y = my + mh - H - MGN - 56; // gorev cubugu payi
    let _ = w.set_position(Position::Physical(PhysicalPosition { x, y }));
    let _ = w.show();
    Some([x, y, W, H])
}

/// Acilista cagrilir: recctl penceresini gizli kurar.
pub(crate) fn build_recctl(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("recctl").is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "recctl", WebviewUrl::App("recctl.html".into()))
        .title("Kayıt")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .inner_size(232.0, 78.0)
        .visible(false)
        .build()?;
    Ok(())
}

/// Cikis aninda yetim ffmpeg kalmamasi icin oturumu oldurur.
pub(crate) fn kill_session() {
    let mut g = match globals().lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if let Some(mut s) = g.session.take() {
        let _ = s.child.kill();
        let _ = s.child.wait();
    }
}

/// Durdur penceresinin + kayit cercevesinin kayit icindeki karsiligi.
/// "x,y,w,h;x,y,w,h" (video piksel uzayi). Pencere kaynagi ayri pencere
/// oldugu icin onda maske gerekmez.
fn make_mask(sess: &RecSession) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(c) = sess.recctl_rect {
        let (rx, ry, rw, rh) = match sess.kind.as_str() {
            "region" => {
                let r = sess.region_phys?;
                (r[0], r[1], r[2], r[3])
            }
            "screen" => (0, 0, i32::MAX, i32::MAX),
            _ => return frame_strips(sess).map(|s| s.join(";")),
        };
        // Kesisim (video sol-ustu = kayit alani sol-ustu).
        let ix = c[0].max(rx);
        let iy = c[1].max(ry);
        let ex = (c[0] + c[2]).min(rx + rw);
        let ey = (c[1] + c[3]).min(ry + rh);
        if ex - ix >= 8 && ey - iy >= 8 {
            parts.push(format!("{},{},{},{}", ix - rx, iy - ry, ex - ix, ey - iy));
        }
    }
    if let Some(strips) = frame_strips(sess) {
        parts.extend(strips);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(";"))
    }
}

/// Kayit cercevesi yedek seritleri (bolge-ici 4px kenarlar; cizgi disarida
/// oldugu icin genelde bos doner, ekran-kenari sizintisini yakalar).
fn frame_strips(sess: &RecSession) -> Option<Vec<String>> {
    if sess.kind.as_str() != "region" {
        return None;
    }
    let r = sess.region_phys?;
    if r[2] < 16 || r[3] < 16 {
        return None;
    }
    Some(vec![
        format!("0,0,{},4", r[2]),
        format!("0,{},{},4", r[3] - 4, r[2]),
        format!("0,4,4,{}", r[3] - 8),
        format!("{},4,4,{}", r[2] - 4, r[3] - 8),
    ])
}

#[cfg(test)]
mod tests {
    use super::parse_dshow_audio;
    use super::{hwnd_valid, make_mask, RecSession};
    use std::path::PathBuf;

    #[test]
    fn dshow_audio_parse() {
        let sample = "[dshow @ 000001] \"Integrated Webcam\" (video)\n\
            [dshow @ 000001] \"Mikrofon (Realtek Audio)\" (audio)\n\
            [dshow @ 000001]   Alternative name \"@device_cm_xxx\"\n\
            [dshow @ 000001] \"Mikrofon (Realtek Audio)\" (audio)\n";
        let names = parse_dshow_audio(sample);
        assert_eq!(names, vec!["Mikrofon (Realtek Audio)".to_string()]);
    }

    #[test]
    fn bad_hwnd_rejected() {
        assert!(!hwnd_valid("0x0"));
        assert!(!hwnd_valid(""));
        assert!(!hwnd_valid("0xZZZ"));
    }

    fn sess(kind: &str, region: Option<[i32; 4]>, ctl: Option<[i32; 4]>) -> RecSession {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/c", "exit", "0"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        crate::video::hide_console(&mut cmd);
        RecSession {
            child: cmd.spawn().unwrap(),
            started: std::time::Instant::now(),
            tmp_path: PathBuf::new(),
            log_path: PathBuf::new(),
            kind: kind.into(),
            region_phys: region,
            recctl_rect: ctl,
        }
    }

    #[test]
    fn mask_region_overlap() {
        // Bolge (100,100,800,600), kontrol (700,600,232,78) → gorece (600,500,200,78)
        // + cerceve yedek seritleri.
        let s = sess("region", Some([100, 100, 800, 600]), Some([700, 600, 232, 78]));
        assert_eq!(
            make_mask(&s).as_deref(),
            Some("600,500,200,78;0,0,800,4;0,596,800,4;0,4,4,592;796,4,4,592")
        );
    }
    #[test]
    fn mask_no_overlap_or_window() {
        // Kesisme yok: yalnizca cerceve seritleri kalir.
        let s = sess("region", Some([0, 0, 100, 100]), Some([700, 600, 232, 78]));
        assert_eq!(
            make_mask(&s).as_deref(),
            Some("0,0,100,4;0,96,100,4;0,4,4,92;96,4,4,92")
        );
        let s = sess("window", None, Some([700, 600, 232, 78]));
        assert!(make_mask(&s).is_none());
        let s = sess("region", Some([0, 0, 100, 100]), None);
        assert_eq!(
            make_mask(&s).as_deref(),
            Some("0,0,100,4;0,96,100,4;0,4,4,92;96,4,4,92")
        );
    }
}
