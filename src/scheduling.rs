//! One wake-up decision shared by countdowns, capture delay, drawing and dialogs.
use std::time::Instant;
pub fn next_wake(deadlines: impl IntoIterator<Item = Option<Instant>>) -> Option<Instant> {
    deadlines.into_iter().flatten().min()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn idle_never_overwrites_a_countdown_or_an_earlier_frame() {
        let now = Instant::now();
        let tick = now + Duration::from_secs(1);
        let frame = now + Duration::from_millis(8);
        let dialog = now + Duration::from_millis(100);
        assert_eq!(next_wake([Some(tick), None, None, None, None]), Some(tick));
        assert_eq!(
            next_wake([Some(tick), None, None, Some(frame), Some(dialog)]),
            Some(frame)
        );
        assert_eq!(next_wake([None, None, None, None, None]), None);
    }
}
