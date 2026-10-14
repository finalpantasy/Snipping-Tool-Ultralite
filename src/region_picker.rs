//! Windows selection UI over the same frozen pixels used for the final snip.
//! GDI paints the snapshot's own DIB section; no second full-screen buffer or
//! full-frame conversion exists.
use crate::capture::{BgraSnapshot, ScreenRect};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, ClientToScreen, CombineRgn, CreateRectRgn, DeleteObject, EndPaint, GetStockObject,
    SetWindowRgn, UpdateWindow, BLACK_BRUSH, HBRUSH, HRGN, PAINTSTRUCT, RGN_DIFF, WHITE_BRUSH,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::*;

struct FrozenSurface<'a> {
    screen: &'a BgraSnapshot,
    paint_failed: bool,
    veil: HWND,
}

/// When the last picker windows were hidden, as milliseconds since `EPOCH`.
static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);
static LAST_DISMISSED_MS: AtomicU64 = AtomicU64::new(u64::MAX);

/// True if picker windows were hidden less than `window` ago. Their removal
/// may not be on screen yet, so a new capture must wait for DWM first.
pub fn dismissed_within(window: Duration) -> bool {
    let last = LAST_DISMISSED_MS.load(Ordering::SeqCst);
    last != u64::MAX
        && (EPOCH.elapsed().as_millis() as u64).saturating_sub(last) < window.as_millis() as u64
}

/// The frozen-screen window. Every exit after it exists records the dismissal.
struct OwnedWindow(HWND);
impl Drop for OwnedWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
        LAST_DISMISSED_MS.store(EPOCH.elapsed().as_millis() as u64, Ordering::SeqCst);
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
        let failed = !empty && (dc.is_invalid() || (*pointer).screen.paint(dc).is_err());
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
            self.start.zip(end).and_then(|(start, end)| {
                let mut rect = ScreenRect::between(start, end);
                // The cursor cannot pass the last pixel, so reaching it means
                // "to the edge": include it rather than stop one short.
                let right = i64::from(self.bounds.x) + i64::from(self.bounds.width) - 1;
                let bottom = i64::from(self.bounds.y) + i64::from(self.bounds.height) - 1;
                if rect.width > 0 && rect.height > 0 {
                    if i64::from(start.0.max(end.0)) == right {
                        rect.width += 1;
                    }
                    if i64::from(start.1.max(end.1)) == bottom {
                        rect.height += 1;
                    }
                }
                rect.intersect(self.bounds)
            })
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

/// Decodes a packed mouse `LPARAM`; each half is a signed coordinate.
fn message_point(pos: u32) -> (i32, i32) {
    (
        (pos & 0xffff) as u16 as i16 as i32,
        (pos >> 16) as u16 as i16 as i32,
    )
}

/// Screen position carried by the mouse message being handled. A release
/// must commit where the button went up, not where a still-moving pointer is
/// by the time a busy UI thread reaches the message (`GetCursorPos`).
unsafe fn cursor(hwnd: HWND, lp: LPARAM) -> Option<(i32, i32)> {
    let (x, y) = message_point(lp.0 as u32);
    let mut p = POINT { x, y };
    ClientToScreen(hwnd, &mut p).ok().ok()?;
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
                (*pointer).start = cursor(hwnd, lp);
                SetCapture(hwnd);
                // Win32 calls can synchronously re-enter this procedure. Never
                // keep a borrowed mutable state reference across those calls.
                let snapshot = *pointer;
                if let Some(end) = snapshot.start {
                    update_selection(hwnd, &snapshot, end);
                }
                return LRESULT(0);
            }
            WM_MOUSEMOVE => {
                let snapshot = *pointer;
                if let Some(end) = cursor(hwnd, lp) {
                    update_selection(hwnd, &snapshot, end);
                }
                return LRESULT(0);
            }
            WM_LBUTTONUP => {
                let end = cursor(hwnd, lp);
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
/// selection windows were hidden and destroyed. It does not wait for DWM to
/// present their removal, so the clipboard is not held up by a display
/// frame; `dismissed_within` lets a following capture wait instead.
pub fn select_region(screen: &BgraSnapshot) -> Result<Option<ScreenRect>, String> {
    select(false, screen, &[])
}
pub fn select_window(
    screen: &BgraSnapshot,
    windows: &[crate::window_detect::WinRect],
) -> Result<Option<ScreenRect>, String> {
    select(true, screen, windows)
}
fn select(
    window_mode: bool,
    screen: &BgraSnapshot,
    windows: &[crate::window_detect::WinRect],
) -> Result<Option<ScreenRect>, String> {
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
        frozen_snapshot_crops_and_paints_exact_pixels_from_memory_dc();
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
            frozen_snapshot_crops_and_paints_exact_pixels_from_memory_dc();
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

    /// A memory DC with a top-down 32-bit DIB section selected.
    struct MemorySurface {
        dc: windows::Win32::Graphics::Gdi::HDC,
        bitmap: windows::Win32::Graphics::Gdi::HBITMAP,
        original: windows::Win32::Graphics::Gdi::HGDIOBJ,
        bits: *mut u8,
        len: usize,
    }
    impl MemorySurface {
        fn new(width: i32, height: i32) -> Self {
            use windows::Win32::Graphics::Gdi::{
                CreateCompatibleDC, CreateDIBSection, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
                BI_RGB, DIB_RGB_COLORS,
            };
            // No GetDC(screen), capture, HWND creation or display presentation.
            unsafe {
                let dc = CreateCompatibleDC(None);
                assert!(!dc.is_invalid());
                let info = BITMAPINFO {
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
                let mut bits = std::ptr::null_mut();
                let bitmap =
                    CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
                assert!(!bits.is_null());
                Self {
                    dc,
                    bitmap,
                    original: SelectObject(dc, bitmap),
                    bits: bits.cast(),
                    len: (width * height * 4) as usize,
                }
            }
        }
        fn bytes(&mut self) -> &mut [u8] {
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::GdiFlush();
                std::slice::from_raw_parts_mut(self.bits, self.len)
            }
        }
    }
    impl Drop for MemorySurface {
        fn drop(&mut self) {
            use windows::Win32::Graphics::Gdi::{DeleteDC, SelectObject};
            unsafe {
                SelectObject(self.dc, self.original);
                let _ = DeleteObject(self.bitmap);
                let _ = DeleteDC(self.dc);
            }
        }
    }

    /// A 4x3 BGRX "desktop" at a negative origin whose fourth byte is 0, as
    /// GDI may leave it. Pixel `i` holds BGR (i, 200, 100), so RGBA (100, 200, i, 255).
    fn synthetic_snapshot() -> BgraSnapshot {
        use windows::Win32::Graphics::Gdi::SetWindowOrgEx;
        let mut source = MemorySurface::new(4, 3);
        for (i, px) in source.bytes().chunks_exact_mut(4).enumerate() {
            px.copy_from_slice(&[i as u8, 200, 100, 0]);
        }
        unsafe {
            // Logical (-3, -2) is the source's first pixel, like a monitor left of
            // and above the primary in virtual-desktop coordinates.
            assert!(SetWindowOrgEx(source.dc, -3, -2, None).as_bool());
            BgraSnapshot::from_dc(
                source.dc,
                ScreenRect {
                    x: -3,
                    y: -2,
                    width: 4,
                    height: 3,
                },
                1.5,
            )
            .unwrap()
        }
    }

    fn rgba(indices: &[u8]) -> Vec<u8> {
        indices.iter().flat_map(|&i| [100, 200, i, 255]).collect()
    }

    #[test]
    fn frozen_snapshot_crops_and_paints_exact_pixels_from_memory_dc() {
        let snapshot = synthetic_snapshot();
        assert_eq!(
            snapshot.bounds(),
            ScreenRect {
                x: -3,
                y: -2,
                width: 4,
                height: 3
            }
        );
        // The snapshot stays in GDI's native order; nothing converted it.
        assert_eq!(&snapshot.bgra()[20..24], &[5, 200, 100, 0]);

        let snip = snapshot
            .region_rgba(ScreenRect {
                x: -2,
                y: -1,
                width: 2,
                height: 2,
            })
            .unwrap();
        assert_eq!(snip.pixels, rgba(&[5, 6, 9, 10]));
        assert_eq!(
            (snip.x_offset, snip.y_offset, snip.width, snip.height),
            (-2, -1, 2, 2)
        );
        assert_eq!(snip.scale_factor, 1.5);

        let mut target = MemorySurface::new(4, 3);
        unsafe { snapshot.paint(target.dc) }.unwrap();
        for (out, src) in target
            .bytes()
            .chunks_exact(4)
            .zip(snapshot.bgra().chunks_exact(4))
        {
            assert_eq!(&out[..3], &src[..3]);
        }
    }

    #[test]
    fn frozen_snapshot_crop_clips_edges_and_converts_only_the_selection() {
        let snapshot = synthetic_snapshot();
        let clipped = snapshot
            .region_rgba(ScreenRect {
                x: -5,
                y: -4,
                width: 4,
                height: 4,
            })
            .unwrap();
        assert_eq!(
            clipped.bounds(),
            ScreenRect {
                x: -3,
                y: -2,
                width: 2,
                height: 2
            }
        );
        assert_eq!(clipped.pixels, rgba(&[0, 1, 4, 5]));

        let full = snapshot.region_rgba(snapshot.bounds()).unwrap();
        assert_eq!(full.pixels, rgba(&(0..12).collect::<Vec<_>>()));

        let one = snapshot
            .region_rgba(ScreenRect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            })
            .unwrap();
        assert_eq!(one.pixels, rgba(&[11]));
        assert!(one.pixels.capacity() < 48, "kept desktop-sized capacity");
        // The source is untouched by any number of crops.
        assert_eq!(&snapshot.bgra()[44..48], &[11, 200, 100, 0]);

        for rect in [
            ScreenRect {
                x: 0,
                y: 0,
                width: 0,
                height: 1,
            },
            ScreenRect {
                x: 100,
                y: 0,
                width: 2,
                height: 2,
            },
        ] {
            assert!(snapshot.region_rgba(rect).is_err());
        }
    }

    #[test]
    fn frozen_snapshot_rejects_empty_and_oversized_before_allocating() {
        let source = MemorySurface::new(1, 1);
        for (width, height) in [(0, 10), (10, 0), (40_000, 40_000), (u32::MAX, 1)] {
            let rect = ScreenRect {
                x: 0,
                y: 0,
                width,
                height,
            };
            assert!(unsafe { BgraSnapshot::from_dc(source.dc, rect, 1.0) }.is_err());
        }
    }

    #[test]
    fn dismissal_is_recorded_when_the_frozen_window_is_released() {
        // A null HWND: DestroyWindow fails harmlessly, no window is involved.
        drop(OwnedWindow(HWND::default()));
        assert!(dismissed_within(Duration::from_secs(60)));
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

    fn packed(x: i32, y: i32) -> u32 {
        ((y as i16 as u16 as u32) << 16) | (x as i16 as u16 as u32)
    }

    #[test]
    fn message_position_decodes_signed_virtual_screen_coordinates() {
        for (x, y) in [
            (0, 0),
            (1880, 1120),
            (-3840, 0),
            (-1, -1),
            (7679, 2159),
            (-3840, -2160),
        ] {
            assert_eq!(super::message_point(packed(x, y)), (x, y));
        }
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
    fn drag_to_the_last_screen_pixel_reaches_the_edge() {
        // The cursor stops on the last pixel, so an exclusive end could
        // never include the rightmost column or bottom row.
        let mut picker = area_picker();
        unsafe {
            assert_eq!(
                rect_tuple(picker.selection_at(Some((199, 149)))),
                Some((20, 30, 180, 120))
            );
            picker.start = Some((199, 149));
            assert_eq!(
                rect_tuple(picker.selection_at(Some((20, 30)))),
                Some((20, 30, 180, 120))
            );
            // Edge to edge selects the whole desktop.
            picker.start = Some((-100, -50));
            assert_eq!(
                rect_tuple(picker.selection_at(Some((199, 149)))),
                Some((-100, -50, 300, 200))
            );
            // A click on the edge is still a click, not a selection.
            picker.start = Some((199, 149));
            assert!(picker.selection_at(Some((199, 149))).is_none());
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
