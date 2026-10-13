//! Delayed-capture countdown overlay: a small always-on-top window showing
//! the remaining seconds, centered on the primary monitor.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

use crate::geometry::Point;

const CD_SIZE: u32 = 120;

pub struct Countdown {
    window: Arc<Window>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    remaining: u32,
    next_tick: Instant,
    deadline: Instant,
}

impl Countdown {
    pub fn start(event_loop: &ActiveEventLoop, seconds: u32) -> Option<Self> {
        // Center on the primary monitor, accounting for the monitor's own
        // position in the virtual desktop.
        let (x, y) = if let Some(monitor) = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
        {
            let size = monitor.size();
            let pos = monitor.position();
            (
                pos.x + (size.width as i32 - CD_SIZE as i32) / 2,
                pos.y + (size.height as i32 - CD_SIZE as i32) / 2,
            )
        } else {
            (800, 400)
        };

        let attrs = WindowAttributes::default()
            .with_title("Snipping Tool UL Countdown")
            .with_inner_size(winit::dpi::PhysicalSize::new(CD_SIZE, CD_SIZE))
            .with_position(winit::dpi::PhysicalPosition::new(x, y))
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false)
            // A delayed capture exists to grab menus and tooltips; taking focus
            // would dismiss them and deactivate the window being captured.
            .with_active(false);

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!("Failed to create countdown window: {e}");
                return None;
            }
        };

        let context = match softbuffer::Context::new(Arc::clone(&window)) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create countdown context: {e}");
                return None;
            }
        };

        let surface = match softbuffer::Surface::new(&context, Arc::clone(&window)) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create countdown surface: {e}");
                return None;
            }
        };

        let started = Instant::now();
        let deadline = started + Duration::from_secs(seconds as u64);
        let mut cd = Self {
            window,
            surface,
            remaining: seconds,
            next_tick: (started + Duration::from_secs(1)).min(deadline),
            deadline,
        };
        cd.render();
        Some(cd)
    }

    pub fn window_id(&self) -> WindowId {
        self.window.id()
    }

    /// The instant at which the next second elapses.
    pub fn next_tick(&self) -> Instant {
        self.next_tick
    }

    /// Advance the countdown if a second has elapsed.
    /// Returns true when the countdown just reached zero (time to capture).
    pub fn tick(&mut self, now: Instant) -> bool {
        if now < self.next_tick {
            return false;
        }
        let (remaining, next_tick) = countdown_at(self.deadline, now);
        self.remaining = remaining;
        self.next_tick = next_tick;
        if self.remaining > 0 {
            self.render();
            false
        } else {
            true
        }
    }

    /// Hide the window immediately — set_visible(false) is processed by the OS
    /// before the next screenshot is taken.
    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    pub fn render(&mut self) {
        // Create a pixmap and draw the countdown number
        let mut pixmap = match tiny_skia::Pixmap::new(CD_SIZE, CD_SIZE) {
            Some(p) => p,
            None => return,
        };

        // Background: theme surface (opaque — softbuffer has no alpha)
        let (bg_r, bg_g, bg_b) = crate::theme::bg_1();
        let pixels_data = pixmap.data_mut();
        for chunk in pixels_data.chunks_exact_mut(4) {
            chunk[0] = bg_r;
            chunk[1] = bg_g;
            chunk[2] = bg_b;
            chunk[3] = 255;
        }

        // Render the number, centered using real font metrics
        let num_str = self.remaining.to_string();
        let text_color = crate::theme::gcolor(crate::theme::accent_alt(), 1.0);
        let font_size = 60.0_f32;

        let text_w = crate::tools::measure_text_width(&num_str, font_size);
        let text_h = crate::tools::measure_text_height(font_size);
        let text_pos = Point::new(
            (CD_SIZE as f32 - text_w) / 2.0,
            (CD_SIZE as f32 - text_h) / 2.0,
        );

        crate::tools::render_text_annotation(
            &mut pixmap,
            &text_pos,
            &num_str,
            &text_color,
            font_size,
        );

        // Copy pixmap to softbuffer surface
        if let (Some(nz_w), Some(nz_h)) = (NonZeroU32::new(CD_SIZE), NonZeroU32::new(CD_SIZE)) {
            if let Err(e) = self.surface.resize(nz_w, nz_h) {
                tracing::error!("Countdown surface resize failed: {e}");
                return;
            }
        }

        if let Ok(mut buffer) = self.surface.buffer_mut() {
            let src = pixmap.data();
            let pixel_count = (CD_SIZE * CD_SIZE) as usize;
            for (i, chunk) in src.chunks_exact(4).take(pixel_count).enumerate() {
                buffer[i] =
                    ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
            }
            let _ = buffer.present();
        }
        // No request_redraw here: tick() renders once per second, and a
        // redraw request from inside render() would loop for the whole delay.
    }
}

/// Derive display/wake state from elapsed time, never from the number of wakeups.
fn countdown_at(deadline: Instant, now: Instant) -> (u32, Instant) {
    let left = deadline.saturating_duration_since(now);
    let remaining = (left.as_secs() + u64::from(left.subsec_nanos() != 0)) as u32;
    let next = deadline - Duration::from_secs(remaining.saturating_sub(1) as u64);
    (remaining, next)
}

#[cfg(test)]
mod clock_tests {
    use super::*;
    #[test]
    fn delayed_wakeup_catches_up_without_extending_capture_delay() {
        let start = Instant::now();
        let deadline = start + Duration::from_secs(3);
        assert_eq!(
            countdown_at(deadline, start + Duration::from_millis(1800)),
            (2, start + Duration::from_secs(2))
        );
        assert_eq!(
            countdown_at(deadline, start + Duration::from_secs(3)),
            (0, deadline)
        );
        assert_eq!(
            countdown_at(deadline, start + Duration::from_secs(7)),
            (0, deadline)
        );
    }
}
