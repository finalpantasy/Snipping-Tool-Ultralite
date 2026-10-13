// X11 screen capture (Linux only).
//
// Captures the root window of the default screen via GetImage, which covers
// the whole virtual desktop (all monitors) just like the Windows backend.
// Under a pure Wayland session this only works through XWayland and will
// usually return the compositor's X11 clients only — native Wayland capture
// (xdg-desktop-portal screencopy) is a separate future backend.

use super::{CaptureError, CapturedScreen, ScreenCapture};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, ImageFormat, ImageOrder};

pub struct X11Capturer;

impl X11Capturer {
    pub fn new() -> Result<Self, CaptureError> {
        Ok(Self)
    }
}

impl ScreenCapture for X11Capturer {
    fn capture_all_screens(&self) -> Result<Vec<CapturedScreen>, CaptureError> {
        capture_root().map(|screen| vec![screen])
    }
}

fn capture_root() -> Result<CapturedScreen, CaptureError> {
    let (conn, screen_num) = x11rb::connect(None).map_err(|e| {
        CaptureError::PlatformError(format!(
            "Failed to connect to X11 display: {e} \
             (Wayland-native capture is not supported yet — a DISPLAY/XWayland session is required)"
        ))
    })?;

    let setup = conn.setup();
    let screen = &setup.roots[screen_num];
    let width = screen.width_in_pixels;
    let height = screen.height_in_pixels;
    if width == 0 || height == 0 {
        return Err(CaptureError::NoDisplay);
    }

    let reply = conn
        .get_image(
            ImageFormat::Z_PIXMAP,
            screen.root,
            0,
            0,
            width,
            height,
            !0u32,
        )
        .map_err(|e| CaptureError::PlatformError(format!("GetImage request failed: {e}")))?
        .reply()
        .map_err(|e| CaptureError::PlatformError(format!("GetImage failed: {e}")))?;

    let bits_per_pixel = setup
        .pixmap_formats
        .iter()
        .find(|f| f.depth == reply.depth)
        .map(|f| f.bits_per_pixel)
        .ok_or_else(|| {
            CaptureError::PlatformError(format!("No pixmap format for depth {}", reply.depth))
        })?;

    if bits_per_pixel != 32 {
        return Err(CaptureError::PlatformError(format!(
            "Unsupported X11 pixel format: depth {} at {} bpp (expected 32 bpp)",
            reply.depth, bits_per_pixel
        )));
    }

    let w = width as usize;
    let h = height as usize;
    let data = reply.data;
    // Rows are padded to the server's scanline pad. At 32 bpp the stride is
    // normally exactly w*4, but derive it from the buffer to be safe.
    let stride = data.len().checked_div(h).unwrap_or(0);
    if stride < w * 4 {
        return Err(CaptureError::PlatformError(
            "GetImage returned an undersized buffer".to_string(),
        ));
    }

    let lsb_first = setup.image_byte_order == ImageOrder::LSB_FIRST;
    let mut pixels = vec![0u8; w * h * 4];
    for y in 0..h {
        let src_row = &data[y * stride..y * stride + w * 4];
        let dst_row = &mut pixels[y * w * 4..(y + 1) * w * 4];
        for x in 0..w {
            let s = &src_row[x * 4..x * 4 + 4];
            let d = &mut dst_row[x * 4..x * 4 + 4];
            // ZPixmap depth-24/32 stores BGRX in LSB-first byte order (the
            // common case for x86 servers); byte-wise XRGB when MSB-first.
            let (r, g, b) = if lsb_first {
                (s[2], s[1], s[0])
            } else {
                (s[1], s[2], s[3])
            };
            d[0] = r;
            d[1] = g;
            d[2] = b;
            d[3] = 255; // force opaque — the X byte is undefined padding
        }
    }

    Ok(CapturedScreen {
        pixels,
        width: width as u32,
        height: height as u32,
        x_offset: 0,
        y_offset: 0,
        scale_factor: 1.0,
    })
}
