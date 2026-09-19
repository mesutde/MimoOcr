//! Windows-oriented screen capture prototypes built on `xcap`.
//!
//! Phase 0 goals: enumerate monitors, capture full screen, capture a logical
//! region, and surface DPI/scale information that OCR pipelines need.
//!
//! Note: this crate currently targets `xcap` 0.2.x where monitor accessors
//! return plain values (not `Result`).

use mimo_ocr_core::{OcrError, OcrImage};

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Scale factor reported by the OS (1.0 = 100% DPI).
    pub scale_factor: f32,
    pub is_primary: bool,
}

pub fn list_monitors() -> Result<Vec<MonitorInfo>, OcrError> {
    let monitors =
        xcap::Monitor::all().map_err(|e| OcrError::Capture(format!("list monitors: {e}")))?;

    let mut out = Vec::new();
    for (index, m) in monitors.into_iter().enumerate() {
        out.push(MonitorInfo {
            index,
            name: m.name().to_string(),
            x: m.x(),
            y: m.y(),
            width: m.width(),
            height: m.height(),
            scale_factor: m.scale_factor(),
            is_primary: m.is_primary(),
        });
    }
    Ok(out)
}

fn xcap_to_ocr_image(img: xcap::image::RgbaImage, source_name: impl Into<String>) -> OcrImage {
    let (w, h) = img.dimensions();
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for px in img.pixels() {
        rgb.extend_from_slice(&[px[0], px[1], px[2]]);
    }
    OcrImage::from_rgb(w, h, rgb, source_name)
}

/// Capture the primary monitor full screen.
pub fn capture_primary() -> Result<OcrImage, OcrError> {
    capture_monitor(0)
}

/// Capture monitor by index (0 = first in xcap order).
pub fn capture_monitor(index: usize) -> Result<OcrImage, OcrError> {
    let monitors =
        xcap::Monitor::all().map_err(|e| OcrError::Capture(format!("list monitors: {e}")))?;
    let monitor = monitors
        .into_iter()
        .nth(index)
        .ok_or_else(|| OcrError::Capture(format!("monitor {index} not found")))?;
    let img = monitor
        .capture_image()
        .map_err(|e| OcrError::Capture(format!("capture monitor {index}: {e}")))?;
    Ok(xcap_to_ocr_image(img, format!("monitor-{index}")))
}

/// Capture a rectangle in monitor-relative physical pixels.
pub fn capture_region(
    monitor_index: usize,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<OcrImage, OcrError> {
    if width == 0 || height == 0 {
        return Err(OcrError::InvalidImage(
            "region width/height must be > 0".into(),
        ));
    }
    let full = capture_monitor(monitor_index)?;
    if x + width > full.width || y + height > full.height {
        return Err(OcrError::InvalidImage(format!(
            "region ({x},{y},{width}x{height}) outside monitor {}x{}",
            full.width, full.height
        )));
    }

    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for row in y..y + height {
        let start = ((row * full.width + x) * 3) as usize;
        let end = start + (width * 3) as usize;
        rgb.extend_from_slice(&full.rgb[start..end]);
    }
    Ok(OcrImage::from_rgb(
        width,
        height,
        rgb,
        format!("monitor-{monitor_index}-region"),
    ))
}

/// Logical (DIP) region → physical pixels using monitor scale factor.
pub fn capture_logical_region(
    monitor_index: usize,
    logical_x: u32,
    logical_y: u32,
    logical_w: u32,
    logical_h: u32,
) -> Result<OcrImage, OcrError> {
    let monitors = list_monitors()?;
    let mon = monitors
        .iter()
        .find(|m| m.index == monitor_index)
        .ok_or_else(|| OcrError::Capture(format!("monitor {monitor_index} not found")))?;
    let scale = mon.scale_factor.max(0.5);
    let x = (logical_x as f32 * scale).round() as u32;
    let y = (logical_y as f32 * scale).round() as u32;
    let w = (logical_w as f32 * scale).round() as u32;
    let h = (logical_h as f32 * scale).round() as u32;
    capture_region(monitor_index, x, y, w.max(1), h.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_monitors_returns_entries_on_windows() {
        match list_monitors() {
            Ok(ms) => {
                assert!(!ms.is_empty(), "expected at least one monitor");
                assert!(ms[0].width > 0);
            }
            Err(e) => {
                eprintln!("capture unavailable in this environment: {e}");
            }
        }
    }
}
