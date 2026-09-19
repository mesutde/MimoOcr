#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use state::AppState;

fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem};
    let capture = MenuItem::with_id(app, "capture_full", "Tam ekran OCR", true, None::<&str>)?;
    let region = MenuItem::with_id(app, "capture_region", "Bölge seçerek OCR", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Pencereyi göster", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Çıkış (uygulamayı kapat)", true, None::<&str>)?;
    Menu::with_items(app, &[&capture, &region, &open, &quit])
}

fn setup_tessdata_env(app: &tauri::AppHandle) {
    use tauri::Manager as _;
    let has_models = |p: &std::path::Path| {
        p.is_dir() && (p.join("tur.traineddata").exists() || p.join("eng.traineddata").exists())
    };

    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(dir) = app.path().resource_dir() {
        candidates.push(dir.join("tessdata"));
        candidates.push(dir.join("tesseract/tessdata"));
        candidates.push(dir.join("tesseract-runtime/tessdata"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            candidates.push(d.join("tesseract-runtime/tessdata"));
            candidates.push(d.join("tesseract/tessdata"));
            candidates.push(d.join("tessdata"));
            let mut p = d.to_path_buf();
            for _ in 0..5 {
                candidates.push(p.join("assets/tesseract-runtime/tessdata"));
                candidates.push(p.join("assets/models/tessdata"));
                if !p.pop() {
                    break;
                }
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("assets/tesseract-runtime/tessdata"));
        candidates.push(cwd.join("assets/models/tessdata"));
        candidates.push(cwd.join("target/debug/tessdata"));
    }

    let tessdata = candidates.into_iter().find(|p| has_models(p));

    // Tesseract binary next to exe / resources / project runtime
    let mut bin_candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(dir) = app.path().resource_dir() {
        bin_candidates.push(dir.join("tesseract/tesseract.exe"));
        bin_candidates.push(dir.join("tesseract-runtime/tesseract.exe"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            bin_candidates.push(d.join("tesseract/tesseract.exe"));
            bin_candidates.push(d.join("tesseract-runtime/tesseract.exe"));
            bin_candidates.push(d.join("assets/tesseract-runtime/tesseract.exe"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        bin_candidates.push(cwd.join("assets/tesseract-runtime/tesseract.exe"));
        bin_candidates.push(cwd.join("target/debug/tesseract/tesseract.exe"));
    }
    bin_candidates.push(std::path::PathBuf::from(
        "C:/Program Files/Tesseract-OCR/tesseract.exe",
    ));
    let tesseract = bin_candidates.into_iter().find(|p| p.exists());

    if let Some(td) = tessdata {
        let td = mimo_ocr_engines::clean_path_for_tesseract(&td);
        // Verify models really exist before exporting env.
        if td.join("eng.traineddata").exists() || td.join("tur.traineddata").exists() {
            std::env::set_var("MIMO_TESSDATA", &td);
            std::env::set_var("TESSDATA_PREFIX", &td);
        }
    }
    if let Some(t) = tesseract {
        let t = mimo_ocr_engines::clean_path_for_tesseract(&t);
        std::env::set_var("MIMO_TESSERACT", &t);
        if let Some(bin) = t.parent() {
            let path = std::env::var("PATH").unwrap_or_default();
            let bin_s = bin.display().to_string();
            if !path.contains(&bin_s) {
                std::env::set_var("PATH", format!("{};{}", bin_s, path));
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            setup_tessdata_env(app.handle());

            let menu = build_tray_menu(app.handle())?;
            let _tray = tauri::tray::TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("Mimo OCR")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "capture_full" => {
                        let handle = app.clone();
                        tauri::async_runtime::spawn_blocking(move || {
                            let _ = commands::ocr_full_screen_inner(&handle, 0, None, false);
                        });
                    }
                    "capture_region" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                            // In-window crop UI — never open a fullscreen OS overlay.
                            let _ = win.eval(
                                "window.mimoStartRegionPick && window.mimoStartRegionPick()",
                            );
                        }
                    }
                    "open" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyO);
            app.global_shortcut().on_shortcut(shortcut, |app, _sc, event| {
                if event.state == ShortcutState::Pressed {
                    let handle = app.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        // balanced — full-screen accurate 2× can freeze the UI
                        let _ = commands::ocr_full_screen_inner(&handle, 0, None, false);
                    });
                }
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_monitors,
            commands::ocr_full_screen,
            commands::ocr_region,
            commands::ocr_path,
            commands::ocr_snapshot_regions,
            commands::copy_text,
            commands::engine_status,
            commands::open_capture_overlay,
            commands::start_region_pick,
            commands::open_live_region_pick,
            commands::complete_live_region_pick,
            commands::cancel_ocr,
            commands::quit_app,
            commands::show_main_window,
            commands::finish_region_pick,
            commands::cancel_region_pick,
            commands::batch_process_files,
        ])
        .on_window_event(|window, event| {
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    if window.label() == "main" {
                        // User asked to close main → real quit.
                        crate::state::allow_exit_now();
                        let app = window.app_handle();
                        if let Some(cap) = app.get_webview_window("capture") {
                            let _ = cap.close();
                        }
                        app.exit(0);
                        api.prevent_close();
                    }
                    if window.label() == "capture" {
                        // Overlay closed → main must be visible again.
                        if let Some(main) = window.app_handle().get_webview_window("main") {
                            let _ = main.show();
                            let _ = main.set_focus();
                        }
                    }
                }
                WindowEvent::Destroyed => {
                    if window.label() == "capture" {
                        if let Some(main) = window.app_handle().get_webview_window("main") {
                            let _ = main.show();
                            let _ = main.set_focus();
                        }
                    }
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building Mimo OCR")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                // Hide-main + close-overlay must NOT quit the app.
                if code.is_none() && !crate::state::exit_allowed() {
                    api.prevent_exit();
                    if let Some(main) = app.get_webview_window("main") {
                        let _ = main.show();
                        let _ = main.set_focus();
                    }
                }
            }
        });
}
