//! Shared quiet native controls. Coordinates are logical; rasterization is physical.
use crate::{
    geometry::Point,
    icons::{blend_pixmap, IconCache},
};
use tiny_skia::{Paint, PathBuilder, Pixmap, Transform};

pub const INK: (u8, u8, u8) = (237, 234, 243);
pub const MUTED: (u8, u8, u8) = (189, 183, 201);
pub const BASE: (u8, u8, u8) = (36, 35, 41);

/// Remove a visible UL window before capture, including minimize animations.
/// Already hidden windows stay hidden; minimizing them could show them again.
pub fn minimize_for_capture(window: &winit::window::Window) -> bool {
    if !window.is_visible().unwrap_or(true) {
        return false;
    }
    #[cfg(target_os = "windows")]
    if let Err(error) = capture_cloak(window, true) {
        tracing::warn!("Could not cloak capture controls: {error}");
    }
    window.set_minimized(true);
    window.set_visible(false);
    true
}

/// Restore only on an explicit reopen or cancelled capture, never on redraw.
pub fn restore_after_capture(window: &winit::window::Window) {
    window.set_visible(true);
    window.set_minimized(false);
    #[cfg(target_os = "windows")]
    if let Err(error) = capture_cloak(window, false) {
        tracing::warn!("Could not restore capture controls: {error}");
    }
}

/// Activate a completed preview once, without sending synthetic input.
pub fn activate_completed_preview(window: &winit::window::Window) {
    if !window.is_visible().unwrap_or(false) || window.is_minimized() == Some(true) {
        return;
    }
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::{
            Foundation::HWND,
            UI::WindowsAndMessaging::{
                GetForegroundWindow, SetForegroundWindow, SetWindowPos, HWND_TOP, SWP_NOACTIVATE,
                SWP_NOMOVE, SWP_NOSIZE,
            },
        };
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let Ok(handle) = window.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(raw) = handle.as_raw() else {
            return;
        };
        let hwnd = HWND(raw.hwnd.get() as _);
        unsafe {
            // Raise a normal window once; this does not make it always on top.
            if let Err(error) = SetWindowPos(
                hwnd,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            ) {
                tracing::warn!("Could not raise capture preview: {error}");
            }
            if GetForegroundWindow() != hwnd && !SetForegroundWindow(hwnd).as_bool() {
                // Honor Windows focus restrictions without input injection,
                // persistent topmost state, retries or changing OS settings.
                window
                    .request_user_attention(Some(winit::window::UserAttentionType::Informational));
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    window.focus_window();
}

#[cfg(target_os = "windows")]
fn capture_cloak(window: &winit::window::Window, cloaked: bool) -> Result<(), String> {
    use windows::Win32::{
        Foundation::{BOOL, HWND},
        Graphics::Dwm::*,
    };
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = window.window_handle().map_err(|e| e.to_string())?;
    let RawWindowHandle::Win32(raw) = handle.as_raw() else {
        return Err("Capture controls have no Win32 window handle".into());
    };
    let hwnd = HWND(raw.hwnd.get() as _);
    unsafe {
        let disabled = BOOL(1);
        // DwmFlush waits for a frame, not the entire fade/minimize animation.
        // Disable our transitions and cloak before changing native visibility.
        if cloaked {
            if let Err(error) = DwmSetWindowAttribute(
                hwnd,
                DWMWA_TRANSITIONS_FORCEDISABLED,
                &disabled as *const _ as _,
                std::mem::size_of::<BOOL>() as u32,
            ) {
                tracing::warn!("Could not disable capture-window transitions: {error}");
            }
        }
        let cloak = BOOL(i32::from(cloaked));
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_CLOAK,
            &cloak as *const _ as _,
            std::mem::size_of::<BOOL>() as u32,
        )
        .map_err(|e| e.to_string())
    }
}
pub(crate) fn font() -> &'static crate::font::LazyFont {
    &crate::font::FONT
}
pub fn ink() -> (u8, u8, u8) {
    crate::theme::text_normal()
}
pub fn muted() -> (u8, u8, u8) {
    crate::theme::text_muted()
}
pub fn base() -> (u8, u8, u8) {
    crate::theme::bg_1()
}
pub fn panel() -> (u8, u8, u8) {
    crate::theme::bg_2()
}
pub fn accent() -> (u8, u8, u8) {
    crate::theme::accent()
}
pub fn focus_color() -> (u8, u8, u8) {
    crate::theme::accent_alt()
}
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x && p.y >= self.y && p.x < self.x + self.w && p.y < self.y + self.h
    }
}
pub struct Canvas<'a> {
    pub pixmap: &'a mut Pixmap,
    pub scale: f32,
    pub icons: &'a mut IconCache,
}
impl Canvas<'_> {
    fn rounded_path(r: Rect, radius: f32) -> Option<tiny_skia::Path> {
        let k = radius.min(r.w / 2.).min(r.h / 2.);
        let mut b = PathBuilder::new();
        b.move_to(r.x + k, r.y);
        b.line_to(r.x + r.w - k, r.y);
        b.quad_to(r.x + r.w, r.y, r.x + r.w, r.y + k);
        b.line_to(r.x + r.w, r.y + r.h - k);
        b.quad_to(r.x + r.w, r.y + r.h, r.x + r.w - k, r.y + r.h);
        b.line_to(r.x + k, r.y + r.h);
        b.quad_to(r.x, r.y + r.h, r.x, r.y + r.h - k);
        b.line_to(r.x, r.y + k);
        b.quad_to(r.x, r.y, r.x + k, r.y);
        b.close();
        b.finish()
    }
    pub fn outline(&mut self, r: Rect, radius: f32, rgb: (u8, u8, u8), width: f32) {
        let mut paint = Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba8(rgb.0, rgb.1, rgb.2, 255));
        if let Some(path) = Self::rounded_path(r, radius) {
            self.pixmap.stroke_path(
                &path,
                &paint,
                &tiny_skia::Stroke {
                    width,
                    ..Default::default()
                },
                Transform::from_scale(self.scale, self.scale),
                None,
            );
        }
    }
    pub fn round(&mut self, r: Rect, radius: f32, rgb: (u8, u8, u8), alpha: f32) {
        let mut paint = Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba8(
            rgb.0,
            rgb.1,
            rgb.2,
            (alpha.clamp(0., 1.) * 255.).round() as u8,
        ));
        if let Some(path) = Self::rounded_path(r, radius) {
            self.pixmap.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::Winding,
                Transform::from_scale(self.scale, self.scale),
                None,
            );
        }
    }
    pub fn label(&mut self, x: f32, y: f32, text: &str, size: f32, rgb: (u8, u8, u8)) {
        draw_text(
            self.pixmap,
            Point::new(x * self.scale, y * self.scale),
            text,
            size * self.scale,
            rgb,
        );
    }
    pub fn centered(&mut self, x: f32, y: f32, text: &str, size: f32, rgb: (u8, u8, u8)) {
        let width = measure(text, size * self.scale) / self.scale;
        self.label(x - width / 2., y, text, size, rgb);
    }
    pub fn centered_in(&mut self, r: Rect, text: &str, size: f32, rgb: (u8, u8, u8)) {
        let (x0, y0, x1, y1) = text_bounds(text, size * self.scale);
        self.label(
            r.x + r.w / 2. - (x0 + x1) / 2. / self.scale,
            r.y + r.h / 2. - (y0 + y1) / 2. / self.scale,
            text,
            size,
            rgb,
        );
    }
    pub fn icon(&mut self, name: &str, x: f32, y: f32, size: f32, rgb: (u8, u8, u8)) {
        let color = format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2);
        if let Some(icon) =
            self.icons
                .get_or_render(name, (size * self.scale).round().max(1.) as u32, &color)
        {
            blend_pixmap(
                self.pixmap,
                icon,
                (x * self.scale).round() as i32,
                (y * self.scale).round() as i32,
            );
        }
    }
    pub fn button(
        &mut self,
        r: Rect,
        label: &str,
        icon: Option<&str>,
        primary: bool,
        hover: bool,
        enabled: bool,
    ) {
        self.round(
            r,
            7.,
            if primary {
                accent()
            } else {
                crate::theme::bg_3()
            },
            if primary {
                1.
            } else if hover {
                0.9
            } else {
                0.65
            },
        );
        let color = if !enabled {
            muted()
        } else if primary {
            crate::theme::text_on_accent()
        } else {
            ink()
        };
        if let Some(icon) = icon {
            let (x0, y0, x1, y1) = text_bounds(label, 13. * self.scale);
            let width = 24. + (x1 - x0) / self.scale;
            let left = r.x + (r.w - width) / 2.;
            self.icon(icon, left, r.y + r.h / 2. - 8., 16., color);
            self.label(
                left + 24. - x0 / self.scale,
                r.y + r.h / 2. - (y0 + y1) / 2. / self.scale,
                label,
                13.,
                color,
            );
        } else {
            self.centered_in(r, label, 13., color);
        }
    }
    /// Event-driven help for pointer hover and keyboard focus, bounded to the viewport.
    pub fn hint(&mut self, anchor: Rect, text: &str, above: bool) {
        let width = measure(text, 11. * self.scale) / self.scale + 20.;
        let viewport = self.pixmap.width() as f32 / self.scale;
        let x = anchor.x.clamp(12., (viewport - width - 12.).max(12.));
        let y = if above {
            anchor.y - 34.
        } else {
            anchor.y + anchor.h + 8.
        };
        self.round(Rect::new(x, y, width, 26.), 6., (54, 51, 62), 1.);
        self.label(x + 10., y + 5., text, 11., INK);
    }
}
pub fn measure(text: &str, size: f32) -> f32 {
    let font = crate::font::FONT.for_text(text);
    text.chars()
        .map(|ch| font.metrics(ch, size).advance_width)
        .sum()
}
pub fn text_bounds(text: &str, size: f32) -> (f32, f32, f32, f32) {
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
            x0 = x0.min(cursor + m.xmin as f32);
            x1 = x1.max(cursor + m.xmin as f32 + m.width as f32);
            y0 = y0.min(ascent - m.ymin as f32 - m.height as f32);
            y1 = y1.max(ascent - m.ymin as f32);
        }
        cursor += m.advance_width;
    }
    if x0.is_finite() {
        (x0, y0, x1, y1)
    } else {
        (0., 0., cursor, size)
    }
}
pub(crate) fn draw_text(p: &mut Pixmap, pos: Point, text: &str, size: f32, rgb: (u8, u8, u8)) {
    let font = crate::font::FONT.for_text(text);
    let ascent = font
        .horizontal_line_metrics(size)
        .map_or(size, |m| m.ascent);
    let mut cursor = pos.x;
    let width = p.width() as i32;
    let height = p.height() as i32;
    for ch in text.chars() {
        let (m, mask) = font.rasterize(ch, size);
        let x = cursor.round() as i32 + m.xmin;
        let y = (pos.y + ascent).round() as i32 - m.ymin - m.height as i32;
        for row in 0..m.height {
            for col in 0..m.width {
                let px = x + col as i32;
                let py = y + row as i32;
                if px < 0 || py < 0 || px >= width || py >= height {
                    continue;
                }
                let a = mask[row * m.width + col] as u32;
                if a == 0 {
                    continue;
                }
                let i = ((py * width + px) * 4) as usize;
                let dst = &mut p.data_mut()[i..i + 4];
                let inv = 255 - a;
                for (c, src) in [rgb.0, rgb.1, rgb.2].iter().enumerate() {
                    dst[c] = ((*src as u32 * a + dst[c] as u32 * inv + 127) / 255) as u8;
                }
                dst[3] = (a + (dst[3] as u32 * inv + 127) / 255) as u8;
            }
        }
        cursor += m.advance_width;
    }
}
pub fn glass_fill(p: &mut Pixmap) {
    let rgb = base();
    p.fill(tiny_skia::Color::from_rgba8(
        rgb.0,
        rgb.1,
        rgb.2,
        crate::theme::glass_alpha(),
    ));
}
/// Composites premultiplied RGBA for an opaque fallback or headless evidence.
pub fn opaque_rgb(rgba: &[u8]) -> Vec<u32> {
    let rgb = base();
    rgba.chunks_exact(4)
        .map(|p| {
            let inv = 255 - p[3] as u32;
            let c = |i: usize, b: u8| p[i] as u32 + (b as u32 * inv + 127) / 255;
            c(0, rgb.0) << 16 | c(1, rgb.1) << 8 | c(2, rgb.2)
        })
        .collect()
}
