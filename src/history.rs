use image::ImageEncoder;
use std::{
    io::{BufWriter, Write},
    path::PathBuf,
    sync::Mutex,
};
#[derive(Default)]
struct HistoryState {
    last_saved: Option<([u8; 32], PathBuf)>,
}
static HISTORY_IO: Mutex<HistoryState> = Mutex::new(HistoryState { last_saved: None });
fn is_history_image(path: &std::path::Path) -> bool {
    path.extension().and_then(|x| x.to_str()).is_some_and(|x| {
        matches!(
            x.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "webp"
        )
    })
}
fn reserve_path(
    dir: &std::path::Path,
    ext: &str,
) -> Result<(PathBuf, PathBuf, std::fs::File), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let stamp = next_stamp(dir, now)?;
    let saturated_sequence = (stamp == u128::MAX).then(|| next_saturated_sequence(dir));
    for offset in 0..10000u128 {
        let path = if let Some(first_sequence) = saturated_sequence {
            let sequence = first_sequence
                .checked_add(offset)
                .ok_or_else(|| "History filename sequence is out of range".to_string())?;
            dir.join(format!(
                "{stamp}~{sequence:020}_{:010}.{ext}",
                std::process::id()
            ))
        } else {
            dir.join(format!(
                "{stamp}_{:010}_{offset:04}.{ext}",
                std::process::id()
            ))
        };
        if path.exists() {
            continue;
        }
        let temp = path.with_extension(format!("{ext}.part"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(file) => return Ok((path, temp, file)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("Unable to reserve a unique history filename".into())
}

/// At the timestamp ceiling, keep new names sortable with a separate suffix.
/// The '~' sorts after normal timestamp suffixes. A global sequence preserves
/// ordering across processes, while the PID keeps simultaneous reservations unique.
fn next_saturated_sequence(dir: &std::path::Path) -> u128 {
    let prefix = format!("{}~", u128::MAX);
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_history_image(path))
        .filter_map(|path| {
            path.file_stem()?
                .to_str()?
                .strip_prefix(&prefix)?
                .split('_')
                .next()?
                .parse::<u128>()
                .ok()
        })
        .max()
        .and_then(|sequence| sequence.checked_add(1))
        .unwrap_or(0)
}
fn publish_file(
    path: PathBuf,
    temp: PathBuf,
    result: Result<(), String>,
) -> Result<PathBuf, String> {
    if let Err(error) =
        result.and_then(|()| std::fs::rename(&temp, &path).map_err(|e| e.to_string()))
    {
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    Ok(path)
}

const MAX_HISTORY: usize = 20;

/// History lives in the LOCAL data dir (e.g. %LocalAppData%) — captures can be
/// sensitive and must not sync into roaming profiles.
pub fn history_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("SNIPPING_TOOL_UL_CONFIG_DIR") {
        return Some(PathBuf::from(path).join("history"));
    }
    dirs::data_local_dir().map(|d| d.join("snipping-tool-ul").join("history"))
}

/// Pre-0.6 history location (roaming config dir).
fn legacy_history_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("snipping-tool-ul").join("history"))
}

/// Move any PNGs from the legacy location into `dir` (best effort, once).
fn migrate_legacy(dir: &std::path::Path) {
    if std::env::var_os("SNIPPING_TOOL_UL_CONFIG_DIR").is_some() {
        return;
    }
    let Some(legacy) = legacy_history_dir() else {
        return;
    };
    if legacy == dir || !legacy.is_dir() {
        return;
    }
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(&legacy) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_history_image(&path) {
                if let Some(name) = path.file_name() {
                    let destination = dir.join(name);
                    if !destination.exists() {
                        let _ = std::fs::rename(&path, destination);
                    }
                }
            }
        }
    }
    let _ = std::fs::remove_dir(&legacy); // only removes if now empty
}

fn ensure_dir() -> Result<PathBuf, String> {
    let dir = history_dir().ok_or("No local data directory")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    migrate_legacy(&dir);
    Ok(dir)
}

/// Save a capture to history. Filename: timestamp-based (e.g., "1711234567.png")
pub fn save_to_history(pixels: &[u8], width: u32, height: u32) -> Result<PathBuf, String> {
    crate::export::validate_image(pixels, width, height)?;
    let mut io = HISTORY_IO.lock().unwrap_or_else(|e| e.into_inner());
    let path = save_pixels_locked(pixels, width, height)?;
    io.last_saved = None;
    Ok(path)
}

fn save_pixels_locked(pixels: &[u8], width: u32, height: u32) -> Result<PathBuf, String> {
    let dir = ensure_dir()?;
    let (path, temp, file) = reserve_path(&dir, "png")?;
    let result = (|| {
        let mut out = BufWriter::new(file);
        image::codecs::png::PngEncoder::new_with_quality(
            &mut out,
            image::codecs::png::CompressionType::Fast,
            image::codecs::png::FilterType::Adaptive,
        )
        .write_image(pixels, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())
    })();
    let path = publish_file(path, temp, result)?;

    // Prune old entries (keep only MAX_HISTORY most recent)
    report_prune_failure(&dir);

    Ok(path)
}

/// Save to history unless the same pixels were the previous history write.
///
/// The editor saves the capture when the preview opens, and again on Copy,
/// Pin and Save; without this, a few Copy clicks fill the 20-entry history
/// with one image and push older captures out. Returns `Ok(None)` when the
/// write was skipped.
pub fn save_to_history_if_new(
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<Option<PathBuf>, String> {
    crate::export::validate_image(pixels, width, height)?;
    let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);
    ctx.update(&width.to_le_bytes());
    ctx.update(&height.to_le_bytes());
    ctx.update(pixels);
    let mut key = [0u8; 32];
    key.copy_from_slice(ctx.finish().as_ref());
    let mut io = HISTORY_IO.lock().unwrap_or_else(|e| e.into_inner());
    let dir = history_dir();
    if io.last_saved.as_ref().is_some_and(|(last, path)| {
        *last == key && path.parent() == dir.as_deref() && path.is_file()
    }) {
        return Ok(None);
    }
    let path = save_pixels_locked(pixels, width, height)?;
    io.last_saved = Some((key, path.clone()));
    Ok(Some(path))
}

/// Copy an already-saved file into history (avoids re-encoding).
pub fn save_to_history_from_file(source: &std::path::Path) -> Result<PathBuf, String> {
    let format = image::ImageReader::open(source)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .format();
    let ext = match format {
        Some(image::ImageFormat::Png) => "png",
        Some(image::ImageFormat::Jpeg) => "jpg",
        Some(image::ImageFormat::WebP) => "webp",
        _ => return Err("Unsupported history image format".into()),
    };
    let mut io = HISTORY_IO.lock().unwrap_or_else(|e| e.into_inner());
    let dir = ensure_dir()?;
    let (path, temp, file) = reserve_path(&dir, ext)?;
    let result = (|| {
        let mut out = BufWriter::new(file);
        let mut input = std::fs::File::open(source).map_err(|e| e.to_string())?;
        std::io::copy(&mut input, &mut out).map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())
    })();
    let path = publish_file(path, temp, result)?;

    report_prune_failure(&dir);
    io.last_saved = None;
    Ok(path)
}

/// Get list of history entries sorted newest first
pub fn list_history() -> Vec<PathBuf> {
    let _guard = HISTORY_IO.lock().unwrap_or_else(|e| e.into_inner());
    let Some(dir) = history_dir() else {
        return vec![];
    };
    migrate_legacy(&dir);
    let mut entries = ordered_entries(&dir);
    entries.truncate(MAX_HISTORY);
    entries
}

/// Delete every history entry. Compatibility helper for non-UI callers.
pub fn clear_history() -> usize {
    clear_history_checked().unwrap_or_else(|error| {
        tracing::warn!("{error}");
        0
    })
}

/// Include legacy files so cleared captures cannot migrate back on next open.
pub fn clear_history_checked() -> Result<usize, String> {
    let mut io = HISTORY_IO.lock().unwrap_or_else(|e| e.into_inner());
    io.last_saved = None;
    let mut dirs = Vec::new();
    if let Some(dir) = history_dir() {
        dirs.push(dir);
    }
    if std::env::var_os("SNIPPING_TOOL_UL_CONFIG_DIR").is_none() {
        if let Some(legacy) = legacy_history_dir() {
            if !dirs.contains(&legacy) {
                dirs.push(legacy);
            }
        }
    }
    clear_directories(&dirs)
}

fn clear_directories(dirs: &[PathBuf]) -> Result<usize, String> {
    let (mut removed, mut failed) = (0, 0);
    for dir in dirs {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                failed += 1;
                continue;
            }
        };
        for entry in entries {
            match entry {
                Ok(entry) if is_history_image(&entry.path()) => {
                    if std::fs::remove_file(entry.path()).is_ok() {
                        removed += 1;
                    } else {
                        failed += 1;
                    }
                }
                Err(_) => failed += 1,
                _ => {}
            }
        }
    }
    if failed == 0 {
        Ok(removed)
    } else {
        Err(format!(
            "Removed {removed} history images; {failed} entries could not be cleared."
        ))
    }
}

fn history_stamp(path: &std::path::Path) -> u128 {
    path.file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.split(['_', '~']).next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
fn ordered_entries(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_history_image(p))
        .collect();
    entries.sort_by(|a, b| {
        history_stamp(b)
            .cmp(&history_stamp(a))
            .then_with(|| b.file_name().cmp(&a.file_name()))
    });
    entries
}
fn next_stamp(dir: &std::path::Path, now: u128) -> Result<u128, String> {
    match ordered_entries(dir).first() {
        Some(path) => {
            let last = history_stamp(path);
            // Once the timestamp is saturated, reserve_path advances the
            // sortable suffix while keeping this timestamp stable.
            Ok(last.checked_add(1).unwrap_or(last).max(now))
        }
        None => Ok(now),
    }
}
fn prune_history(dir: &std::path::Path) -> Result<(), String> {
    let mut failed = 0usize;
    let mut first_error = None;
    for path in ordered_entries(dir).iter().skip(MAX_HISTORY) {
        if let Err(error) = std::fs::remove_file(path) {
            failed += 1;
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
    }
    match (failed, first_error) {
        (0, _) => Ok(()),
        (failed, Some(error)) => Err(format!(
            "Could not prune {failed} old history image(s); first error: {error}"
        )),
        _ => unreachable!(),
    }
}

fn report_prune_failure(dir: &std::path::Path) {
    if let Err(error) = prune_history(dir) {
        tracing::warn!("History save succeeded, but pruning failed: {error}");
    }
}

#[cfg(test)]
mod clear_tests {
    use super::*;
    #[test]
    fn clear_covers_colliding_legacy_files_and_reports_unremovable_entries() {
        let root = std::env::temp_dir().join(format!(
            "ul-history-clear-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let local = root.join("local");
        let legacy = root.join("legacy");
        std::fs::create_dir_all(&local).unwrap();
        std::fs::create_dir(&legacy).unwrap();
        std::fs::write(local.join("123.png"), b"new").unwrap();
        std::fs::write(legacy.join("123.png"), b"old").unwrap();
        std::fs::write(legacy.join("keep.txt"), b"not a capture").unwrap();
        assert_eq!(
            clear_directories(&[local.clone(), legacy.clone()]).unwrap(),
            2
        );
        assert!(ordered_entries(&local).is_empty());
        assert!(ordered_entries(&legacy).is_empty());
        assert!(legacy.join("keep.txt").exists());
        std::fs::create_dir(local.join("locked.png")).unwrap();
        assert!(clear_directories(&[local])
            .unwrap_err()
            .contains("1 entries"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn png_directories_are_excluded_and_saturated_timestamps_keep_new_saves_ordered() {
        let root = std::env::temp_dir().join(format!(
            "ul-history-order-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();

        let saturated_file = root.join(format!("{}.png", u128::MAX));
        let saturated_directory = root.join(format!("{}_000.png", u128::MAX));
        std::fs::write(&saturated_file, b"untrusted filename").unwrap();
        std::fs::create_dir(&saturated_directory).unwrap();

        assert_eq!(ordered_entries(&root), vec![saturated_file.clone()]);
        assert_eq!(next_stamp(&root, 1).unwrap(), u128::MAX);

        let mut saturated_saves = Vec::new();
        for _ in 0..2 {
            let (path, temp, file) = reserve_path(&root, "png").unwrap();
            drop(file);
            saturated_saves.push(publish_file(path, temp, Ok(())).unwrap());
        }
        assert!(saturated_saves[0] < saturated_saves[1]);
        assert_eq!(ordered_entries(&root).first(), saturated_saves.get(1));

        for stamp in 1..=21 {
            std::fs::write(root.join(format!("{stamp}.png")), b"old entry").unwrap();
        }
        assert!(prune_history(&root).is_ok());
        let entries = ordered_entries(&root);
        assert_eq!(entries.len(), MAX_HISTORY);
        assert!(entries.iter().all(|path| path.is_file()));
        assert!(saturated_directory.is_dir());

        std::fs::remove_dir_all(root).unwrap();
    }
}
