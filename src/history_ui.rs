use std::num::NonZeroU32;
#[cfg(test)]
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, TryRecvError},
    Arc,
};

use tiny_skia::{Color as SkiaColor, Paint, Pixmap, Rect, Transform};
use winit::window::Window;

use crate::geometry::Point;
use crate::history_thumbnails::{load_thumbnails, Thumbnail, ThumbnailLoad, HISTORY_LOADS};

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

/// Disconnect before waking the UI, including cancelled or panicking loaders.
struct LoadCompletion {
    sender: Option<std::sync::mpsc::Sender<ThumbnailLoad>>,
    window: std::sync::Weak<Window>,
}
impl Drop for LoadCompletion {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(window) = self.window.upgrade() {
            window.request_redraw();
        }
    }
}

pub struct HistoryWindow {
    pub window: Arc<Window>,
    pub surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    thumbnails: Vec<Thumbnail>,
    stored_entries: usize,
    pub hovered: Option<usize>,
    pub needs_redraw: bool,
    pub cursor_pos: (f32, f32),
    /// Vertical scroll offset in logical pixels (0 = top).
    scroll_offset: f32,
    /// Rect of the "Clear All" header button (x, y, w, h).
    clear_rect: (f32, f32, f32, f32),
    hover_clear: bool,
    thumbnail_scale: f32,
    pending: Option<Receiver<ThumbnailLoad>>,
    cancel_load: Arc<AtomicBool>,
}

impl HistoryWindow {
    pub fn new(
        window: Arc<Window>,
        surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    ) -> Self {
        let thumbnail_scale = window.scale_factor() as f32;
        let thumbnails = Vec::new();

        let mut history = Self {
            window,
            surface,
            thumbnails,
            stored_entries: 0,
            hovered: None,
            needs_redraw: true,
            cursor_pos: (0.0, 0.0),
            scroll_offset: 0.0,
            clear_rect: clear_rect(),
            hover_clear: false,
            thumbnail_scale,
            pending: None,
            cancel_load: Arc::new(AtomicBool::new(false)),
        };
        history.refresh();
        history
    }

    pub fn refresh(&mut self) {
        self.cancel_load.store(true, Ordering::Relaxed);
        let window = Arc::downgrade(&self.window);
        let scale = self.window.scale_factor() as f32;
        self.thumbnail_scale = scale;
        let (tx, rx) = std::sync::mpsc::channel();
        self.pending = Some(rx);
        self.thumbnails.clear();
        self.stored_entries = 0;
        self.hovered = None;
        self.needs_redraw = true;
        self.window.request_redraw();
        let completion = LoadCompletion {
            sender: Some(tx),
            window,
        };
        self.cancel_load = HISTORY_LOADS.submit(move |cancel| {
            let images = load_thumbnails(scale, cancel);
            if !cancel.load(Ordering::Relaxed) {
                let _ = completion.sender.as_ref().unwrap().send(images);
            }
        });
    }

    /// Returns (x, y, w, h) of the thumbnail at the given index, in logical
    /// coordinates with the current scroll applied.
    fn thumb_rect(&self, idx: usize) -> (f32, f32, f32, f32) {
        thumb_rect(&self.thumbnails, idx, self.scroll_offset)
    }

    /// Maximum logical scroll offset for the current grid.
    fn max_scroll(&self) -> f32 {
        max_scroll(self.thumbnails.len())
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

    /// Rasterize at physical resolution, then present every output pixel once.
    pub fn render(&mut self, history_enabled: bool) {
        let phys = self.window.inner_size();
        let (Some(pw), Some(ph)) = (NonZeroU32::new(phys.width), NonZeroU32::new(phys.height))
        else {
            return;
        };
        let scale = self.window.scale_factor() as f32;
        if (scale - self.thumbnail_scale).abs() > 0.01 {
            self.refresh();
        }
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(loaded) => {
                    self.stored_entries = loaded.stored_entries;
                    self.thumbnails = loaded.thumbnails;
                    self.pending = None;
                    self.scroll_offset = self.scroll_offset.min(self.max_scroll());
                }
                Err(TryRecvError::Disconnected) => self.pending = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        let Some(mut pixmap) = Pixmap::new(pw.get(), ph.get()) else {
            return;
        };
        draw_history(
            &mut pixmap,
            &self.thumbnails,
            self.stored_entries,
            self.hovered,
            self.scroll_offset,
            self.hover_clear,
            history_enabled,
            self.pending.is_some(),
        );
        if let Err(e) = self.surface.resize(pw, ph) {
            tracing::error!("History surface resize failed: {e}");
            return;
        }
        if let Ok(mut buffer) = self.surface.buffer_mut() {
            for (dst, src) in buffer.iter_mut().zip(pixmap.data().chunks_exact(4)) {
                *dst = ((src[0] as u32) << 16) | ((src[1] as u32) << 8) | src[2] as u32;
            }
            if let Err(e) = buffer.present() {
                tracing::error!("History presentation failed: {e}");
                return;
            }
        } else {
            return;
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
        if self.stored_entries > 0 && x >= cx && x <= cx + cw && y >= cy && y <= cy + ch {
            match crate::history::clear_history_checked() {
                Ok(removed) => tracing::info!("Cleared {removed} history entries"),
                Err(error) => {
                    tracing::warn!("{error}");
                    let _ = notify_rust::Notification::new()
                        .summary("Snipping Tool UL")
                        .body(&error)
                        .timeout(4000)
                        .show();
                }
            }
            self.refresh(); // Reflect remaining files if a deletion failed.
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
                    crate::clipboard_queue::spawn(move |ticket| {
                        if !crate::clipboard_queue::is_current(ticket) {
                            return;
                        }
                        let rgba = match crate::history_copy::decode(&path) {
                            Ok(image) => image,
                            Err(error) => {
                                tracing::error!("Failed to copy history image: {error}");
                                return;
                            }
                        };
                        if !crate::clipboard_queue::is_current(ticket) {
                            return;
                        }
                        let (w, h) = rgba.dimensions();
                        match crate::export::copy_with_ticket(ticket, &rgba.into_raw(), w, h) {
                            Ok(true) => {
                                tracing::info!("Copied history image to clipboard");
                                let _ = notify_rust::Notification::new()
                                    .summary("Snipping Tool UL")
                                    .body("Copied to clipboard")
                                    .timeout(2000)
                                    .show();
                            }
                            Ok(false) => {} // A newer request superseded this copy.
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

impl Drop for HistoryWindow {
    fn drop(&mut self) {
        self.cancel_load.store(true, Ordering::Relaxed);
    }
}

// ── Drawing helpers ──

fn clear_rect() -> (f32, f32, f32, f32) {
    (WIN_W as f32 - PADDING - 80., 10., 80., 24.)
}
fn content_height(count: usize) -> f32 {
    if count == 0 {
        0.
    } else {
        PADDING + count.div_ceil(COLS) as f32 * (THUMB_H as f32 + GAP)
    }
}
fn max_scroll(count: usize) -> f32 {
    (content_height(count) - (WIN_H as f32 - HEADER_H)).max(0.)
}
fn thumb_rect(thumbnails: &[Thumbnail], idx: usize, scroll: f32) -> (f32, f32, f32, f32) {
    let x = PADDING + (idx % COLS) as f32 * (THUMB_W as f32 + GAP);
    let y = HEADER_H + PADDING + (idx / COLS) as f32 * (THUMB_H as f32 + GAP) - scroll;
    if let Some(t) = thumbnails.get(idx) {
        (x, y, t.width as f32 / t.scale, t.height as f32 / t.scale)
    } else {
        (x, y, THUMB_W as f32, THUMB_H as f32)
    }
}
fn empty_message(enabled: bool) -> (&'static str, &'static str, &'static str) {
    if enabled {
        (
            "No saved captures",
            "Your next screenshot will appear here.",
            "Keeps the last 20 images locally on this PC.",
        )
    } else {
        (
            "Capture history is off",
            "Enable Keep capture history in Options > Settings > General.",
            "Keeps the last 20 images locally. Auto-save is a separate option.",
        )
    }
}
/// Render the production empty state headlessly, without disk access.
pub fn draw_empty_history(pixmap: &mut Pixmap, history_enabled: bool) {
    draw_history(pixmap, &[], 0, None, 0., false, history_enabled, false);
}
fn draw_history(
    pixmap: &mut Pixmap,
    thumbnails: &[Thumbnail],
    stored_entries: usize,
    hovered: Option<usize>,
    scroll_offset: f32,
    hover_clear: bool,
    history_enabled: bool,
    loading: bool,
) {
    let scale = pixmap.width() as f32 / WIN_W as f32;
    // Background
    fill_rect_rgb(
        pixmap,
        0.0,
        0.0,
        WIN_W as f32,
        WIN_H as f32,
        crate::theme::bg_1(),
    );

    if thumbnails.is_empty() {
        let (title, detail, note) = if loading {
            ("Loading captures...", "Preparing local thumbnails.", "")
        } else if stored_entries > 0 {
            (
                "Captures could not be displayed",
                "Stored history images could not be decoded.",
                "Use Clear All to remove them.",
            )
        } else {
            empty_message(history_enabled)
        };
        draw_label(
            pixmap,
            PADDING,
            88.,
            title,
            18.,
            crate::theme::text_normal(),
        );
        draw_label(
            pixmap,
            PADDING,
            126.,
            detail,
            13.,
            crate::theme::text_muted(),
        );
        draw_label(pixmap, PADDING, 154., note, 13., crate::theme::text_muted());
    } else {
        for (i, thumb) in thumbnails.iter().enumerate() {
            let (x, y, tw, th) = thumb_rect(thumbnails, i, scroll_offset);

            // Skip thumbnails fully outside the viewport
            if y + th < HEADER_H || y > WIN_H as f32 {
                continue;
            }

            // Border (highlight on hover)
            let border_color = if hovered == Some(i) {
                crate::theme::accent()
            } else {
                crate::theme::bg_3()
            };
            fill_rect_rgb(pixmap, x - 2.0, y - 2.0, tw + 4.0, th + 4.0, border_color);

            // Render thumbnail pixels, clipped against the header line
            blit_rgba_clipped(
                pixmap,
                (x * scale).round() as i32,
                (y * scale).round() as i32,
                &thumb.pixels,
                thumb.width,
                thumb.height,
                (HEADER_H * scale).ceil() as i32,
            );
        }

        // Scrollbar indicator when the content overflows
        let max_scroll = max_scroll(thumbnails.len());
        if max_scroll > 0.0 {
            let viewport_h = WIN_H as f32 - HEADER_H;
            let track_h = viewport_h - 8.0;
            let knob_h = (viewport_h / content_height(thumbnails.len()) * track_h).max(24.0);
            let knob_y = HEADER_H + 4.0 + (scroll_offset / max_scroll) * (track_h - knob_h);
            fill_rect_rgb(
                pixmap,
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
        pixmap,
        0.0,
        0.0,
        WIN_W as f32,
        HEADER_H - 3.0,
        crate::theme::bg_1(),
    );
    draw_label(
        pixmap,
        PADDING,
        14.0,
        "Recent captures",
        16.0,
        crate::theme::text_normal(),
    );
    // Separator
    fill_rect_rgb(
        pixmap,
        PADDING,
        HEADER_H - 4.0,
        WIN_W as f32 - PADDING * 2.0,
        1.0,
        crate::theme::bg_3(),
    );
    // Clear All button
    if stored_entries > 0 {
        let (cx, cy, cw, ch) = clear_rect();
        let bg = if hover_clear {
            crate::theme::bg_4()
        } else {
            crate::theme::bg_3()
        };
        fill_rect_rgb(pixmap, cx, cy, cw, ch, bg);
        let label = "Clear All";
        let tw = crate::ui::measure(label, 12.0);
        draw_label(
            pixmap,
            cx + (cw - tw) / 2.0,
            cy + 5.0,
            label,
            12.0,
            crate::theme::text_normal(),
        );
    }
}

fn fill_rect_rgb(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, (r, g, b): (u8, u8, u8)) {
    let rect = match Rect::from_xywh(x, y, w, h) {
        Some(r) => r,
        None => return,
    };
    let mut paint = Paint::default();
    paint.set_color(SkiaColor::from_rgba8(r, g, b, 255));
    paint.anti_alias = true;
    let scale = pixmap.width() as f32 / WIN_W as f32;
    pixmap.fill_rect(rect, &paint, Transform::from_scale(scale, scale), None);
}

fn draw_label(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    text: &str,
    font_size: f32,
    (r, g, b): (u8, u8, u8),
) {
    let scale = pixmap.width() as f32 / WIN_W as f32;
    crate::ui::draw_text(
        pixmap,
        Point::new(x * scale, y * scale),
        text,
        font_size * scale,
        (r, g, b),
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    fn red_thumbnail(scale: f32) -> Thumbnail {
        let width = (THUMB_W as f32 * scale) as u32;
        let height = (THUMB_H as f32 * scale) as u32;
        Thumbnail {
            path: PathBuf::from("synthetic.png"),
            pixels: [255, 0, 0, 255].repeat((width * height) as usize),
            width,
            height,
            scale,
        }
    }

    #[test]
    fn native_thumbnails_keep_logical_hit_bounds_across_dpi() {
        for dpi in [1., 1.25, 1.5, 2.] {
            let thumbs = vec![red_thumbnail(dpi)];
            assert_eq!(thumb_rect(&thumbs, 0, 0.), (16., 60., 120., 80.));
            let mut p =
                Pixmap::new((WIN_W as f32 * dpi) as u32, (WIN_H as f32 * dpi) as u32).unwrap();
            draw_history(&mut p, &thumbs, 1, Some(0), 0., false, true, false);
            for (x, y) in [(17., 61.), (135., 139.)] {
                let i = (((y * dpi) as u32 * p.width() + (x * dpi) as u32) * 4) as usize;
                assert_eq!(&p.data()[i..i + 4], &[255, 0, 0, 255]);
            }
        }
    }

    #[test]
    fn scrolled_thumbnail_does_not_overpaint_the_header_at_high_dpi() {
        let dpi = 2.;
        let thumbs = vec![red_thumbnail(dpi)];
        let mut p = Pixmap::new(WIN_W * 2, WIN_H * 2).unwrap();
        draw_history(&mut p, &thumbs, 1, None, 40., false, true, false);
        // Inside the image's horizontal span, but above the header separator.
        let i = ((30 * 2 * p.width() + 125 * 2) * 4) as usize;
        assert_ne!(&p.data()[i..i + 4], &[255, 0, 0, 255]);
        let i = ((50 * 2 * p.width() + 125 * 2) * 4) as usize;
        assert_eq!(&p.data()[i..i + 4], &[255, 0, 0, 255]);
    }

    #[test]
    fn stored_history_without_decodable_thumbnails_still_shows_clear_all() {
        let mut pixmap = Pixmap::new(WIN_W, WIN_H).unwrap();
        draw_history(&mut pixmap, &[], 1, None, 0., false, true, false);

        let i = ((12 * pixmap.width() + 505) * 4) as usize;
        let (r, g, b) = crate::theme::bg_3();
        assert_eq!(&pixmap.data()[i..i + 4], &[r, g, b, 255]);
    }
}
