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

/// Specialize opacity once per row. Ordinary screenshots skip alpha sampling
/// and premultiplication; imported transparency retains the exact blend order.
#[inline]
fn sample_row<const OPAQUE: bool>(
    source: &[u8],
    output: &mut [u8],
    xs: &[(usize, usize, f32)],
    row: usize,
    next: usize,
    fy: f32,
) {
    for (x, &(ix, jx, fx)) in xs.iter().enumerate() {
        let dest = x * 4;
        for c in 0..if OPAQUE { 3 } else { 4 } {
            let sample = |i: usize| {
                let value = source[i + c] as f32;
                if OPAQUE || c == 3 {
                    value
                } else {
                    value * source[i + 3] as f32 / 255.
                }
            };
            let top = sample(row + ix) * (1. - fx) + sample(row + jx) * fx;
            let bottom = sample(next + ix) * (1. - fx) + sample(next + jx) * fx;
            output[dest + c] = (top * (1. - fy) + bottom * fy).round() as u8;
        }
        if OPAQUE {
            output[dest + 3] = 255;
        }
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
    let Some((x, y, w, h)) =
        crate::canvas::source_region(*top_left, *size, screen.width, screen.height)
    else {
        return;
    };
    if !scale.is_finite() || scale <= 0. {
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
    let full_start = xs.iter().position(|(_, cx)| *cx == 1.).unwrap_or(xs.len());
    let full_end = xs
        .iter()
        .rposition(|(_, cx)| *cx == 1.)
        .map_or(full_start, |i| i + 1);
    let mut previous_full_row = None;
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
        // Fully covered pixels replace their destination, and every output row
        // in the same source-block row has identical colors. Copy that interior
        // directly; fractional boundary pixels still blend against each row.
        let spans = if cy == 1. && full_start < full_end {
            if let Some((_, old_y)) = previous_full_row.filter(|(old_by, _)| *old_by == by) {
                let source = old_y as usize * stride + (x0 as usize + full_start) * 4;
                let destination = yy as usize * stride + (x0 as usize + full_start) * 4;
                pixels.copy_within(source..source + (full_end - full_start) * 4, destination);
                [(0, full_start), (full_end, xs.len())]
            } else {
                [(0, xs.len()), (0, 0)]
            }
        } else {
            [(0, xs.len()), (0, 0)]
        };
        for (first, last) in spans {
            let mut previous = None;
            let mut color = [0; 4];
            for dx in first..last {
                let (bx, cx) = xs[dx];
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
        previous_full_row = (cy == 1.).then_some((by, yy));
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
        let bounds = state.editor_canvas_bounds();
        let left = l.x + bounds.x as f64 * l.scale;
        let top = l.y + bounds.y as f64 * l.scale;
        let x0 = (left.max(well.x as f64 * dpi).max(0.) - 0.5).ceil().max(0.) as u32;
        let y0 = (top.max(well.y as f64 * dpi).max(0.) - 0.5).ceil().max(0.) as u32;
        let x1 = ((left + bounds.width as f64 * l.scale)
            .min((well.x + well.w) as f64 * dpi)
            .min(frame.width() as f64)
            - 0.5)
            .ceil()
            .max(0.) as u32;
        let y1 = ((top + bounds.height as f64 * l.scale)
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
            // Only the canvas intersection is retained. Growth never allocates
            // a full-resolution image while dragging, and the image transform
            // stays fixed so the pointer does not chase a shifting canvas.
            base.fill(tiny_skia::Color::WHITE);
            let image_x0 = ((l.x - 0.5).ceil().max(x0 as f64).min(x1 as f64)) as u32;
            let image_x1 = ((l.x + screen.width as f64 * l.scale - 0.5)
                .ceil()
                .max(x0 as f64)
                .min(x1 as f64)) as u32;
            let image_y0 = ((l.y - 0.5).ceil().max(y0 as f64).min(y1 as f64)) as u32;
            let image_y1 = ((l.y + screen.height as f64 * l.scale - 0.5)
                .ceil()
                .max(y0 as f64)
                .min(y1 as f64)) as u32;
            // Each source coordinate is calculated once per axis, not per pixel.
            // Keep the same f32 blend order as the previous bilinear sampler.
            let xs: Vec<_> = (image_x0..image_x1)
                .map(|x| {
                    let sx = ((x as f64 + 0.5 - l.x) / l.scale - 0.5)
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
            for y in image_y0..image_y1 {
                let sy = ((y as f64 + 0.5 - l.y) / l.scale - 0.5)
                    .clamp(0., screen.height.saturating_sub(1) as f64);
                let iy = sy.floor() as u32;
                let row = iy as usize * stride;
                let next = (iy + 1).min(screen.height - 1) as usize * stride;
                let fy = (sy - iy as f64) as f32;
                let start = ((y - y0) as usize * w as usize + (image_x0 - x0) as usize) * 4;
                let output = &mut data[start..start + xs.len() * 4];
                if opaque {
                    sample_row::<true>(&screen.pixels, output, &xs, row, next, fy);
                } else {
                    sample_row::<false>(&screen.pixels, output, &xs, row, next, fy);
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
            if let Annotation::Spotlight { top_left, size } = a {
                let (x, y, w, h) =
                    crate::canvas::source_region(*top_left, *size, c.screen.width, c.screen.height)
                        .unwrap_or((0, 0, 0, 0));
                let clipped = Annotation::Spotlight {
                    top_left: Point::new(x as f32, y as f32),
                    size: crate::geometry::Size::new(w as f32, h as f32),
                };
                Some(transformed(&clipped, c.scale, c.ox, c.oy))
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

#[cfg(test)]
mod speed_tests {
    use super::*;
    include!("../tests/support/pixelation_reference.rs");
    include!("../tests/support/pixelation_draw_before_20261013.rs");

    #[test]
    fn pixelation_scanline_copy_preserves_transparency_and_partial_edges() {
        for (width, height, block) in [(61, 47, 3), (1401, 1403, 2)] {
            let mut source = screen(width, height);
            for (i, px) in source.pixels.chunks_exact_mut(4).enumerate() {
                px[3] = [0, 64, 128, 255][i % 4];
            }
            let ann = Annotation::Pixelate {
                top_left: Point::new(-1.5, -2.25),
                size: crate::geometry::Size::new(width as f32, height as f32),
                block_size: block,
            };
            for scale in [0.67, 1., 3.2, 4.] {
                let mut expected = Pixmap::new(128, 96).unwrap();
                expected.fill(tiny_skia::Color::from_rgba(0.2, 0.3, 0.4, 0.6).unwrap());
                let mut actual = expected.clone();
                previous_draw_pixelation(
                    &mut expected,
                    &ann,
                    &source,
                    scale,
                    -2.25,
                    -3.75,
                    &mut PixelationCache::default(),
                );
                draw_pixelation(
                    &mut actual,
                    &ann,
                    &source,
                    scale,
                    -2.25,
                    -3.75,
                    &mut PixelationCache::default(),
                );
                assert_eq!(
                    actual.data(),
                    expected.data(),
                    "size={width}x{height}, scale={scale}"
                );
            }
        }
    }

    #[test]
    fn specialized_sampler_preserves_opaque_and_premultiplied_midpoints() {
        let opaque = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 200, 100, 50, 255,
        ];
        let translucent = [
            255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 200, 100, 50, 64,
        ];
        let mut output = [0; 4];
        sample_row::<true>(&opaque, &mut output, &[(0, 4, 0.5)], 0, 8, 0.5);
        assert_eq!(output, [114, 89, 76, 255]);
        sample_row::<false>(&translucent, &mut output, &[(0, 4, 0.5)], 0, 8, 0.5);
        assert_eq!(output, [76, 38, 3, 112]);
        sample_row::<false>(&translucent, &mut output, &[(0, 0, 0.0)], 8, 8, 0.0);
        assert_eq!(
            output,
            [0, 0, 0, 0],
            "transparent color must not bleed into a zoomed edge"
        );
    }

    fn screen(width: u32, height: u32) -> crate::capture::CapturedScreen {
        crate::capture::CapturedScreen {
            pixels: (0..width * height)
                .flat_map(|i| {
                    [
                        (i % 251) as u8,
                        (i / 7 % 239) as u8,
                        (i / width % 233) as u8,
                        255,
                    ]
                })
                .collect(),
            width,
            height,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        }
    }

    #[test]
    fn pixelation_row_reuse_matches_reference_at_fractional_edges_and_scales() {
        for (w, h, block, uncached) in [(61, 47, 3, false), (1401, 1403, 2, true)] {
            let source = screen(w, h);
            for (scale, ox, oy) in [
                (1., -2.25, -3.75),
                (4., -17.5, -6.3),
                (0.67, -3.2, 0.7),
                (3.2, 1.3, -4.9),
            ] {
                for origin in [Point::new(0., 0.), Point::new(-1.5, -2.25)] {
                    let ann = Annotation::Pixelate {
                        top_left: origin,
                        size: crate::geometry::Size::new(w as f32, h as f32),
                        block_size: block,
                    };
                    let mut expected = Pixmap::new(128, 96).unwrap();
                    expected.fill(tiny_skia::Color::from_rgba(0.2, 0.3, 0.4, 0.6).unwrap());
                    let mut actual = expected.clone();
                    let mut old_cache = PixelationCache::default();
                    let mut new_cache = PixelationCache::default();
                    reference_pixelation(
                        &mut expected,
                        &ann,
                        &source,
                        scale,
                        ox,
                        oy,
                        &mut old_cache,
                    );
                    draw_pixelation(&mut actual, &ann, &source, scale, ox, oy, &mut new_cache);
                    assert_eq!(
                        actual.data(),
                        expected.data(),
                        "scale={scale}, origin={origin:?}"
                    );
                    assert_eq!(new_cache.bytes == 0, uncached);
                }
            }
        }
    }

    #[test]
    fn retained_bytes_counts_and_releases_live_pixel_buffer() {
        let mut state = OverlayState::new(screen(80, 60), &crate::config::Config::default());
        let mut renderer = PreviewRenderer::default();
        let mut frame = Pixmap::new(660, 390).unwrap();
        let view = EditorView::default();
        renderer.draw(&state, &mut frame, &view, 1.);
        let idle_bytes = renderer.retained_bytes();
        state.text_input_active = true;
        state.text_input_buffer = "synthetic text".into();
        renderer.draw(&state, &mut frame, &view, 1.);
        let live_bytes = renderer.live.as_ref().unwrap().data().len();
        // Live text can grow both base layers as well as adding the live layer.
        assert!(live_bytes >= idle_bytes / 2);
        assert_eq!(renderer.retained_bytes(), live_bytes * 3);
        state.text_input_active = false;
        renderer.draw(&state, &mut frame, &view, 1.);
        assert_eq!(renderer.retained_bytes(), idle_bytes);
    }

    #[test]
    #[ignore = "manual synthetic CPU benchmark"]
    fn benchmark_oversized_pixelation_fallback() {
        use std::{hint::black_box, time::Instant};
        fn median(mut run: impl FnMut()) -> f64 {
            for _ in 0..3 {
                run();
            }
            let mut samples = Vec::new();
            for _ in 0..31 {
                let start = Instant::now();
                run();
                samples.push(start.elapsed().as_secs_f64() * 1000.);
            }
            samples.sort_by(f64::total_cmp);
            samples[15]
        }
        let source = screen(3840, 2160);
        let ann = Annotation::Pixelate {
            top_left: Point::new(0., 0.),
            size: crate::geometry::Size::new(3840., 2160.),
            block_size: 2,
        };
        let mut frame = Pixmap::new(1280, 720).unwrap();
        for scale in [1., 4.] {
            let mut cache = PixelationCache::default();
            let old = median(|| {
                reference_pixelation(&mut frame, &ann, &source, scale, -1.3, -2.7, &mut cache);
                black_box(frame.data());
            });
            let new = median(|| {
                draw_pixelation(&mut frame, &ann, &source, scale, -1.3, -2.7, &mut cache);
                black_box(frame.data());
            });
            assert_eq!(cache.bytes, 0);
            println!("4K pixelation fallback at {scale}x into 1280x720: previous={old:.4}ms optimized={new:.4}ms ratio={:.2}x", old/new);
        }
    }
}
