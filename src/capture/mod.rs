#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::{capture_region, initialize_dpi, virtual_bounds};

/// Physical virtual-desktop coordinates, including negative monitor origins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl ScreenRect {
    pub fn between(a: (i32, i32), b: (i32, i32)) -> Self {
        Self {
            x: a.0.min(b.0),
            y: a.1.min(b.1),
            width: a.0.abs_diff(b.0),
            height: a.1.abs_diff(b.1),
        }
    }

    pub fn intersect(self, bounds: Self) -> Option<Self> {
        let x = i64::from(self.x).max(i64::from(bounds.x));
        let y = i64::from(self.y).max(i64::from(bounds.y));
        let right = (i64::from(self.x) + i64::from(self.width))
            .min(i64::from(bounds.x) + i64::from(bounds.width));
        let bottom = (i64::from(self.y) + i64::from(self.height))
            .min(i64::from(bounds.y) + i64::from(bounds.height));
        (right > x && bottom > y).then_some(Self {
            x: x as i32,
            y: y as i32,
            width: (right - x) as u32,
            height: (bottom - y) as u32,
        })
    }
}

impl std::str::FromStr for ScreenRect {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = value.split(',').map(str::trim).collect();
        if parts.len() != 4 {
            return Err("Use physical x,y,width,height".into());
        }
        let rect = Self {
            x: parts[0].parse().map_err(|_| "Invalid x")?,
            y: parts[1].parse().map_err(|_| "Invalid y")?,
            width: parts[2].parse().map_err(|_| "Invalid width")?,
            height: parts[3].parse().map_err(|_| "Invalid height")?,
        };
        if rect.width == 0 || rect.height == 0 {
            return Err("Region dimensions must be positive".into());
        }
        Ok(rect)
    }
}

#[cfg(target_os = "linux")]
mod x11;

#[cfg(target_os = "linux")]
mod wayland;

/// A captured screen's pixel data and metadata.
#[derive(Debug, Clone)]
pub struct CapturedScreen {
    /// RGBA8 pixels, row-major order.
    pub pixels: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// X offset of this screen in virtual-desktop coordinates.
    pub x_offset: i32,
    /// Y offset of this screen in virtual-desktop coordinates.
    pub y_offset: i32,
    /// Display scale factor (e.g. 1.0, 1.25, 1.5, 2.0).
    pub scale_factor: f64,
}

/// Errors that can occur during screen capture.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("Permission denied")]
    PermissionDenied,
    #[error("No display available")]
    NoDisplay,
    #[error("Platform error: {0}")]
    PlatformError(String),
}

/// Trait for platform-specific screen capture implementations.
pub trait ScreenCapture {
    /// Capture all connected screens and return their pixel data.
    fn capture_all_screens(&self) -> Result<Vec<CapturedScreen>, CaptureError>;
}

/// Create a platform-appropriate screen capturer.
pub fn create_capturer() -> Result<Box<dyn ScreenCapture>, CaptureError> {
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::WindowsCapturer::new()))
    }

    #[cfg(target_os = "linux")]
    {
        // X11 (or XWayland). Native Wayland capture via xdg-desktop-portal
        // screencopy is not implemented yet — see capture/wayland.rs.
        Ok(Box::new(x11::X11Capturer::new()?))
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err(CaptureError::PlatformError(
            "Unsupported platform".to_string(),
        ))
    }
}
