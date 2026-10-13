//! Windows selection UI. GDI/DWM draw a dim veil and a hollow border; no desktop
//! screenshot or CPU framebuffer is allocated until the user finishes selecting.
use crate::capture::{virtual_bounds, ScreenRect};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Gdi::{
    CombineRgn, CreateRectRgn, DeleteObject, GetStockObject, SetWindowRgn, BLACK_BRUSH, HBRUSH,
    RGN_DIFF, WHITE_BRUSH,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::*;

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

fn cursor() -> Option<(i32, i32)> {
    let mut p = POINT::default();
    unsafe {
        GetCursorPos(&mut p).ok()?;
    }
    Some((p.x, p.y))
}

unsafe fn update_selection(hwnd: HWND, state: &Picker, end: (i32, i32)) {
    let rect = if state.window_mode {
        // The caller keeps this immutable vector alive until the picker is destroyed.
        let windows = std::slice::from_raw_parts(state.windows, state.windows_len);
        match crate::window_detect::window_at_point(windows, end.0, end.1).and_then(
            |(x, y, w, h)| {
                ScreenRect {
                    x,
                    y,
                    width: w as u32,
                    height: h as u32,
                }
                .intersect(state.bounds)
            },
        ) {
            Some(rect) => rect,
            None => {
                let _ = ShowWindow(state.border, SW_HIDE);
                return;
            }
        }
    } else {
        let Some(start) = state.start else { return };
        let Some(rect) = ScreenRect::between(start, end).intersect(state.bounds) else {
            return;
        };
        rect
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
                let snapshot = *pointer;
                (*pointer).selected = if snapshot.window_mode {
                    cursor()
                        .and_then(|(x, y)| {
                            crate::window_detect::window_at_point(
                                std::slice::from_raw_parts(snapshot.windows, snapshot.windows_len),
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
                            .intersect(snapshot.bounds)
                        })
                } else {
                    snapshot
                        .start
                        .zip(cursor())
                        .and_then(|(a, b)| ScreenRect::between(a, b).intersect(snapshot.bounds))
                };
                (*pointer).done = true;
                let _ = ReleaseCapture();
                return LRESULT(0);
            }
            WM_KEYDOWN if wp.0 == 0x1b => {
                (*pointer).done = true;
                return LRESULT(0);
            }
            WM_RBUTTONDOWN | WM_CLOSE | WM_CANCELMODE => {
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
/// both windows were destroyed and DWM presented their removal.
pub fn select_region() -> Result<Option<ScreenRect>, String> {
    select(false)
}
pub fn select_window() -> Result<Option<ScreenRect>, String> {
    select(true)
}
fn select(window_mode: bool) -> Result<Option<ScreenRect>, String> {
    let windows = if window_mode {
        crate::window_detect::enumerate_window_rects()
    } else {
        Vec::new()
    };
    unsafe {
        let module = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let instance = HINSTANCE(module.0);
        let bounds = virtual_bounds();
        if bounds.width == 0 || bounds.height == 0 {
            return Err("No display available".into());
        }
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
            bounds.width as i32,
            bounds.height as i32,
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
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        let _ = ShowWindow(border, SW_HIDE);
        let _ = ShowWindow(veil, SW_HIDE);
        let _ = DestroyWindow(border);
        let _ = DestroyWindow(veil);
        let _ = DwmFlush();
        if failed {
            Err("Selection message loop failed".into())
        } else {
            Ok(state.selected)
        }
    }
}
