//! Drag a snip out of UL into another app (Explorer, chat, mail, a browser)
//! as a PNG file. The shell supplies the data object, drag image and drop
//! source, so UL implements no COM interfaces and loads nothing up front.
use std::path::{Path, PathBuf};

/// Drag files older than this are removed; receivers such as chat apps may
/// upload from the file for a while after the drop.
const KEEP_FOR: std::time::Duration = std::time::Duration::from_secs(10 * 60);

/// Files this process wrote, removed when it exits: a quit should not leave
/// a private snip behind in Temp.
static WRITTEN: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

/// Removes the drag files this process wrote, and any older ones.
pub fn remove_own_files() {
    let written = std::mem::take(&mut *WRITTEN.lock().unwrap_or_else(|e| e.into_inner()));
    for path in written {
        let _ = std::fs::remove_file(path);
    }
    remove_old_files(&drag_dir());
}

/// Whether Esc is held, which means the user cancelled the drag.
pub fn escape_down() -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE};
        unsafe { GetAsyncKeyState(VK_ESCAPE.0 as i32) < 0 }
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// UL's own folder under Temp, so cleanup never lists the whole Temp folder.
pub fn drag_dir() -> PathBuf {
    std::env::temp_dir().join("Snipping Tool UL drags")
}

/// Removes drag PNGs in `dir` older than ten minutes, from any UL process.
pub fn remove_old_files(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let png = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("png"));
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > KEEP_FOR);
        if png && old {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Writes the image as "Snip <date> <time>.png" in `dir` (the name the
/// receiving app shows) and returns its path.
pub fn write_drag_file(
    dir: &Path,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<PathBuf, String> {
    crate::export::validate_image(pixels, width, height)?;
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not prepare the drag: {e}"))?;
    remove_old_files(dir);
    let stamp = chrono::Local::now()
        .format("Snip %Y-%m-%d %H.%M.%S")
        .to_string();
    let path = (1..)
        .map(|n| {
            dir.join(if n == 1 {
                format!("{stamp}.png")
            } else {
                format!("{stamp} ({n}).png")
            })
        })
        .find(|p| !p.exists())
        .unwrap_or_else(|| dir.join(format!("{stamp}.png")));
    let png = crate::clipboard_image::encode_png(pixels, width, height)?;
    std::fs::write(&path, png).map_err(|e| format!("Could not write the drag file: {e}"))?;
    WRITTEN
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(path.clone());
    Ok(path)
}

/// Starts a modal shell drag of `path`. Returns whether something accepted
/// the drop. Must run on the UI thread, which winit has set up for OLE.
#[cfg(windows)]
pub fn drag_file(path: &Path) -> Result<bool, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::System::Com::IDataObject;
    use windows::Win32::System::Ole::{OleInitialize, DROPEFFECT_COPY, DROPEFFECT_NONE};
    use windows::Win32::UI::Shell::{
        BHID_DataObject, IShellItem, SHCreateItemFromParsingName, SHDoDragDrop,
    };
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        // Already done by winit for its windows; repeating it is harmless.
        let _ = OleInitialize(None);
        let item: IShellItem = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None)
            .map_err(|e| format!("Could not prepare the drag: {e}"))?;
        let data: IDataObject = item
            .BindToHandler(None, &BHID_DataObject)
            .map_err(|e| format!("Could not prepare the drag: {e}"))?;
        let effect = SHDoDragDrop(None, &data, None, DROPEFFECT_COPY)
            .map_err(|e| format!("Drag failed: {e}"))?;
        Ok(effect != DROPEFFECT_NONE)
    }
}

#[cfg(not(windows))]
pub fn drag_file(_path: &Path) -> Result<bool, String> {
    Err("Dragging images out is available on Windows".into())
}

/// Encodes the image to a drag file in the temp folder and drags it.
pub fn drag_image(pixels: &[u8], width: u32, height: u32) -> Result<bool, String> {
    let path = write_drag_file(&drag_dir(), pixels, width, height)?;
    drag_file(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_files_are_named_pngs_kept_for_a_while_then_removed() {
        let dir = std::env::temp_dir().join(format!("ul-drag-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let pixels = [255u8, 0, 0, 255].repeat(4);
        let first = write_drag_file(&dir, &pixels, 2, 2).unwrap();
        let second = write_drag_file(&dir, &pixels, 2, 2).unwrap();
        assert_ne!(first, second, "two drags in one second need two names");
        let name = second.file_name().unwrap().to_string_lossy().to_string();
        assert!(
            name.starts_with("Snip ") && name.ends_with(".png"),
            "{name}"
        );
        // Fresh files survive the next drag: a receiver may still read them.
        assert!(first.exists());
        let bytes = std::fs::read(&second).unwrap();
        assert_eq!(&bytes[..4], [0x89, b'P', b'N', b'G']);
        // Older than ten minutes: removed. Other files: untouched.
        let note = dir.join("note.txt");
        std::fs::write(&note, b"x").unwrap();
        let hour_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        std::fs::File::options()
            .write(true)
            .open(&first)
            .unwrap()
            .set_modified(hour_ago)
            .unwrap();
        remove_old_files(&dir);
        assert!(!first.exists());
        assert!(second.exists());
        assert!(note.exists());
        assert!(write_drag_file(&dir, &pixels, 3, 2).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
