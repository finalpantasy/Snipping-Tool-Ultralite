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
///
/// Enabling keeps a registration that points at a newer installed version, so
/// an older instance still running after an upgrade cannot reclaim startup.
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let exe = if enabled {
        Some(std::env::current_exe().map_err(|e| format!("Failed to find current exe: {e}"))?)
    } else {
        None
    };
    #[cfg(target_os = "windows")]
    if let (Some(exe), Some(existing)) = (exe.as_deref(), registered_command_windows()) {
        if keeps_newer_registration(&existing, exe, std::path::Path::is_file) {
            tracing::info!("Auto-start already registers a newer installed version: {existing}");
            return Ok(());
        }
    }
    set_enabled_for(enabled, exe.as_deref())
}

/// True when `existing` is the installer's `"<root>\x.y.z\<exe>" --background`
/// command for a newer version than `current_exe` under the same install root,
/// and that newer executable still exists.
pub fn keeps_newer_registration(
    existing: &str,
    current_exe: &std::path::Path,
    exists: impl Fn(&std::path::Path) -> bool,
) -> bool {
    let Some(registered) = existing
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix("\" --background"))
        .filter(|s| !s.contains('"'))
    else {
        return false;
    };
    let registered = std::path::Path::new(registered);
    let (Some(registered_version), Some(current_version)) =
        (version_folder(registered), version_folder(current_exe))
    else {
        return false;
    };
    let same_text = |a: Option<&std::ffi::OsStr>, b: Option<&std::ffi::OsStr>| match (a, b) {
        (Some(a), Some(b)) => a
            .to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy()),
        _ => false,
    };
    let root = |p: &std::path::Path| {
        p.parent()
            .and_then(std::path::Path::parent)
            .map(|r| r.as_os_str().to_owned())
    };
    same_text(root(registered).as_deref(), root(current_exe).as_deref())
        && same_text(registered.file_name(), current_exe.file_name())
        && registered_version > current_version
        && exists(registered)
}

/// Parses the installer's versioned folder name (`0.3.22`) containing `exe`.
fn version_folder(exe: &std::path::Path) -> Option<(u64, u64, u64)> {
    let name = exe.parent()?.file_name()?.to_str()?;
    let mut parts = name.split('.').map(|p| p.parse::<u64>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
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
fn registered_command_windows() -> Option<String> {
    use windows::core::*;
    use windows::Win32::System::Registry::*;

    unsafe {
        let path = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let name = w!("Snipping Tool UL");
        let mut size = 0u32;
        RegGetValueW(
            HKEY_CURRENT_USER,
            path,
            name,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
        .ok()
        .ok()?;
        let mut buf = vec![0u16; (size as usize).div_ceil(2)];
        RegGetValueW(
            HKEY_CURRENT_USER,
            path,
            name,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        )
        .ok()
        .ok()?;
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
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
