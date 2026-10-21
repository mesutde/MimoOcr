//! XLSX (OOXML) icin hizli, akis tabanli (streaming) metin cikarici.
//!
//! openpyxl `read_only` kipinde bazi dosyalarda `<dimension ref="A1:XFD1048576">`
//! yuzunden tum sayfayi (1M satir x 16K sutun) dolasmaya calisiyor; bu modul
//! yalnizca gercek `<row>/<c>` dugumlerini okur. Sayfa adlari (`## ad`),
//! TAB ayraclari ve tarih hucrelerinin okunabilir yazimi korunur.

use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use zip::ZipArchive;

/// `.xlsx` dosyasini akis halinde okuyup duz metin dondurur.
pub fn extract_xlsx_text(path: &str) -> Result<String, String> {
    let file = File::open(path).map_err(|e| format!("xlsx: {e}"))?;
    let mut zip =
        ZipArchive::new(BufReader::new(file)).map_err(|e| format!("xlsx zip: {e}"))?;

    let workbook = read_entry(&mut zip, "xl/workbook.xml")
        .map_err(|e| format!("xlsx: workbook: {e}"))?;
    let rels = read_entry(&mut zip, "xl/_rels/workbook.xml.rels").unwrap_or_default();
    let rel_targets = parse_rels(&rels);
    let (sheets, date1904) = parse_workbook(&workbook, &rel_targets);
    if sheets.is_empty() {
        return Err("xlsx: okunabilir sayfa bulunamadi".into());
    }

    let shared = read_entry(&mut zip, "xl/sharedStrings.xml")
        .map(|b| parse_shared_strings(&b))
        .unwrap_or_default();
    let date_xf = read_entry(&mut zip, "xl/styles.xml")
        .map(|b| parse_styles(&b))
        .unwrap_or_default();

    let mut out = String::new();
    // Cikti boyutu kabaca dosya boyutunun 1-2 kati olur; buyuk dosyalarda
    // yeniden tahsisleri azaltir.
    if let Ok(meta) = std::fs::metadata(path) {
        out.reserve(meta.len() as usize / 2);
    }

    for (name, target) in &sheets {
        let entry = zip
            .by_name(target)
            .map_err(|e| format!("xlsx: {target}: {e}"))?;
        out.push_str("## ");
        out.push_str(name);
        out.push('\n');
        stream_sheet(entry, name, &shared, &date_xf, date1904, &mut out)?;
    }

    if out.trim().is_empty() {
        return Err("xlsx: bos calisma kitabi".into());
    }
    while out.ends_with('\n') {
        out.pop();
    }
    Ok(out)
}

fn read_entry<R: Read + std::io::Seek>(
    zip: &mut ZipArchive<R>,
    name: &str,
) -> Result<Vec<u8>, String> {
    let mut f = zip.by_name(name).map_err(|e| format!("{name}: {e}"))?;
    let mut buf = Vec::with_capacity(f.size() as usize);
    f.read_to_end(&mut buf).map_err(|e| format!("{name}: {e}"))?;
    Ok(buf)
}

fn attr_value(a: &quick_xml::events::attributes::Attribute) -> String {
    let raw = String::from_utf8_lossy(&a.value);
    quick_xml::escape::unescape(&raw)
        .map(|v| v.into_owned())
        .unwrap_or_else(|_| raw.into_owned())
}

fn push_text(dst: &mut String, e: &quick_xml::events::BytesText) {
    match e.unescape() {
        Ok(v) => dst.push_str(&v),
        Err(_) => dst.push_str(&String::from_utf8_lossy(e)),
    }
}

/// Hedefi zip icindeki tam yola cevirir (`xl/worksheets/sheet1.xml`).
fn normalize_target(target: &str) -> String {
    let t = target.trim().trim_start_matches('/');
    if t.starts_with("xl/") {
        t.to_string()
    } else {
        format!("xl/{t}")
    }
}

/// `xl/_rels/workbook.xml.rels`: rId -> zip yolu.
fn parse_rels(bytes: &[u8]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if bytes.is_empty() {
        return map;
    }
    let mut rd = Reader::from_reader(bytes);
    rd.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match rd.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.local_name().as_ref() == b"Relationship" {
                    let mut id = None;
                    let mut target = None;
                    for a in e.attributes().flatten() {
                        match a.key.as_ref() {
                            b"Id" => id = Some(attr_value(&a)),
                            b"Target" => target = Some(attr_value(&a)),
                            _ => {}
                        }
                    }
                    if let (Some(id), Some(target)) = (id, target) {
                        map.insert(id, normalize_target(&target));
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    map
}

/// Sayfa listesi (ad, zip yolu) ve 1904 tarih sistemi bayragi.
fn parse_workbook(
    bytes: &[u8],
    rels: &HashMap<String, String>,
) -> (Vec<(String, String)>, bool) {
    let mut sheets = Vec::new();
    let mut date1904 = false;
    let mut rd = Reader::from_reader(bytes);
    rd.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match rd.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                match e.local_name().as_ref() {
                    b"sheet" => {
                        let mut name = String::new();
                        let mut rid = String::new();
                        for a in e.attributes().flatten() {
                            match a.key.as_ref() {
                                b"name" => name = attr_value(&a),
                                // Namespace'li olabilir: r:id
                                k if k == b"r:id" || k.ends_with(b":id") => {
                                    rid = attr_value(&a)
                                }
                                _ => {}
                            }
                        }
                        if let Some(target) = rels.get(&rid) {
                            // Grafik sayfalari (chartsheet) metin icermez.
                            if target.contains("/worksheets/") {
                                sheets.push((name, target.clone()));
                            }
                        }
                    }
                    b"workbookPr" => {
                        for a in e.attributes().flatten() {
                            if a.key.as_ref() == b"date1904" {
                                let v = attr_value(&a);
                                date1904 = v == "1" || v.eq_ignore_ascii_case("true");
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    (sheets, date1904)
}

/// `xl/sharedStrings.xml` -> sirali metinler (`<si>` basina bir oge).
fn parse_shared_strings(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut rd = Reader::from_reader(bytes);
    rd.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut in_si = false;
    let mut in_t = false;
    let mut in_rph = false;
    let mut cur = String::new();
    loop {
        match rd.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"si" => {
                    in_si = true;
                    cur.clear();
                }
                b"t" => in_t = true,
                b"rPh" => in_rph = true,
                _ => {}
            },
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                b"si" => {
                    if in_si {
                        out.push(std::mem::take(&mut cur));
                    }
                    in_si = false;
                }
                b"t" => in_t = false,
                b"rPh" => in_rph = false,
                _ => {}
            },
            Ok(Event::Text(e)) => {
                if in_si && in_t && !in_rph {
                    push_text(&mut cur, &e);
                }
            }
            Ok(Event::Empty(e)) => {
                // <t/> bos metin.
                if in_si && e.local_name().as_ref() == b"t" && !in_rph {
                    // bos parca, yapacak bir sey yok
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    out
}

/// `xl/styles.xml` -> her hucre stili (cellXfs sirasi) tarih mi?
fn parse_styles(bytes: &[u8]) -> Vec<bool> {
    let mut custom: HashMap<u32, String> = HashMap::new();
    let mut xf_ids: Vec<u32> = Vec::new();
    let mut in_cell_xfs = false;
    let mut rd = Reader::from_reader(bytes);
    rd.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match rd.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"cellXfs" => in_cell_xfs = true,
                b"xf" if in_cell_xfs => {
                    let mut id = 0u32;
                    for a in e.attributes().flatten() {
                        if a.key.as_ref() == b"numFmtId" {
                            id = attr_value(&a).parse().unwrap_or(0);
                        }
                    }
                    xf_ids.push(id);
                }
                b"numFmt" => {
                    let mut id = 0u32;
                    let mut code = String::new();
                    for a in e.attributes().flatten() {
                        match a.key.as_ref() {
                            b"numFmtId" => id = attr_value(&a).parse().unwrap_or(0),
                            b"formatCode" => code = attr_value(&a),
                            _ => {}
                        }
                    }
                    if id > 0 {
                        custom.insert(id, code);
                    }
                }
                _ => {}
            },
            Ok(Event::Empty(e)) => match e.local_name().as_ref() {
                b"xf" if in_cell_xfs => {
                    let mut id = 0u32;
                    for a in e.attributes().flatten() {
                        if a.key.as_ref() == b"numFmtId" {
                            id = attr_value(&a).parse().unwrap_or(0);
                        }
                    }
                    xf_ids.push(id);
                }
                b"numFmt" => {
                    let mut id = 0u32;
                    let mut code = String::new();
                    for a in e.attributes().flatten() {
                        match a.key.as_ref() {
                            b"numFmtId" => id = attr_value(&a).parse().unwrap_or(0),
                            b"formatCode" => code = attr_value(&a),
                            _ => {}
                        }
                    }
                    if id > 0 {
                        custom.insert(id, code);
                    }
                }
                _ => {}
            },
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"cellXfs" {
                    in_cell_xfs = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    xf_ids
        .into_iter()
        .map(|id| is_date_fmt(id, &custom))
        .collect()
}

fn is_date_fmt(id: u32, custom: &HashMap<u32, String>) -> bool {
    // Yerlesik tarih/saat formatlari (Excel belgeleri).
    if matches!(id, 14..=22 | 27..=36 | 45..=47 | 50..=58) {
        return true;
    }
    match custom.get(&id) {
        Some(code) => fmt_has_date(code),
        None => false,
    }
}

/// Ozel sayi formatinda tarih/saat belirteci var mi? Tirnak, koseli parantez
/// (renk/kosul) ve kacis karakterleri yok sayilir.
fn fmt_has_date(code: &str) -> bool {
    let mut clean = String::new();
    let mut in_quote = false;
    let mut in_bracket = false;
    let mut escape = false;
    for ch in code.chars() {
        if escape {
            escape = false;
            continue;
        }
        match ch {
            '\\' => escape = true,
            '"' => in_quote = !in_quote,
            '[' if !in_quote => in_bracket = true,
            ']' if !in_quote => in_bracket = false,
            _ if in_quote || in_bracket => {}
            _ => clean.push(ch.to_ascii_lowercase()),
        }
    }
    if clean.contains("general") {
        return false;
    }
    clean
        .chars()
        .any(|c| matches!(c, 'y' | 'd' | 'h' | 's'))
        || clean.contains('m')
}

fn stream_sheet<R: Read>(
    reader: R,
    _name: &str,
    shared: &[String],
    date_xf: &[bool],
    date1904: bool,
    out: &mut String,
) -> Result<(), String> {
    let mut rd = Reader::from_reader(BufReader::new(reader));
    rd.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut cells: Vec<(usize, String)> = Vec::new();
    let mut cell_buf = String::new();
    let mut col = 0usize;
    let mut next_col = 0usize;
    let mut style = 0usize;
    let mut typ = [0u8; 4];
    let mut typ_len = 0usize;
    let mut in_v = false;
    let mut in_t = false;
    let mut in_rph = false;
    let mut line = String::new();

    loop {
        let ev = match rd.read_event_into(&mut buf) {
            Ok(ev) => ev,
            Err(e) => return Err(format!("xlsx xml: {e}")),
        };
        match ev {
            Event::Start(ref e) | Event::Empty(ref e) => {
                match e.local_name().as_ref() {
                    b"row" if matches!(ev, Event::Start(_)) => {
                        cells.clear();
                        next_col = 0;
                    }
                    b"row" => {}
                    b"c" => {
                        typ_len = 0;
                        style = 0;
                        col = next_col;
                        for a in e.attributes().flatten() {
                            match a.key.as_ref() {
                                b"r" => {
                                    let v = attr_value(&a);
                                    if let Some(c) = col_from_ref(&v) {
                                        col = c;
                                    }
                                }
                                b"s" => {
                                    style = attr_value(&a).parse().unwrap_or(0);
                                }
                                b"t" => {
                                    let v = attr_value(&a);
                                    let b = v.as_bytes();
                                    let n = b.len().min(typ.len());
                                    typ[..n].copy_from_slice(&b[..n]);
                                    typ_len = n;
                                }
                                _ => {}
                            }
                        }
                        cell_buf.clear();
                        in_v = false;
                        in_t = false;
                        in_rph = false;
                        if matches!(ev, Event::Empty(_)) {
                            next_col = col + 1;
                        }
                    }
                    b"v" if matches!(ev, Event::Start(_)) => in_v = true,
                    b"t" if matches!(ev, Event::Start(_)) => in_t = true,
                    b"rPh" if matches!(ev, Event::Start(_)) => in_rph = true,
                    _ => {}
                }
            }
            Event::End(ref e) => match e.local_name().as_ref() {
                b"v" => in_v = false,
                b"t" => in_t = false,
                b"rPh" => in_rph = false,
                b"c" => {
                    next_col = col + 1;
                    let raw = cell_buf.trim();
                    let t = &typ[..typ_len];
                    let mut value: String = if t == b"s" {
                        match raw.parse::<usize>() {
                            Ok(i) => shared.get(i).cloned().unwrap_or_default(),
                            Err(_) => String::new(),
                        }
                    } else if t == b"b" {
                        if raw == "1" {
                            "True".into()
                        } else if raw == "0" {
                            "False".into()
                        } else {
                            String::new()
                        }
                    } else {
                        // openpyxl sayilari float'a cevirip en kisa gosterimi
                        // yazar (73868.800000000003 -> 73868.8); tam sayilar
                        // ise hassasiyet kaybetmeden oldugu gibi kalir.
                        if (t.is_empty() || t == b"n") && !raw.is_empty() {
                            if raw.bytes().any(|b| b == b'.' || b == b'e' || b == b'E') {
                                raw.parse::<f64>()
                                    .map(|n| n.to_string())
                                    .unwrap_or_else(|_| raw.to_string())
                            } else {
                                raw.to_string()
                            }
                        } else {
                            raw.to_string()
                        }
                    };
                    if value.trim().is_empty() {
                        value.clear();
                    } else if t.is_empty() || t == b"n" {
                        if let Some(true) = date_xf.get(style) {
                            if let Ok(num) = value.parse::<f64>() {
                                value = format_serial(num, date1904);
                            }
                        }
                    }
                    if !value.is_empty() {
                        cells.push((col, value));
                    }
                    cell_buf.clear();
                    in_v = false;
                    in_t = false;
                }
                b"row" => {
                    if !cells.is_empty() {
                        cells.sort_by_key(|(c, _)| *c);
                        build_row(&cells, &mut line);
                        out.push_str(&line);
                        out.push('\n');
                        line.clear();
                    }
                }
                _ => {}
            },
            Event::Text(ref e) => {
                if in_v || (in_t && !in_rph) {
                    push_text(&mut cell_buf, e);
                }
            }
            Event::CData(ref e) => {
                if in_v || (in_t && !in_rph) {
                    cell_buf.push_str(&String::from_utf8_lossy(e));
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

fn build_row(cells: &[(usize, String)], line: &mut String) {
    // Hucreler TAB ile ayrilir; arada bos sutun varsa ek TAB eklenir.
    // Asiri seyrek satirlarda (ornegin XFD'de tek deger) devasa bosluk
    // uretmemek icin en fazla 64 bos sutun yazilir.
    let mut expect = 0usize;
    for (i, (col, value)) in cells.iter().enumerate() {
        if i > 0 {
            line.push('\t');
        }
        let gap = col.saturating_sub(expect).min(64);
        for _ in 0..gap {
            line.push('\t');
        }
        line.push_str(value);
        expect = (*col + 1).max(expect);
    }
}

/// `B12` -> 1 (0 tabanli sutun). Harf disi baslar ise None.
fn col_from_ref(r: &str) -> Option<usize> {
    let mut n: usize = 0;
    let mut any = false;
    for b in r.bytes() {
        if b.is_ascii_alphabetic() {
            any = true;
            n = n * 26 + (b.to_ascii_uppercase() - b'A' + 1) as usize;
        } else {
            break;
        }
    }
    if any {
        Some(n.saturating_sub(1))
    } else {
        None
    }
}

/// Excel seri numarasini okunabilir tarih/saat metnine cevirir (openpyxl
/// `str(datetime)` ile ayni bicim: `YYYY-MM-DD HH:MM:SS`).
fn format_serial(v: f64, date1904: bool) -> String {
    if !v.is_finite() || v < 0.0 {
        return trim_num(v);
    }
    let mut days = v.floor() as i64;
    let frac = v - v.floor();
    let mut secs = (frac * 86400.0).round() as i64;
    if secs >= 86400 {
        secs -= 86400;
        days += 1;
    }
    let h = secs / 3600;
    let mi = (secs % 3600) / 60;
    let s = secs % 60;
    if days == 0 && frac > 0.0 {
        return format!("{h:02}:{mi:02}:{s:02}");
    }
    // 1900 tarih sisteminde Excel'in sahte 29 Subat 1900'u (seri 60)
    // nedeniyle 60 oncesi gunlerde ofset bir gun kayar.
    let offset = if date1904 {
        24107
    } else if days < 60 {
        25568
    } else {
        25569
    };
    let unix_days = days - offset;
    let (y, m, d) = civil_from_days(unix_days);
    if y < 1 || y > 9999 {
        return trim_num(v);
    }
    format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

fn trim_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// Howard Hinnant'in `civil_from_days` algoritmasi (gun 0 = 1970-01-01).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::{col_from_ref, format_serial};

    #[test]
    fn col_refs() {
        assert_eq!(col_from_ref("A1"), Some(0));
        assert_eq!(col_from_ref("B12"), Some(1));
        assert_eq!(col_from_ref("AA3"), Some(26));
        assert_eq!(col_from_ref("XFD1"), Some(16383));
        assert_eq!(col_from_ref("1"), None);
    }

    #[test]
    fn serials() {
        assert_eq!(format_serial(42865.0, false), "2017-05-10 00:00:00");
        assert_eq!(format_serial(25569.0, false), "1970-01-01 00:00:00");
        assert_eq!(format_serial(0.5, false), "12:00:00");
        assert_eq!(format_serial(1.0, false), "1900-01-01 00:00:00");
    }
}
