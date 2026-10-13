//! An on-demand About window, independent of Windows notification settings.
use std::sync::Arc;

use tiny_skia::{Pixmap, PixmapPaint, Transform};
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes},
};

use crate::{
    geometry::Point,
    icons::IconCache,
    native_surface::Surface,
    ui::{self, Canvas, Rect},
};

pub const WIDTH: u32 = 440;
pub const HEIGHT: u32 = 300;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn close_rect() -> Rect {
    Rect::new(164., 242., 112., 36.)
}

/// Uses the same physical-pixel font rendering and frost as the main window.
pub fn draw(frame: &mut Pixmap, hovered: bool) {
    ui::glass_fill(frame);
    let scale = frame.width() as f32 / WIDTH as f32;
    // Decode the supplied high-resolution mark only when About is first opened.
    // Retain 64 KiB, enough for crisp rendering through 200% display scaling.
    static LOGO: std::sync::LazyLock<Pixmap> = std::sync::LazyLock::new(|| {
        let source =
            Pixmap::decode_png(include_bytes!("../assets/ul-icon.png")).expect("Embedded logo");
        let mut logo = Pixmap::new(128, 128).unwrap();
        let paint = PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..Default::default()
        };
        logo.draw_pixmap(
            0,
            0,
            source.as_ref(),
            &paint,
            Transform::from_scale(128. / source.width() as f32, 128. / source.height() as f32),
            None,
        );
        logo
    });
    let paint = PixmapPaint {
        quality: tiny_skia::FilterQuality::Bilinear,
        ..Default::default()
    };
    frame.draw_pixmap(
        0,
        0,
        LOGO.as_ref(),
        &paint,
        Transform::from_scale(scale / 2., scale / 2.).post_translate(188. * scale, 20. * scale),
        None,
    );
    let mut icons = IconCache::new();
    let mut c = Canvas {
        pixmap: frame,
        scale,
        icons: &mut icons,
    };
    c.centered(220., 96., "Snipping Tool UL", 24., ui::ink());
    c.centered(220., 131., &format!("Version {VERSION}"), 13., ui::muted());
    c.centered(
        220.,
        158.,
        "Ultralight in performance, not features.",
        14.,
        ui::ink(),
    );
    c.centered(
        220.,
        195.,
        "Open source under the MIT license.",
        12.,
        ui::muted(),
    );
    c.centered(
        220.,
        214.,
        "Third-party notices are included with the app.",
        12.,
        ui::muted(),
    );
    c.button(close_rect(), "Close", None, true, hovered, true);
}

pub struct AboutWindow {
    // Release native presentation resources before the last window reference.
    surface: Surface,
    pub window: Arc<Window>,
    frame: Pixmap,
    pointer: Point,
    pressed_close: bool,
}

impl AboutWindow {
    /// Create hidden, present once, then let the caller choose when to show it.
    pub fn new(event_loop: &ActiveEventLoop) -> Result<Self, String> {
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("About Snipping Tool UL")
                        .with_inner_size(winit::dpi::LogicalSize::new(WIDTH, HEIGHT))
                        .with_resizable(false)
                        .with_maximized(false)
                        .with_visible(false),
                )
                .map_err(|e| e.to_string())?,
        );
        let surface = Surface::new(window.clone())?;
        let size = window.inner_size();
        let frame =
            Pixmap::new(size.width.max(1), size.height.max(1)).ok_or("About allocation failed")?;
        let mut about = Self {
            surface,
            window,
            frame,
            pointer: Point::new(-1., -1.),
            pressed_close: false,
        };
        about.render()?;
        Ok(about)
    }

    pub fn show(&self) {
        self.window.set_visible(true);
        self.window.focus_window();
    }

    pub fn render(&mut self) -> Result<(), String> {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        if (self.frame.width(), self.frame.height()) != (size.width, size.height) {
            self.frame = Pixmap::new(size.width, size.height).ok_or("About allocation failed")?;
        }
        draw(&mut self.frame, close_rect().contains(self.pointer));
        self.surface.present(&self.frame)
    }

    /// Returns true only when this dialog should close; never exits the app.
    pub fn event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CloseRequested => return true,
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && matches!(
                        event.logical_key,
                        Key::Named(NamedKey::Escape | NamedKey::Enter | NamedKey::Space)
                    ) =>
            {
                return true
            }
            WindowEvent::CursorMoved { position, .. } => {
                let old = close_rect().contains(self.pointer);
                let scale = self.window.inner_size().width.max(1) as f32 / WIDTH as f32;
                self.pointer = Point::new(position.x as f32 / scale, position.y as f32 / scale);
                if old != close_rect().contains(self.pointer) {
                    self.window.request_redraw();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer = Point::new(-1., -1.);
                self.window.request_redraw();
            }
            WindowEvent::Focused(false) => self.pressed_close = false,
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                if *state == ElementState::Pressed {
                    self.pressed_close = close_rect().contains(self.pointer);
                } else if std::mem::take(&mut self.pressed_close)
                    && close_rect().contains(self.pointer)
                {
                    return true;
                }
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                crate::launcher::style_window(&self.window);
                if let Err(e) = self.render() {
                    tracing::error!("About presentation failed: {e}");
                }
            }
            _ => {}
        }
        false
    }
}
