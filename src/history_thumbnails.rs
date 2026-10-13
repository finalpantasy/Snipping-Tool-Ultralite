//! Headless History image preparation and its worker lifetime.
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock, Mutex,
    },
};

pub struct Thumbnail {
    pub path: PathBuf,
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub scale: f32,
}
pub struct ThumbnailLoad {
    pub stored_entries: usize,
    pub thumbnails: Vec<Thumbnail>,
}

pub fn decode_thumbnail(path: &Path, scale: f32, cancel: &AtomicBool) -> Option<Thumbnail> {
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    if !scale.is_finite() || scale <= 0. {
        return None;
    }
    let target = (
        (120. * scale).round().clamp(1., 1024.) as u32,
        (80. * scale).round().clamp(1., 1024.) as u32,
    );
    let file = std::fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > 256 * 1024 * 1024 {
        return None;
    }
    let mut input = std::io::BufReader::new(file);
    use std::io::BufRead;
    if input.fill_buf().ok()?.starts_with(b"\x89PNG\r\n\x1a\n") {
        streamed_png(input, path, scale, target, cancel)
    } else {
        drop(input);
        fallback_thumbnail(path, scale, target, cancel)
    }
}

const MAX_DIMENSION: u32 = 65_536;
const MAX_PIXELS: u64 = 128 * 1024 * 1024;
const FALLBACK_BYTES: u64 = 128 * 1024 * 1024;
fn fit(width: u32, height: u32, target: (u32, u32)) -> (u32, u32) {
    let ratio = (target.0 as f64 / width as f64).min(target.1 as f64 / height as f64);
    (
        (width as f64 * ratio).round().max(1.) as u32,
        (height as f64 * ratio).round().max(1.) as u32,
    )
}
fn valid_dimensions(width: u32, height: u32) -> bool {
    width > 0
        && height > 0
        && width <= MAX_DIMENSION
        && height <= MAX_DIMENSION
        && width as u64 * height as u64 <= MAX_PIXELS
}
fn fallback_thumbnail(
    path: &Path,
    scale: f32,
    target: (u32, u32),
    cancel: &AtomicBool,
) -> Option<Thumbnail> {
    use image::ImageDecoder;
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    let mut reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(FALLBACK_BYTES);
    reader.limits(limits);
    let decoder = reader.into_decoder().ok()?;
    let (width, height) = decoder.dimensions();
    if !valid_dimensions(width, height) || decoder.total_bytes() > FALLBACK_BYTES {
        return None;
    }
    let image = image::DynamicImage::from_decoder(decoder).ok()?;
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    let output = image.thumbnail(target.0, target.1).to_rgba8();
    Some(Thumbnail {
        path: path.to_owned(),
        width: output.width(),
        height: output.height(),
        pixels: output.into_raw(),
        scale,
    })
}
fn streamed_png<R: std::io::BufRead + std::io::Seek>(
    input: R,
    path: &Path,
    scale: f32,
    target: (u32, u32),
    cancel: &AtomicBool,
) -> Option<Thumbnail> {
    let mut decoder = png::Decoder::new(input);
    decoder.set_limits(png::Limits {
        bytes: 16 * 1024 * 1024,
    });
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().ok()?;
    let info = reader.info();
    let (width, height) = (info.width, info.height);
    if !valid_dimensions(width, height) {
        return None;
    }
    let (out_w, out_h) = fit(width, height, target);
    if info.interlaced
        || info.bit_depth == png::BitDepth::Sixteen
        || info.animation_control.is_some()
        || out_w > width
        || out_h > height
    {
        drop(reader);
        return fallback_thumbnail(path, scale, target, cancel);
    }
    let (color, depth) = reader.output_color_type();
    if depth != png::BitDepth::Eight {
        return None;
    }
    let channels = match color {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return None,
    };
    let mut row = vec![0u8; width as usize * 4];
    let mut pixels = vec![0u8; out_w as usize * out_h as usize * 4];
    let x_ratio = width as f32 / out_w as f32;
    let y_ratio = height as f32 / out_h as f32;
    let spans: Vec<_> = (0..out_w)
        .map(|x| {
            let left = x as f32 * x_ratio;
            (
                (left.ceil() as u32).min(width - 1),
                ((left + x_ratio).ceil() as u32).min(width),
            )
        })
        .collect();
    let mut decoded_y: i64 = -1;
    let mut sums = vec![[0u64; 4]; out_w as usize];
    for out_y in 0..out_h {
        sums.fill([0; 4]);
        let bottom_f = out_y as f32 * y_ratio;
        let bottom = (bottom_f.ceil() as u32).min(height - 1);
        let top = ((bottom_f + y_ratio).ceil() as u32).clamp(bottom, height);
        for y in bottom..top {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            // Keep the last normalized row: float rounding can make adjacent
            // box-filter bands share a boundary row, as in image::thumbnail.
            if decoded_y > i64::from(y) {
                return None;
            }
            while decoded_y < i64::from(y) {
                if cancel.load(Ordering::Relaxed) {
                    return None;
                }
                let source = reader.next_row().ok()??;
                if source.data().len() != width as usize * channels {
                    return None;
                }
                if channels == 4 {
                    row.copy_from_slice(source.data());
                } else {
                    for (src, dst) in source
                        .data()
                        .chunks_exact(channels)
                        .zip(row.chunks_exact_mut(4))
                    {
                        let rgb = if channels == 3 {
                            [src[0], src[1], src[2]]
                        } else {
                            [src[0]; 3]
                        };
                        dst[..3].copy_from_slice(&rgb);
                        dst[3] = if channels == 2 { src[1] } else { 255 };
                    }
                }
                decoded_y += 1;
            }
            for (sum, &(left, right)) in sums.iter_mut().zip(&spans) {
                for pixel in row[left as usize * 4..right as usize * 4].chunks_exact(4) {
                    for c in 0..4 {
                        sum[c] += pixel[c] as u64;
                    }
                }
            }
        }
        for (x, (sum, &(left, right))) in sums.iter().zip(&spans).enumerate() {
            let n = u64::from(right - left) * u64::from(top - bottom);
            if n == 0 {
                return None;
            }
            let offset = (out_y as usize * out_w as usize + x) * 4;
            for c in 0..4 {
                pixels[offset + c] = ((sum[c] + n / 2) / n) as u8;
            }
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    reader.finish().ok()?;
    Some(Thumbnail {
        path: path.to_owned(),
        pixels,
        width: out_w,
        height: out_h,
        scale,
    })
}
pub fn load_thumbnails(scale: f32, cancel: &AtomicBool) -> ThumbnailLoad {
    let entries = crate::history::list_history();
    let stored_entries = entries.len();
    let thumbnails = entries
        .into_iter()
        .take_while(|_| !cancel.load(Ordering::Relaxed))
        .filter_map(|path| decode_thumbnail(&path, scale, cancel))
        .collect();
    ThumbnailLoad {
        stored_entries,
        thumbnails,
    }
}

struct Task {
    cancel: Arc<AtomicBool>,
    work: Box<dyn FnOnce(&AtomicBool) + Send>,
}
#[derive(Default)]
struct QueueState {
    running: bool,
    current: Option<Arc<AtomicBool>>,
    pending: Option<Task>,
}
#[derive(Default)]
pub struct LatestTaskQueue {
    state: Arc<Mutex<QueueState>>,
    tracked: bool,
}
impl LatestTaskQueue {
    /// Copies are accepted writes: Quit must include their single queue worker.
    /// Tracking is fixed for the queue's lifetime, including replacement jobs.
    pub(crate) fn tracked() -> Self {
        Self {
            tracked: true,
            ..Self::default()
        }
    }
    pub fn submit(&self, work: impl FnOnce(&AtomicBool) + Send + 'static) -> Arc<AtomicBool> {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(current) = &state.current {
            current.store(true, Ordering::Relaxed);
        }
        let replaced = state.pending.replace(Task {
            cancel: cancel.clone(),
            work: Box::new(work),
        });
        if let Some(old) = &replaced {
            old.cancel.store(true, Ordering::Relaxed);
        }
        let start = !state.running;
        state.running = true;
        drop(state);
        drop(replaced);
        if start {
            let shared = self.state.clone();
            let failed = shared.clone();
            let spawned = std::thread::Builder::new()
                .name("UL history".into())
                .spawn(move || loop {
                    let mut state = shared.lock().unwrap_or_else(|e| e.into_inner());
                    let Some(task) = state.pending.take() else {
                        state.current = None;
                        state.running = false;
                        return;
                    };
                    state.current = Some(task.cancel.clone());
                    drop(state);
                    if !task.cancel.load(Ordering::Relaxed)
                        && std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            (task.work)(&task.cancel)
                        }))
                        .is_err()
                    {
                        tracing::warn!("History preparation stopped unexpectedly");
                    }
                });
            match spawned {
                Ok(handle) => {
                    if self.tracked {
                        crate::background::track(handle);
                    }
                }
                Err(error) => {
                    let mut state = failed.lock().unwrap_or_else(|e| e.into_inner());
                    state.running = false;
                    state.current = None;
                    let pending = state.pending.take();
                    if let Some(task) = &pending {
                        task.cancel.store(true, Ordering::Relaxed);
                    }
                    drop(state);
                    drop(pending);
                    tracing::error!("History worker unavailable: {error}");
                }
            }
        }
        cancel
    }
    pub fn is_idle(&self) -> bool {
        !self.state.lock().unwrap_or_else(|e| e.into_inner()).running
    }
}
pub static HISTORY_LOADS: LazyLock<LatestTaskQueue> = LazyLock::new(LatestTaskQueue::default);

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };

    fn encoded_png(width: u32, height: u32, color: image::ExtendedColorType) -> Vec<u8> {
        let channels = color.channel_count() as usize;
        let pixels: Vec<u8> = (0..width as usize * height as usize * channels)
            .map(|i| ((i.wrapping_mul(19) ^ (i / 11).wrapping_mul(23)) % 256) as u8)
            .collect();
        let mut encoded = Vec::new();
        image::codecs::png::PngEncoder::new(&mut encoded)
            .write_image(&pixels, width, height, color)
            .unwrap();
        encoded
    }
    #[test]
    fn streamed_thumbnails_match_full_decode_across_formats_and_ratios() {
        let cancel = AtomicBool::new(false);
        for color in [
            image::ExtendedColorType::Rgba8,
            image::ExtendedColorType::Rgb8,
            image::ExtendedColorType::L8,
            image::ExtendedColorType::La8,
        ] {
            for (w, h) in [(317, 233), (192, 108), (129, 83), (1, 500), (500, 1)] {
                let png = encoded_png(w, h, color);
                for target in [(120, 80), (60, 40), (1, 1)] {
                    let expected = image::load_from_memory(&png)
                        .unwrap()
                        .thumbnail(target.0, target.1)
                        .to_rgba8();
                    let actual = streamed_png(
                        std::io::Cursor::new(&png),
                        Path::new("generated.png"),
                        1.,
                        target,
                        &cancel,
                    )
                    .unwrap();
                    assert_eq!((actual.width, actual.height), expected.dimensions());
                    assert_eq!(
                        actual.pixels,
                        expected.into_raw(),
                        "{color:?} {w}x{h} target={target:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn cancellation_stops_png_reading_before_the_whole_file() {
        use std::io::{Read, Seek, SeekFrom};
        use std::sync::atomic::AtomicUsize;
        struct CancelReader {
            input: std::io::Cursor<Vec<u8>>,
            flag: Arc<AtomicBool>,
            bytes: Arc<AtomicUsize>,
        }
        impl Read for CancelReader {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                let n = self.input.read(output)?;
                if self.bytes.fetch_add(n, Ordering::Relaxed) + n >= 8192 {
                    self.flag.store(true, Ordering::Relaxed);
                }
                Ok(n)
            }
        }
        impl Seek for CancelReader {
            fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
                self.input.seek(from)
            }
        }
        let png = encoded_png(1000, 1000, image::ExtendedColorType::Rgba8);
        let size = png.len();
        assert!(size > 8192);
        let flag = Arc::new(AtomicBool::new(false));
        let bytes = Arc::new(AtomicUsize::new(0));
        let reader = std::io::BufReader::with_capacity(
            512,
            CancelReader {
                input: std::io::Cursor::new(png),
                flag: flag.clone(),
                bytes: bytes.clone(),
            },
        );
        assert!(streamed_png(reader, Path::new("generated.png"), 1., (120, 80), &flag).is_none());
        assert!(flag.load(Ordering::Relaxed));
        assert!(bytes.load(Ordering::Relaxed) < size);
    }
    #[test]
    fn cancelled_and_replaced_jobs_release_their_payloads() {
        let queue = LatestTaskQueue::default();
        let (started, ready) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        queue.submit(move |_| {
            started.send(()).unwrap();
            wait.recv().unwrap();
        });
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        let mut weak = Vec::new();
        for _ in 0..400 {
            let payload = Arc::new(vec![0u8; 128 * 1024]);
            weak.push(Arc::downgrade(&payload));
            let flag = queue.submit(move |_| {
                std::hint::black_box(&payload);
            });
            flag.store(true, Ordering::Relaxed);
        }
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !queue.is_idle() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(queue.is_idle());
        assert!(weak.iter().all(|w| w.upgrade().is_none()));
        let state = queue.state.lock().unwrap();
        assert!(state.current.is_none() && state.pending.is_none());
    }
    #[test]
    fn worker_recovers_after_a_panicking_request() {
        let queue = LatestTaskQueue::default();
        let (ready, started) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        queue.submit(move |_| {
            ready.send(()).unwrap();
            wait.recv().unwrap();
            panic!("synthetic decoder panic");
        });
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        let (done, complete) = mpsc::channel();
        queue.submit(move |_| {
            done.send(()).unwrap();
        });
        release.send(()).unwrap();
        complete.recv_timeout(Duration::from_secs(2)).unwrap();
    }
    #[test]
    fn indexed_png_transparency_matches_legacy_thumbnail() {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 128, 80);
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::Two);
            encoder.set_palette(vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 220, 200, 80]);
            encoder.set_trns(vec![0, 64, 128, 255]);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&vec![0b00011011; 32 * 80]).unwrap();
        }
        let expected = image::load_from_memory(&bytes)
            .unwrap()
            .thumbnail(60, 40)
            .to_rgba8();
        let actual = streamed_png(
            std::io::Cursor::new(bytes),
            Path::new("generated.png"),
            1.,
            (60, 40),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(actual.pixels, expected.into_raw());
    }
    #[test]
    fn jpeg_and_small_or_sixteen_bit_png_keep_existing_scaling() {
        use std::sync::atomic::AtomicUsize;
        static ID: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ul-history-fallback-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let rgb = image::RgbImage::from_fn(64, 40, |x, y| {
            image::Rgb([(x * 3) as u8, (y * 5) as u8, 99])
        });
        let jpeg = dir.join("generated.jpg");
        rgb.save(&jpeg).unwrap();
        let small = dir.join("generated-small.png");
        rgb.save(&small).unwrap();
        let sixteen = dir.join("generated-16.png");
        image::ImageBuffer::from_fn(160, 120, |x, y| {
            image::Rgba([(x * 327) as u16, (y * 491) as u16, 50000u16, 60000u16])
        })
        .save(&sixteen)
        .unwrap();
        for path in [&jpeg, &small, &sixteen] {
            let expected = image::open(path).unwrap().thumbnail(120, 80).to_rgba8();
            let actual = decode_thumbnail(path, 1., &AtomicBool::new(false)).unwrap();
            assert_eq!((actual.width, actual.height), expected.dimensions());
            assert_eq!(actual.pixels, expected.into_raw());
        }
        assert!(decode_thumbnail(&small, f32::NAN, &AtomicBool::new(false)).is_none());
        assert!(decode_thumbnail(&small, 1., &AtomicBool::new(true)).is_none());
        for path in [&jpeg, &small, &sixteen] {
            std::fs::remove_file(path).unwrap();
        }
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn corrupt_or_truncated_png_and_oversized_dimensions_are_rejected() {
        let png = encoded_png(256, 256, image::ExtendedColorType::Rgba8);
        let cancel = AtomicBool::new(false);
        for end in [0, 8, 40, png.len() / 2] {
            assert!(streamed_png(
                std::io::Cursor::new(&png[..end]),
                Path::new("generated.png"),
                1.,
                (120, 80),
                &cancel
            )
            .is_none());
        }
        assert!(!valid_dimensions(u32::MAX, u32::MAX));
        assert!(!valid_dimensions(MAX_DIMENSION + 1, 1));
        assert!(!valid_dimensions(0, 1));
    }
    #[test]
    fn refreshes_coalesce_while_an_older_decoder_is_still_running() {
        let queue = LatestTaskQueue::default();
        let (started, start_rx) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        let first = queue.submit(move |_| {
            started.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        start_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let (result, results) = mpsc::channel();
        let mut flags = Vec::new();
        for i in 0..20 {
            let result = result.clone();
            flags.push(queue.submit(move |_| {
                result.send(i).unwrap();
            }));
        }
        drop(result);
        let ran_early = results.recv_timeout(Duration::from_millis(100)).is_ok();
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !queue.is_idle() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            !ran_early,
            "refresh started parallel decoding instead of coalescing"
        );
        assert!(first.load(Ordering::Relaxed));
        assert!(flags[..19].iter().all(|flag| flag.load(Ordering::Relaxed)));
        assert_eq!(results.try_iter().collect::<Vec<_>>(), vec![19]);
        assert!(queue.is_idle());
    }
}
