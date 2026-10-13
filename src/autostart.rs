/// Check whether Snipping Tool UL is configured to start on login.
pub fn is_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        is_enabled_windows()
    }
    #[cfg(target_os = "linux")]
    {
        is_enabled_linux()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        false
    }
}

/// Enable or disable auto-start on login.
///
/// Uses `current_exe()` to determine which binary to register.
/// If you need to register a *different* path (e.g. during install),
/// use [`set_enabled_for`] instead.
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let exe = if enabled {
        Some(std::env::current_exe().map_err(|e| format!("Failed to find current exe: {e}"))?)
    } else {
        None
    };
    set_enabled_for(enabled, exe.as_deref())
}

/// Enable or disable auto-start, registering a specific exe path.
///
/// When `enabled` is true, `exe_path` must be `Some`.
pub fn set_enabled_for(enabled: bool, exe_path: Option<&std::path::Path>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        set_enabled_windows(enabled, exe_path)
    }
    #[cfg(target_os = "linux")]
    {
        set_enabled_linux(enabled, exe_path)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = (enabled, exe_path);
        Err("Auto-start is not supported on this platform".into())
    }
}

#[cfg(target_os = "windows")]
fn is_enabled_windows() -> bool {
    use windows::core::*;
    use windows::Win32::System::Registry::*;

    unsafe {
        let mut key = HKEY::default();
        let path = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        if RegOpenKeyExW(HKEY_CURRENT_USER, path, 0, KEY_READ, &mut key).is_ok() {
            let name = w!("Snipping Tool UL");
            let result = RegQueryValueExW(key, name, None, None, None, None);
            let _ = RegCloseKey(key);
            result.is_ok()
        } else {
            false
        }
    }
}

#[cfg(target_os = "windows")]
fn set_enabled_windows(enabled: bool, exe_path: Option<&std::path::Path>) -> Result<(), String> {
    use windows::core::*;
    use windows::Win32::System::Registry::*;

    unsafe {
        let mut key = HKEY::default();
        let path = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        // Create rather than open: a fresh profile may not have a Run key yet.
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path,
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            None,
        )
        .ok()
        .map_err(|e| e.to_string())?;

        struct CloseKey(HKEY);
        impl Drop for CloseKey {
            fn drop(&mut self) {
                unsafe {
                    let _ = RegCloseKey(self.0);
                }
            }
        }
        let _close = CloseKey(key);

        if enabled {
            let exe = exe_path.ok_or("exe_path required when enabling autostart")?;
            // Quote the path: unquoted Run values containing spaces are parsed
            // CreateProcess-style (can try an unintended executable before the first space).
            let exe_str = format!("\"{}\" --background", exe.to_string_lossy());
            let value: Vec<u16> = exe_str.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes: &[u8] =
                std::slice::from_raw_parts(value.as_ptr() as *const u8, value.len() * 2);
            RegSetValueExW(key, w!("Snipping Tool UL"), 0, REG_SZ, Some(bytes))
                .ok()
                .map_err(|e| e.to_string())?;
        } else {
            let result = RegDeleteValueW(key, w!("Snipping Tool UL"));
            if result != windows::Win32::Foundation::ERROR_FILE_NOT_FOUND {
                result.ok().map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn is_enabled_linux() -> bool {
    let path = autostart_desktop_path();
    path.is_some_and(|p| p.exists())
}

#[cfg(target_os = "linux")]
fn set_enabled_linux(enabled: bool, exe_path: Option<&std::path::Path>) -> Result<(), String> {
    let path = autostart_desktop_path().ok_or("Could not determine config directory")?;

    if enabled {
        let exe = exe_path.ok_or("exe_path required when enabling autostart")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // Quote Exec so paths containing spaces survive desktop-entry parsing.
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Snipping Tool UL\nExec=\"{}\"\nX-GNOME-Autostart-enabled=true\n",
            exe.display()
        );
        std::fs::write(&path, content).map_err(|e| e.to_string())
    } else {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
fn autostart_desktop_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("autostart").join("snipping-tool-ul.desktop"))
}
