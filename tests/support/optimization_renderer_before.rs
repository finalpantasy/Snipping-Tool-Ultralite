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
    source_format: Option<(usize, u32, u32, bool)>,
    origin: (u32, u32),
    pub resamples: u64,
    pixelations: PixelationCache,
    composed_annotations: Vec<Annotation>,
    composed_meta: Option<(Option<usize>, u64, crate::theme::ThemeMode)>,
    /// Committed layer plus live marks; only allocated while something is live.
    live: Option<Pixmap>,
    pub compositions: u64,
}

fn annotation_bytes(annotations: &[Annotation]) -> usize {
    annotations
        .iter()
        .fold(std::mem::size_of_val(annotations), |n, a| {
            n.saturating_add(match a {
                Annotation::Pencil { points, .. } => {
                    points.len().saturating_mul(std::mem::size_of::<Point>())
                }
                Annotation::Text { text, .. } => text.len(),
                _ => 0,
            })
        })
}
fn blit_canvas(canvas: &Pixmap, frame: &mut Pixmap, origin: (u32, u32)) {
    let (w, h) = (canvas.width(), canvas.height());
    let fw = frame.width();
    for y in 0..h {
        let i = (y * w * 4) as usize;
        let dest = (((y + origin.1) * fw + origin.0) * 4) as usize;
        frame.data_mut()[dest..dest + (w * 4) as usize]
            .copy_from_slice(&canvas.data()[i..i + (w * 4) as usize]);
    }
}
const PIXELATION_BUDGET: usize = 1024 * 1024;
const PIXELATION_GRID_LIMIT: usize = 256;
#[derive(Clone, Copy, PartialEq, Eq)]
struct PixelationKey {
    source: usize,
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    block: u32,
}
struct PixelationGrid {
    key: PixelationKey,
    colors: Vec<[u8; 4]>,
    columns: u32,
}
#[derive(Default)]
struct PixelationCache {
    grids: std::collections::VecDeque<PixelationGrid>,
    bytes: usize,
    builds: u64,
}
fn block_color(
    screen: &crate::capture::CapturedScreen,
    k: PixelationKey,
    bx: u32,
    by: u32,
) -> [u8; 4] {
    let x = k.x + bx * k.block;
    let y = k.y + by * k.block;
    let w = k.block.min(k.w - bx * k.block);
    let h = k.block.min(k.h - by * k.block);
    let mut sum = [0u64; 4];
    for yy in y..y + h {
        let row = yy as usize * screen.width as usize * 4;
        for xx in x..x + w {
            let i = row + xx as usize * 4;
            let alpha = screen.pixels[i + 3] as u64;
            for c in 0..3 {
                sum[c] += screen.pixels[i + c] as u64 * alpha;
            }
            sum[3] += alpha;
        }
    }
    let n = w as u64 * h as u64;
    [
        (sum[0] / (255 * n)) as u8,
        (sum[1] / (255 * n)) as u8,
        (sum[2] / (255 * n)) as u8,
        (sum[3] / n) as u8,
    ]
}
impl PixelationCache {
    fn grid(
        &mut self,
        screen: &crate::capture::CapturedScreen,
        key: PixelationKey,
    ) -> Option<&PixelationGrid> {
        if let Some(i) = self.grids.iter().position(|g| g.key == key) {
            return self.grids.get(i);
        }
        let columns = key.w.div_ceil(key.block);
        let rows = key.h.div_ceil(key.block);
        let bytes = columns as u64 * rows as u64 * 4 + std::mem::size_of::<PixelationGrid>() as u64;
        if bytes > PIXELATION_BUDGET as u64 {
            return None;
        }
        while self.bytes + bytes as usize > PIXELATION_BUDGET
            || self.grids.len() >= PIXELATION_GRID_LIMIT
        {
            if let Some(old) = self.grids.pop_front() {
                self.bytes -= old.colors.len() * 4 + std::mem::size_of::<PixelationGrid>();
            } else {
                break;
            }
        }
        let mut colors = Vec::with_capacity((columns * rows) as usize);
        for by in 0..rows {
            for bx in 0..columns {
                colors.push(block_color(screen, key, bx, by));
            }
        }
        self.bytes += bytes as usize;
        self.builds += 1;
        self.grids.push_back(PixelationGrid {
            key,
            colors,
            columns,
        });
        self.grids.back()
    }
}
fn draw_pixelation(
    p: &mut Pixmap,
    a: &Annotation,
    screen: &crate::capture::CapturedScreen,
    scale: f32,
    ox: f32,
    oy: f32,
    cache: &mut PixelationCache,
) {
    let Annotation::Pixelate {
        top_left,
        size,
        block_size,
    } = a
    else {
        return;
    };
    let x = top_left.x.max(0.) as u32;
    let y = top_left.y.max(0.) as u32;
    let w = ((size.width + top_left.x.min(0.)).max(0.) as u32).min(screen.width.saturating_sub(x));
    let h =
        ((size.height + top_left.y.min(0.)).max(0.) as u32).min(screen.height.saturating_sub(y));
    if w == 0 || h == 0 || !scale.is_finite() || scale <= 0. {
        return;
    }
    let key = PixelationKey {
        source: screen.pixels.as_ptr() as usize,
        width: screen.width,
        height: screen.height,
        x,
        y,
        w,
        h,
        block: (*block_size).max(1) as u32,
    };
    let left = x as f32 * scale + ox;
    let top = y as f32 * scale + oy;
    let right = (x + w) as f32 * scale + ox;
    let bottom = (y + h) as f32 * scale + oy;
    let x0 = left.floor().clamp(0., p.width() as f32) as u32;
    let x1 = right.ceil().clamp(0., p.width() as f32) as u32;
    let y0 = top.floor().clamp(0., p.height() as f32) as u32;
    let y1 = bottom.ceil().clamp(0., p.height() as f32) as u32;
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let grid = cache.grid(screen, key);
    let columns = w.div_ceil(key.block);
    let rows = h.div_ceil(key.block);
    // Sample only visible output pixels. Tiny blocks that exceed the cache budget
    // skip grid allocation, instead of drawing millions of subpixel rectangles.
    let xs: Vec<_> = (x0..x1)
        .map(|xx| {
            let source = ((xx as f32 + 0.5 - left) / scale).clamp(0., w as f32 - f32::EPSILON);
            let bx = ((source / key.block as f32) as u32).min(columns - 1);
            let coverage = ((xx + 1) as f32).min(right) - (xx as f32).max(left);
            (bx, coverage.clamp(0., 1.))
        })
        .collect();
    let stride = p.width() as usize * 4;
    let pixels = p.data_mut();
    // Oversized grids still use only viewport-sized scratch storage. Reuse
    // the visible block row while zoom maps several output rows to the same
    // source blocks instead of averaging those blocks on every output row.
    let mut fallback_colors = if grid.is_none() {
        vec![[0; 4]; xs.len()]
    } else {
        Vec::new()
    };
    let mut fallback_row = None;
    for yy in y0..y1 {
        let source = ((yy as f32 + 0.5 - top) / scale).clamp(0., h as f32 - f32::EPSILON);
        let by = ((source / key.block as f32) as u32).min(rows - 1);
        let cy = (((yy + 1) as f32).min(bottom) - (yy as f32).max(top)).clamp(0., 1.);
        if grid.is_none() && fallback_row != Some(by) {
            let mut previous = None;
            let mut color = [0; 4];
            for (dst, &(bx, _)) in fallback_colors.iter_mut().zip(&xs) {
                if previous != Some(bx) {
                    color = block_color(screen, key, bx, by);
                    previous = Some(bx);
                }
                *dst = color;
            }
            fallback_row = Some(by);
        }
        let mut previous = None;
        let mut color = [0; 4];
        for (dx, &(bx, cx)) in xs.iter().enumerate() {
            if previous != Some(bx) {
                color = grid
                    .map(|g| g.colors[(by * g.columns + bx) as usize])
                    .unwrap_or_else(|| fallback_colors[dx]);
                previous = Some(bx);
            }
            let i = yy as usize * stride + (x0 as usize + dx) * 4;
            let alpha = (cx * cy * 255.).round() as u32;
            if alpha == 255 {
                pixels[i..i + 4].copy_from_slice(&color);
            } else {
                // Pixelation replaces the source within its geometric mask.
                // Interpolate alpha too, rather than layering alpha twice.
                for c in 0..4 {
                    pixels[i + c] =
                        ((color[c] as u32 * alpha + pixels[i + c] as u32 * (255 - alpha) + 127)
                            / 255) as u8;
                }
            }
        }
    }
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
    if let Some((x, y, w, h)) = tools::annotation_resize_bounds(a) {
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
            + self.live.as_ref().map_or(0, |p| p.data().len())
            + self.pixelations.bytes
            + annotation_bytes(&self.composed_annotations)
    }
    pub fn pixelation_cache_bytes(&self) -> usize {
        self.pixelations.bytes
    }
    pub fn pixelation_builds(&self) -> u64 {
        self.pixelations.builds
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
        // Sampling uses pixel centers. Keep a border pixel whenever its center
        // lies in the image, including subpixel-sized images at distant zoom.
        let x0 = (l.x.max(well.x as f64 * dpi).max(0.) - 0.5).ceil().max(0.) as u32;
        let y0 = (l.y.max(well.y as f64 * dpi).max(0.) - 0.5).ceil().max(0.) as u32;
        let x1 = ((l.x + screen.width as f64 * l.scale)
            .min((well.x + well.w) as f64 * dpi)
            .min(frame.width() as f64)
            - 0.5)
            .ceil()
            .max(0.) as u32;
        let y1 = ((l.y + screen.height as f64 * l.scale)
            .min((well.y + well.h) as f64 * dpi)
            .min(frame.height() as f64)
            - 0.5)
            .ceil()
            .max(0.) as u32;
        if x1 <= x0 || y1 <= y0 {
            self.live = None;
            return;
        }
        let (w, h) = (x1 - x0, y1 - y0);
        let relative_x = l.x - x0 as f64;
        let relative_y = l.y - y0 as f64;
        let ox = relative_x as f32;
        let oy = relative_y as f32;
        let scale = l.scale as f32;
        let key = (
            screen.pixels.as_ptr() as usize,
            screen.width,
            screen.height,
            w,
            h,
            relative_x.to_bits(),
            relative_y.to_bits(),
            l.scale.to_bits(),
        );
        // Integer translation changes placement, not the local image or marks.
        self.origin = (x0, y0);
        if self.key != Some(key) {
            self.composed_meta = None;
            let source_id = (screen.pixels.as_ptr() as usize, screen.width, screen.height);
            let opaque = match self.source_format {
                Some((ptr, sw, sh, opaque)) if (ptr, sw, sh) == source_id => opaque,
                _ => {
                    let opaque = screen.pixels.chunks_exact(4).all(|p| p[3] == 255);
                    self.source_format = Some((source_id.0, source_id.1, source_id.2, opaque));
                    opaque
                }
            };
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
            // Each source coordinate is calculated once per axis, not per pixel.
            // Keep the same f32 blend order as the previous bilinear sampler.
            let xs: Vec<_> = (0..w)
                .map(|x| {
                    let sx = ((x0 as f64 + x as f64 + 0.5 - l.x) / l.scale - 0.5)
                        .clamp(0., screen.width.saturating_sub(1) as f64);
                    let ix = sx.floor() as u32;
                    (
                        ix as usize * 4,
                        (ix + 1).min(screen.width - 1) as usize * 4,
                        (sx - ix as f64) as f32,
                    )
                })
                .collect();
            let stride = screen.width as usize * 4;
            let data = base.data_mut();
            for y in 0..h {
                let sy = ((y0 as f64 + y as f64 + 0.5 - l.y) / l.scale - 0.5)
                    .clamp(0., screen.height.saturating_sub(1) as f64);
                let iy = sy.floor() as u32;
                let row = iy as usize * stride;
                let next = (iy + 1).min(screen.height - 1) as usize * stride;
                let fy = (sy - iy as f64) as f32;
                for (x, &(ix, jx, fx)) in xs.iter().enumerate() {
                    let dest = (y as usize * w as usize + x) * 4;
                    for c in 0..4 {
                        let sample = |i: usize| {
                            let value = screen.pixels[i + c] as f32;
                            if opaque || c == 3 {
                                value
                            } else {
                                value * screen.pixels[i + 3] as f32 / 255.
                            }
                        };
                        let top = sample(row + ix) * (1. - fx) + sample(row + jx) * fx;
                        let bottom = sample(next + ix) * (1. - fx) + sample(next + jx) * fx;
                        data[dest + c] = (top * (1. - fy) + bottom * fy).round() as u8;
                    }
                }
            }
            self.key = Some(key);
            self.origin = (x0, y0);
            self.resamples += 1;
        }
        let progress = state.in_progress();
        // An in-progress Spotlight changes the dimming of the whole image, so it
        // is the one live mark that must recompose everything.
        let live_spot = matches!(progress, Some(Annotation::Spotlight { .. }));
        let layered = !live_spot && annotation_bytes(&state.annotations) <= 256 * 1024;
        let meta = (state.selected_index, dpi.to_bits(), crate::theme::mode());
        let fresh = layered
            && self.composed_meta == Some(meta)
            && self.composed_annotations == state.annotations;
        let (Some(base), Some(canvas)) = (&self.base, &mut self.canvas) else {
            return;
        };
        let ctx = Ctx {
            screen,
            scale,
            ox,
            oy,
            dpi: dpi as f32,
            w,
            h,
        };
        if !fresh {
            self.compositions += 1;
            let extra_spot = if live_spot { progress.as_ref() } else { None };
            compose_committed(canvas, base, state, extra_spot, &ctx, &mut self.pixelations);
            if layered {
                if self.composed_annotations != state.annotations {
                    self.composed_annotations = state.annotations.clone();
                }
                self.composed_meta = Some(meta);
            } else {
                self.composed_meta = None;
                self.composed_annotations = Vec::new();
            }
        }
        let has_live = progress.is_some()
            || (state.text_input_active && !state.text_input_buffer.is_empty())
            || (state.active_tool == crate::tools::ToolKind::Eyedropper
                && state.eyedropper_preview.is_some());
        if !has_live {
            self.live = None;
            blit_canvas(canvas, frame, (x0, y0));
            return;
        }
        if !layered {
            self.live = None;
            // Recomposed this frame anyway; draw live items straight onto it.
            draw_live(
                canvas,
                state,
                progress.as_ref(),
                &ctx,
                &mut self.pixelations,
            );
            blit_canvas(canvas, frame, (x0, y0));
            return;
        }
        // Committed marks stay cached while drawing: each pointer move copies
        // the cached layer and paints only the live items on top, in the same
        // order a full composition would use.
        if self
            .live
            .as_ref()
            .map_or(true, |p| (p.width(), p.height()) != (w, h))
        {
            self.live = Pixmap::new(w, h);
        }
        let Some(live) = &mut self.live else {
            return;
        };
        live.data_mut().copy_from_slice(canvas.data());
        draw_live(live, state, progress.as_ref(), &ctx, &mut self.pixelations);
        blit_canvas(live, frame, (x0, y0));
    }
}

/// Viewport mapping shared by the committed and live layers.
struct Ctx<'a> {
    screen: &'a crate::capture::CapturedScreen,
    scale: f32,
    ox: f32,
    oy: f32,
    dpi: f32,
    w: u32,
    h: u32,
}

/// Base pixels, spotlight dimming, finished annotations and the selection guide.
fn compose_committed(
    canvas: &mut Pixmap,
    base: &Pixmap,
    state: &OverlayState,
    extra_spot: Option<&Annotation>,
    c: &Ctx,
    pixelations: &mut PixelationCache,
) {
    let (w, h) = (c.w, c.h);
    canvas.data_mut().copy_from_slice(base.data());
    let spots: Vec<_> = state
        .annotations
        .iter()
        .chain(extra_spot)
        .filter_map(|a| {
            if let Annotation::Spotlight { .. } = a {
                Some(transformed(a, c.scale, c.ox, c.oy))
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
        draw_annotation(canvas, a, c.screen, c.scale, c.ox, c.oy, c.dpi, pixelations);
    }
    if let Some(a) = state.selected_index.and_then(|i| state.annotations.get(i)) {
        guide(canvas, &transformed(a, c.scale, c.ox, c.oy), c.dpi, true);
    }
}

/// The mark being drawn, text being typed and the eyedropper swatch.
fn draw_live(
    canvas: &mut Pixmap,
    state: &OverlayState,
    progress: Option<&Annotation>,
    c: &Ctx,
    pixelations: &mut PixelationCache,
) {
    let (w, h, dpi) = (c.w, c.h, c.dpi);
    if let Some(a) = progress {
        draw_annotation(canvas, a, c.screen, c.scale, c.ox, c.oy, dpi, pixelations);
        if matches!(
            a,
            Annotation::Highlight { .. } | Annotation::Pixelate { .. }
        ) {
            guide(canvas, &transformed(a, c.scale, c.ox, c.oy), dpi, false);
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
            c.screen,
            c.scale,
            c.ox,
            c.oy,
            dpi,
            pixelations,
        );
    }
    if state.active_tool == crate::tools::ToolKind::Eyedropper {
        if let Some(color) = state.eyedropper_preview {
            let at = point(state.last_mouse_pos, c.scale, c.ox, c.oy);
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
}

fn draw_annotation(
    p: &mut Pixmap,
    a: &Annotation,
    screen: &crate::capture::CapturedScreen,
    s: f32,
    ox: f32,
    oy: f32,
    dpi: f32,
    pixelations: &mut PixelationCache,
) {
    match a {
        Annotation::Pixelate { .. } => draw_pixelation(p, a, screen, s, ox, oy, pixelations),
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
