//! Base64 image decoding utilities.
//!
//! Input may be a plain Base64 string, a `data:` URI, or Base64 text with
//! arbitrary whitespace; the `data:...;base64,` prefix is stripped and the
//! whitespace is removed internally.
//!
//! PNG / JPEG / GIF / BMP are supported. The format is detected from the
//! magic number, not from a file extension or MIME declaration.

use base64::Engine;
use fltk::enums::ColorDepth;
use fltk::image::{BmpImage, GifImage, JpegImage, PngImage, RgbImage, SharedImage};
use fltk::prelude::ImageExt;

/// Supported image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Bmp,
}

/// Parse an image out of an arbitrary string and decode it into a
/// [`SharedImage`].
///
/// Supports a `data:` URI prefix, arbitrary whitespace, and both standard
/// and no-pad Base64.
///
/// # Errors
///
/// - Base64 decoding fails;
/// - The magic number does not match any supported format;
/// - The decoded pixel layout is not 3 or 4 bytes per pixel;
/// - The image dimensions are invalid.
pub fn decode_base64_image(input: &str) -> Result<SharedImage, Box<dyn std::error::Error>> {
    let bytes = decode_base64_bytes(input)?;
    let format = detect_image_format(&bytes)
        .ok_or_else(|| "unrecognized image format (magic number mismatch)".to_string())?;
    bytes_to_shared_image(&bytes, format)
}

/// Base64-decode only, stripping any `data:` URI prefix and whitespace.
///
/// Uses a fast path when there is no whitespace, avoiding a needless
/// full copy.
pub fn decode_base64_bytes(input: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let raw = input.trim();

    // Strip the data URI prefix.
    let b64 = if let Some(pos) = raw.find("base64,") {
        &raw[pos + "base64,".len()..]
    } else if raw.starts_with("data:") {
        match raw.find(',') {
            Some(p) => &raw[p + 1..],
            _ => raw,
        }
    } else {
        raw
    };

    // Borrow when there is no whitespace; otherwise clean a single copy.
    let cleaned: std::borrow::Cow<str> = if b64.bytes().any(|b| b.is_ascii_whitespace()) {
        let mut s = String::with_capacity(b64.len());
        s.extend(b64.chars().filter(|c| !c.is_whitespace()));
        std::borrow::Cow::Owned(s)
    } else {
        std::borrow::Cow::Borrowed(b64)
    };

    // Try standard first, then no-pad.
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(cleaned.as_bytes())
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(cleaned.as_bytes()))?;

    Ok(bytes)
}

/// Detect the image format from the magic number.
///
/// Returns `None` if the bytes are not PNG / JPEG / GIF / BMP.
pub fn detect_image_format(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.len() < 8 {
        return None;
    }
    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if bytes[0] == 0x89 && &bytes[1..4] == b"PNG" && bytes[4] == 0x0D {
        return Some(ImageFormat::Png);
    }
    // JPEG: FF D8 FF
    if bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some(ImageFormat::Jpeg);
    }
    // GIF: "GIF8"
    if &bytes[0..4] == b"GIF8" {
        return Some(ImageFormat::Gif);
    }
    // BMP: "BM"
    if &bytes[0..2] == b"BM" {
        return Some(ImageFormat::Bmp);
    }
    None
}

/// Decode bytes of the given format into a [`SharedImage`].
///
/// # Errors
///
/// - Image decoding fails;
/// - Dimensions are invalid (width or height ≤ 0);
/// - The pixel layout is not 3 or 4 bytes per pixel.
pub fn bytes_to_shared_image(
    bytes: &[u8],
    format: ImageFormat,
) -> Result<SharedImage, Box<dyn std::error::Error>> {
    let (data, w, h): (Vec<u8>, i32, i32) = match format {
        ImageFormat::Png => {
            let img = PngImage::from_data(bytes)?;
            (img.to_rgb_data(), img.width(), img.height())
        }
        ImageFormat::Jpeg => {
            let img = JpegImage::from_data(bytes)?;
            (img.to_rgb_data(), img.width(), img.height())
        }
        ImageFormat::Gif => {
            let img = GifImage::from_data(bytes)?;
            (img.to_rgb_data(), img.width(), img.height())
        }
        ImageFormat::Bmp => {
            let img = BmpImage::from_data(bytes)?;
            (img.to_rgb_data(), img.width(), img.height())
        }
    };

    if w <= 0 || h <= 0 {
        return Err(format!("invalid image dimensions: {w}x{h}").into());
    }

    let pixel_count = (w as usize) * (h as usize);
    let bpp = data.len() / pixel_count;

    // Debug info: see the actual layout from the console when a large image
    // fails.
    eprintln!(
        "[splash-page] format={:?}, w={}, h={}, data.len()={}, bpp={}",
        format,
        w,
        h,
        data.len(),
        bpp
    );

    // Consume `data` to avoid a full to_vec() when bpp == 3.
    let rgb = to_rgb_owned(data, pixel_count, bpp)?;

    // RgbImage::new requires the length to be exactly w*h*3.
    let expected = pixel_count * 3;
    if rgb.len() != expected {
        return Err(format!(
            "RGB data length mismatch: got {}, expected {} (w={}, h={}, bpp={})",
            rgb.len(),
            expected,
            w,
            h,
            bpp
        )
        .into());
    }

    let rgb_img = RgbImage::new(&rgb, w, h, ColorDepth::Rgb8)?;
    let shared = SharedImage::from_image(&rgb_img)?;
    Ok(shared)
}

/// Normalize the output of `to_rgb_data()` into RGB (3 bytes per pixel).
///
/// The layout returned by FLTK's `to_rgb_data()` depends on the source
/// image:
/// - images without alpha may return 3 bytes per pixel;
/// - images with alpha return 4 bytes per pixel.
///
/// This function **consumes** `data`: for bpp == 3 it truncates in place and
/// returns it with zero copies; for bpp == 4 it builds a new RGB vector and
/// lets the original `data` drop immediately, so two large buffers never
/// coexist.
fn to_rgb_owned(
    mut data: Vec<u8>,
    pixel_count: usize,
    bpp: usize,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match bpp {
        4 => {
            let mut rgb = Vec::with_capacity(pixel_count * 3);
            for chunk in data.chunks_exact(4).take(pixel_count) {
                rgb.push(chunk[0]);
                rgb.push(chunk[1]);
                rgb.push(chunk[2]);
            }
            Ok(rgb)
        }
        3 => {
            data.truncate(pixel_count * 3);
            Ok(data)
        }
        other => Err(format!(
            "unsupported pixel layout: bpp={} (data.len()={}, pixel_count={})",
            other,
            data.len(),
            pixel_count
        )
        .into()),
    }
}
