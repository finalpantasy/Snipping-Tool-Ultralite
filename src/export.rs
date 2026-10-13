use crate::geometry::{Point, Size};
use crate::tools::{render_annotation, Annotation};
use tiny_skia::{Pixmap, PremultipliedColorU8};

/// Flatten annotations onto a pixel buffer.
///
/// Input and output are straight (non-premultiplied) RGBA8 pixels.
/// Internally converts to premultiplied alpha for tiny-skia rendering,
/// then converts back to straight alpha for the output.
pub fn flatten_annotations(
    pixels: &[u8],
    width: u32,
    height: u32,
    annotations: &[Annotation],
) -> Vec<u8> {
    if annotations.is_empty() {
        return pixels.to_vec();
    }
    let mut pixmap = match Pixmap::new(width, height) {
        Some(p) => p,
        None => return pixels.to_vec(), // zero-dimension: return input unchanged
    };

    // Copy source pixels into pixmap WITH premultiplication
    let expected_len = (width as usize) * (height as usize) * 4;
    if pixels.len() < expected_len {
        return pixels.to_vec(); // undersized buffer: return input unchanged
    }

    // Fast path: screenshots are fully opaque (a=255), so premultiplication is a
    // no-op — we can copy components directly instead of multiplying per pixel.
    let all_opaque = pixels.chunks_exact(4).all(|px| px[3] == 255);
    let pm_pixels = pixmap.pixels_mut();
    if all_opaque {
        for (i, chunk) in pixels.chunks_exact(4).enumerate() {
            pm_pixels[i] = PremultipliedColorU8::from_rgba(chunk[0], chunk[1], chunk[2], 255)
                .unwrap_or(PremultipliedColorU8::TRANSPARENT);
        }
    } else {
        // Straight RGBA in — premultiply each component. The multiply guarantees
        // component <= alpha, so from_rgba cannot fail here.
        for (i, chunk) in pixels.chunks_exact(4).enumerate() {
            let a = chunk[3] as u16;
            let pm = |c: u8| ((c as u16 * a + 127) / 255) as u8;
            pm_pixels[i] =
                PremultipliedColorU8::from_rgba(pm(chunk[0]), pm(chunk[1]), pm(chunk[2]), chunk[3])
                    .unwrap_or(PremultipliedColorU8::TRANSPARENT);
        }
    }

    // Render each annotation
    for annotation in annotations {
        render_annotation(annotation, &mut pixmap, Some(pixels), Some(width));
    }

    // Demultiply back to straight alpha in place; opaque pixels are already
    // straight, so screenshots skip the division and a second full buffer.
    let mut output = pixmap.take();
    for px in output.chunks_exact_mut(4) {
        let a = px[3] as u16;
        if a == 0 {
            px[..3].fill(0);
        } else if a < 255 {
            for c in &mut px[..3] {
                *c = (*c as u16 * 255 / a) as u8;
            }
        }
    }

    output
}

/// Crop a screenshot to a selection region, offset annotations, and flatten.
///
/// `screenshot_pixels` is straight RGBA8 for the full screenshot.
/// Annotations are in screenshot-absolute coordinates and will be offset
/// by (-sel_x, -sel_y) to become selection-relative.
pub fn crop_and_flatten(
    screenshot_pixels: &[u8],
    screenshot_width: u32,
    sel_x: u32,
    sel_y: u32,
    sel_w: u32,
    sel_h: u32,
    annotations: &[Annotation],
) -> Vec<u8> {
    // Clamp selection to screenshot bounds to prevent out-of-bounds access
    let screenshot_height = screenshot_pixels.len() as u32 / (screenshot_width * 4).max(1);
    let clamped_w = sel_w.min(screenshot_width.saturating_sub(sel_x));
    let clamped_h = sel_h.min(screenshot_height.saturating_sub(sel_y));

    // Crop the pixel buffer
    let mut cropped = Vec::with_capacity((clamped_w as usize) * (clamped_h as usize) * 4);
    for row in sel_y..(sel_y + clamped_h) {
        let start = ((row * screenshot_width + sel_x) * 4) as usize;
        let end = start + (clamped_w * 4) as usize;
        if end <= screenshot_pixels.len() {
            cropped.extend_from_slice(&screenshot_pixels[start..end]);
        }
    }

    // Nothing to draw: the crop is already the output, no second copy.
    if annotations.is_empty() {
        return cropped;
    }

    // Offset annotations to selection-relative coordinates
    let offset_annotations: Vec<Annotation> = annotations
        .iter()
        .map(|a| offset_annotation(a, sel_x as f32, sel_y as f32))
        .collect();

    flatten_annotations(&cropped, clamped_w, clamped_h, &offset_annotations)
}

/// Offset an annotation's coordinates by (-dx, -dy).
pub fn offset_annotation(annotation: &Annotation, dx: f32, dy: f32) -> Annotation {
    match annotation {
        Annotation::Arrow {
            start,
            end,
            color,
            thickness,
        } => Annotation::Arrow {
            start: Point::new(start.x - dx, start.y - dy),
            end: Point::new(end.x - dx, end.y - dy),
            color: *color,
            thickness: *thickness,
        },
        Annotation::Rectangle {
            top_left,
            size,
            color,
            thickness,
        } => Annotation::Rectangle {
            top_left: Point::new(top_left.x - dx, top_left.y - dy),
            size: Size::new(size.width, size.height),
            color: *color,
            thickness: *thickness,
        },
        Annotation::Pencil {
            points,
            color,
            thickness,
        } => Annotation::Pencil {
            points: points
                .iter()
                .map(|p| Point::new(p.x - dx, p.y - dy))
                .collect(),
            color: *color,
            thickness: *thickness,
        },
        Annotation::Text {
            position,
            text,
            color,
            font_size,
        } => Annotation::Text {
            position: Point::new(position.x - dx, position.y - dy),
            text: text.clone(),
            color: *color,
            font_size: *font_size,
        },
        Annotation::Pixelate {
            top_left,
            size,
            block_size,
        } => Annotation::Pixelate {
            top_left: Point::new(top_left.x - dx, top_left.y - dy),
            size: Size::new(size.width, size.height),
            block_size: *block_size,
        },
        Annotation::Ellipse {
            center,
            radius_x,
            radius_y,
            color,
            thickness,
        } => Annotation::Ellipse {
            center: Point::new(center.x - dx, center.y - dy),
            radius_x: *radius_x,
            radius_y: *radius_y,
            color: *color,
            thickness: *thickness,
        },
        Annotation::Line {
            start,
            end,
            color,
            thickness,
        } => Annotation::Line {
            start: Point::new(start.x - dx, start.y - dy),
            end: Point::new(end.x - dx, end.y - dy),
            color: *color,
            thickness: *thickness,
        },
        Annotation::Highlight {
            top_left,
            size,
            color,
        } => Annotation::Highlight {
            top_left: Point::new(top_left.x - dx, top_left.y - dy),
            size: Size::new(size.width, size.height),
            color: *color,
        },
        Annotation::StepMarker {
            position,
            number,
            color,
            size,
        } => Annotation::StepMarker {
            position: Point::new(position.x - dx, position.y - dy),
            number: *number,
            color: *color,
            size: *size,
        },
        Annotation::RoundedRect {
            top_left,
            size,
            color,
            thickness,
            radius,
        } => Annotation::RoundedRect {
            top_left: Point::new(top_left.x - dx, top_left.y - dy),
            size: Size::new(size.width, size.height),
            color: *color,
            thickness: *thickness,
            radius: *radius,
        },
        Annotation::Spotlight { top_left, size } => Annotation::Spotlight {
            top_left: Point::new(top_left.x - dx, top_left.y - dy),
            size: Size::new(size.width, size.height),
        },
        Annotation::Measurement { start, end, color } => Annotation::Measurement {
            start: Point::new(start.x - dx, start.y - dy),
            end: Point::new(end.x - dx, end.y - dy),
            color: *color,
        },
    }
}

/// Copy pixel data to the system clipboard.
///
/// `pixels` is straight RGBA8 data.
pub fn copy_to_clipboard(pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
    copy_with_ticket(crate::clipboard_queue::reserve(), pixels, width, height).map(|_| ())
}

pub fn validate_image(pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
    let len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4));
    if width == 0 || height == 0 || len != Some(pixels.len()) {
        return Err("Invalid image dimensions or RGBA buffer length".into());
    }
    Ok(())
}

pub fn copy_with_ticket(
    ticket: u64,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<bool, String> {
    validate_image(pixels, width, height)?;
    // Arboard's Windows backend publishes PNG and DIBV5 with bounded busy retries.
    crate::clipboard_queue::run(ticket, || {
        use arboard::{Clipboard, ImageData};

        let mut clipboard =
            Clipboard::new().map_err(|e| format!("Failed to open clipboard: {e}"))?;
        let img = ImageData {
            width: width as usize,
            height: height as usize,
            bytes: std::borrow::Cow::Borrowed(pixels),
        };
        clipboard
            .set_image(img)
            .map_err(|e| format!("Failed to copy image to clipboard: {e}"))
    })
}

/// Save pixel data to an image file (accepts owned Vec to avoid redundant copy).
///
/// When `default_dir` is `Some`, auto-saves to that directory with a
/// timestamped PNG filename (no dialog). When `None`, opens a native save
/// dialog offering PNG, JPEG, and WebP formats.
///
/// Returns `Ok(Some(path))` on success, `Ok(None)` if the user cancelled,
/// or `Err` on failure.
pub fn save_to_file(
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    default_dir: Option<&std::path::Path>,
) -> Result<Option<String>, String> {
    if let Some(dir) = default_dir {
        let stem = chrono::Local::now()
            .format("snipping_tool_ul_%Y-%m-%d_%H%M%S_%3f")
            .to_string();
        return save_auto_png(&pixels, width, height, dir, &stem)
            .map(|p| Some(p.to_string_lossy().into_owned()));
    }
    if let Some(path) = choose_save_path(None) {
        save_image_to_path(pixels, width, height, &path)?;
        Ok(Some(path.to_string_lossy().into_owned()))
    } else {
        Ok(None)
    }
}

/// Chooser only: no pixel copying, annotation rendering or image encoding.
pub fn choose_save_path(initial_dir: Option<&std::path::Path>) -> Option<std::path::PathBuf> {
    let name = chrono::Local::now()
        .format("snipping_tool_ul_%Y-%m-%d_%H%M%S.png")
        .to_string();
    let mut dialog = rfd::FileDialog::new()
        .set_file_name(&name)
        .add_filter("PNG Image", &["png"])
        .add_filter("JPEG Image", &["jpg", "jpeg"])
        .add_filter("WebP Image", &["webp"]);
    if let Some(dir) = initial_dir.filter(|p| p.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    dialog.save_file()
}

/// Encode an explicit, already chosen path. No dialog is opened here.
pub fn save_image_to_path(
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    path: &std::path::Path,
) -> Result<(), String> {
    validate_image(&pixels, width, height)?;
    let img = image::RgbaImage::from_raw(width, height, pixels).ok_or("Invalid image")?;
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png")
        .to_lowercase();
    if ext == "jpg" || ext == "jpeg" {
        image::DynamicImage::ImageRgba8(img)
            .to_rgb8()
            .save_with_format(path, image::ImageFormat::Jpeg)
    } else {
        let format = if ext == "webp" {
            image::ImageFormat::WebP
        } else {
            image::ImageFormat::Png
        };
        img.save_with_format(path, format)
    }
    .map_err(|e| format!("Failed to save image: {e}"))
}

fn save_auto_png(
    pixels: &[u8],
    width: u32,
    height: u32,
    dir: &std::path::Path,
    stem: &str,
) -> Result<std::path::PathBuf, String> {
    use image::ImageEncoder;
    use std::io::Write;
    validate_image(pixels, width, height)?;
    std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create auto-save folder: {e}"))?;
    for suffix in 0..128 {
        let name = if suffix == 0 {
            format!("{stem}.png")
        } else {
            format!("{stem}_{suffix}.png")
        };
        let path = dir.join(name);
        let file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Failed to create auto-save file: {e}")),
        };
        let mut writer = std::io::BufWriter::new(file);
        image::codecs::png::PngEncoder::new(&mut writer)
            .write_image(pixels, width, height, image::ExtendedColorType::Rgba8)
            .map_err(|e| format!("Failed to encode auto-save: {e}"))?;
        writer
            .flush()
            .map_err(|e| format!("Failed to write auto-save: {e}"))?;
        return Ok(path);
    }
    Err("Auto-save filename collision limit reached; existing captures were preserved.".into())
}

#[cfg(test)]
mod save_tests {
    use super::*;
    struct TempFolder(std::path::PathBuf);
    impl TempFolder {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "ul-save-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            Self(path)
        }
    }
    impl Drop for TempFolder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn auto_save_creates_destination_and_preserves_same_timestamp_captures() {
        let temp = TempFolder::new();
        let dir = temp.0.join("chosen/subfolder");
        let a = save_auto_png(&[1, 2, 3, 255], 1, 1, &dir, "same-time").unwrap();
        let before = std::fs::read(&a).unwrap();
        let b = save_auto_png(&[4, 5, 6, 255], 1, 1, &dir, "same-time").unwrap();
        assert_ne!(a, b);
        assert_eq!(std::fs::read(&a).unwrap(), before);
        assert_eq!(
            image::open(&a).unwrap().to_rgba8().as_raw(),
            &[1, 2, 3, 255]
        );
        assert_eq!(
            image::open(&b).unwrap().to_rgba8().as_raw(),
            &[4, 5, 6, 255]
        );
    }
    #[test]
    fn invalid_auto_save_does_not_create_folders_or_files() {
        let temp = TempFolder::new();
        assert!(save_auto_png(&[], 1, 1, &temp.0, "bad").is_err());
        assert!(!temp.0.exists());
        assert!(save_image_to_path(vec![], 1, 1, &temp.0.join("bad.png")).is_err());
        assert!(!temp.0.exists());
    }
    #[test]
    fn chosen_paths_save_png_jpeg_and_webp_without_opening_dialogs() {
        let temp = TempFolder::new();
        std::fs::create_dir_all(&temp.0).unwrap();
        for ext in ["png", "jpg", "webp"] {
            let path = temp.0.join(format!("capture.{ext}"));
            save_image_to_path([40, 80, 120, 255].repeat(4), 2, 2, &path).unwrap();
            let decoded = image::open(path).unwrap().to_rgba8();
            assert_eq!(decoded.dimensions(), (2, 2));
            if ext != "jpg" {
                assert_eq!(decoded.as_raw(), &[40, 80, 120, 255].repeat(4));
            }
        }
    }
}
