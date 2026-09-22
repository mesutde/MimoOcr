//! Baslatma teshisi: adim adim `%APPDATA%/mimo-ocr/startup.log` dosyasina yazar.
//! Sessiz-olum durumlarinda kullanici bu dosyayi gonderir; hangi adimda
//! kalindigi buradan okunur. Ayrica WebView2 surumunu kayit defterinden okur
//! ve olumcul hatalari yerel mesaj kutusuyla gosterir.

use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Write as _;

use crate::engine::dirs_config_file;

/// Gunluk dosyasi yolu (yoksa olusturulmaya calisilir).
pub fn log_path() -> Option<std::path::PathBuf> {
    dirs_config_file("startup.log")
}

/// Zaman damgali bir satir ekler (dosya yoksa olusturur, her seferinde acar/kapatir).
pub fn mark(step: &str) {
    let Some(path) = log_path() else { return };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let mut line = String::new();
        let _ = writeln!(line, "[{now}] {step}");
        let _ = f.write_all(line.as_bytes());
        let _ = f.flush();
    }
}

/// WebView2 calisma surumunu kayit defterinden okur (`reg query` ile, ek bagimlilik yok).
/// Edge kuruluysa cogu sistemde WebView2 de bulunur; bos donerse suphelidir.
pub fn webview2_version() -> String {
    const GUID: &str = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    const KEY: &str = "SOFTWARE\\Microsoft\\EdgeUpdate\\Clients";
    let mut out = String::new();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        for hive in ["HKLM", "HKCU"] {
            for view in ["/reg:64", "/reg:32"] {
                let mut cmd = std::process::Command::new("reg");
                cmd.args([
                    "query",
                    &format!("{hive}\\{KEY}\\{GUID}"),
                    "/v",
                    "pv",
                    view,
                ]);
                cmd.creation_flags(0x08000000);
                if let Ok(res) = cmd.output() {
                    if res.status.success() {
                        let txt = String::from_utf8_lossy(&res.stdout);
                        for line in txt.lines() {
                            let l = line.trim();
                            if l.starts_with("pv") {
                                let _ = write!(out, "{hive}{view}={l}; ");
                            }
                        }
                    }
                }
            }
        }
    }
    if out.is_empty() {
        out = "(bulunamadı)".to_string();
    }
    out
}

/// Olumcul hatayi mesaj kutusuyla gosterir ve cikar.
/// `windows_subsystem = "windows"` yuzunden konsol olmadigi icin bu kutu
/// sessiz-olum sinifini ortadan kaldirir.
pub fn fatal(title: &str, message: &str) -> ! {
    mark(&format!("FATAL {title}: {message}"));
    let log = log_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(log yolu yok)".to_string());
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(format!("{message}\n\nGünlük: {log}"))
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    std::process::exit(1);
}
