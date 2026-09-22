//! Derleme betigi:
//! 1. Gomulu ffmpeg'in Windows static derlemesini (`assets/ffmpeg/`) saglar.
//!    200 MB'lik exe'ler GitHub'in dosya limitini astigi icin repoda tutulmaz;
//!    ilk derlemede gyan.dev'den indirilip SHA-256 ile dogrulanir, sonraki
//!    derlemelerde yeniden kullanilir.
//! 2. Tauri scaffolding (`tauri_build::build()`).

use std::path::{Path, PathBuf};

/// gyan.dev "release essentials" (ffmpeg 9.0.2) — GPL v3, bkz. THIRD_PARTY_NOTICES.md
const FFMPEG_ZIP_URL: &str =
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
const FFMPEG_ZIP_SHA256: &str =
    "60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba";

fn main() {
    tauri_build::build();
    println!("cargo:rerun-if-changed=build.rs");
    if let Err(e) = ensure_ffmpeg() {
        // ffmpeg yalnizca Video sekmesi icin gerekli; yoklugu derlemeyi
        // durdurmamali (arayuzde gereksinim olarak gosterilir).
        println!("cargo:warning=ffmpeg saglanamadi: {e}");
    }
}

fn assets_ffmpeg_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("assets")
        .join("ffmpeg")
}

fn ensure_ffmpeg() -> Result<(), String> {
    let dir = assets_ffmpeg_dir();
    let ff = dir.join("ffmpeg.exe");
    let fp = dir.join("ffprobe.exe");
    if ff.is_file() && fp.is_file() {
        return Ok(());
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("klasör: {e}"))?;

    let zip_path = std::env::temp_dir().join("mimo-ffmpeg-essentials.zip");
    download(&zip_path)?;
    verify_sha256(&zip_path)?;
    extract_bins(&zip_path, &dir)?;

    if !(ff.is_file() && fp.is_file()) {
        return Err("çıkarma sonrası exe'ler bulunamadı".into());
    }
    Ok(())
}

fn download(zip_path: &Path) -> Result<(), String> {
    if let Ok(md) = std::fs::metadata(zip_path) {
        if md.len() > 50_000_000 {
            return Ok(()); // daha once indirilmis olabilir; hash asagida dogrulanir
        }
    }
    // Windows 10 1809+ ile gelen curl kullanilir (ek bagimlilik yok).
    let st = std::process::Command::new("curl")
        .args(["-L", "--fail", "-o"])
        .arg(zip_path)
        .arg(FFMPEG_ZIP_URL)
        .status()
        .map_err(|e| format!("curl çalıştırılamadı: {e}"))?;
    if !st.success() {
        return Err(format!("indirme başarısız (kod {})", st.code().unwrap_or(-1)));
    }
    Ok(())
}

fn verify_sha256(zip_path: &Path) -> Result<(), String> {
    use sha2::Digest;
    let data = std::fs::read(zip_path).map_err(|e| format!("zip okunamadı: {e}"))?;
    let mut h = sha2::Sha256::new();
    h.update(&data);
    let got = format!("{:x}", h.finalize());
    if !got.eq_ignore_ascii_case(FFMPEG_ZIP_SHA256) {
        return Err(format!("SHA-256 uyuşmazlığı: {got}"));
    }
    Ok(())
}

fn extract_bins(zip_path: &Path, dir: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| format!("zip açılamadı: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("zip geçersiz: {e}"))?;
    for name in ["ffmpeg.exe", "ffprobe.exe"] {
        let entry = (0..zip.len())
            .map(|i| zip.by_index(i).map(|f| f.name().to_string()))
            .filter_map(|r| r.ok())
            .find(|n| n.ends_with(&format!("/bin/{name}")) || n.ends_with(&format!("\\bin\\{name}")))
            .ok_or_else(|| format!("zip içinde {name} yok"))?;
        let mut src = zip
            .by_name(&entry)
            .map_err(|e| format!("çıkarma: {e}"))?;
        let mut dst = std::fs::File::create(dir.join(name)).map_err(|e| format!("yazma: {e}"))?;
        std::io::copy(&mut src, &mut dst).map_err(|e| format!("kopya: {e}"))?;
    }
    Ok(())
}
