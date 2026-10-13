//! SVG icon rendering using Lucide Icons (ISC, with Feather-derived MIT icons) via resvg.
//!
//! Each icon is stored as an SVG string constant. Icons are rendered to tiny_skia::Pixmap
//! on first use and cached for subsequent frames.

use std::collections::HashMap;

/// Wrap SVG path content in a complete Lucide-style SVG document.
/// Lucide icons use a 24x24 viewBox with stroke-based rendering.
fn wrap_svg(inner: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{}</svg>"#,
        inner
    )
}

/// Return the SVG string for a given icon name.
fn get_svg(name: &str) -> Option<String> {
    if name == "ul-logo" {
        return Some(include_str!("../assets/ul-logo.svg").into());
    }
    let inner = match name {
        "crop" => r#"<path d="M3 9V3h6M15 3h6v6M21 15v6h-6M9 21H3v-6"/>"#,
        "plus" => r#"<path d="M12 5v14M5 12h14"/>"#,
        "down" => r#"<path d="m8 10 4 4 4-4"/>"#,
        "right" => r#"<path d="m9 6 6 6-6 6"/>"#,
        "power" => r#"<path d="M12 2v10M6 5a9 9 0 1 0 12 0"/>"#,
        "clock" => r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 7v5l3 2"/>"#,
        "gear" => {
            r#"<circle cx="12" cy="12" r="3"/><path d="m9 3-.6 2.5-2.2 1.3L3.7 6l-1.5 2.6L4 10.5v3L2.2 15.4l1.5 2.6 2.5-.8 2.2 1.3L9 21h6l.6-2.5 2.2-1.3 2.5.8 1.5-2.6L20 13.5v-3l1.8-1.9L20.3 6l-2.5.8-2.2-1.3L15 3Z"/>"#
        }
        "more" => {
            r#"<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>"#
        }
        "check" => r#"<path d="m5 12 4 4L19 6"/>"#,
        "window" => r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18"/>"#,
        "screen" => {
            r#"<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M12 17v4M8 21h8"/>"#
        }
        "preview" => {
            r#"<rect x="3" y="3" width="18" height="18" rx="2"/><path d="m3 16 5-5 4 4 3-3 6 6"/><circle cx="15" cy="8" r="1"/>"#
        }
        "history" => r#"<path d="M3 11a9 9 0 1 1 2 7M3 4v7h7M12 7v5l3 2"/>"#,
        "undo" => r#"<path d="M8 4 3 9l5 5M3 9h10a7 7 0 1 1 0 14"/>"#,
        "redo" => r#"<path d="m16 4 5 5-5 5M21 9H11a7 7 0 1 0 0 14"/>"#,
        "select" => r#"<path d="M4 3v17l5-5 4 6 3-2-4-6h7L4 3Z"/>"#,
        "arrow" => r#"<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>"#,
        "rectangle" => r#"<rect width="18" height="18" x="3" y="3" rx="2"/>"#,
        "circle" => r#"<circle cx="12" cy="12" r="10"/>"#,
        "line" => r#"<path d="M4 20 L20 4"/>"#,
        "pencil" => {
            r#"<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/>"#
        }
        "highlight" => {
            r#"<path d="m9 11-6 6v3h9l3-3"/><path d="m22 12-4.6 4.6a2 2 0 0 1-2.8 0l-5.2-5.2a2 2 0 0 1 0-2.8L14 4"/>"#
        }
        "text" => {
            r#"<polyline points="4 7 4 4 20 4 20 7"/><line x1="9" x2="15" y1="20" y2="20"/><line x1="12" x2="12" y1="4" y2="20"/>"#
        }
        "pixelate" => {
            r#"<rect width="7" height="7" x="2" y="2" rx="1"/><rect width="7" height="7" x="15" y="2" rx="1"/><rect width="7" height="7" x="2" y="15" rx="1"/><rect width="7" height="7" x="15" y="15" rx="1"/>"#
        }
        "step-marker" => {
            r#"<line x1="4" x2="20" y1="9" y2="9"/><line x1="4" x2="20" y1="15" y2="15"/><line x1="10" x2="8" y1="3" y2="21"/><line x1="16" x2="14" y1="3" y2="21"/>"#
        }
        "eyedropper" => {
            r#"<path d="m2 22 1-1h3l9-9"/><path d="M3 21v-3l9-9"/><path d="m15 6 3.4-3.4a2.1 2.1 0 1 1 3 3L18 9l.4.4a2.1 2.1 0 1 1-3 3l-3.8-3.8a2.1 2.1 0 1 1 3-3l.4.4Z"/>"#
        }
        "pin" => {
            r#"<path d="M12 17v5"/><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"/>"#
        }
        "copy" => {
            r#"<rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>"#
        }
        "upload" => {
            r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" x2="12" y1="3" y2="15"/>"#
        }
        "save" => {
            r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>"#
        }
        "rounded-rect" => r#"<rect x="3" y="3" width="18" height="18" rx="5"/>"#,
        "ocr" => {
            r#"<path d="M4 7V4h3"/><path d="M17 4h3v3"/><path d="M4 17v3h3"/><path d="M17 20h3v-3"/><path d="M7 8h10"/><path d="M7 12h10"/><path d="M7 16h10"/>"#
        }
        "spotlight" => {
            r#"<circle cx="12" cy="12" r="3"/><path d="M3 7V3h4"/><path d="M17 3h4v4"/><path d="M21 17v4h-4"/><path d="M7 21H3v-4"/>"#
        }
        "measurement" => {
            r#"<path d="M21.3 15.3a2.4 2.4 0 0 1 0 3.4l-2.6 2.6a2.4 2.4 0 0 1-3.4 0L2.7 8.7a2.41 2.41 0 0 1 0-3.4l2.6-2.6a2.41 2.41 0 0 1 3.4 0Z"/><path d="m14.5 12.5 2-2"/><path d="m11.5 9.5 2-2"/><path d="m8.5 6.5 2-2"/><path d="m17.5 15.5 2-2"/>"#
        }
        _ => return None,
    };
    Some(wrap_svg(inner))
}

/// Render an SVG string to a tiny_skia::Pixmap at the given size and color.
fn render_icon_svg(svg_data: &str, size: u32, color: &str) -> Option<tiny_skia::Pixmap> {
    let colored_svg = svg_data.replace("currentColor", color);

    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&colored_svg, &options).ok()?;

    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    let scale = size as f32 / 24.0;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    Some(pixmap)
}

/// Cache for rendered icon pixmaps, keyed by (name_color, size).
#[derive(Default)]
pub struct IconCache {
    icons: HashMap<(String, u32), tiny_skia::Pixmap>,
}

impl IconCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a cached icon or render it on first access.
    pub fn get_or_render(
        &mut self,
        name: &str,
        size: u32,
        color: &str,
    ) -> Option<&tiny_skia::Pixmap> {
        use std::collections::hash_map::Entry;
        let key = (format!("{}_{}", name, color), size);
        match self.icons.entry(key) {
            Entry::Occupied(e) => Some(e.into_mut()),
            Entry::Vacant(e) => {
                let svg_data = get_svg(name)?;
                let pixmap = render_icon_svg(&svg_data, size, color)?;
                Some(e.insert(pixmap))
            }
        }
    }
}

/// Alpha-blend `src` pixmap onto `dst` pixmap at the given offset.
pub fn blend_pixmap(
    dst: &mut tiny_skia::Pixmap,
    src: &tiny_skia::Pixmap,
    offset_x: i32,
    offset_y: i32,
) {
    let dst_w = dst.width() as i32;
    let dst_h = dst.height() as i32;
    let src_w = src.width() as i32;
    let src_h = src.height() as i32;

    let dst_data = dst.data_mut();
    let src_data = src.data();

    for sy in 0..src_h {
        let dy = offset_y + sy;
        if dy < 0 || dy >= dst_h {
            continue;
        }
        for sx in 0..src_w {
            let dx = offset_x + sx;
            if dx < 0 || dx >= dst_w {
                continue;
            }

            let si = ((sy * src_w + sx) * 4) as usize;
            let di = ((dy * dst_w + dx) * 4) as usize;

            let sa = src_data[si + 3] as u32;
            if sa == 0 {
                continue;
            }
            if sa == 255 {
                dst_data[di..di + 4].copy_from_slice(&src_data[si..si + 4]);
                continue;
            }

            // Standard alpha compositing (src is premultiplied from resvg).
            // +127 rounds to nearest instead of truncating toward darker.
            let inv_sa = 255 - sa;
            for c in 0..3 {
                let s = src_data[si + c] as u32;
                let d = dst_data[di + c] as u32;
                dst_data[di + c] = (s + (d * inv_sa + 127) / 255).min(255) as u8;
            }
            let da = dst_data[di + 3] as u32;
            dst_data[di + 3] = (sa + (da * inv_sa + 127) / 255).min(255) as u8;
        }
    }
}
