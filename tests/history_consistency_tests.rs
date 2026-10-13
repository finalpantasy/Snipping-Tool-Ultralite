use snipping_tool_ul::history;
use std::{
    sync::{Arc, Barrier},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn history_dedup_clear_concurrency_and_clock_rollback_are_consistent() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ul-history-consistency-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    std::env::set_var("SNIPPING_TOOL_UL_CONFIG_DIR", &root);
    let a = [255, 1, 2, 255];
    let b = [22, 33, 44, 255];
    assert!(history::save_to_history_if_new(&a, 1, 1).unwrap().is_some());
    assert!(history::save_to_history_if_new(&a, 1, 1).unwrap().is_none());
    assert_eq!(history::clear_history_checked().unwrap(), 1);
    assert!(history::save_to_history_if_new(&a, 1, 1).unwrap().is_some());
    history::save_to_history(&b, 1, 1).unwrap();
    assert!(history::save_to_history_if_new(&a, 1, 1).unwrap().is_some());
    let last = history::list_history()[0].clone();
    std::fs::remove_file(last).unwrap();
    assert!(history::save_to_history_if_new(&a, 1, 1).unwrap().is_some());
    history::clear_history_checked().unwrap();
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                history::save_to_history_if_new(&a, 1, 1).unwrap().is_some()
            })
        })
        .collect();
    assert_eq!(
        workers
            .into_iter()
            .map(|w| usize::from(w.join().unwrap()))
            .sum::<usize>(),
        1
    );
    assert_eq!(history::list_history().len(), 1);
    history::clear_history_checked().unwrap();
    let dir = history::history_dir().unwrap();
    for i in 0..20 {
        std::fs::write(
            dir.join(format!("{}.png", 90_000_000_000_000u128 + i)),
            b"synthetic old entry",
        )
        .unwrap();
    }

    // A malformed future timestamp must not prevent saves. A PNG-named
    // directory is not a history image, but clear still reports it.
    std::fs::write(dir.join(format!("{}.png", u128::MAX)), b"untrusted entry").unwrap();
    let blocker_dir = dir.join(format!("{}_ignored.png", u128::MAX));
    std::fs::create_dir(&blocker_dir).unwrap();
    assert!(!history::list_history().contains(&blocker_dir));

    let first = history::save_to_history_if_new(&a, 1, 1).unwrap().unwrap();
    assert!(history::save_to_history_if_new(&a, 1, 1).unwrap().is_none());
    assert_eq!(history::list_history()[0], first);
    let second = history::save_to_history(&b, 1, 1).unwrap();
    assert_eq!(history::list_history()[0], second);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 21);
    assert!(first.exists());

    assert!(history::clear_history_checked()
        .unwrap_err()
        .contains("1 entries"));
    assert!(blocker_dir.is_dir());
    std::fs::remove_dir(&blocker_dir).unwrap();
    history::clear_history_checked().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
