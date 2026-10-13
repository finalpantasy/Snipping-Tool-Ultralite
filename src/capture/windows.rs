use super::{CaptureError, CapturedScreen, ScreenCapture, ScreenRect};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
    GetDeviceCaps, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, LOGPIXELSX,
    SRCCOPY,
};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

pub struct WindowsCapturer;

/// Call before any windows, dialogs or CLI screen metrics are accessed.
pub fn initialize_dpi() {
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    // An embedded manifest can establish this first; ERROR_ACCESS_DENIED then
    // simply means Windows has already assigned the process awareness.
    if let Err(e) =
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
    {
        tracing::debug!("DPI context already assigned or unavailable: {e}");
    }
}

pub fn virtual_bounds() -> ScreenRect {
    unsafe {
        ScreenRect {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            width: GetSystemMetrics(SM_CXVIRTUALSCREEN).max(0) as u32,
            height: GetSystemMetrics(SM_CYVIRTUALSCREEN).max(0) as u32,
        }
    }
}

pub fn capture_region(rect: ScreenRect) -> Result<CapturedScreen, CaptureError> {
    let rect = rect
        .intersect(virtual_bounds())
        .ok_or(CaptureError::NoDisplay)?;
    unsafe { capture_rect(rect) }
}

impl WindowsCapturer {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenCapture for WindowsCapturer {
    fn capture_all_screens(&self) -> Result<Vec<CapturedScreen>, CaptureError> {
        unsafe { capture_virtual_desktop() }.map(|screen| vec![screen])
    }
}

unsafe fn capture_virtual_desktop() -> Result<CapturedScreen, CaptureError> {
    capture_region(virtual_bounds())
}

unsafe fn capture_rect(rect: ScreenRect) -> Result<CapturedScreen, CaptureError> {
    let x = rect.x;
    let y = rect.y;
    let width = i32::try_from(rect.width).map_err(|_| CaptureError::NoDisplay)?;
    let height = i32::try_from(rect.height).map_err(|_| CaptureError::NoDisplay)?;

    if width <= 0 || height <= 0 {
        return Err(CaptureError::NoDisplay);
    }

    let hdc_screen = unsafe { GetDC(None) };
    if hdc_screen.is_invalid() {
        return Err(CaptureError::PlatformError(
            "Failed to get screen DC".to_string(),
        ));
    }

    let hdc_mem = unsafe { CreateCompatibleDC(hdc_screen) };
    if hdc_mem.is_invalid() {
        unsafe { ReleaseDC(None, hdc_screen) };
        return Err(CaptureError::PlatformError(
            "Failed to create compatible DC".to_string(),
        ));
    }

    let hbm = unsafe { CreateCompatibleBitmap(hdc_screen, width, height) };
    if hbm.is_invalid() {
        unsafe {
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(None, hdc_screen);
        }
        return Err(CaptureError::PlatformError(
            "Failed to create bitmap".to_string(),
        ));
    }

    let old_bm = unsafe { SelectObject(hdc_mem, hbm) };

    let blt_result = unsafe { BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, x, y, SRCCOPY) };
    if blt_result.is_err() {
        unsafe {
            SelectObject(hdc_mem, old_bm);
            let _ = DeleteObject(hbm);
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(None, hdc_screen);
        }
        return Err(CaptureError::PlatformError("BitBlt failed".to_string()));
    }

    // Extract pixel data via GetDIBits
    // A bitmap must be deselected from the DC while GetDIBits reads it.
    unsafe {
        SelectObject(hdc_mem, old_bm);
    }
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [Default::default()],
    };

    let buf_size = (width as usize) * (height as usize) * 4;
    let mut bgra_pixels: Vec<u8> = vec![0u8; buf_size];

    let lines = unsafe {
        GetDIBits(
            hdc_mem,
            hbm,
            0,
            height as u32,
            Some(bgra_pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };

    // Query system DPI before releasing the screen DC (96 = 100% scaling)
    let dpi = unsafe { GetDeviceCaps(hdc_screen, LOGPIXELSX) };
    let scale_factor = if dpi > 0 { dpi as f64 / 96.0 } else { 1.0 };

    // Clean up GDI resources
    unsafe {
        SelectObject(hdc_mem, old_bm);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);
    }

    if lines != height {
        return Err(CaptureError::PlatformError(
            "GetDIBits returned an incomplete image".to_string(),
        ));
    }

    // Convert BGRA -> RGBA. GetDIBits leaves the fourth byte undefined for
    // BI_RGB captures, so force it opaque — downstream code (tiny-skia
    // premultiplied pixmaps, PNG export, clipboard) relies on alpha == 255.
    for chunk in bgra_pixels.chunks_exact_mut(4) {
        chunk.swap(0, 2); // swap B and R
        chunk[3] = 255;
    }

    Ok(CapturedScreen {
        pixels: bgra_pixels,
        width: width as u32,
        height: height as u32,
        x_offset: x,
        y_offset: y,
        scale_factor,
    })
}
