//! Ekran / dosya / pano kaynaklarÄ±ndan gÃ¶rÃ¼ntÃ¼ edinme ve Ã¶n iÅŸleme.

use std::io::Cursor;

use image::{imageops, DynamicImage, ImageFormat, RgbaImage};

use crate::engine::OcrError;

/// Overlay secimini yakalar — SAF FIZIKSEL matematik.
///
/// Fare CSS pikseli, overlay penceresinin GERCEK konumu (`outer_position`,
/// fiziksel) ve olcegiyle fiziksele cevrilir:
/// `vx = ov_ox + x * ov_sf`. Kesisim xcap monitor dikdortgenleriyle
/// (ham degerler, fiziksel) yapilir; xcap `capture_image()` kendi
/// (x, y, w, h) degerinden BitBlt yaptigi icin bitmap pikseli birebir
/// ayni uzaydadir — bolme/carpma yok, koken varsayimi yok.
///
/// sf=1 iken eski matematige birebir indirgenir (tek monitorde davranis ayni).
/// Karisik DPI'da her monitor parcasi kendi fiziksel yogunluguyla
/// yerlesir (ekran fotografi gibi); olcek farki regularizasyonu gerekmez.
pub fn capture_region(
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    ov_ox: f64,
    ov_oy: f64,
    ov_sf: f64,
) -> Result<Vec<u8>, OcrError> {
    let monitors = xcap::Monitor::all().map_err(|e| OcrError::Image(e.to_string()))?;
    if monitors.is_empty() {
        return Err(OcrError::Image("Monitör bulunamadı".into()));
    }
    let sf = ov_sf.max(f64::EPSILON);
    let vx = ov_ox + x * sf;
    let vy = ov_oy + y * sf;
    let vw = (w * sf).max(1.0);
    let vh = (h * sf).max(1.0);

    let mut canvas = RgbaImage::new(vw.round().max(1.0) as u32, vh.round().max(1.0) as u32);
    let mut hit = false;
    for m in &monitors {
        let mx = m.x().unwrap_or(0) as f64;
        let my = m.y().unwrap_or(0) as f64;
        let mw = m.width().unwrap_or(0) as f64;
        let mh = m.height().unwrap_or(0) as f64;

        let ix1 = vx.max(mx);
        let iy1 = vy.max(my);
        let ix2 = (vx + vw).min(mx + mw);
        let iy2 = (vy + vh).min(my + mh);
        if ix2 <= ix1 || iy2 <= iy1 {
            continue;
        }

        let full = m
            .capture_image()
            .map_err(|e| OcrError::Image(format!("Ekran yakalanamadı: {e}")))?;

        let px = (ix1 - mx).round().max(0.0) as u32;
        let py = (iy1 - my).round().max(0.0) as u32;
        let pw = ((ix2 - ix1).round().max(1.0) as u32).min(full.width().saturating_sub(px));
        let ph = ((iy2 - iy1).round().max(1.0) as u32).min(full.height().saturating_sub(py));
        if pw == 0 || ph == 0 {
            continue;
        }
        let piece = imageops::crop_imm(&full, px, py, pw, ph).to_image();
        let ox = (ix1 - vx).round().max(0.0) as i64;
        let oy = (iy1 - vy).round().max(0.0) as i64;
        imageops::overlay(&mut canvas, &piece, ox, oy);
        hit = true;
    }
    if !hit {
        return Err(OcrError::Image("Seçim ekran sınırları dışında".into()));
    }
    encode_png(&DynamicImage::ImageRgba8(canvas))
}


/// Belirtilen monitörün tamamını yakalar (Tam ekran OCR).
/// Sıra `list_monitors` ile aynı kaynaktan (xcap) gelir; geçersiz indeks ilk monitöre düşer.
pub fn capture_monitor(index: usize) -> Result<Vec<u8>, OcrError> {
    let monitors = xcap::Monitor::all().map_err(|e| OcrError::Image(e.to_string()))?;
    if monitors.is_empty() {
        return Err(OcrError::Image("Monitör bulunamadı".into()));
    }
    let m = monitors.get(index).or(monitors.first()).unwrap();
    let img = m
        .capture_image()
        .map_err(|e| OcrError::Image(format!("Ekran yakalanamadı: {e}")))?;
    encode_png(&DynamicImage::ImageRgba8(img))
}

/// Dosyadan görüntü okur, PNG baytı döner.
pub fn load_file(path: &str) -> Result<Vec<u8>, OcrError> {
    let img = image::open(path).map_err(|e| OcrError::Image(format!("Dosya aÃ§Ä±lamadÄ±: {e}")))?;
    encode_png(&img)
}

/// Panodaki gÃ¶rseli okur, PNG baytÄ± dÃ¶ner.
pub fn load_clipboard() -> Result<Vec<u8>, OcrError> {
    let mut cb = arboard::Clipboard::new().map_err(|e| OcrError::Image(e.to_string()))?;
    let img = cb.get_image().map_err(|_| OcrError::NoClipboardImage)?;
    let (w, h) = (img.width as u32, img.height as u32);
    let rgba = RgbaImage::from_raw(w, h, img.bytes.into_owned())
        .ok_or_else(|| OcrError::Image("Pano gÃ¶rseli Ã§Ã¶zÃ¼mlenemedi".into()))?;
    encode_png(&DynamicImage::ImageRgba8(rgba))
}

/// Ã–n iÅŸleme: isteÄŸe baÄŸlÄ± bÃ¼yÃ¼tme (kÃ¼Ã§Ã¼k ekran yazÄ±larÄ±nda doÄŸruluÄŸu artÄ±rÄ±r).
pub fn preprocess(png: &[u8], scale: u32) -> Result<Vec<u8>, OcrError> {
    if scale <= 1 {
        return Ok(png.to_vec());
    }
    let img = image::load_from_memory(png).map_err(|e| OcrError::Image(e.to_string()))?;
    let w = img.width() * scale;
    let h = img.height() * scale;
    let resized = img.resize_exact(w, h, imageops::FilterType::Lanczos3);
    encode_png(&resized)
}

fn encode_png(img: &DynamicImage) -> Result<Vec<u8>, OcrError> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Png)
        .map_err(|e| OcrError::Image(e.to_string()))?;
    Ok(buf.into_inner())
}
