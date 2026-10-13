pub mod arrow;
pub mod circle;
pub mod highlight;
pub mod line;
pub mod measurement;
pub mod pencil;
pub mod pixelate;
pub mod rectangle;
pub mod rounded_rect;
pub mod spotlight;
pub mod step_marker;
pub mod text;

use crate::geometry::{Color, Point, Size};
use tiny_skia::{Paint, PathBuilder, Pixmap, Stroke, Transform};

/// Maximum number of entries in the undo stack before oldest entries are dropped.
const UNDO_STACK_CAP: usize = 50;

/// A reversible action recorded on the undo/redo stacks.
#[derive(Debug, Clone)]
pub enum UndoAction {
    Batch(Vec<UndoAction>),
    /// An annotation was added at the given index.
    Add(usize),
    /// An annotation was deleted from the given index (stores the deleted annotation).
    Delete(usize, Annotation),
    /// An annotation was modified at the given index (stores the OLD version).
    Modify(usize, Annotation),
}

/// Apply one undo/redo step from `from_stack`, pushing the inverse onto `to_stack`.
/// Returns false if the stack is empty or the action references a stale index.
fn apply_stack_action(
    annotations: &mut Vec<Annotation>,
    from_stack: &mut Vec<UndoAction>,
    to_stack: &mut Vec<UndoAction>,
) -> bool {
    // Peek first — only pop if the action is applicable, so stale actions are
    // not silently lost.
    let actionable = match from_stack.last() {
        Some(UndoAction::Batch(_)) => true,
        Some(UndoAction::Add(idx)) => *idx < annotations.len(),
        Some(UndoAction::Modify(idx, _)) => *idx < annotations.len(),
        Some(UndoAction::Delete(_, _)) => true,
        None => return false,
    };
    if !actionable {
        // Stale index — discard the action rather than silently losing it
        // with no inverse recorded. Log so it can be diagnosed.
        let _ = from_stack.pop();
        tracing::warn!("Discarded undo/redo action with stale index");
        return false;
    }
    let action = from_stack.pop().unwrap();
    match action {
        UndoAction::Batch(actions) => {
            // Validate the complete group on annotation metadata before committing anything.
            let mut candidate = annotations.clone();
            let mut inverses = Vec::new();
            for action in actions {
                let mut inverse = Vec::new();
                if !apply_stack_action(&mut candidate, &mut vec![action], &mut inverse) {
                    return false;
                }
                inverses.extend(inverse);
            }
            *annotations = candidate;
            inverses.reverse();
            to_stack.push(UndoAction::Batch(inverses));
        }
        UndoAction::Add(idx) => {
            let removed = annotations.remove(idx);
            to_stack.push(UndoAction::Delete(idx, removed));
        }
        UndoAction::Delete(idx, ann) => {
            let idx = idx.min(annotations.len());
            annotations.insert(idx, ann);
            to_stack.push(UndoAction::Add(idx));
        }
        UndoAction::Modify(idx, old_ann) => {
            let current = annotations[idx].clone();
            annotations[idx] = old_ann;
            to_stack.push(UndoAction::Modify(idx, current));
        }
    }
    cap_undo_stack(to_stack);
    true
}

/// Apply one undo step: pop from `undo_stack`, reverse it, push the inverse onto `redo_stack`.
pub fn apply_undo(
    annotations: &mut Vec<Annotation>,
    undo_stack: &mut Vec<UndoAction>,
    redo_stack: &mut Vec<UndoAction>,
) -> bool {
    apply_stack_action(annotations, undo_stack, redo_stack)
}

/// Apply one redo step: pop from `redo_stack`, reverse it, push the inverse onto `undo_stack`.
pub fn apply_redo(
    annotations: &mut Vec<Annotation>,
    undo_stack: &mut Vec<UndoAction>,
    redo_stack: &mut Vec<UndoAction>,
) -> bool {
    apply_stack_action(annotations, redo_stack, undo_stack)
}

/// Record a new action on the undo stack, clearing the redo stack (new branch).
pub fn record_undo(
    undo_stack: &mut Vec<UndoAction>,
    redo_stack: &mut Vec<UndoAction>,
    action: UndoAction,
) {
    redo_stack.clear();
    undo_stack.push(action);
    cap_undo_stack(undo_stack);
}

/// Keep the undo stack within the cap by removing the oldest entries.
fn cap_undo_stack(stack: &mut Vec<UndoAction>) {
    if stack.len() > UNDO_STACK_CAP {
        let excess = stack.len() - UNDO_STACK_CAP;
        stack.drain(..excess);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    Arrow {
        start: Point,
        end: Point,
        color: Color,
        thickness: f32,
    },
    Rectangle {
        top_left: Point,
        size: Size,
        color: Color,
        thickness: f32,
    },
    Pencil {
        points: Vec<Point>,
        color: Color,
        thickness: f32,
    },
    Text {
        position: Point,
        text: String,
        color: Color,
        font_size: f32,
    },
    Pixelate {
        top_left: Point,
        size: Size,
        block_size: u8,
    },
    Ellipse {
        center: Point,
        radius_x: f32,
        radius_y: f32,
        color: Color,
        thickness: f32,
    },
    Line {
        start: Point,
        end: Point,
        color: Color,
        thickness: f32,
    },
    Highlight {
        top_left: Point,
        size: Size,
        color: Color,
    },
    StepMarker {
        position: Point,
        number: u32,
        color: Color,
        size: f32,
    },
    RoundedRect {
        top_left: Point,
        size: Size,
        color: Color,
        thickness: f32,
        radius: f32,
    },
    Spotlight {
        top_left: Point,
        size: Size,
    },
    Measurement {
        start: Point,
        end: Point,
        color: Color,
    },
}

pub trait AnnotationTool {
    fn on_mouse_down(&mut self, pos: Point);
    fn on_mouse_move(&mut self, pos: Point);
    fn on_mouse_up(&mut self, pos: Point) -> Option<Annotation>;
    fn is_drawing(&self) -> bool;
    fn in_progress_annotation(&self) -> Option<Annotation>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolKind {
    Select,
    Arrow,
    Rectangle,
    Circle,
    Line,
    Pencil,
    Highlight,
    Text,
    Pixelate,
    StepMarker,
    Eyedropper,
    RoundedRect,
    Spotlight,
    Measurement,
}

/// Compute the three vertices of an arrowhead triangle.
///
/// The tip is at `end`. The two base vertices are placed at `4 * thickness`
/// distance from the tip along the shaft, spread at +/- 30 degrees.
pub fn arrowhead_points(start: Point, end: Point, thickness: f32) -> Vec<Point> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return vec![end, end, end];
    }

    let ux = dx / len;
    let uy = dy / len;

    let side_length = 4.0 * thickness;
    let half_angle: f32 = std::f32::consts::PI / 6.0; // 30 degrees
    let back_dist = side_length * half_angle.cos();
    let perp_offset = side_length * half_angle.sin();

    // Perpendicular direction (rotate unit vector 90 degrees)
    let px = -uy;
    let py = ux;

    let base_x = end.x - ux * back_dist;
    let base_y = end.y - uy * back_dist;

    let p1 = Point::new(base_x + px * perp_offset, base_y + py * perp_offset);
    let p2 = Point::new(base_x - px * perp_offset, base_y - py * perp_offset);

    vec![end, p1, p2]
}

/// Build a rounded rectangle path with the given corner radius.
fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

/// Render any Annotation variant onto a tiny_skia::Pixmap.
///
/// This is the single rendering path used for both interactive preview and export.
pub fn render_annotation(
    annotation: &Annotation,
    pixmap: &mut Pixmap,
    screenshot_pixels: Option<&[u8]>,
    screenshot_width: Option<u32>,
) {
    match annotation {
        Annotation::Rectangle {
            top_left,
            size,
            color,
            thickness,
        } => {
            let mut paint = Paint::default();
            let c: tiny_skia::Color = (*color).into();
            paint.set_color(c);
            paint.anti_alias = true;

            let rect = tiny_skia::Rect::from_xywh(top_left.x, top_left.y, size.width, size.height);
            if let Some(rect) = rect {
                let path = PathBuilder::from_rect(rect);
                let stroke = Stroke {
                    width: *thickness,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
        Annotation::Arrow {
            start,
            end,
            color,
            thickness,
        } => {
            let mut paint = Paint::default();
            let c: tiny_skia::Color = (*color).into();
            paint.set_color(c);
            paint.anti_alias = true;

            // Draw shaft
            let mut pb = PathBuilder::new();
            pb.move_to(start.x, start.y);
            pb.line_to(end.x, end.y);
            if let Some(path) = pb.finish() {
                let stroke = Stroke {
                    width: *thickness,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }

            // Draw arrowhead (filled triangle)
            let points = arrowhead_points(*start, *end, *thickness);
            if points.len() == 3 {
                let mut pb = PathBuilder::new();
                pb.move_to(points[0].x, points[0].y);
                pb.line_to(points[1].x, points[1].y);
                pb.line_to(points[2].x, points[2].y);
                pb.close();
                if let Some(path) = pb.finish() {
                    pixmap.fill_path(
                        &path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                }
            }
        }
        Annotation::Pencil {
            points,
            color,
            thickness,
        } => {
            if points.is_empty() {
                return;
            }
            if points.len() == 1 {
                let mut paint = Paint::default();
                paint.set_color((*color).into());
                paint.anti_alias = true;
                if let Some(path) =
                    PathBuilder::from_circle(points[0].x, points[0].y, *thickness / 2.)
                {
                    pixmap.fill_path(
                        &path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                }
                return;
            }
            let mut pb = PathBuilder::new();
            pb.move_to(points[0].x, points[0].y);
            for pair in points[1..].windows(2) {
                pb.quad_to(
                    pair[0].x,
                    pair[0].y,
                    (pair[0].x + pair[1].x) / 2.,
                    (pair[0].y + pair[1].y) / 2.,
                );
            }
            let end = points.last().unwrap();
            pb.line_to(end.x, end.y);
            if let Some(path) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color((*color).into());
                paint.anti_alias = true;
                let stroke = Stroke {
                    width: *thickness,
                    line_cap: tiny_skia::LineCap::Round,
                    line_join: tiny_skia::LineJoin::Round,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
        Annotation::Text {
            position,
            text,
            color,
            font_size,
        } => {
            render_text_annotation(pixmap, position, text, color, *font_size);
        }
        Annotation::Ellipse {
            center,
            radius_x,
            radius_y,
            color,
            thickness,
        } => {
            let mut paint = Paint::default();
            let c: tiny_skia::Color = (*color).into();
            paint.set_color(c);
            paint.anti_alias = true;

            let cx = center.x;
            let cy = center.y;
            let rx = *radius_x;
            let ry = *radius_y;
            let k: f32 = 0.5522848;
            let kx = rx * k;
            let ky = ry * k;

            let mut pb = PathBuilder::new();
            pb.move_to(cx + rx, cy);
            pb.cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry);
            pb.cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy);
            pb.cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry);
            pb.cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy);
            pb.close();
            if let Some(path) = pb.finish() {
                let stroke = Stroke {
                    width: *thickness,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
        Annotation::Line {
            start,
            end,
            color,
            thickness,
        } => {
            let mut paint = Paint::default();
            let c: tiny_skia::Color = (*color).into();
            paint.set_color(c);
            paint.anti_alias = true;

            let mut pb = PathBuilder::new();
            pb.move_to(start.x, start.y);
            pb.line_to(end.x, end.y);
            if let Some(path) = pb.finish() {
                let stroke = Stroke {
                    width: *thickness,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
        Annotation::Highlight {
            top_left,
            size,
            color,
        } => {
            let rect = tiny_skia::Rect::from_xywh(top_left.x, top_left.y, size.width, size.height);
            if let Some(rect) = rect {
                let mut paint = Paint::default();
                let c = tiny_skia::Color::from_rgba(color.r, color.g, color.b, 0.35);
                if let Some(c) = c {
                    paint.set_color(c);
                }
                // Multiply blend like a real highlighter: dark ink stays readable,
                // light backgrounds take the tint, and overlaps don't wash out.
                paint.blend_mode = tiny_skia::BlendMode::Multiply;
                paint.anti_alias = false;
                pixmap.fill_rect(rect, &paint, Transform::identity(), None);
            }
        }
        Annotation::StepMarker {
            position,
            number,
            color,
            size,
        } => {
            let radius = size / 2.0;

            // Draw filled circle using bezier approximation
            let k: f32 = 0.5522848;
            let kr = radius * k;
            let mut pb = PathBuilder::new();
            pb.move_to(position.x + radius, position.y);
            pb.cubic_to(
                position.x + radius,
                position.y + kr,
                position.x + kr,
                position.y + radius,
                position.x,
                position.y + radius,
            );
            pb.cubic_to(
                position.x - kr,
                position.y + radius,
                position.x - radius,
                position.y + kr,
                position.x - radius,
                position.y,
            );
            pb.cubic_to(
                position.x - radius,
                position.y - kr,
                position.x - kr,
                position.y - radius,
                position.x,
                position.y - radius,
            );
            pb.cubic_to(
                position.x + kr,
                position.y - radius,
                position.x + radius,
                position.y - kr,
                position.x + radius,
                position.y,
            );
            pb.close();

            if let Some(path) = pb.finish() {
                // Fill with annotation color
                let mut paint = Paint::default();
                paint.set_color((*color).into());
                paint.anti_alias = true;
                pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    Transform::identity(),
                    None,
                );

                // White border
                let mut border = Paint::default();
                border.set_color(tiny_skia::Color::WHITE);
                border.anti_alias = true;
                let stroke = Stroke {
                    width: 2.0,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &border, &stroke, Transform::identity(), None);
            }

            // Black text for consistent readability on all circle colors
            let text_color = Color::new(0.0, 0.0, 0.0, 1.0);
            let num_str = number.to_string();
            let fs = size * 0.55;

            // Measure actual text dimensions using fontdue for precise centering
            let font = &*crate::font::FONT;

            // Measure total advance width
            let total_width: f32 = num_str
                .chars()
                .map(|ch| {
                    let (metrics, _) = font.rasterize(ch, fs);
                    metrics.advance_width
                })
                .sum();

            // Get line metrics for vertical centering
            let line_metrics = font.horizontal_line_metrics(fs);
            let ascent = line_metrics.map(|m| m.ascent).unwrap_or(fs * 0.8);
            let descent = line_metrics.map(|m| m.descent.abs()).unwrap_or(fs * 0.2);
            let text_height = ascent + descent;

            // Center horizontally and vertically in the circle
            // render_text_annotation treats position as the TOP of the text
            let text_x = position.x - total_width * 0.5;
            let text_y = position.y - text_height * 0.5;
            render_text_annotation(
                pixmap,
                &Point::new(text_x, text_y),
                &num_str,
                &text_color,
                fs,
            );
        }
        Annotation::Pixelate {
            top_left,
            size,
            block_size,
        } => {
            let src_pixels = match screenshot_pixels {
                Some(p) => p,
                None => return,
            };
            let src_width = match screenshot_width {
                Some(w) => w as usize,
                None => return,
            };

            let bs = (*block_size).max(1) as usize;
            // Shrink the region when the origin is clamped to 0 so the pixelated
            // area matches the annotation rect instead of extending right/down.
            let rw = (size.width + top_left.x.min(0.0)).max(0.0) as usize;
            let rh = (size.height + top_left.y.min(0.0)).max(0.0) as usize;
            let rx = top_left.x.max(0.0) as usize;
            let ry = top_left.y.max(0.0) as usize;
            let pm_w = pixmap.width() as usize;
            let pm_h = pixmap.height() as usize;

            let mut by = 0;
            while by < rh {
                let bh = bs.min(rh - by);
                let mut bx = 0;
                while bx < rw {
                    let bw = bs.min(rw - bx);
                    let px_x = rx + bx;
                    let px_y = ry + by;

                    // Average source pixels in this block
                    let mut sum_r: u64 = 0;
                    let mut sum_g: u64 = 0;
                    let mut sum_b: u64 = 0;
                    let mut count: u64 = 0;

                    for row in 0..bh {
                        for col in 0..bw {
                            let sx = px_x + col;
                            let sy = px_y + row;
                            if sx < src_width {
                                let si = (sy * src_width + sx) * 4;
                                if si + 3 < src_pixels.len() {
                                    sum_r += src_pixels[si] as u64;
                                    sum_g += src_pixels[si + 1] as u64;
                                    sum_b += src_pixels[si + 2] as u64;
                                    count += 1;
                                }
                            }
                        }
                    }

                    if let (Some(avg_r), Some(avg_g), Some(avg_b)) = (
                        sum_r.checked_div(count),
                        sum_g.checked_div(count),
                        sum_b.checked_div(count),
                    ) {
                        let avg_r = avg_r as f32 / 255.0;
                        let avg_g = avg_g as f32 / 255.0;
                        let avg_b = avg_b as f32 / 255.0;

                        // Clamp fill rect to pixmap bounds
                        let fill_x = px_x.min(pm_w);
                        let fill_y = px_y.min(pm_h);
                        let fill_w = bw.min(pm_w.saturating_sub(fill_x));
                        let fill_h = bh.min(pm_h.saturating_sub(fill_y));

                        if fill_w > 0 && fill_h > 0 {
                            if let Some(rect) = tiny_skia::Rect::from_xywh(
                                fill_x as f32,
                                fill_y as f32,
                                fill_w as f32,
                                fill_h as f32,
                            ) {
                                let mut paint = Paint::default();
                                if let Some(c) =
                                    tiny_skia::Color::from_rgba(avg_r, avg_g, avg_b, 1.0)
                                {
                                    paint.set_color(c);
                                } else {
                                    paint.set_color(tiny_skia::Color::BLACK);
                                }
                                paint.anti_alias = false;
                                pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                            }
                        }
                    }

                    bx += bs;
                }
                by += bs;
            }
        }
        Annotation::RoundedRect {
            top_left,
            size,
            color,
            thickness,
            radius,
        } => {
            if let Some(path) =
                rounded_rect_path(top_left.x, top_left.y, size.width, size.height, *radius)
            {
                let mut paint = Paint::default();
                paint.set_color((*color).into());
                paint.anti_alias = true;
                let stroke = Stroke {
                    width: *thickness,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
        Annotation::Spotlight { .. } => {
            // Spotlight rendering is handled in renderer.rs where we have access
            // to the original screenshot pixmap for restoring bright cutout areas.
        }
        Annotation::Measurement { start, end, color } => {
            let mut paint = Paint::default();
            paint.set_color((*color).into());
            paint.anti_alias = true;

            // Solid measurement line (2px, clearly visible)
            let mut pb = PathBuilder::new();
            pb.move_to(start.x, start.y);
            pb.line_to(end.x, end.y);
            if let Some(path) = pb.finish() {
                let stroke = Stroke {
                    width: 2.0,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }

            // Perpendicular end caps (crossbars at each endpoint, like a ruler)
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.1 {
                let px = -dy / len * 7.0;
                let py = dx / len * 7.0;
                for pt in [start, end] {
                    let mut pb = PathBuilder::new();
                    pb.move_to(pt.x + px, pt.y + py);
                    pb.line_to(pt.x - px, pt.y - py);
                    if let Some(path) = pb.finish() {
                        let stroke = Stroke {
                            width: 2.0,
                            ..Stroke::default()
                        };
                        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                    }
                }
            }

            // Endpoint dots (small filled circles for extra visibility)
            for pt in [start, end] {
                let r = 3.0;
                let k: f32 = 0.5522848;
                let mut pb = PathBuilder::new();
                pb.move_to(pt.x + r, pt.y);
                pb.cubic_to(
                    pt.x + r,
                    pt.y + r * k,
                    pt.x + r * k,
                    pt.y + r,
                    pt.x,
                    pt.y + r,
                );
                pb.cubic_to(
                    pt.x - r * k,
                    pt.y + r,
                    pt.x - r,
                    pt.y + r * k,
                    pt.x - r,
                    pt.y,
                );
                pb.cubic_to(
                    pt.x - r,
                    pt.y - r * k,
                    pt.x - r * k,
                    pt.y - r,
                    pt.x,
                    pt.y - r,
                );
                pb.cubic_to(
                    pt.x + r * k,
                    pt.y - r,
                    pt.x + r,
                    pt.y - r * k,
                    pt.x + r,
                    pt.y,
                );
                pb.close();
                if let Some(path) = pb.finish() {
                    pixmap.fill_path(
                        &path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                }
            }

            // Distance label at midpoint
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let distance = (dx * dx + dy * dy).sqrt();
            let label = format!("{:.0} px", distance);
            let mid_x = (start.x + end.x) / 2.0;
            let mid_y = (start.y + end.y) / 2.0;

            // Offset label slightly above the line
            let label_y = mid_y - 12.0;

            // Background pill for readability
            let font_size = 12.0;
            let label_w = label.len() as f32 * font_size * 0.6 + 8.0;
            let label_h = font_size + 6.0;
            let pill_x = mid_x - label_w / 2.0;
            let pill_y = label_y - 2.0;

            // Draw background pill
            if let Some(pill) = tiny_skia::Rect::from_xywh(pill_x, pill_y, label_w, label_h) {
                let mut bg = Paint::default();
                bg.set_color(crate::theme::skia(crate::theme::bg_1(), 0.85));
                bg.anti_alias = false;
                pixmap.fill_rect(pill, &bg, Transform::identity(), None);
            }

            // Draw text
            render_text_annotation(
                pixmap,
                &Point::new(pill_x + 4.0, pill_y + 2.0),
                &label,
                &crate::theme::gcolor(crate::theme::text_normal(), 1.0),
                font_size,
            );
        }
    }
}

/// Measure the rendered width of single-line text using the embedded font's
/// advance metrics — matches exactly what `render_text_annotation` produces.
pub fn measure_text_width(text: &str, font_size: f32) -> f32 {
    let font = crate::font::FONT.for_text(text);
    text.chars()
        .map(|ch| font.metrics(ch, font_size).advance_width)
        .sum()
}

/// Measure the line height (ascent + descent) of the embedded font at a size.
pub fn measure_text_height(font_size: f32) -> f32 {
    let font = &*crate::font::FONT;
    font.horizontal_line_metrics(font_size)
        .map(|m| m.ascent + m.descent.abs())
        .unwrap_or(font_size)
}
pub fn text_ink_bounds(text: &str, size: f32) -> (f32, f32, f32, f32) {
    let font = crate::font::FONT.for_text(text);
    let ascent = font
        .horizontal_line_metrics(size)
        .map_or(size, |m| m.ascent);
    let (mut x0, mut y0, mut x1, mut y1, mut cursor) = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
        0.,
    );
    for ch in text.chars() {
        let m = font.metrics(ch, size);
        if m.width > 0 && m.height > 0 {
            let x = (cursor as f32).round() + m.xmin as f32;
            let y = ascent.round() - m.ymin as f32 - m.height as f32;
            x0 = x0.min(x);
            x1 = x1.max(x + m.width as f32);
            y0 = y0.min(y);
            y1 = y1.max(y + m.height as f32);
        }
        cursor += m.advance_width;
    }
    if x0.is_finite() {
        (x0, y0, x1 - x0, y1 - y0)
    } else {
        (0., 0., cursor, size)
    }
}

/// Rasterize a single-line text annotation onto a pixmap using fontdue.
pub fn render_text_annotation(
    pixmap: &mut Pixmap,
    position: &Point,
    text: &str,
    color: &Color,
    font_size: f32,
) {
    let font = crate::font::FONT.for_text(text);

    let pw = pixmap.width() as i32;
    let ph = pixmap.height() as i32;

    let sr = (color.r * 255.0 + 0.5) as u8;
    let sg = (color.g * 255.0 + 0.5) as u8;
    let sb = (color.b * 255.0 + 0.5) as u8;
    let sa = color.a;

    let mut cursor_x = position.x;
    // Treat click position as the TOP of the text line.
    // Compute baseline from font metrics: baseline = top + ascent
    let line_metrics = font.horizontal_line_metrics(font_size);
    let ascent = line_metrics.map(|m| m.ascent).unwrap_or(font_size * 0.8);
    let baseline_y = position.y + ascent;

    let pixels = pixmap.data_mut();

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, font_size);
        if metrics.width == 0 || metrics.height == 0 {
            cursor_x += metrics.advance_width;
            continue;
        }

        // fontdue: ymin is the distance from baseline to the bottom of the glyph bitmap
        // glyph top = baseline - (height + ymin) ... but we use the standard formula:
        // glyph_y = baseline - glyph_top_offset, where glyph_top_offset = height + ymin
        let glyph_x = cursor_x.round() as i32 + metrics.xmin;
        let glyph_y = baseline_y.round() as i32 - metrics.height as i32 - metrics.ymin;

        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let px_x = glyph_x + col as i32;
                let px_y = glyph_y + row as i32;

                if px_x < 0 || px_x >= pw || px_y < 0 || px_y >= ph {
                    continue;
                }

                let coverage = bitmap[row * metrics.width + col] as f32 / 255.0;
                let alpha = coverage * sa;
                if alpha < 1.0 / 255.0 {
                    continue;
                }

                let idx = ((px_y as u32 * pw as u32 + px_x as u32) * 4) as usize;

                // Alpha-blend: source (straight) over destination (premultiplied).
                // Keep everything in f32 to avoid precision loss at low alpha.
                let src_r_f = sr as f32 * alpha;
                let src_g_f = sg as f32 * alpha;
                let src_b_f = sb as f32 * alpha;
                let src_a_f = alpha * 255.0;

                let inv_alpha = 1.0 - alpha;
                let out_r = src_r_f + pixels[idx] as f32 * inv_alpha;
                let out_g = src_g_f + pixels[idx + 1] as f32 * inv_alpha;
                let out_b = src_b_f + pixels[idx + 2] as f32 * inv_alpha;
                let out_a = src_a_f + pixels[idx + 3] as f32 * inv_alpha;

                pixels[idx] = (out_r + 0.5).min(255.0) as u8;
                pixels[idx + 1] = (out_g + 0.5).min(255.0) as u8;
                pixels[idx + 2] = (out_b + 0.5).min(255.0) as u8;
                pixels[idx + 3] = (out_a + 0.5).min(255.0) as u8;
            }
        }

        cursor_x += metrics.advance_width;
    }
}

/// Distance from a point to a line segment.
fn point_to_segment_distance(point: &Point, a: &Point, b: &Point) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 0.001 {
        return ((point.x - a.x).powi(2) + (point.y - a.y).powi(2)).sqrt();
    }
    let t = ((point.x - a.x) * dx + (point.y - a.y) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj_x = a.x + t * dx;
    let proj_y = a.y + t * dy;
    ((point.x - proj_x).powi(2) + (point.y - proj_y).powi(2)).sqrt()
}

/// Test if a point is close enough to an annotation to "hit" it.
/// Returns true if the point is within `threshold` pixels of the annotation.
pub fn hit_test_annotation(annotation: &Annotation, point: &Point, threshold: f32) -> bool {
    match annotation {
        Annotation::Arrow {
            start,
            end,
            thickness,
            ..
        }
        | Annotation::Line {
            start,
            end,
            thickness,
            ..
        } => point_to_segment_distance(point, start, end) < threshold + thickness / 2.0,
        Annotation::Measurement { start, end, .. } => {
            point_to_segment_distance(point, start, end) < threshold + 1.0
        }
        Annotation::Rectangle { top_left, size, .. }
        | Annotation::RoundedRect { top_left, size, .. } => {
            let r = tiny_skia::Rect::from_xywh(top_left.x, top_left.y, size.width, size.height);
            if let Some(r) = r {
                let near_left = (point.x - r.left()).abs() < threshold
                    && point.y >= r.top() - threshold
                    && point.y <= r.bottom() + threshold;
                let near_right = (point.x - r.right()).abs() < threshold
                    && point.y >= r.top() - threshold
                    && point.y <= r.bottom() + threshold;
                let near_top = (point.y - r.top()).abs() < threshold
                    && point.x >= r.left() - threshold
                    && point.x <= r.right() + threshold;
                let near_bottom = (point.y - r.bottom()).abs() < threshold
                    && point.x >= r.left() - threshold
                    && point.x <= r.right() + threshold;
                near_left || near_right || near_top || near_bottom
            } else {
                false
            }
        }
        Annotation::Ellipse {
            center,
            radius_x,
            radius_y,
            thickness,
            ..
        } => {
            if *radius_x < 0.001 || *radius_y < 0.001 {
                return false;
            }
            let nx = (point.x - center.x) / radius_x;
            let ny = (point.y - center.y) / radius_y;
            let dist = (nx * nx + ny * ny).sqrt();
            (dist - 1.0).abs()
                < threshold / radius_x.min(*radius_y) + thickness / (2.0 * radius_x.min(*radius_y))
        }
        Annotation::Highlight { top_left, size, .. }
        | Annotation::Pixelate { top_left, size, .. }
        | Annotation::Spotlight { top_left, size } => {
            point.x >= top_left.x
                && point.x <= top_left.x + size.width
                && point.y >= top_left.y
                && point.y <= top_left.y + size.height
        }
        Annotation::Pencil {
            points, thickness, ..
        } => points.windows(2).any(|seg| {
            point_to_segment_distance(point, &seg[0], &seg[1]) < threshold + thickness / 2.0
        }),
        Annotation::Text {
            position,
            font_size,
            text,
            ..
        } => {
            let font = crate::font::FONT.for_text(text);
            let text_width: f32 = text
                .chars()
                .map(|c| font.metrics(c, *font_size).advance_width)
                .sum();
            let text_height = font
                .horizontal_line_metrics(*font_size)
                .map_or(*font_size, |m| m.ascent - m.descent);
            point.x >= position.x - threshold
                && point.x <= position.x + text_width + threshold
                && point.y >= position.y - threshold
                && point.y <= position.y + text_height + threshold
        }
        Annotation::StepMarker { position, size, .. } => {
            let dx = point.x - position.x;
            let dy = point.y - position.y;
            (dx * dx + dy * dy).sqrt() < size / 2.0 + threshold
        }
    }
}

/// Move an annotation by (dx, dy).
pub fn move_annotation(annotation: &mut Annotation, dx: f32, dy: f32) {
    match annotation {
        Annotation::Arrow { start, end, .. }
        | Annotation::Line { start, end, .. }
        | Annotation::Measurement { start, end, .. } => {
            start.x += dx;
            start.y += dy;
            end.x += dx;
            end.y += dy;
        }
        Annotation::Rectangle { top_left, .. }
        | Annotation::RoundedRect { top_left, .. }
        | Annotation::Highlight { top_left, .. }
        | Annotation::Pixelate { top_left, .. }
        | Annotation::Spotlight { top_left, .. } => {
            top_left.x += dx;
            top_left.y += dy;
        }
        Annotation::Ellipse { center, .. } => {
            center.x += dx;
            center.y += dy;
        }
        Annotation::Pencil { points, .. } => {
            for p in points.iter_mut() {
                p.x += dx;
                p.y += dy;
            }
        }
        Annotation::Text { position, .. } | Annotation::StepMarker { position, .. } => {
            position.x += dx;
            position.y += dy;
        }
    }
}

/// Update the color of an annotation.
pub fn recolor_annotation(annotation: &mut Annotation, new_color: Color) {
    match annotation {
        Annotation::Arrow { color, .. }
        | Annotation::Rectangle { color, .. }
        | Annotation::RoundedRect { color, .. }
        | Annotation::Ellipse { color, .. }
        | Annotation::Line { color, .. }
        | Annotation::Pencil { color, .. }
        | Annotation::Highlight { color, .. }
        | Annotation::Text { color, .. }
        | Annotation::StepMarker { color, .. }
        | Annotation::Measurement { color, .. } => {
            *color = new_color;
        }
        Annotation::Pixelate { .. } | Annotation::Spotlight { .. } => {} // no color
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Resize an annotation by dragging a corner handle to a new position.
pub fn resize_annotation(annotation: &mut Annotation, handle: ResizeHandle, new_pos: Point) {
    if let Some((bx, by, bw, bh)) = annotation_bounding_box(annotation) {
        // Compute the new bounding box based on which corner was dragged
        let (new_x, new_y, new_w, new_h) = match handle {
            ResizeHandle::TopLeft => {
                let new_w = (bx + bw) - new_pos.x;
                let new_h = (by + bh) - new_pos.y;
                (new_pos.x, new_pos.y, new_w, new_h)
            }
            ResizeHandle::TopRight => {
                let new_w = new_pos.x - bx;
                let new_h = (by + bh) - new_pos.y;
                (bx, new_pos.y, new_w, new_h)
            }
            ResizeHandle::BottomLeft => {
                let new_w = (bx + bw) - new_pos.x;
                let new_h = new_pos.y - by;
                (new_pos.x, by, new_w, new_h)
            }
            ResizeHandle::BottomRight => {
                let new_w = new_pos.x - bx;
                let new_h = new_pos.y - by;
                (bx, by, new_w, new_h)
            }
        };

        // Don't allow negative/zero size
        if new_w < 4.0 || new_h < 4.0 {
            return;
        }

        // Guard against degenerate bounding box (division by zero)
        if bw < 0.001 || bh < 0.001 {
            return;
        }

        // Scale the annotation to fit the new bounding box
        let sx = new_w / bw;
        let sy = new_h / bh;

        // Apply transform based on annotation type
        match annotation {
            Annotation::Arrow { start, end, .. }
            | Annotation::Line { start, end, .. }
            | Annotation::Measurement { start, end, .. } => {
                start.x = new_x + (start.x - bx) * sx;
                start.y = new_y + (start.y - by) * sy;
                end.x = new_x + (end.x - bx) * sx;
                end.y = new_y + (end.y - by) * sy;
            }
            Annotation::Rectangle { top_left, size, .. }
            | Annotation::RoundedRect { top_left, size, .. }
            | Annotation::Highlight { top_left, size, .. }
            | Annotation::Pixelate { top_left, size, .. }
            | Annotation::Spotlight { top_left, size } => {
                top_left.x = new_x;
                top_left.y = new_y;
                size.width = new_w;
                size.height = new_h;
            }
            Annotation::Ellipse {
                center,
                radius_x,
                radius_y,
                ..
            } => {
                *center = Point::new(new_x + new_w / 2.0, new_y + new_h / 2.0);
                *radius_x = new_w / 2.0;
                *radius_y = new_h / 2.0;
            }
            Annotation::Pencil { points, .. } => {
                for p in points.iter_mut() {
                    p.x = new_x + (p.x - bx) * sx;
                    p.y = new_y + (p.y - by) * sy;
                }
            }
            Annotation::Text {
                position,
                font_size,
                ..
            } => {
                position.x = new_x;
                position.y = new_y;
                *font_size *= sy; // scale font size vertically
            }
            Annotation::StepMarker { position, size, .. } => {
                *position = Point::new(new_x + new_w / 2.0, new_y + new_h / 2.0);
                *size = new_w.min(new_h);
            }
        }
    }
}

/// Compute the bounding box of an annotation as (x, y, w, h).
pub fn annotation_bounding_box(annotation: &Annotation) -> Option<(f32, f32, f32, f32)> {
    match annotation {
        Annotation::Arrow {
            start,
            end,
            thickness,
            ..
        }
        | Annotation::Line {
            start,
            end,
            thickness,
            ..
        } => {
            let min_x = start.x.min(end.x) - thickness / 2.0;
            let min_y = start.y.min(end.y) - thickness / 2.0;
            let max_x = start.x.max(end.x) + thickness / 2.0;
            let max_y = start.y.max(end.y) + thickness / 2.0;
            Some((min_x, min_y, max_x - min_x, max_y - min_y))
        }
        Annotation::Measurement { start, end, .. } => {
            let pad = 3.0; // endpoint circle radius
            let min_x = start.x.min(end.x) - pad;
            let min_y = start.y.min(end.y) - pad;
            let max_x = start.x.max(end.x) + pad;
            let max_y = start.y.max(end.y) + pad;
            Some((min_x, min_y, max_x - min_x, max_y - min_y))
        }
        Annotation::Rectangle { top_left, size, .. }
        | Annotation::RoundedRect { top_left, size, .. } => {
            Some((top_left.x, top_left.y, size.width, size.height))
        }
        Annotation::Ellipse {
            center,
            radius_x,
            radius_y,
            ..
        } => Some((
            center.x - radius_x,
            center.y - radius_y,
            radius_x * 2.0,
            radius_y * 2.0,
        )),
        Annotation::Highlight { top_left, size, .. }
        | Annotation::Pixelate { top_left, size, .. }
        | Annotation::Spotlight { top_left, size } => {
            Some((top_left.x, top_left.y, size.width, size.height))
        }
        Annotation::Pencil {
            points, thickness, ..
        } => {
            if points.is_empty() {
                return None;
            }
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for p in points {
                min_x = min_x.min(p.x);
                min_y = min_y.min(p.y);
                max_x = max_x.max(p.x);
                max_y = max_y.max(p.y);
            }
            let half = thickness / 2.0;
            Some((
                min_x - half,
                min_y - half,
                max_x - min_x + *thickness,
                max_y - min_y + *thickness,
            ))
        }
        Annotation::Text {
            position,
            font_size,
            text,
            ..
        } => {
            let (x, y, w, h) = text_ink_bounds(text, *font_size);
            Some((position.x + x, position.y + y, w, h))
        }
        Annotation::StepMarker { position, size, .. } => {
            let half = size / 2.0;
            Some((position.x - half, position.y - half, *size, *size))
        }
    }
}
