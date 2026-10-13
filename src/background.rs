//! Tracked worker threads for file writes that must finish before the process
//! exits (auto-save, history, Save, explicit image copies). Network uploads
//! stay detached: an exiting process should not wait on the network.
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

struct Workers(Mutex<Vec<JoinHandle<()>>>);
static WORKERS: Workers = Workers(Mutex::new(Vec::new()));

/// Spawn `f` on a new thread and remember its handle until it finishes.
pub fn spawn<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    WORKERS.spawn(f);
}

pub(crate) fn track(handle: JoinHandle<()>) {
    WORKERS.track(handle);
}

/// Bound every shutdown stage with the same absolute deadline.
pub fn join_worker_until(handle: JoinHandle<()>, deadline: Instant) -> bool {
    while !handle.is_finished() {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return false;
        };
        std::thread::sleep(remaining.min(Duration::from_millis(5)));
    }
    let _ = handle.join();
    true
}

impl Workers {
    fn spawn(&self, f: impl FnOnce() + Send + 'static) {
        let handle = std::thread::spawn(f);
        self.track(handle);
    }

    fn track(&self, handle: JoinHandle<()>) {
        let mut workers = self.0.lock().unwrap_or_else(|e| e.into_inner());
        // Finished handles are dropped here, so the list stays short in a long
        // tray session instead of growing by one per capture.
        workers.retain(|h| !h.is_finished());
        workers.push(handle);
    }

    /// Wait for every tracked worker, giving up after `timeout` in total.
    fn join_all(&self, timeout: Duration) {
        self.join_until(Instant::now() + timeout);
    }

    fn join_until(&self, deadline: Instant) {
        let handles: Vec<_> = {
            let mut workers = self.0.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *workers)
        };
        for handle in handles {
            if !join_worker_until(handle, deadline) {
                tracing::warn!("Exiting with a background file write still running");
                return;
            }
        }
    }
}

pub fn join_all(timeout: Duration) {
    WORKERS.join_all(timeout);
}

pub fn join_all_until(deadline: Instant) {
    WORKERS.join_until(deadline);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn a_blocked_ocr_join_cannot_extend_the_shared_shutdown_deadline() {
        let (release, blocked) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            blocked.recv().unwrap();
        });
        let workers = Workers(Mutex::new(Vec::new()));
        let (write_release, write_blocked) = std::sync::mpsc::channel();
        workers.spawn(move || {
            write_blocked.recv().unwrap();
        });
        let start = Instant::now();
        let deadline = start + Duration::from_millis(40);
        assert!(!join_worker_until(worker, deadline));
        workers.join_until(deadline);
        assert!(start.elapsed() < Duration::from_millis(250));
        release.send(()).unwrap();
        write_release.send(()).unwrap();
    }

    #[test]
    fn join_all_waits_for_tracked_work() {
        let workers = Workers(Mutex::new(Vec::new()));
        let done = Arc::new(AtomicBool::new(false));
        let flag = done.clone();
        workers.spawn(move || {
            std::thread::sleep(Duration::from_millis(40));
            flag.store(true, Ordering::SeqCst);
        });
        workers.join_all(Duration::from_secs(5));
        assert!(done.load(Ordering::SeqCst));
    }

    #[test]
    fn join_all_gives_up_after_timeout() {
        let workers = Workers(Mutex::new(Vec::new()));
        let (release, wait) = std::sync::mpsc::channel();
        let (done, completion) = std::sync::mpsc::channel();
        workers.spawn(move || {
            wait.recv().unwrap();
            done.send(()).unwrap();
        });
        let started = Instant::now();
        workers.join_all(Duration::from_millis(30));
        assert!(started.elapsed() < Duration::from_millis(250));
        release.send(()).unwrap();
        completion.recv_timeout(Duration::from_secs(5)).unwrap();
    }
}
