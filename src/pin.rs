//! Pinned always-on-top capture windows.
//!
//! A pin shows a captured selection in a borderless floating window framed by
//! a themed border. Drag to move, Ctrl+drag to drop the image into another
//! app, middle-click to copy, right-click to reveal the backing temp file,
//! Escape or close to dismiss.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
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

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

/// Both the window and its encoder own this guard. The file is removed only
/// after the window closes AND the encoder has released its file handle.
struct TempImage {
    path: PathBuf,
}

impl Drop for TempImage {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub struct PinnedWindow {
    window: Arc<Window>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    /// RGBA pixels of the pinned image (includes border + shadow frame).
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    dragging: bool,
    drag_start: Option<winit::dpi::PhysicalPosition<f64>>,
    temp_path: Option<Arc<TempImage>>,
    ctrl: bool,
    pointer: winit::dpi::PhysicalPosition<f64>,
    /// Where a Ctrl+press started; moving away from it drags the image out.
    drag_out_from: Option<winit::dpi::PhysicalPosition<f64>>,
}

impl PinnedWindow {
    /// The captured image inside the frame, as straight RGBA.
    fn image_pixels(&self) -> (Vec<u8>, u32, u32) {
        let border = PIN_BORDER;
        let img_w = self.width.saturating_sub(border * 2 + PIN_SHADOW);
        let img_h = self.height.saturating_sub(border * 2 + PIN_SHADOW);
        let mut out = Vec::with_capacity((img_w * img_h * 4) as usize);
        for y in 0..img_h {
            let start = (((y + border) * self.width + border) * 4) as usize;
            let end = start + (img_w * 4) as usize;
            if let Some(row) = self.pixels.get(start..end) {
                out.extend_from_slice(row);
            }
        }
        (out, img_w, img_h)
    }

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

        // Save the capture (without the frame) to a temp file for reveal support.
        let temp_path = {
            use image::ImageEncoder;
            let temp_dir = std::env::temp_dir();
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
            let path = temp_dir.join(format!(
                "snipping_tool_ul_pin_{}_{ts}_{sequence}.png",
                std::process::id()
            ));
            let owner = Arc::new(TempImage { path });
            // Encode off the UI thread so the pin appears immediately; a reveal
            // before the file lands simply opens the folder.
            let file_owner = owner.clone();
            let file_pixels = pixels.to_vec();
            crate::background::spawn(move || {
                let file_path = &file_owner.path;
                let encoded = std::fs::File::create(file_path)
                    .map_err(|e| e.to_string())
                    .and_then(|file| {
                        let mut out = std::io::BufWriter::new(file);
                        image::codecs::png::PngEncoder::new_with_quality(
                            &mut out,
                            image::codecs::png::CompressionType::Fast,
                            image::codecs::png::FilterType::Adaptive,
                        )
                        .write_image(&file_pixels, img_w, img_h, image::ExtendedColorType::Rgba8)
                        .map_err(|e| e.to_string())?;
                        use std::io::Write;
                        out.flush().map_err(|e| e.to_string())
                    });
                match encoded {
                    Ok(()) => tracing::info!("Pin temp file saved to {}", file_path.display()),
                    Err(e) => {
                        tracing::warn!("Failed to save pin temp file: {e}");
                        let _ = std::fs::remove_file(file_path);
                    }
                }
            });
            Some(owner)
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
            ctrl: false,
            pointer: winit::dpi::PhysicalPosition::new(0., 0.),
            drag_out_from: None,
        };
        if let Err(error) = pin.render() {
            tracing::error!("Failed to present pin: {error}");
            return None;
        }
        pin.window.set_cursor(CursorIcon::Grab);
        tracing::info!("Pinned {}x{} capture to screen", total_w, total_h);
        Some(pin)
    }

    pub fn window_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn render(&mut self) -> Result<(), String> {
        let (w, h) = (self.width, self.height);
        let (nw, nh) = (
            NonZeroU32::new(w).ok_or("Empty pin width")?,
            NonZeroU32::new(h).ok_or("Empty pin height")?,
        );
        self.surface.resize(nw, nh).map_err(|e| e.to_string())?;
        let mut buffer = self.surface.buffer_mut().map_err(|e| e.to_string())?;
        for (pixel, rgba) in buffer.iter_mut().zip(self.pixels.chunks_exact(4)) {
            *pixel = ((rgba[0] as u32) << 16) | ((rgba[1] as u32) << 8) | rgba[2] as u32;
        }
        buffer.present().map_err(|e| e.to_string())
    }

    /// Handle an event for this pin's window. Returns true when the pin
    /// should be closed (Escape pressed or window close requested).
    pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
        if let WindowEvent::CursorMoved { position, .. } = event {
            self.pointer = *position;
        }
        match event {
            WindowEvent::CloseRequested => return true,

            WindowEvent::RedrawRequested => {
                if let Err(error) = self.render() {
                    tracing::warn!("Pin redraw failed: {error}");
                }
            }

            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if let Key::Named(NamedKey::Escape) = &event.logical_key {
                    return true;
                }
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.ctrl = modifiers.state().control_key();
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if self.ctrl => {
                self.drag_out_from = Some(self.pointer);
            }

            WindowEvent::CursorMoved { position, .. } if self.drag_out_from.is_some() => {
                let start = self.drag_out_from.unwrap_or(*position);
                if (position.x - start.x).hypot(position.y - start.y) > 6. {
                    self.drag_out_from = None;
                    let (pixels, w, h) = self.image_pixels();
                    if let Err(e) = crate::drag_out::drag_image(&pixels, w, h) {
                        tracing::error!("Pin drag out failed: {e}");
                    }
                    self.ctrl = ctrl_down();
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
                    self.drag_out_from = None;
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
                    reveal_in_file_manager(&path.path);
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Middle,
                ..
            } => {
                // Copy the capture itself, not the framed buffer: the frame
                // carries the accent border and transparent shadow corners.
                let (pixels, w, h) = self.image_pixels();
                let ticket = crate::clipboard_queue::reserve();
                crate::background::spawn(move || {
                    match export::copy_with_ticket(ticket, &pixels, w, h) {
                        Ok(true) => {
                            let _ = Notification::new()
                                .summary("Snipping Tool UL")
                                .body("Pin image copied to clipboard")
                                .timeout(2000)
                                .show();
                        }
                        Ok(false) => {} // A newer request superseded this copy.
                        Err(e) => {
                            tracing::error!("Pin clipboard copy failed: {e}");
                        }
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

/// Whether Ctrl is held right now, for after a modal loop ate its key-up.
fn ctrl_down() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL};
        unsafe { GetKeyState(VK_CONTROL.0 as i32) < 0 }
    }
    #[cfg(not(target_os = "windows"))]
    {
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

#[cfg(test)]
mod temp_tests {
    use super::*;
    use std::sync::mpsc;

    fn owner() -> Arc<TempImage> {
        let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        Arc::new(TempImage {
            path: std::env::temp_dir().join(format!(
                "ul-pin-lifetime-test-{}-{sequence}.png",
                std::process::id()
            )),
        })
    }

    #[test]
    fn closing_before_encoder_creates_file_still_cleans_it() {
        let window = owner();
        let path = window.path.clone();
        let encoder = window.clone();
        let (start, wait) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            wait.recv().unwrap();
            std::fs::write(&encoder.path, b"synthetic image").unwrap();
            assert!(encoder.path.exists());
        });
        drop(window);
        start.send(()).unwrap();
        worker.join().unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn closing_during_encoding_keeps_file_until_encoder_releases_it() {
        let window = owner();
        let path = window.path.clone();
        let encoder = window.clone();
        let (ready, opened) = mpsc::channel();
        let (finish, wait) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            {
                use std::io::Write;
                let mut file = std::fs::File::create(&encoder.path).unwrap();
                ready.send(()).unwrap();
                wait.recv().unwrap();
                file.write_all(b"synthetic image").unwrap();
            }
            drop(encoder);
        });
        opened.recv().unwrap();
        drop(window);
        assert!(path.exists());
        finish.send(()).unwrap();
        worker.join().unwrap();
        assert!(!path.exists());
    }
}
