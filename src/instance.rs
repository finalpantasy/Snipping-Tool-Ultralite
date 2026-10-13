//! Idle single-instance activation: no polling or resident desktop capture.
#[cfg(target_os = "windows")]
pub struct Instance(windows::Win32::Foundation::HANDLE);
#[cfg(target_os = "windows")]
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
#[cfg(not(target_os = "windows"))]
pub struct Instance;

/// None means an existing process received the request to show its home window.
pub fn acquire(
    request_show: bool,
    on_show: impl Fn() + Send + 'static,
) -> Result<Option<Instance>, String> {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::{
            core::w,
            Win32::{
                Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, WAIT_OBJECT_0},
                System::Threading::{
                    CreateEventW, CreateMutexW, OpenEventW, SetEvent, WaitForSingleObject,
                    EVENT_MODIFY_STATE, INFINITE,
                },
                UI::WindowsAndMessaging::{
                    AllowSetForegroundWindow, FindWindowW, GetWindowThreadProcessId, ASFW_ANY,
                },
            },
        };
        let mutex = CreateMutexW(None, false, w!("Local\\SnippingToolUL.SingleInstance"))
            .map_err(|e| e.to_string())?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(mutex);
            if !request_show {
                return Ok(None);
            }
            if let Ok(hwnd) = FindWindowW(None, w!("Snipping Tool UL")) {
                let mut pid = 0;
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
                if pid != 0 {
                    let _ = AllowSetForegroundWindow(pid);
                }
            } else {
                // Home is destroyed while the app sits in the tray, so there is
                // no window to find. This launcher is foreground; let the running
                // process take focus when it recreates Home.
                let _ = AllowSetForegroundWindow(ASFW_ANY);
            }
            for _ in 0..25 {
                if let Ok(event) = OpenEventW(
                    EVENT_MODIFY_STATE,
                    false,
                    w!("Local\\SnippingToolUL.ShowHome"),
                ) {
                    let r = SetEvent(event).map_err(|e| e.to_string());
                    let _ = CloseHandle(event);
                    r?;
                    return Ok(None);
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            return Err("The running instance is still starting; try again.".into());
        }
        let event = match CreateEventW(None, false, false, w!("Local\\SnippingToolUL.ShowHome")) {
            Ok(e) => e,
            Err(e) => {
                let _ = CloseHandle(mutex);
                return Err(e.to_string());
            }
        };
        // HANDLE cannot be sent directly. Transfer its owned value into a
        // waiting thread; that thread closes it if the wait ever fails.
        let raw = event.0 as usize;
        std::thread::spawn(move || {
            let event = windows::Win32::Foundation::HANDLE(raw as _);
            loop {
                if WaitForSingleObject(event, INFINITE) != WAIT_OBJECT_0 {
                    break;
                }
                on_show();
            }
            let _ = CloseHandle(event);
        });
        Ok(Some(Instance(mutex)))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (request_show, on_show);
        Ok(Some(Instance))
    }
}
