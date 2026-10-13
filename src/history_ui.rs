use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use tiny_skia::{Color as SkiaColor, Paint, Pixmap, Rect, Transform};
use winit::window::Window;

use crate::geometry::{Color, Point};
use crate::tools::{measure_text_width, render_text_annotation};

/// Width / height of the history window in LOGICAL pixels (the window is
/// created with LogicalSize and the UI is scaled to the physical surface).
pub const WIN_W: u32 = 600;
pub const WIN_H: u32 = 400;

/// Thumbnail grid layout
const COLS: usize = 4;
const THUMB_W: u32 = 120;
const THUMB_H: u32 = 80;
const PADDING: f32 = 16.0;
const GAP: f32 = 12.0;
const HEADER_H: f32 = 44.0;

/// A loaded thumbnail: path, RGBA pixels, width, height.
struct Thumbnail {
    path: PathBuf,
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

pub struct HistoryWindow {
    pub window: Arc<Window>,
    pub surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    thumbnails: Vec<Thumbnail>,
    pub hovered: Option<usize>,
    pub needs_redraw: bool,
    pub cursor_pos: (f32, f32),
    /// Vertical scroll offset in logical pixels (0 = top).
    scroll_offset: f32,
    /// Rect of the "Clear All" header button (x, y, w, h).
    clear_rect: (f32, f32, f32, f32),
    hover_clear: bool,
}

impl HistoryWindow {
    pub fn new(
        window: Arc<Window>,
        surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    ) -> Self {
        let thumbnails = load_thumbnails();

        let clear_w = 80.0;
        Self {
            window,
            surface,
            thumbnails,
            hovered: None,
            needs_redraw: true,
            cursor_pos: (0.0, 0.0),
            scroll_offset: 0.0,
            clear_rect: (WIN_W as f32 - PADDING - clear_w, 10.0, clear_w, 24.0),
            hover_clear: false,
        }
    }

    /// Returns (x, y, w, h) of the thumbnail at the given index, in logical
    /// coordinates with the current scroll applied.
    fn thumb_rect(&self, idx: usize) -> (f32, f32, f32, f32) {
        let col = idx % COLS;
        let row = idx / COLS;
        let cell_w = THUMB_W as f32 + GAP;
        let cell_h = THUMB_H as f32 + GAP;
        let x = PADDING + col as f32 * cell_w;
        let y = HEADER_H + PADDING + row as f32 * cell_h - self.scroll_offset;
        if let Some(thumb) = self.thumbnails.get(idx) {
            (x, y, thumb.width as f32, thumb.height as f32)
        } else {
            (x, y, THUMB_W as f32, THUMB_H as f32)
        }
    }

    /// Total height of the thumbnail grid content (without scroll).
    fn content_height(&self) -> f32 {
        if self.thumbnails.is_empty() {
            return 0.0;
        }
        let rows = self.thumbnails.len().div_ceil(COLS);
        PADDING + rows as f32 * (THUMB_H as f32 + GAP)
    }

    fn max_scroll(&self) -> f32 {
        (self.content_height() - (WIN_H as f32 - HEADER_H)).max(0.0)
    }

    /// Scroll by a wheel delta (positive = scroll up). Returns true if moved.
    pub fn on_scroll(&mut self, delta: f32) -> bool {
        let old = self.scroll_offset;
        self.scroll_offset = (self.scroll_offset - delta * 40.0).clamp(0.0, self.max_scroll());
        if (self.scroll_offset - old).abs() > 0.01 {
            self.needs_redraw = true;
            true
        } else {
            false
        }
    }

    /// Full render of the history UI into the softbuffer surface.
    pub fn render(&mut self) {
        let mut pixmap = match Pixmap::new(WIN_W, WIN_H) {
            Some(p) => p,
            None => return,
        };

        // Background
        fill_rect_rgb(
            &mut pixmap,
            0.0,
            0.0,
            WIN_W as f32,
            WIN_H as f32,
            crate::theme::bg_1(),
        );

        if self.thumbnails.is_empty() {
            draw_label(
                &mut pixmap,
                PADDING,
                HEADER_H + PADDING + 20.0,
                "No captures yet.",
                14.0,
                crate::theme::text_muted(),
            );
        } else {
            for (i, thumb) in self.thumbnails.iter().enumerate() {
                let (x, y, tw, th) = self.thumb_rect(i);

                // Skip thumbnails fully outside the viewport
                if y + th < HEADER_H || y > WIN_H as f32 {
                    continue;
                }

                // Border (highlight on hover)
                let border_color = if self.hovered == Some(i) {
                    crate::theme::accent()
                } else {
                    crate::theme::bg_3()
                };
                fill_rect_rgb(
                    &mut pixmap,
                    x - 2.0,
                    y - 2.0,
                    tw + 4.0,
                    th + 4.0,
                    border_color,
                );

                // Render thumbnail pixels, clipped against the header line
                blit_rgba_clipped(
                    &mut pixmap,
                    x as i32,
                    y as i32,
                    &thumb.pixels,
                    thumb.width,
                    thumb.height,
                    HEADER_H as i32,
                );
            }

            // Scrollbar indicator when the content overflows
            let max_scroll = self.max_scroll();
            if max_scroll > 0.0 {
                let viewport_h = WIN_H as f32 - HEADER_H;
                let track_h = viewport_h - 8.0;
                let knob_h = (viewport_h / self.content_height() * track_h).max(24.0);
                let knob_y =
                    HEADER_H + 4.0 + (self.scroll_offset / max_scroll) * (track_h - knob_h);
                fill_rect_rgb(
                    &mut pixmap,
                    WIN_W as f32 - 6.0,
                    knob_y,
                    3.0,
                    knob_h,
                    crate::theme::bg_4(),
                );
            }
        }

        // Header drawn last so scrolled thumbnails never overlap it
        fill_rect_rgb(
            &mut pixmap,
            0.0,
            0.0,
            WIN_W as f32,
            HEADER_H - 3.0,
            crate::theme::bg_1(),
        );
        draw_label(
            &mut pixmap,
            PADDING,
            14.0,
            "Recent Captures",
            16.0,
            crate::theme::text_normal(),
        );
        // Separator
        fill_rect_rgb(
            &mut pixmap,
            PADDING,
            HEADER_H - 4.0,
            WIN_W as f32 - PADDING * 2.0,
            1.0,
            crate::theme::bg_3(),
        );
        // Clear All button
        if !self.thumbnails.is_empty() {
            let (cx, cy, cw, ch) = self.clear_rect;
            let bg = if self.hover_clear {
                crate::theme::bg_4()
            } else {
                crate::theme::bg_3()
            };
            fill_rect_rgb(&mut pixmap, cx, cy, cw, ch, bg);
            let label = "Clear All";
            let tw = measure_text_width(label, 12.0);
            draw_label(
                &mut pixmap,
                cx + (cw - tw) / 2.0,
                cy + 5.0,
                label,
                12.0,
                crate::theme::text_normal(),
            );
        }

        // Present, scaling the logical pixmap to the physical surface size
        // (nearest neighbor) so the window is usable on high-DPI displays.
        let phys = self.window.inner_size();
        let pw = phys.width.max(1);
        let ph = phys.height.max(1);
        if let (Some(nz_w), Some(nz_h)) = (NonZeroU32::new(pw), NonZeroU32::new(ph)) {
            if let Err(e) = self.surface.resize(nz_w, nz_h) {
                tracing::error!("History surface resize failed: {e}");
                return;
            }
        }

        if let Ok(mut buffer) = self.surface.buffer_mut() {
            let src = pixmap.data();
            let src_w = WIN_W as usize;
            let src_h = WIN_H as usize;
            for y in 0..ph as usize {
                let sy = (y * src_h / ph as usize).min(src_h - 1);
                for x in 0..pw as usize {
                    let sx = (x * src_w / pw as usize).min(src_w - 1);
                    let si = (sy * src_w + sx) * 4;
                    buffer[y * pw as usize + x] = ((src[si] as u32) << 16)
                        | ((src[si + 1] as u32) << 8)
                        | (src[si + 2] as u32);
                }
            }
            let _ = buffer.present();
        }

        self.needs_redraw = false;
    }

    /// Update cursor position (physical pixels). Returns true if hover changed.
    pub fn on_cursor_moved(&mut self, x: f32, y: f32) -> bool {
        // Scale physical cursor position to logical coordinates
        let scale = self.window.scale_factor() as f32;
        let x = x / scale;
        let y = y / scale;
        self.cursor_pos = (x, y);

        let old_hovered = self.hovered;
        let old_clear = self.hover_clear;

        let (cx, cy, cw, ch) = self.clear_rect;
        self.hover_clear = x >= cx && x <= cx + cw && y >= cy && y <= cy + ch;

        self.hovered = None;
        if y >= HEADER_H {
            for i in 0..self.thumbnails.len() {
                let (tx, ty, tw, th) = self.thumb_rect(i);
                if x >= tx && x <= tx + tw && y >= ty && y <= ty + th {
                    self.hovered = Some(i);
                    break;
                }
            }
        }
        old_hovered != self.hovered || old_clear != self.hover_clear
    }

    /// Handle a click at the stored (logical) cursor position.
    /// Returns true if the UI changed and needs a redraw.
    pub fn on_click(&mut self) -> bool {
        let (x, y) = self.cursor_pos;

        // Clear All button
        let (cx, cy, cw, ch) = self.clear_rect;
        if !self.thumbnails.is_empty() && x >= cx && x <= cx + cw && y >= cy && y <= cy + ch {
            let removed = crate::history::clear_history();
            tracing::info!("Cleared {} history entries", removed);
            self.thumbnails.clear();
            self.scroll_offset = 0.0;
            self.hovered = None;
            self.needs_redraw = true;
            return true;
        }

        if y < HEADER_H {
            return false;
        }

        for i in 0..self.thumbnails.len() {
            let (tx, ty, tw, th) = self.thumb_rect(i);
            if x >= tx && x <= tx + tw && y >= ty && y <= ty + th {
                if let Some(thumb) = self.thumbnails.get(i) {
                    // Decode + clipboard copy on a background thread — the
                    // full-resolution image can be large.
                    let path = thumb.path.clone();
                    std::thread::spawn(move || {
                        let Ok(img) = image::open(&path) else {
                            tracing::error!("Failed to open history image {}", path.display());
                            return;
                        };
                        let rgba = img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        match crate::export::copy_to_clipboard(&rgba.into_raw(), w, h) {
                            Ok(()) => {
                                tracing::info!("Copied history image to clipboard");
                                let _ = notify_rust::Notification::new()
                                    .summary("Snipping Tool UL")
                                    .body("Copied to clipboard")
                                    .timeout(2000)
                                    .show();
                            }
                            Err(e) => {
                                tracing::error!("Failed to copy history image: {e}");
                            }
                        }
                    });
                }
                return false;
            }
        }
        false
    }
}

fn load_thumbnails() -> Vec<Thumbnail> {
    crate::history::list_history()
        .into_iter()
        .filter_map(|path| {
            let img = image::open(&path).ok()?;
            let thumb = img.thumbnail(THUMB_W, THUMB_H);
            let rgba = thumb.to_rgba8();
            let (w, h) = rgba.dimensions();
            Some(Thumbnail {
                path,
                pixels: rgba.into_raw(),
                width: w,
                height: h,
            })
        })
        .collect()
}

// ── Drawing helpers ──

fn fill_rect_rgb(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, (r, g, b): (u8, u8, u8)) {
    let rect = match Rect::from_xywh(x, y, w, h) {
        Some(r) => r,
        None => return,
    };
    let mut paint = Paint::default();
    paint.set_color(SkiaColor::from_rgba8(r, g, b, 255));
    paint.anti_alias = true;
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

fn draw_label(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    text: &str,
    font_size: f32,
    (r, g, b): (u8, u8, u8),
) {
    let color = Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0);
    let pos = Point::new(x, y);
    render_text_annotation(pixmap, &pos, text, &color, font_size);
}

/// Blit RGBA pixels onto a pixmap, skipping rows above `min_y` (header clip).
fn blit_rgba_clipped(
    pixmap: &mut Pixmap,
    dst_x: i32,
    dst_y: i32,
    pixels: &[u8],
    w: u32,
    h: u32,
    min_y: i32,
) {
    let pm_w = pixmap.width() as i32;
    let pm_h = pixmap.height() as i32;
    let data = pixmap.data_mut();
    for y in 0..h as i32 {
        let py = dst_y + y;
        if py < min_y || py >= pm_h {
            continue;
        }
        for x in 0..w as i32 {
            let px = dst_x + x;
            if px < 0 || px >= pm_w {
                continue;
            }
            let src_i = ((y * w as i32 + x) * 4) as usize;
            let dst_i = ((py * pm_w + px) * 4) as usize;
            if src_i + 3 < pixels.len() && dst_i + 3 < data.len() {
                // Premultiply alpha for tiny-skia
                let a = pixels[src_i + 3] as u32;
                data[dst_i] = ((pixels[src_i] as u32 * a) / 255) as u8;
                data[dst_i + 1] = ((pixels[src_i + 1] as u32 * a) / 255) as u8;
                data[dst_i + 2] = ((pixels[src_i + 2] as u32 * a) / 255) as u8;
                data[dst_i + 3] = pixels[src_i + 3];
            }
        }
    }
}
