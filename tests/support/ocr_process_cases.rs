//! Headless supervisor tests; real Windows OCR runs only in an explicit child fixture.
use super::*;
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn wait_idle(active: &AtomicBool) {
    let start = Instant::now();
    while active.load(Ordering::Acquire) {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "native worker did not release ownership"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn a_status_query_failure_keeps_the_flight_until_terminal_state() {
    use windows::Foundation::AsyncStatus;
    static BUSY: AtomicBool = AtomicBool::new(false);
    let (release, blocked) = mpsc::channel();
    let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = cancelled.clone();
    let error = request(
        vec![255; 4],
        1,
        1,
        Control::new(Duration::from_secs(2)),
        &BUSY,
        move |_, _, _, control| {
            let mut calls = 0;
            native::wait_for_terminal(
                control,
                || {
                    calls += 1;
                    match calls {
                        1 => Ok(AsyncStatus::Started),
                        2..=4 => Err("injected status-query failure".into()),
                        _ => {
                            blocked.recv().unwrap();
                            Ok(AsyncStatus::Completed)
                        }
                    }
                },
                || {
                    observed.fetch_add(1, Ordering::SeqCst);
                },
            )?;
            Ok("must not return a late success".into())
        },
    )
    .unwrap_err();
    let held = BUSY.load(Ordering::Acquire);
    let retry = request(
        vec![255; 4],
        1,
        1,
        Control::new(Duration::from_secs(1)),
        &BUSY,
        |_, _, _, _| panic!("Unknown native state must not admit another worker"),
    );
    let _ = release.send(());
    wait_idle(&BUSY);
    assert!(error.contains("status-query failure"), "{error}");
    assert!(retry.unwrap_err().contains("still finishing"));
    assert!(
        held,
        "A status-query failure admitted retries without terminal-state evidence"
    );
    assert_eq!(cancelled.load(Ordering::SeqCst), 1);
}

#[test]
fn timeout_returns_even_if_backend_ignores_stop_and_blocks_retries_until_cleanup() {
    static BUSY: AtomicBool = AtomicBool::new(false);
    let (release, blocked) = mpsc::channel();
    let start = Instant::now();
    let error = request(
        vec![255; 4],
        1,
        1,
        Control::new(Duration::from_millis(40)),
        &BUSY,
        move |_, _, _, _| {
            blocked.recv().unwrap();
            Ok("late result".into())
        },
    )
    .unwrap_err();
    assert!(error.contains("timed out"));
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(BUSY.load(Ordering::Acquire));
    let second = request(
        vec![255; 4],
        1,
        1,
        Control::new(Duration::from_secs(1)),
        &BUSY,
        |_, _, _, _| panic!("second native worker must not start"),
    );
    assert!(second.unwrap_err().contains("still finishing"));
    release.send(()).unwrap();
    wait_idle(&BUSY);
    assert_eq!(
        request(
            vec![255; 4],
            1,
            1,
            Control::new(Duration::from_secs(1)),
            &BUSY,
            |_, _, _, _| Ok("retry works".into())
        )
        .unwrap(),
        "retry works"
    );
}

#[test]
fn cancellation_returns_before_uncooperative_backend_and_releases_after_completion() {
    static BUSY: AtomicBool = AtomicBool::new(false);
    let control = Control::new(Duration::from_secs(5));
    let cancel = control.clone();
    let (release, blocked) = mpsc::channel();
    let trigger = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        cancel.request(Stop::Cancelled);
    });
    let start = Instant::now();
    let error = request(vec![255; 4], 1, 1, control, &BUSY, move |_, _, _, _| {
        blocked.recv().unwrap();
        Ok("late".into())
    })
    .unwrap_err();
    assert!(error.contains("cancelled"));
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(BUSY.load(Ordering::Acquire));
    release.send(()).unwrap();
    trigger.join().unwrap();
    wait_idle(&BUSY);
}

#[test]
fn late_success_cannot_escape_cancellation() {
    static BUSY: AtomicBool = AtomicBool::new(false);
    let control = Control::new(Duration::from_secs(1));
    let error = request(vec![255; 4], 1, 1, control, &BUSY, |_, _, _, control| {
        control.request(Stop::Cancelled);
        Ok("must not publish".into())
    })
    .unwrap_err();
    assert!(error.contains("cancelled"));
    wait_idle(&BUSY);
}

#[test]
fn stopped_or_invalid_input_never_starts_native_work() {
    static BUSY: AtomicBool = AtomicBool::new(false);
    let control = Control::new(Duration::from_secs(1));
    control.request(Stop::Cancelled);
    assert!(
        request(vec![255; 4], 1, 1, control, &BUSY, |_, _, _, _| panic!(
            "cancelled work started"
        ))
        .unwrap_err()
        .contains("cancelled")
    );
    assert!(request(
        vec![],
        1,
        1,
        Control::new(Duration::from_secs(1)),
        &BUSY,
        |_, _, _, _| panic!("invalid work started")
    )
    .is_err());
    assert!(!BUSY.load(Ordering::Acquire));
    assert!(extract_text(&[], 1, 1).is_err());
    assert!(extract_text_owned(vec![0; 4], 0, 1).is_err());
}

#[test]
fn successful_worker_preserves_unicode_and_releases_flight() {
    static BUSY: AtomicBool = AtomicBool::new(false);
    let text = request(
        vec![255; 4],
        1,
        1,
        Control::new(Duration::from_secs(1)),
        &BUSY,
        |_, _, _, _| Ok("café 世界".into()),
    )
    .unwrap();
    assert_eq!(text, "café 世界");
    assert!(!BUSY.load(Ordering::Acquire));
}

#[test]
fn panic_disconnect_releases_flight() {
    static BUSY: AtomicBool = AtomicBool::new(false);
    assert!(request(
        vec![255; 4],
        1,
        1,
        Control::new(Duration::from_secs(1)),
        &BUSY,
        |_, _, _, _| panic!("synthetic native panic")
    )
    .unwrap_err()
    .contains("unexpectedly"));
    wait_idle(&BUSY);
}

#[test]
fn text_decoding_handles_unicode_whitespace_and_limits_before_allocating_output() {
    assert_eq!(
        text_result(&"\n café 世界 \r\n".encode_utf16().collect::<Vec<_>>()).unwrap(),
        "café 世界"
    );
    assert!(text_result(&" \n".encode_utf16().collect::<Vec<_>>())
        .unwrap_err()
        .contains("No text"));
    assert!(text_result(&vec![0x754c; OUTPUT_LIMIT / 3 + 1])
        .unwrap_err()
        .contains("size limit"));
    assert_eq!(text_result(&[0xd800]).unwrap(), "�");
}

fn pixels(width: u32, height: u32, colored: bool) -> Vec<u8> {
    let mut data = vec![255; width as usize * height as usize * 4];
    let font = fontdue::Font::from_bytes(
        include_bytes!("../../assets/font.ttf") as &[u8],
        fontdue::FontSettings::default(),
    )
    .unwrap();
    let lines = [
        "SNIPPING TOOL OCR TEST 12345",
        "The quick brown fox jumps over the lazy dog.",
        "Generated text only. No desktop capture.",
    ];
    let rows = if width > 3000 { 36 } else { 3 };
    let columns = if width > 3000 { 3 } else { 1 };
    for line in 0..rows {
        for column in 0..columns {
            let mut x = 30.0_f32 + column as f32 * 1100.0;
            let color = if colored {
                [[255, 0, 0], [0, 102, 255], [0, 192, 0]][line % 3]
            } else {
                [0, 0, 0]
            };
            for ch in lines[line % 3].chars() {
                let (m, mask) = font.rasterize(ch, 36.0);
                let left = x.round() as i32 + m.xmin;
                let top = 60 + line as i32 * 56 - m.height as i32 - m.ymin;
                for row in 0..m.height {
                    for col in 0..m.width {
                        let px = left + col as i32;
                        let py = top + row as i32;
                        if px >= 0 && py >= 0 && px < width as i32 && py < height as i32 {
                            let at = (py as usize * width as usize + px as usize) * 4;
                            let coverage = mask[row * m.width + col] as u32;
                            for channel in 0..3 {
                                data[at + channel] = ((color[channel] as u32 * coverage
                                    + 255 * (255 - coverage)
                                    + 127)
                                    / 255)
                                    as u8;
                            }
                        }
                    }
                }
                x += m.advance_width;
            }
        }
    }
    data
}

#[test]
fn real_native_fixture() {
    let Ok(mode) = std::env::var("SNIPPING_TOOL_UL_REAL_OCR_CASE") else {
        return;
    };
    match mode.as_str() {
        "repeat" | "colored" => {
            for i in 0..12 {
                let text =
                    extract_text_owned(pixels(1920, 1080, mode == "colored"), 1920, 1080).unwrap();
                assert!(
                    text.contains("12345") && text.to_uppercase().contains("SNIPPING"),
                    "run {i}: {text}"
                );
                wait_idle(&ACTIVE);
            }
        }
        "cancel" | "timeout" | "shutdown" => {
            let control = Control::new(Duration::from_secs(10));
            let trigger_control = control.clone();
            let trigger_mode = mode.clone();
            let trigger = std::thread::spawn(move || {
                let started = Instant::now();
                while !trigger_control.recognition_started.load(Ordering::Acquire) {
                    assert!(started.elapsed() < Duration::from_secs(5));
                    std::thread::sleep(Duration::from_millis(1));
                }
                std::thread::sleep(Duration::from_millis(20));
                if trigger_mode == "shutdown" {
                    cancel_pending();
                } else {
                    trigger_control.request(if trigger_mode == "timeout" {
                        Stop::TimedOut
                    } else {
                        Stop::Cancelled
                    });
                }
                Instant::now()
            });
            let error = request(
                pixels(3840, 2160, false),
                3840,
                2160,
                control,
                &ACTIVE,
                native::recognize,
            )
            .unwrap_err();
            let signalled = trigger.join().unwrap();
            assert!(
                error.contains(if mode == "timeout" {
                    "timed out"
                } else {
                    "cancelled"
                }),
                "{error}"
            );
            assert!(signalled.elapsed() < Duration::from_secs(2));
            wait_idle(&ACTIVE);
            println!(
                "native {mode}: caller stopped and backend released in {:?}",
                signalled.elapsed()
            );
        }
        _ => panic!("unknown real case"),
    }
}
