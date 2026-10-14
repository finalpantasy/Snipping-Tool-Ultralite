//! Canvas extents are derived from visible marks, never accumulated in the image.
use crate::tools::{annotation_bounding_box, Annotation};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl CanvasBounds {
    pub fn image(width: u32, height: u32) -> Self {
        Self {
            x: 0.,
            y: 0.,
            width: width as f32,
            height: height as f32,
        }
    }

    pub fn include(&mut self, annotation: &Annotation) {
        // These tools only affect existing image pixels; they create no new ink.
        if matches!(
            annotation,
            Annotation::Pixelate { .. } | Annotation::Spotlight { .. }
        ) {
            return;
        }
        let Some((mut x, mut y, mut w, mut h)) = annotation_bounding_box(annotation) else {
            return;
        };
        match annotation {
            Annotation::Rectangle { thickness, .. }
            | Annotation::RoundedRect { thickness, .. }
            | Annotation::Ellipse { thickness, .. } => {
                x -= thickness / 2.;
                y -= thickness / 2.;
                w += thickness;
                h += thickness;
            }
            Annotation::StepMarker {
                position,
                size,
                number,
                ..
            } => {
                x -= 1.;
                y -= 1.;
                w += 2.;
                h += 2.; // White two-pixel border.
                let text = number.to_string();
                let fs = size * 0.55;
                let font = crate::font::FONT.for_text(&text);
                let metrics = font.horizontal_line_metrics(fs);
                let text_height = metrics.map(|m| m.ascent + m.descent.abs()).unwrap_or(fs);
                let (ix, iy, iw, ih) = crate::tools::text_ink_bounds(&text, fs);
                let left = position.x - crate::tools::measure_text_width(&text, fs) / 2. + ix;
                let top = position.y - text_height / 2. + iy;
                let right = (x + w).max(left + iw);
                let bottom = (y + h).max(top + ih);
                x = x.min(left);
                y = y.min(top);
                w = right - x;
                h = bottom - y;
            }
            Annotation::Measurement { start, end, .. } => {
                let length = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
                // Perpendicular endpoint ticks extend beyond the endpoint circles.
                if length > 0. {
                    let tx = ((end.y - start.y) / length * 7.).abs() + 1.;
                    let ty = ((end.x - start.x) / length * 7.).abs() + 1.;
                    let right = (x + w).max(start.x.max(end.x) + tx);
                    let bottom = (y + h).max(start.y.max(end.y) + ty);
                    x = x.min(start.x.min(end.x) - tx);
                    y = y.min(start.y.min(end.y) - ty);
                    w = right - x;
                    h = bottom - y;
                }
                let label = format!("{length:.0} px");
                let label_w = label.len() as f32 * 12. * 0.6 + 8.;
                let left = (start.x + end.x) / 2. - label_w / 2.;
                let top = (start.y + end.y) / 2. - 14.;
                let right = (x + w).max(left + label_w);
                let bottom = (y + h).max(top + 18.);
                x = x.min(left);
                y = y.min(top);
                w = right - x;
                h = bottom - y;
            }
            _ => {}
        }
        if ![x, y, w, h, x + w, y + h].iter().all(|n| n.is_finite()) || w <= 0. || h <= 0. {
            return;
        }
        let right = (self.x + self.width).max((x + w).ceil());
        let bottom = (self.y + self.height).max((y + h).ceil());
        self.x = self.x.min(x.floor());
        self.y = self.y.min(y.floor());
        self.width = right - self.x;
        self.height = bottom - self.y;
    }

    /// Bound the full-resolution allocation, independently of viewport rendering.
    pub fn export_size(self) -> Option<(u32, u32)> {
        if ![self.width, self.height]
            .iter()
            .all(|n| n.is_finite() && *n > 0. && *n <= 65_536.)
        {
            return None;
        }
        let (w, h) = (self.width as u32, self.height as u32);
        (w as u64 * h as u64 <= 128 * 1024 * 1024).then_some((w, h))
    }
}

pub fn bounds(width: u32, height: u32, annotations: &[Annotation]) -> CanvasBounds {
    let mut bounds = CanvasBounds::image(width, height);
    for a in annotations {
        bounds.include(a);
    }
    bounds
}

/// Clip image effects in source-pixel space, before translating onto a larger canvas.
/// Preserve the renderer's integer origin/extent rounding, including fractional edges.
pub fn source_region(
    top_left: crate::geometry::Point,
    size: crate::geometry::Size,
    width: u32,
    height: u32,
) -> Option<(u32, u32, u32, u32)> {
    if ![top_left.x, top_left.y, size.width, size.height]
        .iter()
        .all(|n| n.is_finite())
    {
        return None;
    }
    let x = top_left.x.max(0.) as u32;
    let y = top_left.y.max(0.) as u32;
    let w = ((size.width + top_left.x.min(0.)).max(0.) as u32).min(width.saturating_sub(x));
    let h = ((size.height + top_left.y.min(0.)).max(0.) as u32).min(height.saturating_sub(y));
    (w > 0 && h > 0).then_some((x, y, w, h))
}

/// Expand the raw image and move annotations together, keeping screenshot pixels intact.
pub fn prepare_export(
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    annotations: Vec<Annotation>,
) -> Option<(Vec<u8>, u32, u32, Vec<Annotation>)> {
    if pixels.len()
        != (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?
    {
        return None;
    }
    let b = bounds(width, height, &annotations);
    let (w, h) = b.export_size()?;
    if b == CanvasBounds::image(width, height) {
        return Some((pixels, w, h, annotations));
    }
    let mut expanded = Vec::new();
    expanded
        .try_reserve_exact((w as usize).checked_mul(h as usize)?.checked_mul(4)?)
        .ok()?;
    expanded.resize(w as usize * h as usize * 4, 255);
    let (dx, dy) = ((-b.x) as usize, (-b.y) as usize);
    for row in 0..height as usize {
        let src = row * width as usize * 4;
        let dst = ((row + dy) * w as usize + dx) * 4;
        expanded
            .get_mut(dst..dst + width as usize * 4)?
            .copy_from_slice(&pixels[src..src + width as usize * 4]);
    }
    // Both image effects preserve the original pixels and integer raster boundaries.
    let annotations = annotations
        .iter()
        .filter_map(|a| {
            let clipped;
            let a = if let Annotation::Pixelate {
                top_left,
                size,
                block_size,
            } = a
            {
                let (x, y, w, h) = source_region(*top_left, *size, width, height)?;
                clipped = Annotation::Pixelate {
                    top_left: crate::geometry::Point::new(x as f32, y as f32),
                    size: crate::geometry::Size::new(w as f32, h as f32),
                    block_size: *block_size,
                };
                &clipped
            } else if let Annotation::Spotlight { top_left, size } = a {
                // An entirely outside spotlight still dims the whole canvas.
                let (x, y, w, h) =
                    source_region(*top_left, *size, width, height).unwrap_or((0, 0, 0, 0));
                clipped = Annotation::Spotlight {
                    top_left: crate::geometry::Point::new(x as f32, y as f32),
                    size: crate::geometry::Size::new(w as f32, h as f32),
                };
                &clipped
            } else {
                a
            };
            Some(crate::export::offset_annotation(a, b.x, b.y))
        })
        .collect();
    Some((expanded, w, h, annotations))
}
