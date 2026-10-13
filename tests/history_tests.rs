use std::fs;
use std::sync::Mutex;
use std::time::Duration;

/// Skip history tests if no config directory is available (e.g., headless CI).
fn require_config_dir() -> bool {
    snipping_tool_ul::history::history_dir().is_some()
}

/// Mutex to serialize history tests that share the same directory.
static HISTORY_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn save_creates_file_in_history_dir() {
    let _lock = HISTORY_LOCK.lock().unwrap();
    if !require_config_dir() {
        eprintln!("Skipping: no config directory available");
        return;
    }
    let pixels: Vec<u8> = vec![255, 0, 0, 255]; // 1x1 red pixel
    let result = snipping_tool_ul::history::save_to_history(&pixels, 1, 1);
    assert!(result.is_ok(), "save_to_history should succeed");
    let path = result.unwrap();
    assert!(path.exists(), "Saved file should exist on disk");
    assert_eq!(
        path.extension().and_then(|e| e.to_str()),
        Some("png"),
        "Should be a .png file"
    );
    let _ = fs::remove_file(&path);
}

#[test]
fn list_history_returns_entries_sorted_newest_first() {
    let _lock = HISTORY_LOCK.lock().unwrap();
    if !require_config_dir() {
        eprintln!("Skipping: no config directory available");
        return;
    }
    let pixels: Vec<u8> = vec![0, 0, 255, 255]; // 1x1 blue pixel

    let path1 = snipping_tool_ul::history::save_to_history(&pixels, 1, 1).unwrap();
    std::thread::sleep(Duration::from_secs(1));
    let path2 = snipping_tool_ul::history::save_to_history(&pixels, 1, 1).unwrap();

    let entries = snipping_tool_ul::history::list_history();
    assert!(entries.len() >= 2, "Should have at least 2 entries");

    assert_eq!(
        entries[0].file_name(),
        path2.file_name(),
        "First entry should be newest"
    );

    let _ = fs::remove_file(&path1);
    let _ = fs::remove_file(&path2);
}

#[test]
fn prune_keeps_only_max_entries() {
    let _lock = HISTORY_LOCK.lock().unwrap();
    if !require_config_dir() {
        eprintln!("Skipping: no config directory available");
        return;
    }
    let pixels: Vec<u8> = vec![0, 255, 0, 255]; // 1x1 green pixel
    let mut paths = vec![];

    for _ in 0..22 {
        if let Ok(p) = snipping_tool_ul::history::save_to_history(&pixels, 1, 1) {
            paths.push(p);
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let entries = snipping_tool_ul::history::list_history();
    assert!(
        entries.len() <= 20,
        "History should be pruned to at most 20, got {}",
        entries.len()
    );

    for p in &paths {
        let _ = fs::remove_file(p);
    }
    for e in &entries {
        let _ = fs::remove_file(e);
    }
}
