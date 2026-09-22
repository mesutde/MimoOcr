//! Ekran / dosya / pano kaynaklarÄ±ndan gÃ¶rÃ¼ntÃ¼ edinme ve Ã¶n iÅŸleme.

use std::io::Cursor;

use image::{imageops, DynamicImage, ImageFormat, RgbaImage};

use crate::engine::OcrError;

/// Sanal masaüstünü (tüm monitörlerin birleşimini) kaplayan overlay
/// penceresinin yerel mantıksal koordinatlarıyla verilen bölgeyi yakalar.
/// Her monitörün kendi DPI ölçeği fiziksel piksel dönüşümünde hesaba katılır.
pub fn capture_region(x: f64, y: f64, w: f64, h: f64) -> Result<Vec<u8>, OcrError> {
    let monitors = xcap::Monitor::all().map_err(|e| OcrError::Image(e.to_string()))?;
    if monitors.is_empty() {
        return Err(OcrError::Image("Monitör bulunamadı".into()));
    }

    // Sanal masaüstü kökeni (en küçük mantıksal x/y); overlay penceresi burada başlar
    let origin_x = monitors.iter().map(|m| m.x().unwrap_or(0)).min().unwrap_or(0) as f64;
    let origin_y = monitors.iter().map(|m| m.y().unwrap_or(0)).min().unwrap_or(0) as f64;

    // Overlay yerel koordinatı → sanal masaüstü mantıksal koordinatı
    let vx = x + origin_x;
    let vy = y + origin_y;

    // Parçaların yerleştirileceği tuval; ilk kesişen monitörün ölçeği baz alınır
    let mut canvas: Option<RgbaImage> = None;
    let mut canvas_sf = 1.0f64;

    for m in &monitors {
        let sf = m.scale_factor().unwrap_or(1.0) as f64;
        let mx = m.x().unwrap_or(0) as f64;
        let my = m.y().unwrap_or(0) as f64;
        let mw = m.width().unwrap_or(0) as f64 / sf;
        let mh = m.height().unwrap_or(0) as f64 / sf;

        // Sanal masaüstü mantıksal uzayında kesişim
        let ix1 = vx.max(mx);
        let iy1 = vy.max(my);
        let ix2 = (vx + w).min(mx + mw);
        let iy2 = (vy + h).min(my + mh);
        if ix2 <= ix1 || iy2 <= iy1 {
            continue;
        }

        let full = m
            .capture_image()
            .map_err(|e| OcrError::Image(format!("Ekran yakalanamadı: {e}")))?;

        // Kesişimi monitör yerel fiziksel pikseline çevir
        let px = ((ix1 - mx) * sf).round().max(0.0) as u32;
        let py = ((iy1 - my) * sf).round().max(0.0) as u32;
        let pw = ((ix2 - ix1) * sf).round().max(1.0) as u32;
        let ph = ((iy2 - iy1) * sf).round().max(1.0) as u32;
        let pw = pw.min(full.width().saturating_sub(px));
        let ph = ph.min(full.height().saturating_sub(py));
        if pw == 0 || ph == 0 {
            continue;
        }
        let piece = imageops::crop_imm(&full, px, py, pw, ph).to_image();

        if canvas.is_none() {
            canvas_sf = sf;
            canvas = Some(RgbaImage::new(
                (w * sf).round().max(1.0) as u32,
                (h * sf).round().max(1.0) as u32,
            ));
        }
        if let Some(c) = canvas.as_mut() {
            // Parça farklı ölçekli monitörden geldiyse tuval ölçeğine yeniden boyutlandır
            let piece = if (sf - canvas_sf).abs() > f64::EPSILON {
                let nw = ((piece.width() as f64) * canvas_sf / sf).round().max(1.0) as u32;
                let nh = ((piece.height() as f64) * canvas_sf / sf).round().max(1.0) as u32;
                imageops::resize(&piece, nw, nh, imageops::FilterType::Lanczos3)
            } else {
                piece
            };
            let ox = ((ix1 - vx) * canvas_sf).round().max(0.0) as i64;
            let oy = ((iy1 - vy) * canvas_sf).round().max(0.0) as i64;
            imageops::overlay(c, &piece, ox, oy);
        }
    }

    let canvas = canvas.ok_or_else(|| OcrError::Image("Seçim ekran sınırları dışında".into()))?;
    encode_png(&DynamicImage::ImageRgba8(canvas))
}


/// Dosyadan gÃ¶rÃ¼ntÃ¼ okur, PNG baytÄ± dÃ¶ner.
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
