//! Preview scaling and input share exactly the same letterbox transform.
use crate::geometry::Point;

fn transform(cw: u32, ch: u32, vw: u32, vh: u32) -> (f64, f64, f64) {
    let scale = (vw as f64 / cw.max(1) as f64)
        .min(vh as f64 / ch.max(1) as f64)
        .max(f64::EPSILON);
    (
        scale,
        (vw as f64 - cw as f64 * scale) / 2.0,
        (vh as f64 - ch as f64 * scale) / 2.0,
    )
}

pub fn image_point(x: f64, y: f64, cw: u32, ch: u32, vw: u32, vh: u32) -> Point {
    let (scale, ox, oy) = transform(cw, ch, vw, vh);
    Point::new(((x - ox) / scale) as f32, ((y - oy) / scale) as f32)
}

pub fn present(pixmap: &tiny_skia::Pixmap, dst: &mut [u32], vw: u32, vh: u32) {
    let src = pixmap.data();
    if pixmap.width() == vw && pixmap.height() == vh {
        for (pixel, rgba) in dst.iter_mut().zip(src.chunks_exact(4)) {
            *pixel = (rgba[0] as u32) << 16 | (rgba[1] as u32) << 8 | rgba[2] as u32;
        }
        return;
    }
    dst.fill(0x18181c);
    let (scale, ox, oy) = transform(pixmap.width(), pixmap.height(), vw, vh);
    // Presentation alone is scaled. Original pixels and annotation coordinates
    // stay unchanged, so clipboard/export retain the physical capture resolution.
    for y in 0..vh {
        let sy = ((y as f64 + 0.5 - oy) / scale).floor();
        if sy < 0.0 || sy >= pixmap.height() as f64 {
            continue;
        }
        for x in 0..vw {
            let sx = ((x as f64 + 0.5 - ox) / scale).floor();
            if sx < 0.0 || sx >= pixmap.width() as f64 {
                continue;
            }
            let i = (sy as usize * pixmap.width() as usize + sx as usize) * 4;
            if let Some(pixel) = dst.get_mut(y as usize * vw as usize + x as usize) {
                *pixel = (src[i] as u32) << 16 | (src[i + 1] as u32) << 8 | src[i + 2] as u32;
            }
        }
    }
}
/// Leave enough room for every configured control and a footer below the image.
pub fn canvas_size(width: u32, height: u32, buttons: usize) -> (u32, u32) {
    let toolbar_width =
        crate::overlay::toolbar::Toolbar::toolbar_width_dynamic(buttons).ceil() as u32;
    (
        width.max(toolbar_width.saturating_add(16)),
        height.saturating_add(64).max(400),
    )
}
pub fn controls_width(buttons: usize) -> u32 {
    (crate::overlay::toolbar::Toolbar::toolbar_width_dynamic(buttons).ceil() as u32)
        .saturating_add(16)
}

#[derive(Debug, Clone, Copy)]
pub struct EditorLayout {
    pub scale: f64,
    pub x: f64,
    pub y: f64,
    pub footer_scale: f64,
    pub footer_x: f64,
    pub footer_y: f64,
}
#[derive(Debug, Clone, Copy)]
pub struct EditorView {
    pub zoom: f64,
    pan_x: f64,
    pan_y: f64,
    pan_drag: Option<(f64, f64)>,
}
impl Default for EditorView {
    fn default() -> Self {
        Self {
            zoom: 1.,
            pan_x: 0.,
            pan_y: 0.,
            pan_drag: None,
        }
    }
}
impl EditorView {
    /// Space only recenters a quiescent Select tool, never typing or dragging.
    pub fn recenter_for_select(&mut self, state: &crate::state::OverlayState) -> bool {
        if state.active_tool != crate::tools::ToolKind::Select
            || state.text_input_active
            || state.select_drag_start.is_some()
            || state.resize_handle.is_some()
            || self.is_panning()
        {
            return false;
        }
        *self = Self::default();
        true
    }
    pub fn begin_pan(&mut self, x: f64, y: f64, vw: u32, vh: u32, dpi: f64) -> bool {
        if !x.is_finite() || !y.is_finite() || !dpi.is_finite() || dpi <= 0. {
            return false;
        }
        let canvas =
            crate::editor_ui::EditorUi::canvas_rect(vw as f32 / dpi as f32, vh as f32 / dpi as f32);
        if !canvas.contains(Point::new((x / dpi) as f32, (y / dpi) as f32)) {
            return false;
        }
        self.pan_drag = Some((x, y));
        true
    }
    pub fn is_panning(&self) -> bool {
        self.pan_drag.is_some()
    }
    pub fn drag_pan(&mut self, x: f64, y: f64) -> bool {
        let Some((old_x, old_y)) = self.pan_drag else {
            return false;
        };
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        self.pan_drag = Some((x, y));
        self.pan_x += x - old_x;
        self.pan_y += y - old_y;
        x != old_x || y != old_y
    }
    pub fn end_pan(&mut self) -> bool {
        self.pan_drag.take().is_some()
    }

    /// A generous pointer target stays the same logical size at every image zoom/DPI.
    pub fn annotation_hit_slop(&self, iw: u32, ih: u32, vw: u32, vh: u32, dpi: f64) -> f32 {
        (9. * dpi / self.framed_layout(iw, ih, vw, vh, dpi).scale) as f32
    }
    pub fn framed_layout(&self, iw: u32, ih: u32, vw: u32, vh: u32, dpi: f64) -> EditorLayout {
        let area =
            crate::editor_ui::EditorUi::image_rect(vw as f32 / dpi as f32, vh as f32 / dpi as f32);
        let aw = area.w as f64 * dpi;
        let ah = area.h as f64 * dpi;
        let base = (aw / iw.max(1) as f64)
            .min(ah / ih.max(1) as f64)
            .min(1.)
            .max(f64::EPSILON);
        let scale = base * self.zoom;
        EditorLayout {
            scale,
            x: area.x as f64 * dpi + (aw - iw as f64 * scale) / 2. + self.pan_x,
            y: area.y as f64 * dpi + (ah - ih as f64 * scale) / 2. + self.pan_y,
            footer_scale: dpi,
            footer_x: (vw as f64 - 612. * dpi) / 2.,
            footer_y: vh as f64 - 84. * dpi,
        }
    }
    pub fn framed_point(
        &self,
        x: f64,
        y: f64,
        iw: u32,
        ih: u32,
        vw: u32,
        vh: u32,
        dpi: f64,
    ) -> Point {
        let l = self.framed_layout(iw, ih, vw, vh, dpi);
        Point::new(((x - l.x) / l.scale) as f32, ((y - l.y) / l.scale) as f32)
    }
    pub fn zoom_framed(
        &mut self,
        x: f64,
        y: f64,
        steps: f64,
        iw: u32,
        ih: u32,
        vw: u32,
        vh: u32,
        dpi: f64,
    ) {
        if !steps.is_finite() || !x.is_finite() || !y.is_finite() {
            return;
        }
        let canvas =
            crate::editor_ui::EditorUi::canvas_rect(vw as f32 / dpi as f32, vh as f32 / dpi as f32);
        if !canvas.contains(Point::new((x / dpi) as f32, (y / dpi) as f32)) {
            return;
        }
        let old = self.framed_layout(iw, ih, vw, vh, dpi);
        let (px, py) = ((x - old.x) / old.scale, (y - old.y) / old.scale);
        self.zoom = (self.zoom * 1.2f64.powf(steps.clamp(-8., 8.))).clamp(0.1, 16.);
        let new = self.framed_layout(iw, ih, vw, vh, dpi);
        self.pan_x += x - (new.x + px * new.scale);
        self.pan_y += y - (new.y + py * new.scale);
    }
    pub fn paint_framed(
        &self,
        source: &tiny_skia::Pixmap,
        frame: &mut tiny_skia::Pixmap,
        dpi: f64,
    ) {
        let (vw, vh) = (frame.width(), frame.height());
        let l = self.framed_layout(source.width(), source.height(), vw, vh, dpi);
        let canvas =
            crate::editor_ui::EditorUi::canvas_rect(vw as f32 / dpi as f32, vh as f32 / dpi as f32);
        let x0 = (canvas.x as f64 * dpi).max(0.) as u32;
        let y0 = (canvas.y as f64 * dpi).max(0.) as u32;
        let x1 = ((canvas.x + canvas.w) as f64 * dpi).min(vw as f64) as u32;
        let y1 = ((canvas.y + canvas.h) as f64 * dpi).min(vh as f64) as u32;
        for y in y0..y1 {
            let sy = ((y as f64 + 0.5 - l.y) / l.scale).floor();
            if sy < 0. || sy >= source.height() as f64 {
                continue;
            }
            for x in x0..x1 {
                let sx = ((x as f64 + 0.5 - l.x) / l.scale).floor();
                if sx < 0. || sx >= source.width() as f64 {
                    continue;
                }
                let i = (sy as usize * source.width() as usize + sx as usize) * 4;
                let j = (y as usize * vw as usize + x as usize) * 4;
                frame.data_mut()[j..j + 4].copy_from_slice(&source.data()[i..i + 4]);
            }
        }
    }
    pub fn layout(&self, iw: u32, ih: u32, cw: u32, vw: u32, vh: u32, dpi: f64) -> EditorLayout {
        let fs = (vw as f64 / cw.max(1) as f64).min(dpi.max(0.1));
        let fy = (vh as f64 - 64. * fs).max(0.);
        let base = (vw as f64 / iw.max(1) as f64)
            .min(fy / ih.max(1) as f64)
            .min(1.)
            .max(f64::EPSILON);
        let scale = base * self.zoom;
        EditorLayout {
            scale,
            x: (vw as f64 - iw as f64 * scale) / 2. + self.pan_x,
            y: (fy - ih as f64 * scale) / 2. + self.pan_y,
            footer_scale: fs.max(f64::EPSILON),
            footer_x: (vw as f64 - cw as f64 * fs) / 2.,
            footer_y: fy,
        }
    }
    pub fn point(
        &self,
        x: f64,
        y: f64,
        iw: u32,
        ih: u32,
        cw: u32,
        vw: u32,
        vh: u32,
        dpi: f64,
    ) -> Point {
        let l = self.layout(iw, ih, cw, vw, vh, dpi);
        if y >= l.footer_y {
            Point::new(
                ((x - l.footer_x) / l.footer_scale) as f32,
                (ih as f64 + (y - l.footer_y) / l.footer_scale) as f32,
            )
        } else {
            Point::new(((x - l.x) / l.scale) as f32, ((y - l.y) / l.scale) as f32)
        }
    }
    pub fn zoom_at(
        &mut self,
        x: f64,
        y: f64,
        steps: f64,
        iw: u32,
        ih: u32,
        cw: u32,
        vw: u32,
        vh: u32,
        dpi: f64,
    ) {
        if !steps.is_finite() || !x.is_finite() || !y.is_finite() {
            return;
        }
        let old = self.layout(iw, ih, cw, vw, vh, dpi);
        if y >= old.footer_y {
            return;
        }
        let px = (x - old.x) / old.scale;
        let py = (y - old.y) / old.scale;
        self.zoom = (self.zoom * 1.2f64.powf(steps.clamp(-8., 8.))).clamp(0.1, 16.);
        let new = self.layout(iw, ih, cw, vw, vh, dpi);
        self.pan_x += x - (new.x + px * new.scale);
        self.pan_y += y - (new.y + py * new.scale);
    }
    pub fn present(
        &self,
        p: &tiny_skia::Pixmap,
        dst: &mut [u32],
        vw: u32,
        vh: u32,
        iw: u32,
        ih: u32,
        cw: u32,
        dpi: f64,
    ) {
        let l = self.layout(iw, ih, cw, vw, vh, dpi);
        let src = p.data();
        dst.fill(0x18181c);
        // Sample only visible destination pixels. Zoom never allocates a larger image.
        for y in 0..vh {
            for x in 0..vw {
                let (sx, sy) = if y as f64 + 0.5 >= l.footer_y {
                    (
                        ((x as f64 + 0.5 - l.footer_x) / l.footer_scale).floor(),
                        ih as f64 + ((y as f64 + 0.5 - l.footer_y) / l.footer_scale).floor(),
                    )
                } else {
                    (
                        ((x as f64 + 0.5 - l.x) / l.scale).floor(),
                        ((y as f64 + 0.5 - l.y) / l.scale).floor(),
                    )
                };
                let footer = y as f64 + 0.5 >= l.footer_y;
                if sx < 0.
                    || sy < 0.
                    || sx
                        >= if footer {
                            cw.min(p.width()) as f64
                        } else {
                            iw.min(p.width()) as f64
                        }
                    || sy
                        >= if footer {
                            (ih.saturating_add(64)) as f64
                        } else {
                            ih as f64
                        }
                    || sy >= p.height() as f64
                {
                    continue;
                }
                let i = (sy as usize * p.width() as usize + sx as usize) * 4;
                if let Some(d) = dst.get_mut(y as usize * vw as usize + x as usize) {
                    *d = (src[i] as u32) << 16 | (src[i + 1] as u32) << 8 | src[i + 2] as u32;
                }
            }
        }
    }
}
