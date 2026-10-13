//! Viewport-sized editing. Original capture pixels are touched only for sampling/export.
use crate::{
    geometry::Point,
    preview::EditorView,
    state::OverlayState,
    tools::{self, Annotation},
    ui,
};
use tiny_skia::{Paint, PathBuilder, Pixmap, Stroke, Transform};
#[derive(Default)]
pub struct PreviewRenderer {
    base: Option<Pixmap>,
    canvas: Option<Pixmap>,
    key: Option<(usize, u32, u32, u32, u32, u64, u64, u64)>,
    origin: (u32, u32),
    pub resamples: u64,
}
fn point(p: Point, s: f32, ox: f32, oy: f32) -> Point {
    Point::new(p.x * s + ox, p.y * s + oy)
}
pub fn transformed(a: &Annotation, s: f32, ox: f32, oy: f32) -> Annotation {
    let mut a = a.clone();
    match &mut a {
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
            *start = point(*start, s, ox, oy);
            *end = point(*end, s, ox, oy);
            *thickness *= s;
        }
        Annotation::Measurement { start, end, .. } => {
            *start = point(*start, s, ox, oy);
            *end = point(*end, s, ox, oy);
        }
        Annotation::Rectangle {
            top_left,
            size,
            thickness,
            ..
        } => {
            *top_left = point(*top_left, s, ox, oy);
            size.width *= s;
            size.height *= s;
            *thickness *= s;
        }
        Annotation::RoundedRect {
            top_left,
            size,
            thickness,
            radius,
            ..
        } => {
            *top_left = point(*top_left, s, ox, oy);
            size.width *= s;
            size.height *= s;
            *thickness *= s;
            *radius *= s;
        }
        Annotation::Highlight { top_left, size, .. }
        | Annotation::Pixelate { top_left, size, .. }
        | Annotation::Spotlight { top_left, size } => {
            *top_left = point(*top_left, s, ox, oy);
            size.width *= s;
            size.height *= s;
        }
        Annotation::Ellipse {
            center,
            radius_x,
            radius_y,
            thickness,
            ..
        } => {
            *center = point(*center, s, ox, oy);
            *radius_x *= s;
            *radius_y *= s;
            *thickness *= s;
        }
        Annotation::Pencil {
            points, thickness, ..
        } => {
            for p in points {
                *p = point(*p, s, ox, oy);
            }
            *thickness *= s;
        }
        Annotation::Text {
            position,
            font_size,
            ..
        } => {
            *position = point(*position, s, ox, oy);
            *font_size *= s;
        }
        Annotation::StepMarker { position, size, .. } => {
            *position = point(*position, s, ox, oy);
            *size *= s;
        }
    }
    a
}
fn rect(p: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, rgb: (u8, u8, u8), alpha: f32) {
    if let Some(r) = tiny_skia::Rect::from_xywh(x, y, w, h) {
        let mut paint = Paint::default();
        paint.set_color(ui_color(rgb, alpha));
        p.fill_rect(r, &paint, Transform::identity(), None);
    }
}
fn ui_color(rgb: (u8, u8, u8), a: f32) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(rgb.0, rgb.1, rgb.2, (a * 255.).round() as u8)
}
fn guide(p: &mut Pixmap, a: &Annotation, dpi: f32, handles: bool) {
    if let Some((x, y, w, h)) = tools::annotation_bounding_box(a) {
        if let Some(r) = tiny_skia::Rect::from_xywh(x, y, w.max(1.), h.max(1.)) {
            let mut paint = Paint::default();
            paint.anti_alias = true;
            paint.set_color(ui_color((255, 255, 255), 0.9));
            p.stroke_path(
                &PathBuilder::from_rect(r),
                &paint,
                &Stroke {
                    width: 3. * dpi,
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );
            if handles {
                for (cx, cy) in [(x, y), (x + w, y), (x, y + h), (x + w, y + h)] {
                    rect(
                        p,
                        cx - 3. * dpi,
                        cy - 3. * dpi,
                        6. * dpi,
                        6. * dpi,
                        (255, 255, 255),
                        1.,
                    );
                    rect(
                        p,
                        cx - 2. * dpi,
                        cy - 2. * dpi,
                        4. * dpi,
                        4. * dpi,
                        ui::accent(),
                        1.,
                    );
                }
            }
            paint.set_color(ui_color(ui::accent(), 1.));
            p.stroke_path(
                &PathBuilder::from_rect(r),
                &paint,
                &Stroke {
                    width: dpi,
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );
        }
    }
}
impl PreviewRenderer {
    pub fn retained_bytes(&self) -> usize {
        self.base.as_ref().map_or(0, |p| p.data().len())
            + self.canvas.as_ref().map_or(0, |p| p.data().len())
    }
    pub fn draw(&mut self, state: &OverlayState, frame: &mut Pixmap, view: &EditorView, dpi: f64) {
        let screen = &state.screenshot;
        let l = view.framed_layout(
            screen.width,
            screen.height,
            frame.width(),
            frame.height(),
            dpi,
        );
        let well = crate::editor_ui::EditorUi::canvas_rect(
            frame.width() as f32 / dpi as f32,
            frame.height() as f32 / dpi as f32,
        );
        let x0 = (l.x.max(well.x as f64 * dpi).max(0.)).ceil() as u32;
        let y0 = (l.y.max(well.y as f64 * dpi).max(0.)).ceil() as u32;
        let x1 = (l.x + screen.width as f64 * l.scale)
            .min((well.x + well.w) as f64 * dpi)
            .min(frame.width() as f64)
            .floor()
            .max(0.) as u32;
        let y1 = (l.y + screen.height as f64 * l.scale)
            .min((well.y + well.h) as f64 * dpi)
            .min(frame.height() as f64)
            .floor()
            .max(0.) as u32;
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let (w, h) = (x1 - x0, y1 - y0);
        let ox = l.x as f32 - x0 as f32;
        let oy = l.y as f32 - y0 as f32;
        let scale = l.scale as f32;
        let key = (
            screen.pixels.as_ptr() as usize,
            screen.width,
            screen.height,
            w,
            h,
            l.x.to_bits(),
            l.y.to_bits(),
            l.scale.to_bits(),
        );
        if self.key != Some(key) || self.origin != (x0, y0) {
            if self
                .base
                .as_ref()
                .map_or(true, |p| (p.width(), p.height()) != (w, h))
            {
                self.base = Pixmap::new(w, h);
                self.canvas = Pixmap::new(w, h);
            }
            let Some(base) = &mut self.base else {
                return;
            };
            for y in 0..h {
                for x in 0..w {
                    let sx = ((x0 as f64 + x as f64 + 0.5 - l.x) / l.scale - 0.5)
                        .clamp(0., screen.width.saturating_sub(1) as f64);
                    let sy = ((y0 as f64 + y as f64 + 0.5 - l.y) / l.scale - 0.5)
                        .clamp(0., screen.height.saturating_sub(1) as f64);
                    let (ix, iy) = (sx.floor() as u32, sy.floor() as u32);
                    let (jx, jy) = (
                        (ix + 1).min(screen.width - 1),
                        (iy + 1).min(screen.height - 1),
                    );
                    let (fx, fy) = ((sx - ix as f64) as f32, (sy - iy as f64) as f32);
                    let dest = ((y * w + x) * 4) as usize;
                    for c in 0..4 {
                        let sample = |xx, yy| {
                            screen.pixels[((yy * screen.width + xx) * 4) as usize + c] as f32
                        };
                        let top = sample(ix, iy) * (1. - fx) + sample(jx, iy) * fx;
                        let bottom = sample(ix, jy) * (1. - fx) + sample(jx, jy) * fx;
                        base.data_mut()[dest + c] = (top * (1. - fy) + bottom * fy).round() as u8;
                    }
                }
            }
            self.key = Some(key);
            self.origin = (x0, y0);
            self.resamples += 1;
        }
        let (Some(base), Some(canvas)) = (&self.base, &mut self.canvas) else {
            return;
        };
        canvas.data_mut().copy_from_slice(base.data());
        let progress = state.in_progress();
        let spots: Vec<_> = state
            .annotations
            .iter()
            .chain(progress.iter())
            .filter_map(|a| {
                if let Annotation::Spotlight { .. } = a {
                    Some(transformed(a, scale, ox, oy))
                } else {
                    None
                }
            })
            .collect();
        if !spots.is_empty() {
            rect(canvas, 0., 0., w as f32, h as f32, (0, 0, 0), 0.5);
            for a in spots {
                if let Some((x, y, rw, rh)) = tools::annotation_bounding_box(&a) {
                    let xe = (x + rw).clamp(0., w as f32) as u32;
                    let ye = (y + rh).clamp(0., h as f32) as u32;
                    let x = x.max(0.) as u32;
                    let y = y.max(0.) as u32;
                    for row in y..ye {
                        if xe > x {
                            let i = ((row * w + x) * 4) as usize;
                            let end = ((row * w + xe) * 4) as usize;
                            canvas.data_mut()[i..end].copy_from_slice(&base.data()[i..end]);
                        }
                    }
                }
            }
        }
        for a in &state.annotations {
            draw_annotation(canvas, a, screen, scale, ox, oy, dpi as f32);
        }
        if let Some(a) = state.selected_index.and_then(|i| state.annotations.get(i)) {
            guide(canvas, &transformed(a, scale, ox, oy), dpi as f32, true);
        }
        if let Some(a) = progress {
            draw_annotation(canvas, &a, screen, scale, ox, oy, dpi as f32);
            if matches!(
                a,
                Annotation::Highlight { .. } | Annotation::Pixelate { .. }
            ) {
                guide(canvas, &transformed(&a, scale, ox, oy), dpi as f32, false);
            }
        }
        if state.text_input_active && !state.text_input_buffer.is_empty() {
            draw_annotation(
                canvas,
                &Annotation::Text {
                    position: state.text_input_position,
                    text: state.text_input_buffer.clone(),
                    color: state.current_color,
                    font_size: state.text_input_font_size,
                },
                screen,
                scale,
                ox,
                oy,
                dpi as f32,
            );
        }
        if state.active_tool == crate::tools::ToolKind::Eyedropper {
            if let Some(color) = state.eyedropper_preview {
                let at = point(state.last_mouse_pos, scale, ox, oy);
                let x = (at.x + 14. * dpi as f32).clamp(0., (w as f32 - 100. * dpi as f32).max(0.));
                let y = (at.y + 14. * dpi as f32).clamp(0., (h as f32 - 28. * dpi as f32).max(0.));
                rect(
                    canvas,
                    x,
                    y,
                    100. * dpi as f32,
                    28. * dpi as f32,
                    ui::panel(),
                    1.,
                );
                rect(
                    canvas,
                    x + 5. * dpi as f32,
                    y + 5. * dpi as f32,
                    18. * dpi as f32,
                    18. * dpi as f32,
                    (
                        (color.r * 255.) as u8,
                        (color.g * 255.) as u8,
                        (color.b * 255.) as u8,
                    ),
                    1.,
                );
                tools::render_text_annotation(
                    canvas,
                    &Point::new(x + 28. * dpi as f32, y + 5. * dpi as f32),
                    &format!(
                        "#{:02X}{:02X}{:02X}",
                        (color.r * 255.) as u8,
                        (color.g * 255.) as u8,
                        (color.b * 255.) as u8
                    ),
                    &crate::theme::gcolor(ui::ink(), 1.),
                    10. * dpi as f32,
                );
            }
        }
        let fw = frame.width();
        for y in 0..h {
            let i = (y * w * 4) as usize;
            let dest = (((y + y0) * fw + x0) * 4) as usize;
            frame.data_mut()[dest..dest + (w * 4) as usize]
                .copy_from_slice(&canvas.data()[i..i + (w * 4) as usize]);
        }
    }
}
fn draw_annotation(
    p: &mut Pixmap,
    a: &Annotation,
    screen: &crate::capture::CapturedScreen,
    s: f32,
    ox: f32,
    oy: f32,
    dpi: f32,
) {
    match a {
        Annotation::Pixelate {
            top_left,
            size,
            block_size,
        } => {
            let (rx, ry) = (top_left.x.max(0.) as u32, top_left.y.max(0.) as u32);
            let rw = (size.width + top_left.x.min(0.)).max(0.) as u32;
            let rh = (size.height + top_left.y.min(0.)).max(0.) as u32;
            let bs = (*block_size).max(1) as u32;
            for by in (0..rh).step_by(bs as usize) {
                for bx in (0..rw).step_by(bs as usize) {
                    let bw = bs.min(rw - bx);
                    let bh = bs.min(rh - by);
                    let x = (rx + bx) as f32 * s + ox;
                    let y = (ry + by) as f32 * s + oy;
                    if x + bw as f32 * s < 0.
                        || y + bh as f32 * s < 0.
                        || x >= p.width() as f32
                        || y >= p.height() as f32
                    {
                        continue;
                    }
                    let mut sum = [0u64; 3];
                    let mut n = 0;
                    for yy in ry + by..(ry + by + bh).min(screen.height) {
                        for xx in rx + bx..(rx + bx + bw).min(screen.width) {
                            let i = ((yy * screen.width + xx) * 4) as usize;
                            for c in 0..3 {
                                sum[c] += screen.pixels[i + c] as u64;
                            }
                            n += 1;
                        }
                    }
                    if n > 0 {
                        rect(
                            p,
                            x,
                            y,
                            bw as f32 * s,
                            bh as f32 * s,
                            ((sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8),
                            1.,
                        );
                    }
                }
            }
        }
        Annotation::Measurement { start, end, color } => {
            let a = point(*start, s, ox, oy);
            let b = point(*end, s, ox, oy);
            let mut path = PathBuilder::new();
            path.move_to(a.x, a.y);
            path.line_to(b.x, b.y);
            let visible_len = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
            if visible_len > 0.1 {
                let px = -(b.y - a.y) / visible_len * 7. * dpi;
                let py = (b.x - a.x) / visible_len * 7. * dpi;
                for pt in [a, b] {
                    path.move_to(pt.x + px, pt.y + py);
                    path.line_to(pt.x - px, pt.y - py);
                }
            }
            let mut paint = Paint::default();
            paint.set_color((*color).into());
            paint.anti_alias = true;
            if let Some(path) = path.finish() {
                p.stroke_path(
                    &path,
                    &paint,
                    &Stroke {
                        width: 2. * dpi,
                        ..Default::default()
                    },
                    Transform::identity(),
                    None,
                );
            }
            for pt in [a, b] {
                if let Some(dot) = PathBuilder::from_circle(pt.x, pt.y, 3. * dpi) {
                    p.fill_path(
                        &dot,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                }
            }
            let length = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
            let label = format!("{length:.0} px");
            let label_width = tools::measure_text_width(&label, 12. * dpi) + 8. * dpi;
            let label_x = (a.x + b.x) / 2. - label_width / 2.;
            let label_y = (a.y + b.y) / 2. - 14. * dpi;
            rect(
                p,
                label_x,
                label_y,
                label_width,
                18. * dpi,
                ui::panel(),
                0.9,
            );
            tools::render_text_annotation(
                p,
                &Point::new(label_x + 4. * dpi, label_y + 2. * dpi),
                &label,
                &crate::theme::gcolor(ui::ink(), 1.),
                12. * dpi,
            );
        }
        _ => tools::render_annotation(&transformed(a, s, ox, oy), p, None, None),
    }
}
