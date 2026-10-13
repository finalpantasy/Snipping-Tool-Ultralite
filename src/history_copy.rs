//! Bounded original-resolution History decoding. Clipboard publication is separate.
use image::ImageDecoder;
use std::path::Path;
const MAX_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ENCODED_BYTES: u64 = 256 * 1024 * 1024;

pub fn decode(path: &Path) -> Result<image::RgbaImage, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_ENCODED_BYTES {
        return Err("History image file is too large to copy safely".into());
    }
    let mut reader = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(65_536);
    limits.max_image_height = Some(65_536);
    limits.max_alloc = Some(MAX_BYTES);
    reader.limits(limits);
    let decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let (width, height) = decoder.dimensions();
    let rgba_bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|n| n.checked_mul(4));
    if width == 0
        || height == 0
        || rgba_bytes.is_none_or(|n| n > MAX_BYTES)
        || decoder.total_bytes() > MAX_BYTES
    {
        return Err("History image exceeds the 128 MiB decoded-image limit".into());
    }
    let image = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    Ok(consume(image))
}

fn consume(image: image::DynamicImage) -> image::RgbaImage {
    // The usual capture PNG is already RGBA8; keep its allocation, never clone it.
    image.into_rgba8()
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            Self(std::env::temp_dir().join(format!(
                "ul-history-copy-{}-{}.png",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            )))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    #[test]
    fn file_decode_keeps_capture_pixels_and_rejects_corruption() {
        let file = Fixture::new();
        let pixels = image::RgbaImage::from_fn(31, 19, |x, y| {
            image::Rgba([x as u8, y as u8, 90, (x + y) as u8])
        });
        pixels.save(&file.0).unwrap();
        assert_eq!(decode(&file.0).unwrap(), pixels);
        std::fs::write(&file.0, b"broken PNG").unwrap();
        assert!(decode(&file.0).is_err());
    }
    #[test]
    fn huge_png_header_is_rejected_before_a_full_image_is_allocated() {
        let file = Fixture::new();
        // Only an authored PNG header, no enormous pixel allocation or file.
        let mut encoder =
            png::Encoder::new(std::fs::File::create(&file.0).unwrap(), 65_536, 65_536);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_chunk(png::chunk::IDAT, &[]).unwrap();
        drop(writer);
        assert!(decode(&file.0).is_err());
    }
    #[test]
    fn rgba_storage_is_transferred_without_another_full_image_allocation() {
        let pixels = image::RgbaImage::from_pixel(64, 32, image::Rgba([17, 29, 255, 128]));
        let address = pixels.as_raw().as_ptr();
        let output = consume(image::DynamicImage::ImageRgba8(pixels));
        assert_eq!(output.as_raw().as_ptr(), address);
        assert_eq!(output.get_pixel(20, 10).0, [17, 29, 255, 128]);
    }
    #[test]
    fn grayscale_conversion_preserves_original_resolution_and_alpha() {
        let image = image::GrayAlphaImage::from_pixel(31, 19, image::LumaA([80, 127]));
        let result = consume(image::DynamicImage::ImageLumaA8(image));
        assert_eq!(result.dimensions(), (31, 19));
        assert_eq!(result.get_pixel(5, 6).0, [80, 80, 80, 127]);
    }
}
