//! Serializes clipboard writes and discards delayed work superseded by a newer request.
use std::sync::{
    atomic::{AtomicU64, Ordering},
    LazyLock, Mutex, TryLockError,
};
static GENERATION: AtomicU64 = AtomicU64::new(0);
static WRITER: Mutex<()> = Mutex::new(());
static PREPARATION: LazyLock<crate::history_thumbnails::LatestTaskQueue> =
    LazyLock::new(crate::history_thumbnails::LatestTaskQueue::tracked);
pub fn reserve() -> u64 {
    GENERATION.fetch_add(1, Ordering::SeqCst).wrapping_add(1)
}
pub fn is_current(ticket: u64) -> bool {
    GENERATION.load(Ordering::SeqCst) == ticket
}
/// Keep one active preparation and the newest pending request. Reserve first,
/// so newer capture/OCR copies also invalidate a slow History decode.
pub fn spawn(task: impl FnOnce(u64) + Send + 'static) {
    let ticket = reserve();
    PREPARATION.submit(move |cancel| {
        if !cancel.load(Ordering::Relaxed) && is_current(ticket) {
            task(ticket);
        }
    });
}

pub fn run(ticket: u64, write: impl FnOnce() -> Result<(), String>) -> Result<bool, String> {
    run_cancellable(ticket, || false, write)
}

pub fn run_cancellable(
    ticket: u64,
    cancelled: impl Fn() -> bool,
    write: impl FnOnce() -> Result<(), String>,
) -> Result<bool, String> {
    let guard = loop {
        if !is_current(ticket) || cancelled() {
            return Ok(false);
        }
        // A panic inside one write must not disable later clipboard copies.
        match WRITER.try_lock() {
            Ok(guard) => break guard,
            Err(TryLockError::Poisoned(error)) => break error.into_inner(),
            Err(TryLockError::WouldBlock) => {
                std::thread::sleep(std::time::Duration::from_millis(2))
            }
        }
    };
    if !is_current(ticket) || cancelled() {
        return Ok(false);
    }
    write()?;
    drop(guard);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn superseded_work_never_touches_clipboard() {
        let old = reserve();
        let current = reserve();
        assert!(!run(old, || panic!("Stale clipboard operation executed")).unwrap());
        assert!(run(current, || Ok(())).unwrap());
        assert_eq!(run(current, || Err("busy".into())).unwrap_err(), "busy");

        // Model a slow history decode finishing after a newer copy. Only the
        // newer user request may publish, regardless of worker completion order.
        let (release, wait) = std::sync::mpsc::channel();
        let (older_done, older_rx) = std::sync::mpsc::channel();
        let (older_started, started_rx) = std::sync::mpsc::channel();
        spawn(move |ticket| {
            older_started.send(()).unwrap();
            wait.recv().unwrap();
            assert!(!run(ticket, || panic!("Delayed decode overwrote newer copy")).unwrap());
            older_done.send(()).unwrap();
        });
        started_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let (newer_done, newer_rx) = std::sync::mpsc::channel();
        spawn(move |ticket| {
            assert!(run(ticket, || Ok(())).unwrap());
            newer_done.send(()).unwrap();
        });
        release.send(()).unwrap();
        older_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        newer_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
    }
}
