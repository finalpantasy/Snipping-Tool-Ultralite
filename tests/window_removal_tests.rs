use snipping_tool_ul::ui;
use std::time::Duration;

// One test per binary: the tracker is process-wide state.
#[test]
fn capture_waits_only_after_a_ul_window_left_the_screen() {
    assert!(!ui::window_removed_within(Duration::from_secs(3600)));
    ui::note_window_removed();
    assert!(ui::window_removed_within(Duration::from_secs(1)));
    std::thread::sleep(Duration::from_millis(30));
    assert!(!ui::window_removed_within(Duration::from_millis(10)));
}
