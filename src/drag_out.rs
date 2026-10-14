//! Drag a snip out of UL into another app (Explorer, chat, mail, a browser)
//! as a PNG file. The shell supplies the data object, drag image and drop
//! source, so UL implements no COM interfaces and loads nothing up front.
use std::path::{Path, PathBuf};

const PREFIX: &str = "snipping_tool_ul_drag_";

/// Where a drag's PNG is written. Receivers such as chat apps may read the
/// file after the drop, so it stays until the next drag or until UL exits.
fn drag_path(dir: &Path, stamp: u128) -> PathBuf {
    dir.join(format!("{PREFIX}{}_{stamp}.png", std::process::id()))
}

/// Removes earlier drag files from `dir`, keeping `keep`.
pub fn remove_old_files(dir: &Path, keep: Option<&Path>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let ours = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(PREFIX) && n.ends_with(".png"));
        if ours && Some(path.as_path()) != keep {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Writes the image as a PNG drag file in `dir` and returns its path.
pub fn write_drag_file(
    dir: &Path,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<PathBuf, String> {
    crate::export::validate_image(pixels, width, height)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = drag_path(dir, stamp);
    let png = crate::clipboard_image::encode_png(pixels, width, height)?;
    std::fs::write(&path, png).map_err(|e| format!("Could not write the drag file: {e}"))?;
    remove_old_files(dir, Some(&path));
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
    let path = write_drag_file(&std::env::temp_dir(), pixels, width, height)?;
    drag_file(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_files_are_png_and_only_the_newest_is_kept() {
        let dir = std::env::temp_dir().join(format!("ul-drag-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let unrelated = dir.join("keep-me.png");
        std::fs::write(&unrelated, b"x").unwrap();
        let pixels = [255u8, 0, 0, 255].repeat(4);
        let first = write_drag_file(&dir, &pixels, 2, 2).unwrap();
        let second = write_drag_file(&dir, &pixels, 2, 2).unwrap();
        assert_ne!(first, second);
        assert!(!first.exists(), "an older drag file was left behind");
        let bytes = std::fs::read(&second).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert!(unrelated.exists());
        remove_old_files(&dir, None);
        assert!(!second.exists());
        assert!(write_drag_file(&dir, &pixels, 3, 2).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
