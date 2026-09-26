//! Mimo OCR — Tauri uygulama çekirdeği.
//!
//! Katmanlar: ön yüz (TS) → komutlar → çekirdek (capture/preprocess) → motor (engine).

mod capture;
mod commands;
mod batch;
mod engine;
mod sheet;
mod web;
#[cfg(windows)]
mod engine_winocr;
mod models;
mod startup_log;
mod video;

use std::sync::{Arc, Mutex};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use commands::AppState;
use engine::OcrOptions;

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn open_overlay(app: &tauri::AppHandle) {
    // Secim sirasinda ana pencere gizlenir ki kullanici istedigi alani secebilsin.
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    if let Some(ov) = app.get_webview_window("overlay") {
        let _ = ov.show();
        let _ = ov.set_focus();
    }
}

fn build_overlay(app: &tauri::AppHandle) -> tauri::Result<()> {
    // Sanal masaustu kapsama garantisi.
    //
    // Tao `position()`/`size()` FIZIKSEL piksel dondurur; `.position()` /
    // `.inner_size()` ise MANTIKSAL piksel bekler. Karisik DPI'da fiziksel
    // konum + mantiksal boyut karisimi birlesimi KUCUK hesaplar ve pencere
    // sagdan/alttan kisa kalir (bosluk) — cift monitor kaymasinin kaynagi.
    //
    // Cozum: iki birlesimi de hesapla; konumu MINIMUMA, boyutu MAKSIMUMA kur
    // (+ pay). Tao her nasil yorumlarsa yorumlasin pencere ekrani TAM kaplar;
    // disari tasan kisim gorunmezdir, bosluk ise hatadir. Secim eslemesi
    // capture.rs'te pencerenin GERCEK konumundan yapildigi icin bu tasma
    // isabeti etkilemez. sf=1 iken iki birlesim aynidir (tek monitorde
    // davranis degismez).
    let monitors = app.available_monitors().unwrap_or_default();
    let (mut pmin_x, mut pmin_y, mut pmax_x, mut pmax_y) =
        (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let (mut lmin_x, mut lmin_y, mut lmax_x, mut lmax_y) =
        (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (i, m) in monitors.iter().enumerate() {
        let sf = m.scale_factor().max(f64::EPSILON);
        let px = m.position().x as f64;
        let py = m.position().y as f64;
        let pw = m.size().width as f64;
        let ph = m.size().height as f64;
        let (lx, ly, lw, lh) = (px / sf, py / sf, pw / sf, ph / sf);
        if i == 0 {
            (pmin_x, pmin_y, pmax_x, pmax_y) = (px, py, px + pw, py + ph);
            (lmin_x, lmin_y, lmax_x, lmax_y) = (lx, ly, lx + lw, ly + lh);
        } else {
            pmin_x = pmin_x.min(px);
            pmin_y = pmin_y.min(py);
            pmax_x = pmax_x.max(px + pw);
            pmax_y = pmax_y.max(py + ph);
            lmin_x = lmin_x.min(lx);
            lmin_y = lmin_y.min(ly);
            lmax_x = lmax_x.max(lx + lw);
            lmax_y = lmax_y.max(ly + lh);
        }
    }

    let mut b = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("overlay.html".into()))
        .title("Mimo OCR — Bölge Seç")
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false);

    if !monitors.is_empty() {
        // Kapsama garantisi: konum = iki uzayin minimumu (sol/ust her
        // yorumda kapali), boyut = maksimumu + pay (sag/alt her yorumda kapali).
        let ox = pmin_x.min(lmin_x);
        let oy = pmin_y.min(lmin_y);
        let ow = ((pmax_x - pmin_x).max(lmax_x - lmin_x) + 64.0).max(64.0);
        let oh = ((pmax_y - pmin_y).max(lmax_y - lmin_y) + 64.0).max(64.0);
        b = b.position(ox, oy).inner_size(ow, oh);
    } else {
        b = b.maximized(true);
    }
    b.build()?;
    Ok(())
}

fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let mi_capture = MenuItem::with_id(app, "capture", "Bölge Yakala (Ctrl+Shift+X)", true, None::<&str>)?;
    let mi_show = MenuItem::with_id(app, "show", "Mimo OCR'ı Aç", true, None::<&str>)?;
    let mi_clear = MenuItem::with_id(app, "clear-all", "Tümünü Temizle", true, None::<&str>)?;
    let mi_quit = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&mi_capture, &mi_show, &mi_clear, &mi_quit])?;

    let Some(icon) = app.default_window_icon().cloned() else {
        eprintln!("Tepsi simgesi yok, tepsi atlanıyor.");
        return Ok(());
    };
    TrayIconBuilder::with_id("mimo-tray")
        .tooltip("Mimo OCR")
        .icon(icon)
        .menu(&menu)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "capture" => open_overlay(app),
            "show" => show_main(app),
            "clear-all" => {
                use tauri::Emitter;
                show_main(app);
                let _ = app.emit("clear-all", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = ev
            {
                // Tek tik: uygulamayi one getirip dogrudan bolge yakalamayi baslat.
                show_main(tray.app_handle());
                open_overlay(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Ikinci calistirma: yeni pencere acma, mevcudu one getir.
            show_main(app);
        }))
        .on_window_event(|window, event| {
            // Kapat (X) pencereyi yok etmez, tepsiye gizler.
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            startup_log::mark(&format!("setup başladı (v{})", env!("CARGO_PKG_VERSION")));
            // Baslik cubugu: uygulama adi + surum (surum Cargo'dan otomatik).
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_title(&format!("Mimo OCR v{}", env!("CARGO_PKG_VERSION")));
            }
            startup_log::mark(&format!(
                "webview2: {}",
                startup_log::webview2_version()
            ));
            // Motor: Tesseract 5. Bulunamazsa uygulama yine de acilir;
            // arayuzdeki uyari bandi kurulum/yol secimi sunar.
            match engine::TesseractCli::detect() {
                Ok(engine) => {
                    startup_log::mark(&format!(
                        "motor bulundu: {}",
                        engine.exe_path().display()
                    ));
                    app.manage(AppState {
                        engine: Mutex::new(Some(Arc::new(engine))),
                        options: Mutex::new(OcrOptions::default()),
                        active_engine: Mutex::new("tesseract".to_string()),
                        last_region: Mutex::new(None),
                    });
                }
                Err(e) => {
                    startup_log::mark(&format!("motor yok (devam): {e}"));
                    eprintln!("Tesseract bulunamadı, motorsuz başlanıyor: {e}");
                    app.manage(AppState {
                        engine: Mutex::new(None),
                        options: Mutex::new(OcrOptions::default()),
                        active_engine: Mutex::new("tesseract".to_string()),
                        last_region: Mutex::new(None),
                    });
                }
            }

            if let Err(e) = build_overlay(app.handle()) {
                startup_log::mark(&format!("overlay hatası (devam): {e}"));
                eprintln!("Overlay kurulamadı: {e}");
            } else {
                startup_log::mark("overlay hazır");
            }
            if let Err(e) = build_tray(app.handle()) {
                startup_log::mark(&format!("tepsi hatası (devam): {e}"));
                eprintln!("Tepsi kurulamadı: {e}");
            } else {
                startup_log::mark("tepsi hazır");
            }

            // Küresel kısayol: Ctrl+Shift+X → bölge yakalama
            let handle = app.handle().clone();
            if let Err(e) = app.global_shortcut().on_shortcut(
                "Ctrl+Shift+X",
                move |_app, _sc, ev| {
                    if ev.state == ShortcutState::Pressed {
                        open_overlay(&handle);
                    }
                },
            ) {
                startup_log::mark(&format!("kısayol hatası (devam): {e}"));
                eprintln!("Kısayol kaydedilemedi: {e}");
            } else {
                startup_log::mark("kısayol hazır");
            }

            startup_log::mark("setup tamam, pencere açılıyor");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::begin_capture,
            commands::cancel_capture,
            commands::complete_capture,
            commands::re_capture_last,
            commands::ocr_run,
            commands::ocr_bytes,
            commands::copy_text,
            commands::copy_image,
            commands::save_text,
            commands::set_options,
            commands::get_options,
            commands::list_models,
            commands::install_model,
            commands::remove_model,
            commands::engine_languages,
            commands::engine_status,
            commands::rescan_engine,
            commands::set_engine_path,
            commands::list_engines,
            commands::set_engine,
            commands::ocr_fullscreen,
            commands::ocr_preview_regions,
            batch::list_monitors,
            batch::ocr_path,
            batch::batch_process_files,
            sheet::import_sheet_url,
            sheet::import_doc_url,
            web::detect_web_url,
            web::import_direct_url,
            video::video_support_info,
            video::video_extract_batch,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            startup_log::fatal(
                "Mimo OCR başlatılamadı",
                &format!("Açılış başarısız: {e}"),
            )
        });
}
