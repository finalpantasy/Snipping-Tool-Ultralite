//! Click-to-run OCR using the built-in Windows engine, with no temporary files.
#[cfg(target_os = "windows")]
use std::sync::atomic::AtomicU8;
use std::sync::atomic::{AtomicBool, Ordering};

static SHUTDOWN: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Irreversibly cancel OCR for process shutdown before joining the caller.
pub fn cancel_pending() {
    SHUTDOWN.store(true, Ordering::Release);
}
pub fn shutdown_requested() -> bool {
    SHUTDOWN.load(Ordering::Acquire)
}

#[cfg(target_os = "windows")]
#[path = "ocr/native.rs"]
mod native;

#[cfg(target_os = "windows")]
const OCR_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
#[cfg(target_os = "windows")]
const OUTPUT_LIMIT: usize = 1024 * 1024;

#[cfg(target_os = "windows")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stop {
    Cancelled,
    TimedOut,
}
#[cfg(target_os = "windows")]
impl Stop {
    fn message(self) -> &'static str {
        match self {
            Self::Cancelled => "OCR cancelled during shutdown",
            Self::TimedOut => "OCR timed out; try a smaller region",
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct Control {
    started: std::time::Instant,
    timeout: std::time::Duration,
    abort: std::sync::Arc<AtomicU8>,
    failure: std::sync::Arc<std::sync::OnceLock<String>>,
    #[cfg(test)]
    recognition_started: std::sync::Arc<AtomicBool>,
}
#[cfg(target_os = "windows")]
impl Control {
    fn new(timeout: std::time::Duration) -> Self {
        Self {
            started: std::time::Instant::now(),
            timeout,
            abort: std::sync::Arc::new(AtomicU8::new(0)),
            failure: std::sync::Arc::new(std::sync::OnceLock::new()),
            #[cfg(test)]
            recognition_started: std::sync::Arc::new(AtomicBool::new(false)),
        }
    }
    fn stop(&self) -> Option<Stop> {
        if shutdown_requested() {
            return Some(Stop::Cancelled);
        }
        match self.abort.load(Ordering::Acquire) {
            1 => Some(Stop::Cancelled),
            2 => Some(Stop::TimedOut),
            _ if self.started.elapsed() >= self.timeout => Some(Stop::TimedOut),
            _ => None,
        }
    }
    fn request(&self, reason: Stop) {
        self.abort.store(
            if reason == Stop::Cancelled { 1 } else { 2 },
            Ordering::Release,
        );
    }
    fn check(&self) -> Result<(), String> {
        if let Some(error) = self.failure.get() {
            return Err(error.clone());
        }
        self.stop()
            .map_or(Ok(()), |stop| Err(stop.message().into()))
    }
}

#[cfg(target_os = "windows")]
struct Flight(&'static AtomicBool);
#[cfg(target_os = "windows")]
impl Drop for Flight {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Borrowed-input convenience API. The application uses the owned variant to
/// avoid copying its already isolated selection into the native worker.
#[cfg(target_os = "windows")]
pub fn extract_text(pixels: &[u8], width: u32, height: u32) -> Result<String, String> {
    let control = Control::new(OCR_TIMEOUT);
    control.check()?;
    crate::export::validate_image(pixels, width, height)?;
    request(
        pixels.to_vec(),
        width,
        height,
        control,
        &ACTIVE,
        native::recognize,
    )
}

/// Move an isolated RGBA selection into the one native OCR worker.
#[cfg(target_os = "windows")]
pub fn extract_text_owned(pixels: Vec<u8>, width: u32, height: u32) -> Result<String, String> {
    request(
        pixels,
        width,
        height,
        Control::new(OCR_TIMEOUT),
        &ACTIVE,
        native::recognize,
    )
}

#[cfg(target_os = "windows")]
fn request<F>(
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    control: Control,
    active: &'static AtomicBool,
    run: F,
) -> Result<String, String>
where
    F: FnOnce(&[u8], u32, u32, &Control) -> Result<String, String> + Send + 'static,
{
    control.check()?;
    crate::export::validate_image(&pixels, width, height)?;
    active
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "Previous OCR is still finishing; try again shortly".to_string())?;
    let flight = Flight(active);
    let (send, recv) = std::sync::mpsc::channel();
    let worker_control = control.clone();
    std::thread::Builder::new()
        .name("ocr-native".into())
        .spawn(move || {
            // Ownership remains here even if the caller stops waiting. A slow OS
            // activation/cancellation can never accumulate additional OCR workers.
            let result = run(&pixels, width, height, &worker_control);
            drop(pixels);
            drop(flight);
            let _ = send.send(result);
        })
        .map_err(|e| format!("Failed to start OCR: {e}"))?;
    loop {
        if let Some(error) = control.failure.get() {
            control.request(Stop::Cancelled);
            return Err(error.clone());
        }
        if let Some(reason) = control.stop() {
            control.request(reason);
            return Err(reason.message().into());
        }
        let remaining = control.timeout.saturating_sub(control.started.elapsed());
        match recv.recv_timeout(remaining.min(std::time::Duration::from_millis(5))) {
            Ok(result) => {
                control.check()?;
                return result;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err("OCR stopped unexpectedly; try again".into())
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn text_result(text: &[u16]) -> Result<String, String> {
    let mut bytes = 0;
    for ch in char::decode_utf16(text.iter().copied()) {
        bytes += ch.unwrap_or(char::REPLACEMENT_CHARACTER).len_utf8();
        if bytes > OUTPUT_LIMIT {
            return Err("OCR output exceeded its size limit".into());
        }
    }
    let text = String::from_utf16_lossy(text).trim().to_owned();
    if text.is_empty() {
        Err("No text found in the selected region".into())
    } else {
        Ok(text)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn extract_text(_pixels: &[u8], _width: u32, _height: u32) -> Result<String, String> {
    Err("OCR is only available on Windows 10+".into())
}
#[cfg(not(target_os = "windows"))]
pub fn extract_text_owned(_pixels: Vec<u8>, _width: u32, _height: u32) -> Result<String, String> {
    Err("OCR is only available on Windows 10+".into())
}

#[cfg(all(test, target_os = "windows"))]
#[path = "../tests/support/ocr_process_cases.rs"]
mod tests;
