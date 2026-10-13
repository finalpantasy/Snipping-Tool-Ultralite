use std::fs;
use std::sync::Mutex;
use std::time::Duration;

/// Always isolate destructive pruning tests from the user's actual history.
fn require_config_dir() -> bool {
    static TEST_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    let dir = TEST_DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!(
            "ul-history-tests-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("SNIPPING_TOOL_UL_CONFIG_DIR", &dir);
        dir
    });
    snipping_tool_ul::history::history_dir() == Some(dir.join("history"))
}

/// Mutex to serialize history tests that share the same directory.
static HISTORY_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn saved_jpeg_and_webp_keep_their_real_formats_in_history() {
    let _guard = HISTORY_LOCK.lock().unwrap();
    assert!(require_config_dir());
    let root = snipping_tool_ul::history::history_dir()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    for extension in ["jpg", "webp", "png"] {
        let source = root.join(format!("sample.{extension}"));
        snipping_tool_ul::export::save_image_to_path([255, 0, 0, 255].repeat(16), 4, 4, &source)
            .unwrap();
        let path = snipping_tool_ul::history::save_to_history_from_file(&source).unwrap();
        assert_eq!(path.extension().unwrap(), extension);
        assert_eq!(
            std::fs::read(&source).unwrap(),
            std::fs::read(&path).unwrap()
        );
        assert_eq!(image::open(&path).unwrap().width(), 4);
        assert!(snipping_tool_ul::history::list_history().contains(&path));
    }
    snipping_tool_ul::history::clear_history();
}

#[test]
fn concurrent_history_saves_never_overwrite_each_other_or_publish_partial_images() {
    let _guard = HISTORY_LOCK.lock().unwrap();
    assert!(require_config_dir());
    snipping_tool_ul::history::clear_history();
    let joins: Vec<_> = (0..12u8)
        .map(|i| {
            std::thread::spawn(move || {
                snipping_tool_ul::history::save_to_history(&[i, 0, 0, 255], 1, 1).unwrap()
            })
        })
        .collect();
    let paths: std::collections::HashSet<_> =
        joins.into_iter().map(|j| j.join().unwrap()).collect();
    assert_eq!(paths.len(), 12);
    let colors: std::collections::HashSet<_> = paths
        .iter()
        .map(|p| image::open(p).unwrap().to_rgba8().as_raw()[0])
        .collect();
    assert_eq!(colors.len(), 12);
    assert_eq!(snipping_tool_ul::history::list_history().len(), 12);
    snipping_tool_ul::history::clear_history();
}

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

#[test]
fn saturated_history_names_save_in_order_and_png_directories_remain_clearable() {
    let _lock = HISTORY_LOCK.lock().unwrap();
    assert!(require_config_dir());
    let _ = snipping_tool_ul::history::clear_history();
    let dir = snipping_tool_ul::history::history_dir().unwrap();
    fs::create_dir_all(&dir).unwrap();

    let saturated_file = dir.join(format!("{}.png", u128::MAX));
    let png_directory = dir.join(format!("{}_directory.png", u128::MAX));
    fs::write(&saturated_file, b"untrusted filename").unwrap();
    fs::create_dir(&png_directory).unwrap();
    for stamp in 1..=20 {
        fs::write(dir.join(format!("{stamp}.png")), b"old history file").unwrap();
    }

    assert!(!snipping_tool_ul::history::list_history().contains(&png_directory));
    let first = snipping_tool_ul::history::save_to_history(&[1, 2, 3, 255], 1, 1).unwrap();
    let second = snipping_tool_ul::history::save_to_history(&[4, 5, 6, 255], 1, 1).unwrap();
    assert!(first.file_name() < second.file_name());
    assert_eq!(
        snipping_tool_ul::history::list_history().first(),
        Some(&second)
    );
    assert_eq!(snipping_tool_ul::history::list_history().len(), 20);
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 21);
    assert!(png_directory.is_dir());

    assert!(snipping_tool_ul::history::clear_history_checked()
        .unwrap_err()
        .contains("1 entries"));
    assert!(png_directory.is_dir());
    fs::remove_dir(&png_directory).unwrap();
    snipping_tool_ul::history::clear_history_checked().unwrap();
}
