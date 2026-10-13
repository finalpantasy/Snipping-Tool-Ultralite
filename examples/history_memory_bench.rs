//! Generated PNGs and requested Rust heap bytes only; no GUI or user images.
use image::ImageEncoder;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};
struct MeasuredAllocator;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
fn add(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Ordering::SeqCst) + bytes;
    PEAK.fetch_max(live, Ordering::SeqCst);
}
unsafe impl GlobalAlloc for MeasuredAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            add(layout.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(layout);
        if !p.is_null() {
            add(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        System.dealloc(p, layout);
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = System.realloc(p, layout, size);
        if !next.is_null() {
            LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
            add(size);
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: MeasuredAllocator = MeasuredAllocator;

fn handle_count() -> u32 {
    #[cfg(target_os = "windows")]
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
    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let width: u32 = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(3840);
    let mode = args.get(2).map(String::as_str).unwrap_or("legacy");
    let height = width * 9 / 16;
    let path = std::path::Path::new("fixtures").join(format!("generated-{width}.png"));
    std::fs::create_dir_all("fixtures").unwrap();
    if !path.exists() {
        let mut pixels = vec![255; width as usize * height as usize * 4];
        for (i, p) in pixels.chunks_exact_mut(4).enumerate() {
            p[0] = (i % 251) as u8;
            p[1] = (i / width as usize % 241) as u8;
        }
        image::codecs::png::PngEncoder::new_with_quality(
            std::io::BufWriter::new(std::fs::File::create(&path).unwrap()),
            image::codecs::png::CompressionType::Fast,
            image::codecs::png::FilterType::Adaptive,
        )
        .write_image(&pixels, width, height, image::ExtendedColorType::Rgba8)
        .unwrap();
    }
    let base = LIVE.load(Ordering::SeqCst);
    PEAK.store(base, Ordering::SeqCst);
    if mode == "soak" {
        use snipping_tool_ul::{
            capture::{CapturedScreen, ScreenRect},
            config::Config,
            history_thumbnails::LatestTaskQueue,
            preview::EditorView,
            preview_render::PreviewRenderer,
            state::OverlayState,
        };
        let queue = LatestTaskQueue::default();
        let wait = || {
            let deadline = Instant::now() + std::time::Duration::from_secs(10);
            while !queue.is_idle() && Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            assert!(queue.is_idle());
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        let job = path.clone();
        queue.submit(move |cancel| {
            std::hint::black_box(snipping_tool_ul::history_thumbnails::decode_thumbnail(
                &job, 1., cancel,
            ));
        });
        wait();
        let handle_before = handle_count();
        let base = LIVE.load(Ordering::SeqCst);
        PEAK.store(base, Ordering::SeqCst);
        let mut plateau = Vec::new();
        for batch in 0..10 {
            for i in 0..50 {
                let path = path.clone();
                let flag = queue.submit(move |cancel| {
                    std::hint::black_box(snipping_tool_ul::history_thumbnails::decode_thumbnail(
                        &path, 1., cancel,
                    ));
                });
                if i != 49 {
                    flag.store(true, Ordering::Relaxed);
                }
            }
            wait();
            for _ in 0..20 {
                let snapshot = CapturedScreen {
                    pixels: vec![255; 640 * 480 * 4],
                    width: 640,
                    height: 480,
                    x_offset: -100,
                    y_offset: 0,
                    scale_factor: 1.,
                };
                let crop = snapshot
                    .into_region(ScreenRect {
                        x: -50,
                        y: 30,
                        width: 400,
                        height: 300,
                    })
                    .unwrap();
                let mut state = OverlayState::new(crop, &Config::default());
                state.preview_mode = true;
                let mut renderer = PreviewRenderer::default();
                let mut frame = tiny_skia::Pixmap::new(660, 390).unwrap();
                renderer.draw(&state, &mut frame, &EditorView::default(), 1.);
            }
            plateau.push((batch, LIVE.load(Ordering::SeqCst).saturating_sub(base)));
        }
        let handle_after = handle_count();
        println!("soak requests=500 capture_render_cycles=200 peak_rust_bytes={} heap_plateau={plateau:?} handles_before={handle_before} handles_after={handle_after}",PEAK.load(Ordering::SeqCst)-base);
        assert_eq!(handle_after, handle_before);
        assert!(plateau.last().unwrap().1 <= 8192);
        return;
    }
    let start = Instant::now();
    let (retained, width_out, height_out, keep): (_, _, _, Box<dyn std::any::Any>) =
        if mode == "copy" {
            let image = snipping_tool_ul::history_copy::decode(&path).unwrap();
            (
                image.as_raw().len(),
                image.width(),
                image.height(),
                Box::new(image),
            )
        } else if mode == "copy-legacy" {
            let image = image::open(&path).unwrap();
            let rgba = image.to_rgba8();
            (
                rgba.as_raw().len(),
                rgba.width(),
                rgba.height(),
                Box::new((image, rgba)),
            )
        } else if mode == "stream" {
            let thumbnail = snipping_tool_ul::history_thumbnails::decode_thumbnail(
                &path,
                1.,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .unwrap();
            (
                thumbnail.pixels.len(),
                thumbnail.width,
                thumbnail.height,
                Box::new(thumbnail),
            )
        } else {
            let image = image::open(&path).unwrap();
            let thumbnail = image.thumbnail(120, 80).to_rgba8();
            (
                thumbnail.as_raw().len(),
                thumbnail.width(),
                thumbnail.height(),
                Box::new((image, thumbnail)),
            )
        };
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    let peak = PEAK.load(Ordering::SeqCst) - base;
    drop(keep);
    let remaining = LIVE.load(Ordering::SeqCst).saturating_sub(base);
    println!("{mode} {width}x{height} elapsed_ms={ms:.3} peak_rust_bytes={peak} thumbnail_bytes={retained} output={width_out}x{height_out} remaining_rust_bytes={remaining}");
    if mode == "stream" {
        let thumb = snipping_tool_ul::history_thumbnails::decode_thumbnail(
            &path,
            2.,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        image::save_buffer(
            format!("fixtures/thumbnail-{width}.png"),
            &thumb.pixels,
            thumb.width,
            thumb.height,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}
