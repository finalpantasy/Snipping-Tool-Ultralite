use super::{CaptureError, CapturedScreen, ScreenCapture, ScreenRect};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject,
    GdiFlush, GetDIBits, GetDeviceCaps, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    CAPTUREBLT, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, LOGPIXELSX, SRCCOPY,
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

/// Freeze the whole virtual desktop for the selection UI. The screen is copied
/// once, straight into a DIB section in GDI's own layout; nothing converts or
/// copies the full frame again before it is painted.
pub fn capture_frozen_desktop() -> Result<BgraSnapshot, CaptureError> {
    let rect = virtual_bounds();
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return Err(CaptureError::PlatformError(
                "Failed to get screen DC".to_string(),
            ));
        }
        // 96 DPI is 100% scaling.
        let dpi = GetDeviceCaps(screen, LOGPIXELSX);
        let scale_factor = if dpi > 0 { dpi as f64 / 96.0 } else { 1.0 };
        let snapshot = BgraSnapshot::from_dc(screen, rect, scale_factor);
        ReleaseDC(None, screen);
        snapshot
    }
}

/// A frozen desktop held in a top-down 32-bit DIB section: native BGRA with an
/// undefined fourth byte. Painting it is a plain BitBlt, and only a selected
/// rectangle is ever converted to the RGBA `CapturedScreen` contract.
///
/// Owns GDI objects bound to the creating thread's use; it is neither `Send`
/// nor `Sync`.
pub struct BgraSnapshot {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *const u8,
    bounds: ScreenRect,
    scale_factor: f64,
}

impl BgraSnapshot {
    /// Copy `rect` of `source` (a screen or memory DC, in its own coordinates)
    /// into a new snapshot whose bounds are `rect`.
    ///
    /// # Safety
    /// `source` must be a valid device context for the duration of the call.
    pub unsafe fn from_dc(
        source: HDC,
        rect: ScreenRect,
        scale_factor: f64,
    ) -> Result<Self, CaptureError> {
        let width = i32::try_from(rect.width).map_err(|_| CaptureError::NoDisplay)?;
        let height = i32::try_from(rect.height).map_err(|_| CaptureError::NoDisplay)?;
        if width <= 0 || height <= 0 {
            return Err(CaptureError::NoDisplay);
        }
        (rect.width as usize)
            .checked_mul(rect.height as usize)
            .and_then(|n| n.checked_mul(4))
            .filter(|&n| n <= i32::MAX as usize)
            .ok_or_else(|| CaptureError::PlatformError("Screen image is too large".into()))?;
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return Err(CaptureError::PlatformError(
                "Failed to create compatible DC".to_string(),
            ));
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(bitmap) if !bits.is_null() => bitmap,
            Ok(bitmap) => {
                let _ = DeleteObject(bitmap);
                let _ = DeleteDC(dc);
                return Err(CaptureError::PlatformError(
                    "Not enough memory for screen snapshot".into(),
                ));
            }
            Err(_) => {
                let _ = DeleteDC(dc);
                return Err(CaptureError::PlatformError(
                    "Not enough memory for screen snapshot".into(),
                ));
            }
        };
        let snapshot = Self {
            dc,
            bitmap,
            previous: SelectObject(dc, bitmap),
            bits: bits.cast(),
            bounds: rect,
            scale_factor,
        };
        // Include existing layered popups/tooltips in the frozen screen image.
        BitBlt(
            dc,
            0,
            0,
            width,
            height,
            source,
            rect.x,
            rect.y,
            SRCCOPY | CAPTUREBLT,
        )
        .map_err(|_| CaptureError::PlatformError("BitBlt failed".to_string()))?;
        // The CPU reads the section directly; finish any batched GDI work.
        let _ = GdiFlush();
        Ok(snapshot)
    }

    /// Physical virtual-desktop bounds of the snapshot.
    pub fn bounds(&self) -> ScreenRect {
        self.bounds
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Native BGRX rows, top-down, `width * 4` bytes each.
    pub fn bgra(&self) -> &[u8] {
        let len = self.bounds.width as usize * self.bounds.height as usize * 4;
        // SAFETY: the section holds exactly `len` bytes, lives as long as
        // `self`, and GDI only writes to it inside `from_dc`.
        unsafe { std::slice::from_raw_parts(self.bits, len) }
    }

    /// Draw the snapshot at the DC origin. GDI clips the copy to the DC's
    /// clip region, so a partial repaint only copies what was exposed.
    ///
    /// # Safety
    /// `target` must be a valid device context.
    pub unsafe fn paint(&self, target: HDC) -> Result<(), String> {
        BitBlt(
            target,
            0,
            0,
            self.bounds.width as i32,
            self.bounds.height as i32,
            self.dc,
            0,
            0,
            SRCCOPY,
        )
        .map_err(|_| "Could not display the frozen screen".to_string())
    }

    /// Copy the selected physical pixels as straight, opaque RGBA. No new
    /// desktop read occurs here and the full snapshot is never converted.
    pub fn region_rgba(&self, region: ScreenRect) -> Result<CapturedScreen, CaptureError> {
        let region = region
            .intersect(self.bounds)
            .ok_or(CaptureError::NoDisplay)?;
        let stride = self.bounds.width as usize * 4;
        let row_bytes = region.width as usize * 4;
        let len = row_bytes
            .checked_mul(region.height as usize)
            .ok_or_else(|| CaptureError::PlatformError("Invalid snapshot pixels".into()))?;
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(len).map_err(|_| {
            CaptureError::PlatformError("Not enough memory for selected image".into())
        })?;
        let x = (i64::from(region.x) - i64::from(self.bounds.x)) as usize;
        let y = (i64::from(region.y) - i64::from(self.bounds.y)) as usize;
        pixels.resize(len, 0);
        let source = self.bgra();
        for (out, row) in pixels.chunks_exact_mut(row_bytes).zip(y..) {
            let offset = row * stride + x * 4;
            // GDI leaves the fourth byte undefined; RGBA consumers need 255.
            for (o, p) in out
                .chunks_exact_mut(4)
                .zip(source[offset..offset + row_bytes].chunks_exact(4))
            {
                o.copy_from_slice(&[p[2], p[1], p[0], 255]);
            }
        }
        Ok(CapturedScreen {
            pixels,
            width: region.width,
            height: region.height,
            x_offset: region.x,
            y_offset: region.y,
            scale_factor: self.scale_factor,
        })
    }
}

impl Drop for BgraSnapshot {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            let _ = DeleteObject(self.bitmap);
            let _ = DeleteDC(self.dc);
        }
    }
}

unsafe fn capture_rect(rect: ScreenRect) -> Result<CapturedScreen, CaptureError> {
    let x = rect.x;
    let y = rect.y;
    let width = i32::try_from(rect.width).map_err(|_| CaptureError::NoDisplay)?;
    let height = i32::try_from(rect.height).map_err(|_| CaptureError::NoDisplay)?;

    if width <= 0 || height <= 0 {
        return Err(CaptureError::NoDisplay);
    }

    // Reserve before taking GDI resources so allocation failure cannot leak
    // handles. Frozen selection temporarily needs the whole virtual desktop.
    let buf_size = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| CaptureError::PlatformError("Screen image is too large".into()))?;
    let mut bgra_pixels = Vec::new();
    bgra_pixels
        .try_reserve_exact(buf_size)
        .map_err(|_| CaptureError::PlatformError("Not enough memory for screen snapshot".into()))?;
    bgra_pixels.resize(buf_size, 0u8);

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

    // Include existing layered popups/tooltips in the frozen screen image.
    let blt_result = unsafe {
        BitBlt(
            hdc_mem,
            0,
            0,
            width,
            height,
            hdc_screen,
            x,
            y,
            SRCCOPY | CAPTUREBLT,
        )
    };
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
