//! Mocked clipboard work only, no clipboard API or user files.
use std::{sync::mpsc, thread, time::Duration};

#[test]
fn shutdown_waits_for_an_accepted_history_copy_worker() {
    let (started, start_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let (finished, finished_rx) = mpsc::channel();
    snipping_tool_ul::clipboard_queue::spawn(move |_| {
        started.send(()).unwrap();
        release_rx.recv().unwrap();
        finished.send(()).unwrap();
    });
    start_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let (joining, join_rx) = mpsc::channel();
    let (stopped, stop_rx) = mpsc::channel();
    let shutdown = thread::spawn(move || {
        joining.send(()).unwrap();
        snipping_tool_ul::background::join_all(Duration::from_secs(2));
        stopped.send(()).unwrap();
    });
    join_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let returned_early = stop_rx.recv_timeout(Duration::from_millis(100)).is_ok();
    release.send(()).unwrap();
    finished_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    shutdown.join().unwrap();
    assert!(
        !returned_early,
        "Quit ignored an accepted history-copy worker"
    );
}
