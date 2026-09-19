use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mimo_ocr_core::{CancellationToken, OcrDocument};

/// Set only on intentional quit (Çıkış / X / tepsi).
/// Prevents Tauri from auto-exiting when the capture overlay is the last
/// visible window while main is hidden.
pub static ALLOW_EXIT: AtomicBool = AtomicBool::new(false);

pub fn allow_exit_now() {
    ALLOW_EXIT.store(true, Ordering::SeqCst);
}

pub fn exit_allowed() -> bool {
    ALLOW_EXIT.load(Ordering::SeqCst)
}

#[derive(Default)]
pub struct AppState {
    pub last_document: Mutex<Option<OcrDocument>>,
    pub last_image_path: Mutex<Option<String>>,
    pub langs: Mutex<String>,
    pub busy: Arc<AtomicBool>,
    pub cancel: Mutex<CancellationToken>,
    /// Options captured when live region pick starts.
    pub pick_langs: Mutex<String>,
    pub pick_engine: Mutex<String>,
    pub pick_accurate: Mutex<bool>,
}

impl AppState {
    pub fn store_pick_opts(&self, langs: &str, engine: &str, accurate: bool) {
        if let Ok(mut g) = self.pick_langs.lock() {
            *g = langs.to_string();
        }
        if let Ok(mut g) = self.pick_engine.lock() {
            *g = engine.to_string();
        }
        if let Ok(mut g) = self.pick_accurate.lock() {
            *g = accurate;
        }
    }

    pub fn load_pick_opts(&self) -> (Option<String>, Option<String>, bool) {
        let langs = self
            .pick_langs
            .lock()
            .ok()
            .filter(|g| !g.trim().is_empty())
            .map(|g| g.clone());
        let engine = self
            .pick_engine
            .lock()
            .ok()
            .filter(|g| !g.trim().is_empty())
            .map(|g| g.clone());
        let accurate = self
            .pick_accurate
            .lock()
            .map(|g| *g)
            .unwrap_or(false);
        (langs, engine, accurate)
    }

    #[allow(dead_code)]
    pub fn langs(&self) -> String {
        self.langs
            .lock()
            .map(|g| {
                if g.is_empty() {
                    "tur,eng".to_string()
                } else {
                    g.clone()
                }
            })
            .unwrap_or_else(|_| "tur,eng".to_string())
    }
}
