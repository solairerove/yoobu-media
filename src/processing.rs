// Rust key concept: ownership.
// process_image takes &[u8] — a borrowed slice, no data is copied.
// It returns Vec<u8> — a new owned buffer with the result.
// The compiler guarantees the source bytes cannot be modified while we are reading them.

use image::{imageops::FilterType, DynamicImage};

use crate::error::AppError;

/// Supported input formats.
#[derive(Debug, PartialEq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    WebP,
}

/// Detects the image format from magic bytes — does not trust the Content-Type header.
///
/// JPEG: FF D8 FF
/// PNG:  89 50 4E 47 0D 0A 1A 0A
/// WebP: RIFF....WEBP
pub fn detect_format(bytes: &[u8]) -> Result<ImageFormat, AppError> {
    if bytes.len() < 12 {
        return Err(AppError::BadRequest("File is too small to be a valid image".into()));
    }

    if bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Ok(ImageFormat::Jpeg);
    }

    if bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        return Ok(ImageFormat::Png);
    }

    if &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Ok(ImageFormat::WebP);
    }

    Err(AppError::BadRequest(
        "Unsupported format. Allowed: jpeg, png, webp".into(),
    ))
}

/// Full pipeline: decode → resize → encode to WebP.
pub fn process_image(bytes: &[u8], max_dimension: u32, quality: f32) -> Result<Vec<u8>, AppError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| AppError::BadRequest(format!("Cannot decode image: {e}")))?;

    let img = resize_if_needed(img, max_dimension);

    encode_to_webp(&img, quality)
}

// If the image already fits within max_dimension, leave it untouched.
// Otherwise scale it down preserving aspect ratio (resize fits into a max×max box).
fn resize_if_needed(img: DynamicImage, max_dim: u32) -> DynamicImage {
    if img.width() <= max_dim && img.height() <= max_dim {
        return img;
    }
    img.resize(max_dim, max_dim, FilterType::Lanczos3)
}

fn encode_to_webp(img: &DynamicImage, quality: f32) -> Result<Vec<u8>, AppError> {
    let encoder = webp::Encoder::from_image(img)
        .map_err(|e| AppError::ProcessingError(format!("WebP encoder init: {e}")))?;

    // encode(quality) — lossy, range 0.0..=100.0
    // encode_lossless() — lossless, larger file size
    let webp_data = encoder.encode(quality);

    Ok(webp_data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal valid JPEG header bytes
    const MINIMAL_JPEG: &[u8] = &[
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00,
        0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06,
        0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D,
        0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D,
        0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28,
        0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
        0x3C, 0x2E, 0x33, 0x34, 0x32,
    ];

    const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
    const WEBP_MAGIC: &[u8] = &[
        0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50,
    ];

    #[test]
    fn detects_jpeg() {
        assert_eq!(detect_format(MINIMAL_JPEG).unwrap(), ImageFormat::Jpeg);
    }

    #[test]
    fn detects_png() {
        assert_eq!(detect_format(PNG_MAGIC).unwrap(), ImageFormat::Png);
    }

    #[test]
    fn detects_webp() {
        assert_eq!(detect_format(WEBP_MAGIC).unwrap(), ImageFormat::WebP);
    }

    #[test]
    fn rejects_unknown_format() {
        let garbage = &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B];
        assert!(detect_format(garbage).is_err());
    }

    #[test]
    fn rejects_too_small() {
        assert!(detect_format(&[0xFF, 0xD8]).is_err());
    }

    #[test]
    fn resize_not_triggered_when_fits() {
        // 10×10 fits within 1200×1200 — resize must not fire
        let img = DynamicImage::new_rgb8(10, 10);
        let resized = resize_if_needed(img, 1200);
        assert_eq!(resized.width(), 10);
        assert_eq!(resized.height(), 10);
    }

    #[test]
    fn resize_scales_down_wide_image() {
        // 2400×600 with max_dim=1200: longest side = 2400, scale = 0.5 → 1200×300
        let img = DynamicImage::new_rgb8(2400, 600);
        let resized = resize_if_needed(img, 1200);
        assert!(resized.width() <= 1200);
        assert!(resized.height() <= 1200);
    }

    #[test]
    fn process_image_produces_webp_output() {
        // Encode a real PNG in-memory and run it through the full pipeline
        let img = DynamicImage::new_rgb8(100, 100);
        let mut png_bytes: Vec<u8> = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .unwrap();

        let result = process_image(&png_bytes, 1200, 80.0).unwrap();

        // WebP files start with "RIFF....WEBP"
        assert_eq!(&result[0..4], b"RIFF");
        assert_eq!(&result[8..12], b"WEBP");
        assert!(!result.is_empty());
    }
}
