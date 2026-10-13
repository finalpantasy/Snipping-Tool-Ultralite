//! Window detection for "Capture Window" mode.
//!
//! Enumerates all visible, non-minimised windows and returns their screen-space
//! rectangles in front-to-back Z-order. Snipping Tool UL's own windows are excluded by
//! comparing the owning process id against ours.

/// A screen-space rectangle: (x, y, width, height).
pub type WinRect = (i32, i32, i32, i32);

/// Return visible window rects in Z-order (front-to-back).
#[cfg(target_os = "windows")]
pub fn enumerate_window_rects() -> Vec<WinRect> {
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Dwm::{
        DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
    };
    use windows::Win32::UI::WindowsAndMessaging::*;

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: EnumWindows invokes this callback synchronously on the calling
        // thread, so the mutable reference is unique for the duration of the call.
        // The pointer originates from a live `&mut Vec<WinRect>` in `enumerate_window_rects`.
        let rects = &mut *(lparam.0 as *mut Vec<WinRect>);

        unsafe {
            if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
                return BOOL(1);
            }

            // Skip our own windows (overlay, pins, settings, history) by PID.
            let mut pid: u32 = 0;
            let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == std::process::id() {
                return BOOL(1);
            }

            // Skip cloaked windows: UWP apps keep invisible "visible" windows
            // around that would otherwise be selectable but show nothing.
            let mut cloaked: u32 = 0;
            let cloak_ok = DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                &mut cloaked as *mut u32 as *mut _,
                std::mem::size_of::<u32>() as u32,
            );
            if cloak_ok.is_ok() && cloaked != 0 {
                return BOOL(1);
            }

            // Prefer the DWM extended frame bounds: GetWindowRect includes the
            // invisible resize border / drop shadow (~7px per side on Win10+).
            let mut rect = RECT::default();
            let dwm_ok = DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rect as *mut RECT as *mut _,
                std::mem::size_of::<RECT>() as u32,
            );
            if dwm_ok.is_err() && GetWindowRect(hwnd, &mut rect).is_err() {
                return BOOL(1);
            }

            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            // Skip tiny/invisible windows
            if w > 1 && h > 1 {
                rects.push((rect.left, rect.top, w, h));
            }
        }

        BOOL(1) // continue enumeration
    }

    let mut rects: Vec<WinRect> = Vec::new();

    unsafe {
        let _ = EnumWindows(
            Some(enum_callback),
            LPARAM(&mut rects as *mut Vec<WinRect> as isize),
        );
    }

    rects
}

/// Find the front-most window rect that contains the given screen-space point.
pub fn window_at_point(rects: &[WinRect], screen_x: i32, screen_y: i32) -> Option<WinRect> {
    rects.iter().copied().find(|&(wx, wy, ww, wh)| {
        ww > 0
            && wh > 0
            && screen_x >= wx
            && (screen_x as i64) < wx as i64 + ww as i64
            && screen_y >= wy
            && (screen_y as i64) < wy as i64 + wh as i64
    })
}

#[cfg(not(target_os = "windows"))]
pub fn enumerate_window_rects() -> Vec<WinRect> {
    Vec::new()
}
