//! Generated payloads and mocked writers only; never calls a clipboard API.
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc, Arc, Condvar, Mutex,
};
use std::{thread, time::Duration};
static SERIAL: Mutex<()> = Mutex::new(());

#[test]
fn repeated_copy_batches_release_payloads_and_worker_handles() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    fn handles() -> u32 {
        #[cfg(windows)]
        {
            let mut count = 0;
            unsafe {
                windows::Win32::System::Threading::GetProcessHandleCount(
                    windows::Win32::System::Threading::GetCurrentProcess(),
                    &mut count,
                )
                .unwrap();
            }
            count
        }
        #[cfg(not(windows))]
        {
            0
        }
    }
    snipping_tool_ul::background::join_all(Duration::from_secs(3));
    let before = handles();
    for _ in 0..50 {
        let (started, start_rx) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let payload = Arc::new(vec![255u8; 512 * 1024]);
        let first = Arc::downgrade(&payload);
        snipping_tool_ul::clipboard_queue::spawn(move |_| {
            started.send(()).unwrap();
            blocked.recv().unwrap();
            drop(payload);
        });
        start_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let mut obsolete = Vec::new();
        for _ in 0..19 {
            let payload = Arc::new(vec![255u8; 512 * 1024]);
            obsolete.push(Arc::downgrade(&payload));
            snipping_tool_ul::clipboard_queue::spawn(move |_| drop(payload));
        }
        // Superseded pending closures are released even before active work ends.
        assert!(obsolete[..18].iter().all(|p| p.upgrade().is_none()));
        release.send(()).unwrap();
        snipping_tool_ul::background::join_all(Duration::from_secs(3));
        assert!(first.upgrade().is_none());
        assert!(obsolete.iter().all(|p| p.upgrade().is_none()));
    }
    let after = handles();
    println!("copy_cleanup batches=50 requests=1000 retained_payloads=0 handles_before={before} handles_after={after}");
    assert!(after <= before, "Repeated copy workers retained OS handles");
}

#[test]
fn repeated_history_requests_do_not_start_parallel_decodes() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let blocked = Arc::new((Mutex::new(false), Condvar::new()));
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let (started, start_rx) = mpsc::channel();
    const REQUESTS: usize = 1000;
    let (released, release_rx) = mpsc::channel();
    for i in 0..REQUESTS {
        let blocked = blocked.clone();
        let active = active.clone();
        let peak = peak.clone();
        let started = started.clone();
        let released = released.clone();
        snipping_tool_ul::clipboard_queue::spawn(move |_| {
            let pixels = vec![255u8; 2 * 1024 * 1024];
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(current, Ordering::SeqCst);
            started.send(i).unwrap();
            let (lock, wake) = &*blocked;
            let mut ready = lock.lock().unwrap();
            while !*ready {
                ready = wake.wait(ready).unwrap();
            }
            drop(pixels);
            active.fetch_sub(1, Ordering::SeqCst);
            released.send(i).unwrap();
        });
        if i == 0 {
            assert_eq!(start_rx.recv_timeout(Duration::from_secs(2)).unwrap(), 0);
        }
    }
    thread::sleep(Duration::from_millis(100));
    let concurrent = peak.load(Ordering::SeqCst);
    {
        let (lock, wake) = &*blocked;
        *lock.lock().unwrap() = true;
        wake.notify_all();
    }
    snipping_tool_ul::background::join_all(Duration::from_secs(3));
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert_eq!(
        concurrent, 1,
        "Superseded History requests retained parallel decoded payloads"
    );
    assert_eq!(
        start_rx.try_iter().collect::<Vec<_>>(),
        vec![REQUESTS - 1],
        "Only the newest pending request should decode"
    );
    let mut dropped = release_rx.try_iter().collect::<Vec<_>>();
    dropped.sort_unstable();
    assert_eq!(dropped, vec![0, REQUESTS - 1]);
}

#[test]
fn ocr_cancellation_does_not_wait_for_a_slow_clipboard_writer() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (held, held_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let ticket = snipping_tool_ul::clipboard_queue::reserve();
    let holder = thread::spawn(move || {
        snipping_tool_ul::clipboard_queue::run(ticket, || {
            held.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(())
        })
    });
    held_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let ticket = snipping_tool_ul::clipboard_queue::reserve();
    let cancelled = Arc::new(AtomicBool::new(false));
    let stop = cancelled.clone();
    let (finished, finished_rx) = mpsc::channel();
    let (waiting, waiting_rx) = mpsc::channel();
    let waiter = thread::spawn(move || {
        let attempts = AtomicUsize::new(0);
        let result = snipping_tool_ul::clipboard_queue::run_cancellable(
            ticket,
            || {
                let cancelled = stop.load(Ordering::SeqCst);
                if attempts.fetch_add(1, Ordering::SeqCst) == 1 {
                    // A second admission attempt proves the writer was blocked.
                    waiting.send(()).unwrap();
                }
                cancelled
            },
            || panic!("OCR must not publish after cancellation"),
        );
        finished.send(result).unwrap();
    });
    waiting_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    cancelled.store(true, Ordering::SeqCst);
    let stopped = finished_rx.recv_timeout(Duration::from_millis(100)).is_ok();
    release.send(()).unwrap();
    holder.join().unwrap().unwrap();
    waiter.join().unwrap();
    assert!(
        stopped,
        "OCR cancellation remained blocked behind the clipboard mutex"
    );
}
