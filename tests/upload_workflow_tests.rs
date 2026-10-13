//! No HTTP requests, clipboard access or windows: exercise production upload decisions.
use snipping_tool_ul::upload::{image_key, UploadDecision::*, UploadSession};
use std::time::{Duration, Instant};

#[test]
fn unchanged_upload_reuses_link_even_when_confirmation_is_enabled() {
    let key = image_key(&[10, 20, 30, 255], 1, 1, "");
    let mut session = UploadSession::default();
    assert_eq!(session.request(key, true, false), Confirm);
    // Repeated ordinary Upload clicks never count as confirmation.
    assert_eq!(session.request(key, true, false), Confirm);
    assert_eq!(session.request(key, true, true), Start);
    for _ in 0..20_000 {
        assert_eq!(session.request(key, true, true), Busy);
    }
    assert!(session.finish(key, Some("https://files.catbox.moe/test.png".into())));
    for _ in 0..20_000 {
        assert_eq!(
            session.request(key, true, false),
            Reuse("https://files.catbox.moe/test.png".into())
        );
    }
}

#[test]
fn edits_dimensions_or_destinations_change_the_upload_identity() {
    let key = image_key(&[10, 20, 30, 255, 40, 50, 60, 255], 2, 1, "");
    assert_ne!(
        key,
        image_key(&[10, 20, 31, 255, 40, 50, 60, 255], 2, 1, "")
    );
    assert_ne!(
        key,
        image_key(&[10, 20, 30, 255, 40, 50, 60, 255], 1, 2, "")
    );
    assert_ne!(
        key,
        image_key(
            &[10, 20, 30, 255, 40, 50, 60, 255],
            2,
            1,
            "synthetic-client"
        )
    );
}

#[test]
fn changed_image_and_cancel_cannot_inherit_confirmation() {
    let mut s = UploadSession::default();
    assert_eq!(s.request([1; 32], true, false), Confirm);
    assert_eq!(s.request([2; 32], true, true), Confirm);
    s.cancel();
    assert_eq!(s.request([2; 32], true, true), Confirm);
    assert_eq!(s.request([2; 32], true, true), Start);
}

#[test]
fn failed_upload_is_retryable_and_late_events_cannot_finish_another_upload() {
    let mut s = UploadSession::default();
    let now = Instant::now();
    assert_eq!(s.request_at([1; 32], false, false, now), Start);
    assert!(!s.finish([2; 32], Some("wrong".into())));
    assert_eq!(s.request([2; 32], false, false), Busy);
    assert!(s.finish([1; 32], None));
    assert_eq!(s.request([1; 32], true, false), Confirm);
    assert_eq!(
        s.request_at([1; 32], true, true, now + Duration::from_secs(5)),
        Start
    );
    assert!(s.finish([1; 32], Some("existing".into())));
    assert_eq!(
        s.request_at([2; 32], false, false, now + Duration::from_secs(10)),
        Start
    );
    assert!(!s.finish([1; 32], Some("late".into())));
    assert_eq!(s.request([2; 32], false, false), Busy);
}

#[test]
fn link_cache_keeps_only_eight_small_entries_and_supports_undo() {
    let mut s = UploadSession::default();
    let now = Instant::now();
    for i in 1..=9 {
        assert_eq!(
            s.request_at(
                [i; 32],
                false,
                false,
                now + Duration::from_secs(i as u64 * 5)
            ),
            Start
        );
        s.finish([i; 32], Some(format!("link-{i}")));
    }
    assert_eq!(s.request([8; 32], true, false), Reuse("link-8".into()));
    assert_eq!(s.request([1; 32], true, false), Confirm);
}

#[test]
fn cooldown_blocks_click_spam_and_allows_the_exact_five_second_boundary() {
    let now = Instant::now();
    let mut s = UploadSession::default();
    assert_eq!(s.request_at([1; 32], false, false, now), Start);
    assert!(s.finish([1; 32], Some("existing".into())));
    for _ in 0..20_000 {
        assert_eq!(
            s.request_at([2; 32], false, false, now),
            Cooldown(Duration::from_secs(5))
        );
    }
    assert_eq!(
        s.request_at([2; 32], false, false, now + Duration::from_millis(4999)),
        Cooldown(Duration::from_millis(1))
    );
    assert_eq!(
        s.request_at([1; 32], true, false, now + Duration::from_secs(4)),
        Reuse("existing".into())
    );
    assert_eq!(
        s.request_at([2; 32], false, false, now + Duration::from_secs(5)),
        Start
    );
}

#[test]
fn failed_attempts_count_and_confirmations_or_cancellations_do_not_reset_the_clock() {
    let now = Instant::now();
    let mut s = UploadSession::default();
    assert_eq!(s.request_at([1; 32], true, false, now), Confirm);
    s.cancel();
    assert_eq!(s.request_at([1; 32], true, true, now), Confirm);
    // Opening/cancelling the prompt never delayed this first attempt.
    assert_eq!(s.request_at([1; 32], true, true, now), Start);
    assert!(s.finish([1; 32], None));
    let later = now + Duration::from_secs(1);
    assert_eq!(s.request_at([1; 32], true, false, later), Confirm);
    assert_eq!(
        s.request_at([1; 32], true, true, later),
        Cooldown(Duration::from_secs(4))
    );
    // The confirmation remains valid while waiting; no automatic upload is queued.
    assert_eq!(
        s.request_at([1; 32], true, true, now + Duration::from_secs(5)),
        Start
    );
}

#[test]
fn cooldown_spans_images_and_destinations_without_an_hourly_or_daily_cap() {
    let now = Instant::now();
    let mut s = UploadSession::default();
    let key = image_key(&[1, 2, 3, 255], 1, 1, "");
    let other_host = image_key(&[1, 2, 3, 255], 1, 1, "synthetic-client");
    assert_eq!(s.request_at(key, false, false, now), Start);
    assert!(s.finish(key, None));
    assert_eq!(
        s.request_at(other_host, false, false, now + Duration::from_secs(4)),
        Cooldown(Duration::from_secs(1))
    );
    for i in 1..=20_000 {
        assert_eq!(
            s.request_at(other_host, false, false, now + Duration::from_secs(i * 5)),
            Start
        );
        assert!(s.finish(other_host, None));
    }
}
