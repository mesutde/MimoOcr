//! Web sekmesi: Google Sheets (herkese acik paylasimli) → CSV/XLSX/Markdown.
//!
//! Kullanim: `.../spreadsheets/d/{ID}/...[?gid=N]` URL'i verilir; `gviz`
//! CSV ucu okunur (`/export` giris isteyen dosyalarda 401 verir, `gviz`
//! baglantiyla paylasilanlarda calisir). Gizli sayfada anlasilir hata doner.
//! Ag kullanimi yalniz kullanici istegiyle olur (model indirme gibi).

use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetResultDto {
    pub path: String,
    pub name: String,
    pub rows: usize,
    pub cols: usize,
    pub format: String,
}

/// URL'den spreadsheet kimligi + gid cikarir.
fn parse_sheet_url(url: &str) -> Result<(String, String), String> {
    let url = url.trim();
    let marker = "docs.google.com/spreadsheets/d/";
    let pos = url
        .find(marker)
        .ok_or_else(|| "Google Sheets bağlantısı değil (docs.google.com/spreadsheets/d/… olmalı).".to_string())?;
    let rest = &url[pos + marker.len()..];
    let id: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if id.is_empty() {
        return Err("Sayfa kimliği (ID) okunamadı.".into());
    }
    // ?gid=123 veya #gid=123
    let gid = ["?gid=", "&gid=", "#gid="]
        .iter()
        .filter_map(|k| {
            url.find(k).map(|i| {
                url[i + k.len()..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
            })
        })
        .find(|g: &String| !g.is_empty())
        .unwrap_or_else(|| "0".to_string());
    Ok((id, gid))
}

fn gviz_csv_url(id: &str, gid: &str) -> String {
    format!("https://docs.google.com/spreadsheets/d/{id}/gviz/tq?tqx=out:csv&gid={gid}")
}

fn fetch_csv_text(url: &str) -> Result<String, String> {
    let resp = reqwest::blocking::get(url).map_err(|e| format!("İndirilemedi: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err("Bu sayfa herkese açık değil (paylaşım: bağlantıya sahip olanlar).".into());
    }
    if !status.is_success() {
        return Err(format!("HTTP {status} — sayfa okunamadı."));
    }
    resp.text()
        .map_err(|e| format!("Metin okunamadı: {e}"))
}

fn parse_csv_rows(text: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut rows = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| format!("CSV çözülemedi: {e}"))?;
        rows.push(rec.iter().map(|s| s.to_string()).collect());
    }
    // Tamamen bos satirlari at (gviz dolgusu).
    rows.retain(|r: &Vec<String>| r.iter().any(|c| !c.trim().is_empty()));
    if rows.is_empty() {
        return Err("Sayfada veri yok.".into());
    }
    Ok(rows)
}

fn md_escape(cell: &str) -> String {
    cell.replace('|', "\\|").replace('\n', "<br>")
}

fn write_markdown(rows: &[Vec<String>], dest: &Path, title: &str) -> Result<(), String> {
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut out = format!("# {title}\n\n");
    for (i, row) in rows.iter().enumerate() {
        let cells: Vec<String> = (0..width)
            .map(|c| row.get(c).map(|s| md_escape(s)).unwrap_or_default())
            .collect();
        out.push_str(&format!("| {} |\n", cells.join(" | ")));
        if i == 0 {
            out.push_str(&format!("|{}|\n", vec!["---"; width].join("|")));
        }
    }
    std::fs::write(dest, out).map_err(|e| format!("Yazılamadı: {e}"))
}

fn write_xlsx(rows: &[Vec<String>], dest: &Path, title: &str) -> Result<(), String> {
    use rust_xlsxwriter::Workbook;
    let mut wb = Workbook::new();
    let sheet_name: String = title.chars().take(28).collect();
    let ws = wb
        .add_worksheet()
        .set_name(sheet_name)
        .map_err(|e| format!("Sayfa adı: {e}"))?;
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            ws.write_string(r as u32, c as u16, cell)
                .map_err(|e| format!("Hücre ({r},{c}): {e}"))?;
        }
    }
    wb.save(dest).map_err(|e| format!("XLSX yazılamadı: {e}"))
}

use std::path::Path;

/// Google Docs URL'den belge kimligi cikarir.
fn parse_doc_url(url: &str) -> Result<String, String> {
    let url = url.trim();
    let marker = "docs.google.com/document/d/";
    let pos = url
        .find(marker)
        .ok_or_else(|| "Google Docs bağlantısı değil (docs.google.com/document/d/… olmalı).".to_string())?;
    let rest = &url[pos + marker.len()..];
    let id: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if id.is_empty() {
        return Err("Belge kimliği (ID) okunamadı.".into());
    }
    Ok(id)
}

fn doc_export_url(id: &str, format: &str) -> String {
    format!("https://docs.google.com/document/d/{id}/export?format={format}")
}

fn fetch_bytes(url: &str) -> Result<(Vec<u8>, String), String> {
    let resp = reqwest::blocking::get(url).map_err(|e| format!("İndirilemedi: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err("Bu belge herkese açık değil (paylaşım: bağlantıya sahip olanlar).".into());
    }
    if !status.is_success() {
        return Err(format!("HTTP {status} — belge okunamadı."));
    }
    let ctype = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    // Bogus HTML giris sayfasi yakalama (giris isteyen belgelerde export HTML doner).
    let bytes = resp.bytes().map_err(|e| format!("Bayt okunamadı: {e}"))?.to_vec();
    if ctype.contains("text/html") {
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(300)]).to_lowercase();
        if head.contains("accounts.google.com") || head.contains("sign in") || head.contains("giris") {
            return Err("Bu belge giriş istiyor (herkese açık paylaşılmamış).".into());
        }
    }
    Ok((bytes, ctype))
}

/// Google Docs URL → out_dir altina docx/odt/txt/pdf/md indirir.
#[tauri::command]
pub async fn import_doc_url(
    url: String,
    out_dir: String,
    format: Option<String>,
) -> Result<SheetResultDto, String> {
    let format = format.unwrap_or_else(|| "docx".into()).to_ascii_lowercase();
    if !["docx", "odt", "txt", "pdf", "md"].contains(&format.as_str()) {
        return Err("Format docx, odt, txt, pdf veya md olmalı.".into());
    }
    let id = parse_doc_url(&url)?;
    let out_dir = PathBuf::from(&out_dir);

    tauri::async_runtime::spawn_blocking(move || -> Result<SheetResultDto, String> {
        std::fs::create_dir_all(&out_dir)
            .map_err(|e| format!("Çıktı klasörü açılamadı: {e}"))?;
        let short: String = id.chars().take(12).collect();
        let name = format!("google-doc-{short}");
        let dest = out_dir.join(format!("{name}.{format}"));
        // DOCX/ODT/PDF/TXT Google tarafindan uretilir; MD icin TXT alinip cevrilir.
        let dl_format = if format == "md" { "txt" } else { format.as_str() };
        let (bytes, _) = fetch_bytes(&doc_export_url(&id, dl_format))?;
        if bytes.is_empty() {
            return Err("Belge boş döndü.".into());
        }
        if format == "md" {
            let text = String::from_utf8_lossy(&bytes).to_string();
            let body = format!("# {name}\n\n```\n{text}\n```\n");
            std::fs::write(&dest, body).map_err(|e| format!("Yazılamadı: {e}"))?;
            let rows = text.lines().count();
            return Ok(SheetResultDto {
                path: dest.display().to_string(),
                name,
                rows,
                cols: 1,
                format,
            });
        }
        std::fs::write(&dest, &bytes).map_err(|e| format!("Yazılamadı: {e}"))?;
        Ok(SheetResultDto {
            path: dest.display().to_string(),
            name,
            rows: 1,
            cols: 1,
            format,
        })
    })
    .await
    .map_err(|e| format!("Görev yarıda kesildi: {e}"))?
}

/// Google Sheets URL → out_dir altina csv/xlsx/md indirir.
#[tauri::command]
pub async fn import_sheet_url(
    url: String,
    out_dir: String,
    format: Option<String>,
) -> Result<SheetResultDto, String> {
    let format = format.unwrap_or_else(|| "csv".into()).to_ascii_lowercase();
    if !["csv", "xlsx", "md"].contains(&format.as_str()) {
        return Err("Format csv, xlsx veya md olmalı.".into());
    }
    let (id, gid) = parse_sheet_url(&url)?;
    let out_dir = PathBuf::from(&out_dir);

    tauri::async_runtime::spawn_blocking(move || -> Result<SheetResultDto, String> {
        std::fs::create_dir_all(&out_dir)
            .map_err(|e| format!("Çıktı klasörü açılamadı: {e}"))?;
        let text = fetch_csv_text(&gviz_csv_url(&id, &gid))?;
        let rows = parse_csv_rows(&text)?;
        let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let short: String = id.chars().take(12).collect();
        let name = format!("google-sheet-{short}-g{gid}");
        let dest = out_dir.join(format!("{name}.{format}"));
        match format.as_str() {
            "csv" => std::fs::write(&dest, text).map_err(|e| format!("Yazılamadı: {e}"))?,
            "md" => write_markdown(&rows, &dest, &name)?,
            _ => write_xlsx(&rows, &dest, &name)?,
        }
        Ok(SheetResultDto {
            path: dest.display().to_string(),
            name,
            rows: rows.len(),
            cols,
            format,
        })
    })
    .await
    .map_err(|e| format!("Görev yarıda kesildi: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_url_parse() {
        let (id, gid) = parse_sheet_url(
            "https://docs.google.com/spreadsheets/d/1i2-ABC_xYz/edit?gid=0#gid=0",
        )
        .unwrap();
        assert_eq!(id, "1i2-ABC_xYz");
        assert_eq!(gid, "0");
        let (id2, gid2) =
            parse_sheet_url("https://docs.google.com/spreadsheets/d/XYZ123/edit?gid=7").unwrap();
        assert_eq!((id2.as_str(), gid2.as_str()), ("XYZ123", "7"));
        assert!(parse_sheet_url("https://example.com/x").is_err());
    }

    #[test]
    fn doc_url_parse() {
        let id = parse_doc_url(
            "https://docs.google.com/document/d/1HxVf92Nx7p25Ak-JrJrZPR5Wo1DZ7RRvb4XZHt9eVXE/edit?usp=sharing",
        )
        .unwrap();
        assert_eq!(id, "1HxVf92Nx7p25Ak-JrJrZPR5Wo1DZ7RRvb4XZHt9eVXE");
        assert!(parse_doc_url("https://docs.google.com/spreadsheets/d/X/edit").is_err());
    }

    #[test]
    fn md_tablo_kurar() {
        let rows = vec![
            vec!["Ad".to_string(), "Rol".to_string()],
            vec!["Ali | Veli".to_string(), "Dev\nOps".to_string()],
        ];
        let p = std::env::temp_dir().join("mimo-md-test.md");
        write_markdown(&rows, &p, "t").unwrap();
        let t = std::fs::read_to_string(&p).unwrap();
        assert!(t.contains("| Ad | Rol |"));
        assert!(t.contains("|---|---|"));
        assert!(t.contains("Ali \\| Veli"));
        let _ = std::fs::remove_file(&p);
    }
}
