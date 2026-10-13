//! Windows selection UI over the same frozen pixels used for the final snip.
//! GDI borrows the snapshot directly; no second full-screen CPU buffer is needed.
use crate::capture::{CapturedScreen, ScreenRect};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CombineRgn, CreateRectRgn, DeleteObject, EndPaint, GetStockObject, SetWindowRgn,
    StretchDIBits, UpdateWindow, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLACK_BRUSH, DIB_RGB_COLORS,
    HBRUSH, HDC, HRGN, PAINTSTRUCT, RGN_DIFF, SRCCOPY, WHITE_BRUSH,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::*;

struct FrozenSurface<'a> {
    screen: &'a CapturedScreen,
    paint_failed: bool,
    veil: HWND,
}

/// Borrowed exclusively during selection. Use GDI's native BGRA layout without
/// another desktop buffer, then restore the public RGBA contract on every exit.
struct NativeSnapshot<'a> {
    screen: &'a mut CapturedScreen,
}
impl<'a> NativeSnapshot<'a> {
    fn new(screen: &'a mut CapturedScreen) -> Self {
        swap_red_blue(&mut screen.pixels);
        Self { screen }
    }
}
impl Drop for NativeSnapshot<'_> {
    fn drop(&mut self) {
        swap_red_blue(&mut self.screen.pixels);
    }
}
fn swap_red_blue(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
}

struct OwnedWindow(HWND);
impl Drop for OwnedWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

/// Paint the guarded native BGRA snapshot without GDI's costly bitfield conversion.
unsafe fn paint_snapshot(dc: HDC, screen: &CapturedScreen) -> Result<(), String> {
    let width = i32::try_from(screen.width).map_err(|_| "Snapshot is too wide")?;
    let height = i32::try_from(screen.height).map_err(|_| "Snapshot is too tall")?;
    let expected = (screen.width as usize)
        .checked_mul(screen.height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or("Invalid snapshot size")?;
    if width <= 0 || height <= 0 || expected != screen.pixels.len() {
        return Err("Invalid snapshot pixels".into());
    }
    let header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let lines = StretchDIBits(
        dc,
        0,
        0,
        width,
        height,
        0,
        0,
        width,
        height,
        Some(screen.pixels.as_ptr().cast()),
        &header,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
    // Repaints may be clipped to an exposed subset of the desktop.
    if lines <= 0 {
        Err("Could not display the frozen screen".into())
    } else {
        Ok(())
    }
}

unsafe extern "system" fn frozen_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    if msg == WM_ERASEBKGND {
        return LRESULT(1);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut FrozenSurface<'_>;
    if msg == WM_PAINT && !pointer.is_null() {
        let mut paint = PAINTSTRUCT::default();
        let dc = BeginPaint(hwnd, &mut paint);
        let empty =
            paint.rcPaint.right <= paint.rcPaint.left || paint.rcPaint.bottom <= paint.rcPaint.top;
        let failed = !empty && (dc.is_invalid() || paint_snapshot(dc, (*pointer).screen).is_err());
        let _ = EndPaint(hwnd, &paint);
        if failed {
            (*pointer).paint_failed = true;
            if !(*pointer).veil.is_invalid() {
                let _ = PostMessageW((*pointer).veil, WM_CANCELMODE, WPARAM(0), LPARAM(0));
            }
        }
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

#[derive(Clone, Copy)]
struct Picker {
    bounds: ScreenRect,
    start: Option<(i32, i32)>,
    selected: Option<ScreenRect>,
    done: bool,
    border: HWND,
    windows: *const crate::window_detect::WinRect,
    windows_len: usize,
    window_mode: bool,
}

impl Picker {
    /// Pure selection calculation shared by the displayed region and commit.
    unsafe fn selection_at(&self, end: Option<(i32, i32)>) -> Option<ScreenRect> {
        if self.window_mode {
            if self.windows.is_null() {
                None
            } else {
                end.and_then(|(x, y)| {
                    crate::window_detect::window_at_point(
                        std::slice::from_raw_parts(self.windows, self.windows_len),
                        x,
                        y,
                    )
                })
                .and_then(|(x, y, w, h)| {
                    ScreenRect {
                        x,
                        y,
                        width: w as u32,
                        height: h as u32,
                    }
                    .intersect(self.bounds)
                })
            }
        } else {
            self.start
                .zip(end)
                .and_then(|(start, end)| ScreenRect::between(start, end).intersect(self.bounds))
        }
    }

    /// A stray release from the initiating UI click must not finish the picker.
    unsafe fn finish_click(&mut self, end: Option<(i32, i32)>) -> bool {
        if self.done || self.start.is_none() {
            return false;
        }
        self.selected = self.selection_at(end);
        self.done = true;
        true
    }
}

fn cursor() -> Option<(i32, i32)> {
    let mut p = POINT::default();
    unsafe {
        GetCursorPos(&mut p).ok()?;
    }
    Some((p.x, p.y))
}

unsafe fn update_selection(hwnd: HWND, state: &Picker, end: (i32, i32)) {
    let Some(rect) = state.selection_at(Some(end)) else {
        let _ = ShowWindow(state.border, SW_HIDE);
        if !state.window_mode {
            // Remove the old cutout when a drag collapses or moves offscreen.
            // A null region restores the full rectangular dim veil.
            let _ = SetWindowRgn(hwnd, HRGN::default(), true);
        }
        return;
    };
    // Cut the selected area out of the dim veil. Region ownership transfers to
    // Windows only after a successful SetWindowRgn; temporary regions are freed.
    if !state.window_mode {
        let veil = CreateRectRgn(0, 0, state.bounds.width as i32, state.bounds.height as i32);
        let hole = CreateRectRgn(
            rect.x - state.bounds.x,
            rect.y - state.bounds.y,
            rect.x - state.bounds.x + rect.width as i32,
            rect.y - state.bounds.y + rect.height as i32,
        );
        CombineRgn(veil, veil, hole, RGN_DIFF);
        let _ = DeleteObject(hole);
        if SetWindowRgn(hwnd, veil, true) == 0 {
            let _ = DeleteObject(veil);
        }
    }

    let width = rect.width as i32 + 4;
    let height = rect.height as i32 + 4;
    let frame = CreateRectRgn(0, 0, width, height);
    let inside = CreateRectRgn(2, 2, width - 2, height - 2);
    CombineRgn(frame, frame, inside, RGN_DIFF);
    let _ = DeleteObject(inside);
    if SetWindowRgn(state.border, frame, true) == 0 {
        let _ = DeleteObject(frame);
    }
    let _ = SetWindowPos(
        state.border,
        HWND_TOPMOST,
        rect.x - 2,
        rect.y - 2,
        width,
        height,
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    );
}

unsafe extern "system" fn picker_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Picker;
    if !pointer.is_null() {
        match msg {
            WM_LBUTTONDOWN => {
                (*pointer).start = cursor();
                SetCapture(hwnd);
                // Win32 calls can synchronously re-enter this procedure. Never
                // keep a borrowed mutable state reference across those calls.
                let snapshot = *pointer;
                if let Some(end) = cursor() {
                    update_selection(hwnd, &snapshot, end);
                }
                return LRESULT(0);
            }
            WM_MOUSEMOVE => {
                let snapshot = *pointer;
                if let Some(end) = cursor() {
                    update_selection(hwnd, &snapshot, end);
                }
                return LRESULT(0);
            }
            WM_LBUTTONUP => {
                let end = cursor();
                if !(*pointer).finish_click(end) {
                    return LRESULT(0);
                }
                let _ = ReleaseCapture();
                return LRESULT(0);
            }
            WM_KEYDOWN if wp.0 == 0x1b => {
                (*pointer).done = true;
                return LRESULT(0);
            }
            WM_RBUTTONDOWN | WM_CLOSE | WM_CANCELMODE | WM_DISPLAYCHANGE => {
                (*pointer).done = true;
                return LRESULT(0);
            }
            WM_ACTIVATE if wp.0 & 0xffff == WA_INACTIVE as usize => {
                (*pointer).done = true;
                return LRESULT(0);
            }
            WM_CAPTURECHANGED if (*pointer).start.is_some() && !(*pointer).done => {
                (*pointer).done = true;
                return LRESULT(0);
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

unsafe extern "system" fn border_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wp, lp)
}

/// Owns all HWNDs/region state on the calling worker thread. Returning means
/// selection windows were destroyed and DWM presented their removal.
pub fn select_region(screen: &mut CapturedScreen) -> Result<Option<ScreenRect>, String> {
    select(false, screen, &[])
}
pub fn select_window(
    screen: &mut CapturedScreen,
    windows: &[crate::window_detect::WinRect],
) -> Result<Option<ScreenRect>, String> {
    select(true, screen, windows)
}
fn select(
    window_mode: bool,
    screen: &mut CapturedScreen,
    windows: &[crate::window_detect::WinRect],
) -> Result<Option<ScreenRect>, String> {
    let native = NativeSnapshot::new(screen);
    let screen = &*native.screen;
    unsafe {
        let module = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let instance = HINSTANCE(module.0);
        let bounds = screen.bounds();
        if bounds.width == 0 || bounds.height == 0 {
            return Err("No display available".into());
        }
        let width = i32::try_from(bounds.width).map_err(|_| "Snapshot is too wide")?;
        let height = i32::try_from(bounds.height).map_err(|_| "Snapshot is too tall")?;
        let class = WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("SnippingToolUL.Selection"),
            lpfnWndProc: Some(picker_proc),
            hCursor: LoadCursorW(None, IDC_CROSS).unwrap_or_default(),
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            ..Default::default()
        };
        let border_class = WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("SnippingToolUL.Border"),
            lpfnWndProc: Some(border_proc),
            hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
            ..Default::default()
        };
        let frozen_class = WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("SnippingToolUL.FrozenScreen"),
            lpfnWndProc: Some(frozen_proc),
            ..Default::default()
        };
        // Classes are process-wide and reused for subsequent captures.
        if RegisterClassW(&class) == 0
            && windows::Win32::Foundation::GetLastError()
                != windows::Win32::Foundation::ERROR_CLASS_ALREADY_EXISTS
        {
            return Err(windows::core::Error::from_win32().to_string());
        }
        if RegisterClassW(&border_class) == 0
            && windows::Win32::Foundation::GetLastError()
                != windows::Win32::Foundation::ERROR_CLASS_ALREADY_EXISTS
        {
            return Err(windows::core::Error::from_win32().to_string());
        }
        if RegisterClassW(&frozen_class) == 0
            && windows::Win32::Foundation::GetLastError()
                != windows::Win32::Foundation::ERROR_CLASS_ALREADY_EXISTS
        {
            return Err(windows::core::Error::from_win32().to_string());
        }
        // Surface outlives its HWND. The borrowed pixels remain immutable until
        // all callbacks have ended and the window has been destroyed.
        let mut surface = FrozenSurface {
            screen,
            paint_failed: false,
            veil: HWND::default(),
        };
        let frozen = OwnedWindow(
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                frozen_class.lpszClassName,
                w!("Snipping Tool UL frozen screen"),
                WS_POPUP,
                bounds.x,
                bounds.y,
                width,
                height,
                None,
                None,
                instance,
                Some(&mut surface as *mut FrozenSurface<'_> as *const _),
            )
            .map_err(|e| e.to_string())?,
        );
        let border = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
            border_class.lpszClassName,
            w!("Snipping Tool UL selection border"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            instance,
            None,
        )
        .map_err(|e| e.to_string())?;
        let mut state = Picker {
            bounds,
            start: None,
            selected: None,
            done: false,
            border,
            windows: windows.as_ptr(),
            windows_len: windows.len(),
            window_mode,
        };
        let veil = match CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_APPWINDOW | WS_EX_LAYERED,
            class.lpszClassName,
            w!("Snipping Tool UL selection — Esc to cancel"),
            WS_POPUP,
            bounds.x,
            bounds.y,
            width,
            height,
            None,
            None,
            instance,
            Some(&mut state as *mut Picker as *const _),
        ) {
            Ok(hwnd) => hwnd,
            Err(e) => {
                let _ = DestroyWindow(border);
                return Err(e.to_string());
            }
        };
        if let Err(e) = SetLayeredWindowAttributes(veil, COLORREF(0), 70, LWA_ALPHA) {
            let _ = DestroyWindow(veil);
            let _ = DestroyWindow(border);
            return Err(e.to_string());
        }
        surface.veil = veil;
        let _ = ShowWindow(frozen.0, SW_SHOWNOACTIVATE);
        let _ = UpdateWindow(frozen.0);
        if surface.paint_failed {
            let _ = DestroyWindow(veil);
            let _ = DestroyWindow(border);
            return Err("Could not display the frozen screen".into());
        }
        let _ = ShowWindow(veil, SW_SHOW);
        let _ = SetForegroundWindow(veil);
        let mut message = MSG::default();
        let mut failed = false;
        loop {
            // The window callback updates this field through the registered state pointer.
            if state.done {
                break;
            }
            let code = GetMessageW(&mut message, None, 0, 0).0;
            if code <= 0 {
                failed = code < 0;
                break;
            }
            // GetMessageW may dispatch a sent cancellation before returning
            // an already queued mouse release. Do not commit that stale event.
            if state.done {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        let _ = ShowWindow(border, SW_HIDE);
        let _ = ShowWindow(veil, SW_HIDE);
        let _ = ShowWindow(frozen.0, SW_HIDE);
        let _ = DestroyWindow(border);
        let _ = DestroyWindow(veil);
        drop(frozen);
        let _ = DwmFlush();
        if surface.paint_failed {
            Err("Could not display the frozen screen".into())
        } else if failed {
            Err("Selection message loop failed".into())
        } else {
            Ok(state.selected)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "manual headless GDI resource soak"]
    fn repeated_memory_presentations_release_gdi_resources() {
        use windows::Win32::System::Threading::{
            GetCurrentProcess, GetGuiResources, GetProcessHandleCount, GR_GDIOBJECTS,
        };
        frozen_surface_draws_exact_rgba_into_memory_dc();
        let process = unsafe { GetCurrentProcess() };
        let before = unsafe { GetGuiResources(process, GR_GDIOBJECTS) };
        // Verify the counter sees a live allocation before trusting its baseline.
        let probe = unsafe { windows::Win32::Graphics::Gdi::CreateCompatibleDC(None) };
        assert!(!probe.is_invalid());
        let live = unsafe { GetGuiResources(process, GR_GDIOBJECTS) };
        assert!(unsafe { windows::Win32::Graphics::Gdi::DeleteDC(probe) }.as_bool());
        assert!(live > before, "GDI counter did not observe the probe DC");
        let mut handles_before = 0;
        unsafe {
            GetProcessHandleCount(process, &mut handles_before).unwrap();
        }
        for _ in 0..256 {
            frozen_surface_draws_exact_rgba_into_memory_dc();
        }
        let after = unsafe { GetGuiResources(process, GR_GDIOBJECTS) };
        let mut handles_after = 0;
        unsafe {
            GetProcessHandleCount(process, &mut handles_after).unwrap();
        }
        println!("memory_presentations=256 gdi_before={before} gdi_after={after} handles_before={handles_before} handles_after={handles_after}");
        assert_eq!(after, before);
        assert_eq!(handles_after, handles_before);
    }

    #[test]
    fn frozen_surface_draws_exact_rgba_into_memory_dc() {
        use windows::Win32::Graphics::Gdi::{
            CreateCompatibleDC, CreateDIBSection, DeleteDC, GdiFlush, SelectObject,
            BITMAPINFOHEADER, BI_RGB, HBITMAP, HGDIOBJ,
        };
        struct MemorySurface {
            dc: HDC,
            bitmap: HBITMAP,
            original: HGDIOBJ,
        }
        impl Drop for MemorySurface {
            fn drop(&mut self) {
                unsafe {
                    if !self.bitmap.is_invalid() {
                        SelectObject(self.dc, self.original);
                        let _ = DeleteObject(self.bitmap);
                    }
                    let _ = DeleteDC(self.dc);
                }
            }
        }
        // No GetDC(screen), capture, HWND creation or display presentation.
        unsafe {
            let dc = CreateCompatibleDC(HDC::default());
            assert!(!dc.is_invalid());
            let mut target = MemorySurface {
                dc,
                bitmap: HBITMAP::default(),
                original: HGDIOBJ::default(),
            };
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: 2,
                    biHeight: -2,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            target.bitmap =
                CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
            target.original = SelectObject(dc, target.bitmap);
            assert!(!bits.is_null());
            let mut screen = CapturedScreen {
                pixels: vec![
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 17, 99, 201, 255,
                ],
                width: 2,
                height: 2,
                x_offset: -2,
                y_offset: -1,
                scale_factor: 1.5,
            };
            let expected = screen.pixels.clone();
            let native = NativeSnapshot::new(&mut screen);
            paint_snapshot(dc, native.screen).unwrap();
            let _ = GdiFlush();
            let output = std::slice::from_raw_parts(bits.cast::<u8>(), 16);
            for (rgba, bgra) in expected.chunks_exact(4).zip(output.chunks_exact(4)) {
                assert_eq!(&bgra[..3], &[rgba[2], rgba[1], rgba[0]]);
            }
            drop(native);
            assert_eq!(screen.pixels, expected);
        }
    }

    #[test]
    fn native_snapshot_restores_rgba_on_early_return_and_unwind() {
        let mut screen = CapturedScreen {
            pixels: vec![17, 99, 201, 255, 4, 5, 6, 255],
            width: 2,
            height: 1,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.0,
        };
        let original = screen.pixels.clone();
        let pointer = screen.pixels.as_ptr();
        let operation = |screen: &mut CapturedScreen| -> Result<(), ()> {
            let native = NativeSnapshot::new(screen);
            assert_eq!(&native.screen.pixels[..4], &[201, 99, 17, 255]);
            Err(())
        };
        assert!(operation(&mut screen).is_err());
        assert_eq!(screen.pixels, original);
        assert_eq!(screen.pixels.as_ptr(), pointer);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _native = NativeSnapshot::new(&mut screen);
            panic!("synthetic picker failure");
        }))
        .is_err());
        assert_eq!(screen.pixels, original);
        assert_eq!(screen.pixels.as_ptr(), pointer);
    }

    /// Explicitly invoked CPU benchmark. No HWND, screen read or clipboard.
    #[test]
    #[ignore = "manual synthetic frozen capture benchmark"]
    fn benchmark_frozen_capture_costs() {
        use std::hint::black_box;
        use std::time::Instant;
        use windows::Win32::Graphics::Gdi::{
            CreateCompatibleDC, CreateDIBSection, DeleteDC, GdiFlush, SelectObject,
            BITMAPINFOHEADER, BI_RGB, HBITMAP, HGDIOBJ,
        };
        struct MemorySurface {
            dc: HDC,
            bitmap: HBITMAP,
            original: HGDIOBJ,
        }
        impl Drop for MemorySurface {
            fn drop(&mut self) {
                unsafe {
                    if !self.bitmap.is_invalid() {
                        SelectObject(self.dc, self.original);
                        let _ = DeleteObject(self.bitmap);
                    }
                    let _ = DeleteDC(self.dc);
                }
            }
        }
        fn sample(mut operation: impl FnMut() -> f64) -> (f64, f64) {
            for _ in 0..3 {
                black_box(operation());
            }
            let mut samples: Vec<_> = (0..31).map(|_| operation()).collect();
            samples.sort_by(f64::total_cmp);
            (samples[15], samples[29])
        }
        for (width, height) in [(1920u32, 1080u32), (3840, 2160), (7680, 2160)] {
            let mut pixels = vec![0; width as usize * height as usize * 4];
            for (i, px) in pixels.chunks_exact_mut(4).enumerate() {
                px.copy_from_slice(&[(i % 251) as u8, (i / width as usize % 241) as u8, 173, 255]);
            }
            let mut screen = CapturedScreen {
                pixels,
                width,
                height,
                x_offset: -1920,
                y_offset: 0,
                scale_factor: 1.0,
            };
            unsafe {
                let dc = CreateCompatibleDC(HDC::default());
                assert!(!dc.is_invalid());
                let mut target = MemorySurface {
                    dc,
                    bitmap: HBITMAP::default(),
                    original: HGDIOBJ::default(),
                };
                let info = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: width as i32,
                        biHeight: -(height as i32),
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let mut bits = std::ptr::null_mut();
                target.bitmap =
                    CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
                target.original = SelectObject(dc, target.bitmap);
                let native = NativeSnapshot::new(&mut screen);
                let draw = sample(|| {
                    let start = Instant::now();
                    paint_snapshot(dc, native.screen).unwrap();
                    let _ = GdiFlush();
                    start.elapsed().as_secs_f64() * 1000.0
                });
                println!(
                    "frozen_draw {width}x{height} source_bytes={} median_ms={:.6} p95_ms={:.6}",
                    native.screen.pixels.capacity(),
                    draw.0,
                    draw.1
                );
            }
            let conversion = sample(|| {
                let mut snapshot = screen.clone();
                let start = Instant::now();
                let native = NativeSnapshot::new(&mut snapshot);
                black_box(&native.screen.pixels);
                drop(native);
                start.elapsed().as_secs_f64() * 1000.0
            });
            println!(
                "frozen_convert_pair {width}x{height} median_ms={:.6} p95_ms={:.6}",
                conversion.0, conversion.1
            );
            for (crop_width, crop_height) in [(200, 150), (800, 600), (width, height)] {
                let region = ScreenRect {
                    x: screen.x_offset,
                    y: 0,
                    width: crop_width,
                    height: crop_height,
                };
                let crop = sample(|| {
                    // Synthetic input setup is outside the timed owned-crop call.
                    let snapshot = screen.clone();
                    let start = Instant::now();
                    let selected = snapshot.into_region(region).unwrap();
                    black_box(&selected.pixels);
                    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                    drop(selected);
                    elapsed
                });
                println!("frozen_crop {width}x{height} -> {crop_width}x{crop_height} median_ms={:.6} p95_ms={:.6}", crop.0, crop.1);
            }
        }
    }

    fn area_picker() -> Picker {
        Picker {
            bounds: ScreenRect {
                x: -100,
                y: -50,
                width: 300,
                height: 200,
            },
            start: Some((20, 30)),
            selected: None,
            done: false,
            border: HWND::default(),
            windows: std::ptr::null(),
            windows_len: 0,
            window_mode: false,
        }
    }

    fn rect_tuple(rect: Option<ScreenRect>) -> Option<(i32, i32, u32, u32)> {
        rect.map(|r| (r.x, r.y, r.width, r.height))
    }

    #[test]
    fn collapsed_drag_clears_selection_then_can_expand_again() {
        let picker = area_picker();
        unsafe {
            assert_eq!(
                rect_tuple(picker.selection_at(Some((90, 80)))),
                Some((20, 30, 70, 50))
            );
            for end in [(20, 30), (20, 80), (90, 30)] {
                assert!(picker.selection_at(Some(end)).is_none());
            }
            assert_eq!(
                rect_tuple(picker.selection_at(Some((0, 10)))),
                Some((0, 10, 20, 20))
            );
        }
    }

    #[test]
    fn reverse_drag_clips_to_negative_screen_bounds_and_matches_commit() {
        let mut picker = area_picker();
        let end = Some((-200, -100));
        let preview = unsafe { picker.selection_at(end) };
        assert_eq!(rect_tuple(preview), Some((-100, -50, 120, 80)));
        assert!(unsafe { picker.finish_click(end) });
        assert_eq!(rect_tuple(picker.selected), rect_tuple(preview));
    }

    #[test]
    fn window_hit_miss_hit_never_keeps_stale_rectangle() {
        let rects = [(-120, -80, 100, 100), (30, 40, 50, 60)];
        let mut picker = area_picker();
        picker.window_mode = true;
        picker.windows = rects.as_ptr();
        picker.windows_len = rects.len();
        unsafe {
            assert_eq!(
                rect_tuple(picker.selection_at(Some((-30, -10)))),
                Some((-100, -50, 80, 70))
            );
            assert!(picker.selection_at(Some((10, 10))).is_none());
            assert_eq!(
                rect_tuple(picker.selection_at(Some((40, 50)))),
                Some((30, 40, 50, 60))
            );
        }
    }

    #[test]
    fn missing_cursor_or_window_list_has_no_selection() {
        let mut picker = area_picker();
        assert!(unsafe { picker.selection_at(None) }.is_none());
        picker.window_mode = true;
        assert!(unsafe { picker.selection_at(Some((40, 50))) }.is_none());
    }

    #[test]
    fn queued_release_cannot_commit_after_cancellation() {
        let mut picker = area_picker();
        picker.done = true;
        assert!(!unsafe { picker.finish_click(Some((90, 80))) });
        assert!(picker.selected.is_none());
        assert!(picker.done);
    }

    #[test]
    fn initiating_release_does_not_cancel_then_real_drag_finishes() {
        let mut picker = Picker {
            bounds: ScreenRect {
                x: 0,
                y: 0,
                width: 300,
                height: 200,
            },
            start: None,
            selected: None,
            done: false,
            border: HWND::default(),
            windows: std::ptr::null(),
            windows_len: 0,
            window_mode: false,
        };
        assert!(!unsafe { picker.finish_click(Some((40, 20))) });
        assert!(!picker.done);
        picker.start = Some((20, 30));
        assert!(unsafe { picker.finish_click(Some((90, 80))) });
        assert!(picker.done);
        let r = picker.selected.unwrap();
        assert_eq!((r.x, r.y, r.width, r.height), (20, 30, 70, 50));
    }
}
