//! Lightweight image preprocessing for screen/scan OCR quality.

use crate::OcrImage;

#[derive(Debug, Clone, Copy)]
pub struct PreprocessOptions {
    /// Scale factor applied when the image is small or for accurate mode.
    pub upscale: f32,
    /// Convert to grayscale before OCR.
    pub grayscale: bool,
    /// Simple local contrast boost (0 = off, 1..3 reasonable).
    pub contrast: f32,
    /// Unsharp-like sharpening amount (0 = off).
    pub sharpen: f32,
}

impl Default for PreprocessOptions {
    fn default() -> Self {
        Self {
            upscale: 1.0,
            grayscale: true,
            contrast: 1.15,
            sharpen: 0.2,
        }
    }
}

impl PreprocessOptions {
    /// Fast path for interactive screen OCR.
    pub fn fast() -> Self {
        Self {
            upscale: 1.0,
            grayscale: true,
            contrast: 1.1,
            sharpen: 0.15,
        }
    }

    /// Stronger path for difficult screenshots / small text / TR diacritics.
    pub fn accurate() -> Self {
        Self {
            upscale: 2.0,
            grayscale: true,
            contrast: 1.25,
            sharpen: 0.35,
        }
    }

    pub fn for_quality(quality: crate::QualityMode) -> Self {
        match quality {
            crate::QualityMode::Fast => Self {
                upscale: 1.0,
                grayscale: true,
                contrast: 1.05,
                sharpen: 0.1,
            },
            crate::QualityMode::Balanced => Self::fast(),
            crate::QualityMode::Accurate => Self::accurate(),
        }
    }

    /// Cap upscale on large captures so full-screen OCR cannot freeze the machine.
    /// 1920×1080 @ 2× → 3840×2160 is too heavy for interactive use.
    pub fn for_quality_sized(quality: crate::QualityMode, width: u32, height: u32) -> Self {
        let mut opts = Self::for_quality(quality);
        let max_side = width.max(height);
        if max_side >= 2200 {
            opts.upscale = 1.0;
        } else if max_side >= 1400 {
            opts.upscale = opts.upscale.min(1.25);
        } else if max_side >= 900 {
            opts.upscale = opts.upscale.min(1.5);
        }
        opts
    }
}

fn rgb_to_luma(r: u8, g: u8, b: u8) -> u8 {
    // Rec. 601-ish
    ((r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000) as u8
}

fn clamp_u8(v: f32) -> u8 {
    if v < 0.0 {
        0
    } else if v > 255.0 {
        255
    } else {
        v as u8
    }
}

/// Nearest-neighbor upscale (keeps OCR boxes simple; quality is enough for screen text).
fn upscale_rgb(width: u32, height: u32, rgb: &[u8], scale: f32) -> (u32, u32, Vec<u8>) {
    if (scale - 1.0).abs() < 0.01 {
        return (width, height, rgb.to_vec());
    }
    let nw = ((width as f32) * scale).round().max(1.0) as u32;
    let nh = ((height as f32) * scale).round().max(1.0) as u32;
    let mut out = vec![0u8; (nw as usize) * (nh as usize) * 3];
    for y in 0..nh {
        let sy = ((y as f32) / scale).floor() as u32;
        let sy = sy.min(height.saturating_sub(1));
        for x in 0..nw {
            let sx = ((x as f32) / scale).floor() as u32;
            let sx = sx.min(width.saturating_sub(1));
            let si = ((sy as usize) * (width as usize) + (sx as usize)) * 3;
            let di = ((y as usize) * (nw as usize) + (x as usize)) * 3;
            out[di] = rgb[si];
            out[di + 1] = rgb[si + 1];
            out[di + 2] = rgb[si + 2];
        }
    }
    (nw, nh, out)
}

fn grayscale_rgb(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let n = (width as usize) * (height as usize);
    let mut out = Vec::with_capacity(n * 3);
    for i in 0..n {
        let r = rgb[i * 3];
        let g = rgb[i * 3 + 1];
        let b = rgb[i * 3 + 2];
        let y = rgb_to_luma(r, g, b);
        out.extend_from_slice(&[y, y, y]);
    }
    out
}

fn contrast_boost(_width: u32, _height: u32, rgb: &mut [u8], amount: f32) {
    if (amount - 1.0).abs() < 0.01 {
        return;
    }
    for px in rgb.chunks_exact_mut(3) {
        for c in px.iter_mut() {
            let v = (*c as f32 - 128.0) * amount + 128.0;
            *c = clamp_u8(v);
        }
    }
}

/// 3x3 sharpen on grayscale-ish RGB.
fn sharpen_rgb(width: u32, height: u32, rgb: &mut [u8], amount: f32) {
    if amount <= 0.01 || width < 3 || height < 3 {
        return;
    }
    let src = rgb.to_vec();
    let w = width as usize;
    let h = height as usize;
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let idx = y * w + x;
            let c = idx * 3;
            // only use red channel if grayscale copies match; use each channel independently
            for ch in 0..3 {
                let center = src[c + ch] as f32;
                let up = src[((y - 1) * w + x) * 3 + ch] as f32;
                let down = src[((y + 1) * w + x) * 3 + ch] as f32;
                let left = src[(y * w + x - 1) * 3 + ch] as f32;
                let right = src[(y * w + x + 1) * 3 + ch] as f32;
                let sharp = center + amount * (5.0 * center - up - down - left - right);
                rgb[c + ch] = clamp_u8(sharp);
            }
        }
    }
}

/// Apply preprocessing pipeline used by engines before OCR.
pub fn preprocess(img: &OcrImage, opts: &PreprocessOptions) -> OcrImage {
    let (w, h, mut rgb) = upscale_rgb(img.width, img.height, &img.rgb, opts.upscale);
    if opts.grayscale {
        rgb = grayscale_rgb(w, h, &rgb);
    }
    contrast_boost(w, h, &mut rgb, opts.contrast);
    sharpen_rgb(w, h, &mut rgb, opts.sharpen);
    OcrImage::from_rgb(w, h, rgb, format!("{}#pre", img.source_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker() -> OcrImage {
        let mut rgb = Vec::new();
        for y in 0..16u32 {
            for x in 0..16u32 {
                let v = if (x + y) % 2 == 0 { 20 } else { 230 };
                rgb.extend_from_slice(&[v, v, v]);
            }
        }
        OcrImage::from_rgb(16, 16, rgb, "checker")
    }

    #[test]
    fn upscale_doubles_dimensions() {
        let img = checker();
        let out = preprocess(
            &img,
            &PreprocessOptions {
                upscale: 2.0,
                grayscale: true,
                contrast: 1.0,
                sharpen: 0.0,
            },
        );
        assert_eq!(out.width, 32);
        assert_eq!(out.height, 32);
        assert_eq!(out.rgb.len(), 32 * 32 * 3);
    }

    #[test]
    fn grayscale_keeps_channels_equal() {
        let mut rgb = vec![0u8; 3 * 4 * 4];
        for i in 0..16 {
            rgb[i * 3] = 200;
            rgb[i * 3 + 1] = 50;
            rgb[i * 3 + 2] = 10;
        }
        let img = OcrImage::from_rgb(4, 4, rgb, "color");
        let out = preprocess(
            &img,
            &PreprocessOptions {
                upscale: 1.0,
                grayscale: true,
                contrast: 1.0,
                sharpen: 0.0,
            },
        );
        for px in out.rgb.chunks_exact(3) {
            assert_eq!(px[0], px[1]);
            assert_eq!(px[1], px[2]);
        }
    }
}
