//! Mimo OCR — Tauri uygulama çekirdeği.
//!
//! Katmanlar: ön yüz (TS) → komutlar → çekirdek (capture/preprocess) → motor (engine).

mod capture;
mod commands;
mod engine;
mod models;
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
    if let Some(ov) = app.get_webview_window("overlay") {
        let _ = ov.show();
        let _ = ov.set_focus();
    }
}

fn build_overlay(app: &tauri::AppHandle) -> tauri::Result<()> {
    // Sanal masaüstü: tüm monitörlerin mantıksal birleşimi (Windows'ta köken
    // negatif olabilir; mantıksal koordinatlar capture.rs ile uyumludur).
    let monitors = app.available_monitors().unwrap_or_default();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (i, m) in monitors.iter().enumerate() {
        let sf = m.scale_factor();
        let mx = m.position().x as f64;
        let my = m.position().y as f64;
        let mw = m.size().width as f64 / sf;
        let mh = m.size().height as f64 / sf;
        if i == 0 {
            min_x = mx;
            min_y = my;
            max_x = mx + mw;
            max_y = my + mh;
        } else {
            min_x = min_x.min(mx);
            min_y = min_y.min(my);
            max_x = max_x.max(mx + mw);
            max_y = max_y.max(my + mh);
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
        b = b.position(min_x, min_y)
            .inner_size((max_x - min_x).max(64.0), (max_y - min_y).max(64.0));
    } else {
        b = b.maximized(true);
    }
    b.build()?;
    Ok(())
}

fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let mi_capture = MenuItem::with_id(app, "capture", "Bölge Yakala (Ctrl+Shift+X)", true, None::<&str>)?;
    let mi_show = MenuItem::with_id(app, "show", "Mimo OCR'ı Aç", true, None::<&str>)?;
    let mi_quit = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&mi_capture, &mi_show, &mi_quit])?;

    TrayIconBuilder::with_id("mimo-tray")
        .tooltip("Mimo OCR")
        .icon(app.default_window_icon().cloned().unwrap())
        .menu(&menu)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "capture" => open_overlay(app),
            "show" => show_main(app),
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
                show_main(tray.app_handle());
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Motor: Tesseract 5 (CLI bağdaştırıcısı)
            let engine = engine::TesseractCli::detect().map_err(|e| {
                eprintln!("Motor bulunamadı: {e}");
                e
            })?;
            app.manage(AppState {
                engine: Arc::new(engine),
                options: Mutex::new(OcrOptions::default()),
                last_region: Mutex::new(None),
            });

            build_overlay(app.handle())?;
            build_tray(app.handle())?;

            // Küresel kısayol: Ctrl+Shift+X → bölge yakalama
            let handle = app.handle().clone();
            app.global_shortcut()
                .on_shortcut("Ctrl+Shift+X", move |_app, _sc, ev| {
                    if ev.state == ShortcutState::Pressed {
                        open_overlay(&handle);
                    }
                })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::begin_capture,
            commands::cancel_capture,
            commands::complete_capture,
            commands::re_capture_last,
            commands::ocr_run,
            commands::copy_text,
            commands::save_text,
            commands::set_options,
            commands::get_options,
            commands::list_models,
            commands::install_model,
            commands::remove_model,
            commands::available_languages,
            video::video_support_info,
            video::video_extract_batch,
        ])
        .run(tauri::generate_context!())
        .expect("Mimo OCR çalıştırılamadı");
}
