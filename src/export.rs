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
    if width == 0 || height == 0 {
        return pixels.to_vec();
    }
    let Some(expected_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
    else {
        return pixels.to_vec();
    };
    if expected_len != pixels.len() {
        return pixels.to_vec();
    }
    if annotations.is_empty() {
        return pixels.to_vec();
    }
    let mut pixmap = match Pixmap::new(width, height) {
        Some(p) => p,
        None => return pixels.to_vec(),
    };

    // Exact validation above keeps every source pixel aligned with the pixmap.
    // Fast path: screenshots are fully opaque (a=255), so premultiplication is a
    // no-op — we can copy components directly instead of multiplying per pixel.
    let all_opaque = pixels.chunks_exact(4).all(|px| px[3] == 255);
    if all_opaque {
        // Opaque straight RGBA is already valid premultiplied data: one memcpy
        // instead of a per-pixel constructor (3.4 ms -> 0.4 ms at 1080p).
        pixmap.data_mut().copy_from_slice(&pixels[..expected_len]);
    } else {
        let pm_pixels = pixmap.pixels_mut();
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

    // Spotlight: dim everything, then restore the spotlit rectangles from the
    // source. The editor does this in renderer.rs; exports used to skip it, so
    // Copy/Save/Upload/Pin produced an image with no dimming at all.
    let spotlights: Vec<(Point, Size)> = annotations
        .iter()
        .filter_map(|a| match a {
            Annotation::Spotlight { top_left, size } => Some((*top_left, *size)),
            _ => None,
        })
        .collect();
    if !spotlights.is_empty() {
        apply_spotlights(&mut pixmap, pixels, width, &spotlights);
    }

    // Render each annotation exactly once, over the dimmed base
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

/// Dim `pixmap` by 50% black and copy each spotlight rectangle back from the
/// straight-alpha `source` (same dimensions as the pixmap). Shared by the
/// editor renderer and the export path so both produce the same pixels.
pub fn apply_spotlights(
    pixmap: &mut Pixmap,
    source: &[u8],
    source_width: u32,
    spotlights: &[(Point, Size)],
) {
    let (pw, ph) = (pixmap.width(), pixmap.height());
    if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, pw as f32, ph as f32) {
        let mut dim = tiny_skia::Paint::default();
        dim.set_color(tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, 0.5).unwrap());
        dim.anti_alias = false;
        pixmap.fill_rect(r, &dim, tiny_skia::Transform::identity(), None);
    }
    let data = pixmap.data_mut();
    for (tl, sz) in spotlights {
        // Shrink the region when the origin is clamped so the cutout matches
        // the annotation rect instead of extending right/down.
        let sw = (sz.width + tl.x.min(0.0)).max(0.0) as u32;
        let sh = (sz.height + tl.y.min(0.0)).max(0.0) as u32;
        let sx = tl.x.max(0.0) as u32;
        let sy = tl.y.max(0.0) as u32;
        let copy_w = sw
            .min(source_width.saturating_sub(sx))
            .min(pw.saturating_sub(sx));
        if copy_w == 0 {
            continue;
        }
        // Float-to-u32 casts saturate at u32::MAX, so clamp the exclusive end
        // with saturating addition before iterating very large spotlights.
        let end_y = sy.saturating_add(sh).min(ph);
        let Some(copy_bytes) = (copy_w as usize).checked_mul(4) else {
            continue;
        };
        for y in sy..end_y {
            let src_start = (y as usize)
                .checked_mul(source_width as usize)
                .and_then(|offset| offset.checked_add(sx as usize))
                .and_then(|offset| offset.checked_mul(4));
            let dst_start = (y as usize)
                .checked_mul(pw as usize)
                .and_then(|offset| offset.checked_add(sx as usize))
                .and_then(|offset| offset.checked_mul(4));
            let Some((src_start, dst_start)) = src_start.zip(dst_start) else {
                continue;
            };
            let Some(src_end) = src_start.checked_add(copy_bytes) else {
                continue;
            };
            let Some(dst_end) = dst_start.checked_add(copy_bytes) else {
                continue;
            };
            if src_end <= source.len() && dst_end <= data.len() {
                // Opaque source pixels are identical in straight and
                // premultiplied form; translucent ones are premultiplied here.
                for (dst, src) in data[dst_start..dst_end]
                    .chunks_exact_mut(4)
                    .zip(source[src_start..src_end].chunks_exact(4))
                {
                    let a = src[3] as u16;
                    if a == 255 {
                        dst.copy_from_slice(src);
                    } else {
                        for c in 0..3 {
                            dst[c] = ((src[c] as u16 * a + 127) / 255) as u8;
                        }
                        dst[3] = src[3];
                    }
                }
            }
        }
    }
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
    // Validate row geometry before deriving the height. All indexing below is
    // in usize so u32 row-byte and pixel-offset multiplication cannot wrap.
    let Some(row_bytes) = (screenshot_width as usize).checked_mul(4) else {
        return Vec::new();
    };
    if row_bytes == 0 || screenshot_pixels.len() % row_bytes != 0 {
        return Vec::new();
    }
    let screenshot_height = screenshot_pixels.len() / row_bytes;

    // Clamp selection to the validated screenshot bounds.
    let clamped_w = sel_w.min(screenshot_width.saturating_sub(sel_x));
    let start_y = sel_y as usize;
    let clamped_h = (sel_h as usize).min(screenshot_height.saturating_sub(start_y));
    let Some(start_x_bytes) = (sel_x as usize).checked_mul(4) else {
        return Vec::new();
    };
    let Some(crop_row_bytes) = (clamped_w as usize).checked_mul(4) else {
        return Vec::new();
    };
    let Some(cropped_len) = crop_row_bytes.checked_mul(clamped_h) else {
        return Vec::new();
    };
    let Some(end_y) = start_y.checked_add(clamped_h) else {
        return Vec::new();
    };

    // Crop the pixel buffer, returning an empty result if allocation or any
    // source offset is not representable.
    let mut cropped = Vec::new();
    if cropped.try_reserve_exact(cropped_len).is_err() {
        return Vec::new();
    }
    for row in start_y..end_y {
        let Some(start) = row
            .checked_mul(row_bytes)
            .and_then(|offset| offset.checked_add(start_x_bytes))
        else {
            return Vec::new();
        };
        let Some(end) = start.checked_add(crop_row_bytes) else {
            return Vec::new();
        };
        if end > screenshot_pixels.len() {
            return Vec::new();
        }
        cropped.extend_from_slice(&screenshot_pixels[start..end]);
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

    flatten_annotations(&cropped, clamped_w, clamped_h as u32, &offset_annotations)
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
    copy_with_ticket_sequence(ticket, pixels, width, height).map(|s| s.is_some())
}

/// Like `copy_with_ticket`, but also returns the clipboard sequence number
/// the copy produced, read while the clipboard writer is still held.
/// `Ok(None)` means a newer request was kept.
pub fn copy_with_ticket_sequence(
    ticket: u64,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<Option<u32>, String> {
    validate_image(pixels, width, height)?;
    let mut sequence = 0;
    // Windows: PNG and DIBV5 are built before the clipboard opens, then
    // published with bounded busy retries.
    #[cfg(windows)]
    let copied = crate::clipboard_queue::run(ticket, || {
        let inside = crate::clipboard_image::write(pixels, width, height)?;
        // Windows may count the change when the clipboard closes. One step
        // past the value read inside is still UL's copy; more means another
        // app wrote in between, so the copy is not claimed (Copy stays shown).
        let after = clipboard_sequence();
        sequence = if after.wrapping_sub(inside) <= 1 {
            after
        } else {
            0
        };
        Ok(())
    })?;
    #[cfg(not(windows))]
    let copied = crate::clipboard_queue::run(ticket, || {
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
    })?;
    // Sequence 0 marks an unconfirmed copy: it still succeeded.
    Ok(copied.then_some(sequence))
}

/// The clipboard's change counter. It moves whenever any app copies.
pub fn clipboard_sequence() -> u32 {
    #[cfg(windows)]
    return unsafe { windows::Win32::System::DataExchange::GetClipboardSequenceNumber() };
    #[cfg(not(windows))]
    0
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
    // Encode beside the destination and rename on success, so a failed or
    // interrupted encode never truncates a file the user chose to overwrite.
    let (part, file) = reserve_save_file(path)?;
    let encoded = (|| -> Result<(), String> {
        let mut writer = std::io::BufWriter::new(file);
        if ext == "jpg" || ext == "jpeg" {
            // The crate default (75, subsampled) smears text edges in screenshots.
            let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, 92)
                .encode_image(&rgb)
                .map_err(|e| e.to_string())?;
        } else if ext == "webp" {
            img.write_with_encoder(image::codecs::webp::WebPEncoder::new_lossless(&mut writer))
                .map_err(|e| e.to_string())?;
        } else {
            img.write_with_encoder(image::codecs::png::PngEncoder::new(&mut writer))
                .map_err(|e| e.to_string())?;
        }
        use std::io::Write;
        writer.flush().map_err(|e| e.to_string())
    })();
    if let Err(e) = encoded {
        let _ = std::fs::remove_file(&part);
        return Err(format!("Failed to save image: {e}"));
    }
    std::fs::rename(&part, path).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        format!("Failed to save image: {e}")
    })
}

/// Each writer owns its staging file. Neither a pre-existing sibling nor a
/// concurrent save may be truncated or removed by this save's cleanup.
fn reserve_save_file(
    path: &std::path::Path,
) -> Result<(std::path::PathBuf, std::fs::File), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let name = path
        .file_name()
        .ok_or("Save destination must name a file")?;
    for _ in 0..128 {
        let mut part_name = name.to_owned();
        part_name.push(format!(
            ".ul-{}-{}.part",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let part = path.with_file_name(part_name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part)
        {
            Ok(file) => return Ok((part, file)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Failed to create save file: {e}")),
        }
    }
    Err("Unable to reserve a temporary save file; existing files were preserved".into())
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
        let written = image::codecs::png::PngEncoder::new(&mut writer)
            .write_image(pixels, width, height, image::ExtendedColorType::Rgba8)
            .map_err(|e| format!("Failed to encode auto-save: {e}"))
            .and_then(|()| {
                writer
                    .flush()
                    .map_err(|e| format!("Failed to write auto-save: {e}"))
            });
        if let Err(e) = written {
            // A truncated PNG would sit in the auto-save folder looking valid.
            drop(writer);
            let _ = std::fs::remove_file(&path);
            return Err(e);
        }
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
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "ul-save-test-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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
