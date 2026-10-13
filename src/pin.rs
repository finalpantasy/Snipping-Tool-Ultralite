//! Pinned always-on-top capture windows.
//!
//! A pin shows a captured selection in a borderless floating window framed by
//! a themed border. Drag to move, middle-click to copy, right-click to
//! reveal the backing temp file, Escape or close to dismiss.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use notify_rust::Notification;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{CursorIcon, Window, WindowAttributes, WindowId, WindowLevel};

use crate::export;

/// Border thickness for pinned windows (themed frame)
const PIN_BORDER: u32 = 3;
/// Shadow offset for pinned windows
const PIN_SHADOW: u32 = 2;

pub struct PinnedWindow {
    window: Arc<Window>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    /// RGBA pixels of the pinned image (includes border + shadow frame).
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    dragging: bool,
    drag_start: Option<winit::dpi::PhysicalPosition<f64>>,
    temp_path: Option<PathBuf>,
}

impl Drop for PinnedWindow {
    fn drop(&mut self) {
        // Clean the drag-and-drop temp file up on close AND on app exit.
        if let Some(ref path) = self.temp_path {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl PinnedWindow {
    /// Create a pin for `pixels` (RGBA, `img_w` x `img_h`), positioned so the
    /// image content sits at screen position (`screen_x`, `screen_y`).
    pub fn create(
        event_loop: &ActiveEventLoop,
        pixels: &[u8],
        img_w: u32,
        img_h: u32,
        screen_x: i32,
        screen_y: i32,
    ) -> Option<Self> {
        let border = PIN_BORDER;
        let shadow = PIN_SHADOW;
        let total_w = img_w + border * 2 + shadow;
        let total_h = img_h + border * 2 + shadow;

        // Build the framed pixel buffer
        let mut framed = vec![0u8; (total_w * total_h * 4) as usize];

        // Shadow fill (dark in both themes, offset bottom-right)
        for y in shadow..total_h {
            for x in shadow..total_w {
                let i = ((y * total_w + x) * 4) as usize;
                if i + 3 < framed.len() {
                    framed[i] = 0;
                    framed[i + 1] = 0;
                    framed[i + 2] = 0;
                    framed[i + 3] = 100; // semi-transparent (visible in the saved PNG)
                }
            }
        }

        // Border fill (theme accent)
        let (bd_r, bd_g, bd_b) = crate::theme::accent();
        for y in 0..total_h - shadow {
            for x in 0..total_w - shadow {
                let i = ((y * total_w + x) * 4) as usize;
                if i + 3 < framed.len() {
                    framed[i] = bd_r; // theme accent R
                    framed[i + 1] = bd_g;
                    framed[i + 2] = bd_b;
                    framed[i + 3] = 255;
                }
            }
        }

        // Copy the actual image inside the border
        for y in 0..img_h {
            for x in 0..img_w {
                let src = ((y * img_w + x) * 4) as usize;
                let dst = (((y + border) * total_w + x + border) * 4) as usize;
                if src + 3 < pixels.len() && dst + 3 < framed.len() {
                    framed[dst..dst + 4].copy_from_slice(&pixels[src..src + 4]);
                }
            }
        }

        let pin_x = screen_x - border as i32;
        let pin_y = screen_y - border as i32;

        let attrs = WindowAttributes::default()
            .with_title("Snipping Tool UL Pin")
            .with_inner_size(winit::dpi::PhysicalSize::new(total_w, total_h))
            .with_position(winit::dpi::PhysicalPosition::new(pin_x, pin_y))
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_decorations(false);

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!("Failed to create pin window: {e}");
                return None;
            }
        };

        let context = match softbuffer::Context::new(Arc::clone(&window)) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create pin softbuffer context: {e}");
                return None;
            }
        };

        let surface = match softbuffer::Surface::new(&context, Arc::clone(&window)) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create pin softbuffer surface: {e}");
                return None;
            }
        };

        // Save the pin image to a temp file for drag-and-drop / reveal support
        let temp_path = {
            let temp_dir = std::env::temp_dir();
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let path = temp_dir.join(format!("snipping_tool_ul_pin_{ts}.png"));
            image::RgbaImage::from_raw(total_w, total_h, framed.clone()).and_then(|img| {
                match img.save(&path) {
                    Ok(()) => {
                        tracing::info!("Pin temp file saved to {}", path.display());
                        Some(path)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to save pin temp file: {e}");
                        None
                    }
                }
            })
        };

        let mut pin = Self {
            window,
            surface,
            pixels: framed,
            width: total_w,
            height: total_h,
            dragging: false,
            drag_start: None,
            temp_path,
        };
        pin.render();
        pin.window.set_cursor(CursorIcon::Grab);
        tracing::info!("Pinned {}x{} capture to screen", total_w, total_h);
        Some(pin)
    }

    pub fn window_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn render(&mut self) {
        let (w, h) = (self.width, self.height);
        if let (Some(nz_w), Some(nz_h)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
            if self.surface.resize(nz_w, nz_h).is_err() {
                return;
            }
            if let Ok(mut buffer) = self.surface.buffer_mut() {
                let pixel_count = (w * h) as usize;
                for (i, chunk) in self.pixels.chunks_exact(4).take(pixel_count).enumerate() {
                    buffer[i] =
                        ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
                }
                let _ = buffer.present();
            }
        }
    }

    /// Handle an event for this pin's window. Returns true when the pin
    /// should be closed (Escape pressed or window close requested).
    pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CloseRequested => return true,

            WindowEvent::RedrawRequested => self.render(),

            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if let Key::Named(NamedKey::Escape) = &event.logical_key {
                    return true;
                }
            }

            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => {
                    self.dragging = true;
                    self.drag_start = None;
                    self.window.set_cursor(CursorIcon::Grabbing);
                }
                ElementState::Released => {
                    self.dragging = false;
                    self.drag_start = None;
                    self.window.set_cursor(CursorIcon::Grab);
                }
            },

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                if let Some(ref path) = self.temp_path {
                    reveal_in_file_manager(path);
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Middle,
                ..
            } => {
                let pixels = self.pixels.clone();
                let (w, h) = (self.width, self.height);
                std::thread::spawn(move || match export::copy_to_clipboard(&pixels, w, h) {
                    Ok(()) => {
                        let _ = Notification::new()
                            .summary("Snipping Tool UL")
                            .body("Pin image copied to clipboard")
                            .timeout(2000)
                            .show();
                    }
                    Err(e) => {
                        tracing::error!("Pin clipboard copy failed: {e}");
                    }
                });
            }

            WindowEvent::CursorMoved { position, .. } if self.dragging => {
                if let Some(start) = self.drag_start {
                    let dx = position.x - start.x;
                    let dy = position.y - start.y;
                    if let Ok(current_pos) = self.window.outer_position() {
                        let new_x = current_pos.x + dx as i32;
                        let new_y = current_pos.y + dy as i32;
                        self.window
                            .set_outer_position(winit::dpi::PhysicalPosition::new(new_x, new_y));
                    }
                    // Don't update drag_start — cursor position is window-relative,
                    // so it stays constant while the window tracks the cursor.
                } else {
                    self.drag_start = Some(*position);
                }
            }

            _ => {}
        }
        false
    }
}

#[cfg(target_os = "windows")]
fn reveal_in_file_manager(path: &std::path::Path) {
    let _ = std::process::Command::new("explorer")
        .arg("/select,")
        .arg(path)
        .spawn();
}

#[cfg(target_os = "linux")]
fn reveal_in_file_manager(path: &std::path::Path) {
    let _ = std::process::Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .spawn();
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn reveal_in_file_manager(_path: &std::path::Path) {}
