use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256, Sha512};
use tauri::{AppHandle, Emitter};

const BUFSIZE: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct FileHash {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub hashes: BTreeMap<String, String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompareEntry {
    pub rel_path: String,
    pub status: String,
    pub a: Option<FileHash>,
    pub b: Option<FileHash>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompareResult {
    pub dir_a: String,
    pub dir_b: String,
    pub same: usize,
    pub different: usize,
    pub only_a: usize,
    pub only_b: usize,
    pub error: usize,
    pub entries: Vec<CompareEntry>,
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn norm_algos(algos: &[String]) -> Vec<String> {
    let mut out: Vec<String> = algos
        .iter()
        .map(|a| a.trim().to_ascii_lowercase())
        .filter(|a| matches!(a.as_str(), "md5" | "sha1" | "sha256" | "sha512" | "crc32"))
        .collect();
    out.sort();
    out.dedup();
    if out.is_empty() {
        out.push("sha256".to_string());
    }
    out
}

fn hash_file(path: &Path, algos: &[String]) -> Result<(u64, BTreeMap<String, String>), String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);

    let want_md5 = algos.iter().any(|a| a == "md5");
    let want_sha1 = algos.iter().any(|a| a == "sha1");
    let want_sha256 = algos.iter().any(|a| a == "sha256");
    let want_sha512 = algos.iter().any(|a| a == "sha512");
    let want_crc32 = algos.iter().any(|a| a == "crc32");

    let mut md5 = if want_md5 { Some(md5::Md5::new()) } else { None };
    let mut sha1 = if want_sha1 { Some(sha1::Sha1::new()) } else { None };
    let mut sha256 = if want_sha256 { Some(Sha256::new()) } else { None };
    let mut sha512 = if want_sha512 { Some(Sha512::new()) } else { None };
    let mut crc32 = if want_crc32 { Some(crc32fast::Hasher::new()) } else { None };

    let mut buf = vec![0u8; BUFSIZE];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        let chunk = &buf[..n];
        if let Some(h) = md5.as_mut() {
            h.update(chunk);
        }
        if let Some(h) = sha1.as_mut() {
            h.update(chunk);
        }
        if let Some(h) = sha256.as_mut() {
            h.update(chunk);
        }
        if let Some(h) = sha512.as_mut() {
            h.update(chunk);
        }
        if let Some(h) = crc32.as_mut() {
            h.update(chunk);
        }
    }

    let mut hashes = BTreeMap::new();
    if let Some(h) = md5 {
        hashes.insert("md5".to_string(), to_hex(h.finalize().as_slice()));
    }
    if let Some(h) = sha1 {
        hashes.insert("sha1".to_string(), to_hex(h.finalize().as_slice()));
    }
    if let Some(h) = sha256 {
        hashes.insert("sha256".to_string(), to_hex(h.finalize().as_slice()));
    }
    if let Some(h) = sha512 {
        hashes.insert("sha512".to_string(), to_hex(h.finalize().as_slice()));
    }
    if let Some(h) = crc32 {
        hashes.insert("crc32".to_string(), format!("{:08x}", h.finalize()));
    }
    Ok((size, hashes))
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn make_hash(path: &Path, algos: &[String]) -> FileHash {
    let name = file_name_of(path);
    match hash_file(path, algos) {
        Ok((size, hashes)) => FileHash {
            path: path.display().to_string(),
            name,
            size,
            hashes,
            error: None,
        },
        Err(e) => FileHash {
            path: path.display().to_string(),
            name,
            size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
            hashes: BTreeMap::new(),
            error: Some(e),
        },
    }
}

fn pair_status(ha: &FileHash, hb: &FileHash) -> &'static str {
    if ha.error.is_some() || hb.error.is_some() {
        "error"
    } else if ha.hashes == hb.hashes {
        "same"
    } else {
        "different"
    }
}

/// Tum dosyalari (uzanti filtresi olmadan) toplar; gizli klasorlere girmez.
fn collect_files(root: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = match std::fs::read_dir(&d) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let mut subdirs: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if recursive {
                    let hidden = p
                        .file_name()
                        .map(|n| n.to_string_lossy().starts_with('.'))
                        .unwrap_or(false);
                    if !hidden {
                        subdirs.push(p);
                    }
                }
            } else if p.is_file() {
                out.push(p);
            }
        }
        subdirs.sort();
        for s in subdirs.into_iter().rev() {
            stack.push(s);
        }
    }
    out.sort();
    out
}

#[tauri::command]
pub async fn hash_files(
    app: AppHandle,
    files: Vec<String>,
    algos: Vec<String>,
) -> Vec<FileHash> {
    let algos = norm_algos(&algos);
    let total = files.len();
    tauri::async_runtime::spawn_blocking(move || {
        let mut out = Vec::with_capacity(total);
        for (i, f) in files.iter().enumerate() {
            let path = PathBuf::from(f);
            let name = file_name_of(&path);
            let _ = app.emit(
                "hash-progress",
                serde_json::json!({"index": i + 1, "total": total, "name": name}),
            );
            out.push(make_hash(&path, &algos));
        }
        out
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub fn expand_all_dirs(dirs: Vec<String>, recursive: bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for dir in &dirs {
        let root = PathBuf::from(dir);
        if !root.is_dir() {
            continue;
        }
        for p in collect_files(&root, recursive) {
            out.push(p.display().to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}

fn rel_key(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
}

#[tauri::command]
pub async fn compare_dirs(
    app: AppHandle,
    dir_a: String,
    dir_b: String,
    algos: Vec<String>,
    recursive: bool,
) -> Result<CompareResult, String> {
    let root_a = PathBuf::from(&dir_a);
    let root_b = PathBuf::from(&dir_b);
    if !root_a.is_dir() {
        return Err(format!("{dir_a} bir klasör değil."));
    }
    if !root_b.is_dir() {
        return Err(format!("{dir_b} bir klasör değil."));
    }
    let algos = norm_algos(&algos);
    let ra = root_a.clone();
    let rb = root_b.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let files_a = collect_files(&ra, recursive);
        let files_b = collect_files(&rb, recursive);
        let map_a: BTreeMap<String, PathBuf> = files_a
            .iter()
            .map(|p| (rel_key(&ra, p), p.clone()))
            .collect();
        let map_b: BTreeMap<String, PathBuf> = files_b
            .iter()
            .map(|p| (rel_key(&rb, p), p.clone()))
            .collect();

        let mut keys: Vec<String> = map_a.keys().chain(map_b.keys()).cloned().collect();
        keys.sort();
        keys.dedup();

        let total = keys.len();
        let mut entries = Vec::with_capacity(total);
        let (mut same, mut different, mut only_a, mut only_b, mut error) = (0, 0, 0, 0, 0);
        for (i, k) in keys.iter().enumerate() {
            let _ = app.emit(
                "hash-progress",
                serde_json::json!({"index": i + 1, "total": total, "name": k}),
            );
            let a_path = map_a.get(k);
            let b_path = map_b.get(k);
            let rel_display = a_path
                .or(b_path)
                .map(|p| {
                    p.strip_prefix(&ra)
                        .or_else(|_| p.strip_prefix(&rb))
                        .unwrap_or(p)
                        .display()
                        .to_string()
                })
                .unwrap_or_else(|| k.clone());

            match (a_path, b_path) {
                (Some(pa), Some(pb)) => {
                    let ha = make_hash(pa, &algos);
                    let hb = make_hash(pb, &algos);
                    let status = pair_status(&ha, &hb);
                    match status {
                        "error" => error += 1,
                        "same" => same += 1,
                        _ => different += 1,
                    }
                    entries.push(CompareEntry {
                        rel_path: rel_display,
                        status: status.to_string(),
                        a: Some(ha),
                        b: Some(hb),
                    });
                }
                (Some(pa), None) => {
                    only_a += 1;
                    entries.push(CompareEntry {
                        rel_path: rel_display,
                        status: "only_a".to_string(),
                        a: Some(make_hash(pa, &algos)),
                        b: None,
                    });
                }
                (None, Some(pb)) => {
                    only_b += 1;
                    entries.push(CompareEntry {
                        rel_path: rel_display,
                        status: "only_b".to_string(),
                        a: None,
                        b: Some(make_hash(pb, &algos)),
                    });
                }
                (None, None) => {}
            }
        }

        Ok(CompareResult {
            dir_a: ra.display().to_string(),
            dir_b: rb.display().to_string(),
            same,
            different,
            only_a,
            only_b,
            error,
            entries,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Verilen dosyanin bulundugu klasoru sistem dosya yoneticisinde acar.
#[tauri::command]
pub fn show_in_folder(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err("Dosya bulunamadı.".to_string());
    }
    let dir = if p.is_dir() {
        p.clone()
    } else {
        p.parent().map(|d| d.to_path_buf()).unwrap_or(p.clone())
    };

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer.exe")
            .arg(&dir)
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("mimo-hash-{tag}-{}-{n}", std::process::id()))
    }

    #[test]
    fn hash_known_vectors() {
        let dir = temp_dir("vec");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.txt");
        std::fs::write(&f, b"abc").unwrap();
        let algos = norm_algos(&[
            "MD5".into(),
            "sha1".into(),
            "sha256".into(),
            "sha512".into(),
            "crc32".into(),
        ]);
        let (size, h) = hash_file(&f, &algos).unwrap();
        assert_eq!(size, 3);
        assert_eq!(h["md5"], "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(h["sha1"], "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(h["sha256"], "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(h["crc32"], "352441c2");
        assert_eq!(h["sha512"], "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pair_status_flags_changed_files() {
        let mut a = BTreeMap::new();
        a.insert("sha256".to_string(), "aa".to_string());
        let mut b = BTreeMap::new();
        b.insert("sha256".to_string(), "bb".to_string());
        let ha = FileHash {
            path: "x\\a.pdf".into(),
            name: "a.pdf".into(),
            size: 1,
            hashes: a.clone(),
            error: None,
        };
        let hb = FileHash {
            path: "y\\a.pdf".into(),
            name: "a.pdf".into(),
            size: 1,
            hashes: b,
            error: None,
        };
        let same = FileHash { hashes: a, ..ha.clone() };
        let err = FileHash { hashes: BTreeMap::new(), error: Some("x".into()), ..ha.clone() };
        assert_eq!(pair_status(&ha, &hb), "different");
        assert_eq!(pair_status(&ha, &same), "same");
        assert_eq!(pair_status(&ha, &err), "error");
    }

    #[test]
    fn collect_skips_hidden_dirs_when_recursive() {
        let root = temp_dir("walk");
        let sub = root.join("sub");
        let hidden = root.join(".git");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(root.join("a.txt"), b"a").unwrap();
        std::fs::write(sub.join("b.txt"), b"b").unwrap();
        std::fs::write(hidden.join("c.txt"), b"c").unwrap();
        assert_eq!(collect_files(&root, false).len(), 1);
        assert_eq!(collect_files(&root, true).len(), 2);
        let _ = std::fs::remove_dir_all(&root);
    }
}
