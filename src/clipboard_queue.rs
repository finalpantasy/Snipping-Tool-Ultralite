//! Serializes clipboard writes and discards delayed work superseded by a newer request.
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
static GENERATION: AtomicU64 = AtomicU64::new(0);
static WRITER: Mutex<()> = Mutex::new(());
pub fn reserve() -> u64 {
    GENERATION.fetch_add(1, Ordering::SeqCst).wrapping_add(1)
}
pub fn run(ticket: u64, write: impl FnOnce() -> Result<(), String>) -> Result<bool, String> {
    let _guard = WRITER
        .lock()
        .map_err(|_| "Clipboard writer unavailable".to_owned())?;
    if GENERATION.load(Ordering::SeqCst) != ticket {
        return Ok(false);
    }
    write()?;
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
    }
}
