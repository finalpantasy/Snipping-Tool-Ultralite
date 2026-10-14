//! Snip latency components with generated pixels at three 4K monitors.
//! Memory DCs and DIB sections only: no screen DC, capture, window or clipboard.
//!
//! Run with `cargo run --release --example snip_latency_bench`.
#[cfg(target_os = "windows")]
fn main() {
    use snipping_tool_ul::capture::{BgraSnapshot, CapturedScreen, ScreenRect};
    use std::{hint::black_box, time::Instant};
    use windows::Win32::Graphics::Gdi::*;

    const WIDTH: u32 = 11520;
    const HEIGHT: u32 = 2160;
    const SELECTION: ScreenRect = ScreenRect {
        x: 1000,
        y: 600,
        width: 1280,
        height: 720,
    };

    fn ms(start: Instant) -> f64 {
        start.elapsed().as_secs_f64() * 1000.0
    }
    fn median(mut samples: Vec<f64>) -> (f64, f64) {
        samples.sort_by(f64::total_cmp);
        let p95 = samples[(samples.len() * 95 / 100).min(samples.len() - 1)];
        (samples[samples.len() / 2], p95)
    }
    fn info(width: u32, height: u32) -> BITMAPINFO {
        BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    /// A memory DC with a DIB section selected; stands in for the screen
    /// (capture source) or a window surface (paint target).
    struct Section {
        dc: HDC,
        bitmap: HBITMAP,
        old: HGDIOBJ,
        bits: *mut u8,
    }
    impl Section {
        fn new(width: u32, height: u32) -> Self {
            unsafe {
                let dc = CreateCompatibleDC(None);
                let mut bits = std::ptr::null_mut();
                let bitmap =
                    CreateDIBSection(dc, &info(width, height), DIB_RGB_COLORS, &mut bits, None, 0)
                        .unwrap();
                let old = SelectObject(dc, bitmap);
                Self {
                    dc,
                    bitmap,
                    old,
                    bits: bits.cast(),
                }
            }
        }
    }
    impl Drop for Section {
        fn drop(&mut self) {
            unsafe {
                SelectObject(self.dc, self.old);
                let _ = DeleteObject(self.bitmap);
                let _ = DeleteDC(self.dc);
            }
        }
    }

    let len = WIDTH as usize * HEIGHT as usize * 4;
    // Desktop-like content: flat panels with text-like noise.
    let source = Section::new(WIDTH, HEIGHT);
    unsafe {
        let pixels = std::slice::from_raw_parts_mut(source.bits, len);
        for (i, px) in pixels.chunks_exact_mut(4).enumerate() {
            let x = i % WIDTH as usize;
            let y = i / WIDTH as usize;
            let ink = (x * 7 + y * 13).is_multiple_of(29) as u8 * 180;
            px.copy_from_slice(&[240 - ink, 238 - ink, 235 - ink, 0]);
        }
    }
    let target = Section::new(WIDTH, HEIGHT);
    let runs = 15;

    // Before: Vec zero-fill, BitBlt into a 32-bit DDB, GetDIBits, RGBA swap,
    // the picker's swap back to BGRA, then StretchDIBits for the first paint.
    let mut before = [vec![], vec![], vec![], vec![], vec![], vec![]];
    let mut release_before = [vec![], vec![], vec![]];
    for _ in 0..runs {
        unsafe {
            let start = Instant::now();
            let mut pixels = Vec::new();
            pixels.try_reserve_exact(len).unwrap();
            pixels.resize(len, 0u8);
            before[0].push(ms(start));

            let start = Instant::now();
            let ddb = CreateBitmap(WIDTH as i32, HEIGHT as i32, 1, 32, None);
            let dc = CreateCompatibleDC(None);
            let old = SelectObject(dc, ddb);
            BitBlt(
                dc,
                0,
                0,
                WIDTH as i32,
                HEIGHT as i32,
                source.dc,
                0,
                0,
                SRCCOPY,
            )
            .unwrap();
            SelectObject(dc, old);
            before[1].push(ms(start));

            let start = Instant::now();
            let mut bmi = info(WIDTH, HEIGHT);
            let lines = GetDIBits(
                dc,
                ddb,
                0,
                HEIGHT,
                Some(pixels.as_mut_ptr().cast()),
                &mut bmi,
                DIB_RGB_COLORS,
            );
            assert_eq!(lines, HEIGHT as i32);
            let _ = DeleteObject(ddb);
            let _ = DeleteDC(dc);
            before[2].push(ms(start));

            let start = Instant::now();
            for chunk in pixels.chunks_exact_mut(4) {
                chunk.swap(0, 2);
                chunk[3] = 255;
            }
            before[3].push(ms(start));

            let start = Instant::now();
            for chunk in pixels.chunks_exact_mut(4) {
                chunk.swap(0, 2);
            }
            before[4].push(ms(start));

            let start = Instant::now();
            let header = info(WIDTH, HEIGHT);
            StretchDIBits(
                target.dc,
                0,
                0,
                WIDTH as i32,
                HEIGHT as i32,
                0,
                0,
                WIDTH as i32,
                HEIGHT as i32,
                Some(pixels.as_ptr().cast()),
                &header,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            let _ = GdiFlush();
            before[5].push(ms(start));

            // Release: swap back to RGBA, crop (frees the desktop), clipboard prep.
            let start = Instant::now();
            for chunk in pixels.chunks_exact_mut(4) {
                chunk.swap(0, 2);
            }
            release_before[0].push(ms(start));
            let screen = CapturedScreen {
                pixels,
                width: WIDTH,
                height: HEIGHT,
                x_offset: 0,
                y_offset: 0,
                scale_factor: 1.5,
            };
            let start = Instant::now();
            let snip = screen.into_region(SELECTION).unwrap();
            release_before[1].push(ms(start));
            let start = Instant::now();
            let png = snipping_tool_ul::clipboard_image::encode_png(
                &snip.pixels,
                snip.width,
                snip.height,
            )
            .unwrap();
            let mut dib = vec![0; snipping_tool_ul::clipboard_image::dibv5_len(1280, 720).unwrap()];
            snipping_tool_ul::clipboard_image::write_dibv5(&mut dib, &snip.pixels, 1280, 720);
            release_before[2].push(ms(start));
            black_box((png, dib));
        }
    }

    // After: BitBlt straight into a DIB section, BitBlt it for the first paint,
    // and convert only the selected rectangle to RGBA.
    let mut after = [vec![], vec![]];
    let mut release_after = [vec![], vec![], vec![]];
    for _ in 0..runs {
        unsafe {
            let start = Instant::now();
            let snapshot = BgraSnapshot::from_dc(
                source.dc,
                ScreenRect {
                    x: 0,
                    y: 0,
                    width: WIDTH,
                    height: HEIGHT,
                },
                1.5,
            )
            .unwrap();
            after[0].push(ms(start));

            let start = Instant::now();
            snapshot.paint(target.dc).unwrap();
            let _ = GdiFlush();
            after[1].push(ms(start));

            let start = Instant::now();
            let snip = snapshot.region_rgba(SELECTION).unwrap();
            release_after[0].push(ms(start));
            let start = Instant::now();
            let png = snipping_tool_ul::clipboard_image::encode_png(
                &snip.pixels,
                snip.width,
                snip.height,
            )
            .unwrap();
            let mut dib = vec![0; snipping_tool_ul::clipboard_image::dibv5_len(1280, 720).unwrap()];
            snipping_tool_ul::clipboard_image::write_dibv5(&mut dib, &snip.pixels, 1280, 720);
            release_after[1].push(ms(start));
            black_box((png, dib));
            let start = Instant::now();
            drop(snapshot);
            release_after[2].push(ms(start));
        }
    }

    println!(
        "Synthetic {WIDTH}x{HEIGHT} desktop ({} MB), {runs} runs, median / p95 ms",
        len >> 20
    );
    println!("start, before:");
    for (label, samples) in [
        "  zero-filled Vec",
        "  BitBlt into DDB (copy cost only)",
        "  GetDIBits into Vec",
        "  BGRA to RGBA pass",
        "  picker swap back to BGRA",
        "  StretchDIBits first paint",
    ]
    .iter()
    .zip(before)
    {
        let (m, p) = median(samples);
        println!("{label:<42} {m:>7.2} {p:>7.2}");
    }
    println!("start, after:");
    for (label, samples) in [
        "  BitBlt into DIB section (copy cost only)",
        "  BitBlt first paint",
    ]
    .iter()
    .zip(after)
    {
        let (m, p) = median(samples);
        println!("{label:<42} {m:>7.2} {p:>7.2}");
    }
    println!("release, before:");
    for (label, samples) in [
        "  picker swap back to RGBA",
        "  crop + free desktop",
        "  PNG + DIBV5 1280x720",
    ]
    .iter()
    .zip(release_before)
    {
        let (m, p) = median(samples);
        println!("{label:<42} {m:>7.2} {p:>7.2}");
    }
    println!("release, after:");
    for (label, samples) in [
        "  crop + RGBA convert",
        "  PNG + DIBV5 1280x720",
        "  free desktop (after clipboard)",
    ]
    .iter()
    .zip(release_after)
    {
        let (m, p) = median(samples);
        println!("{label:<42} {m:>7.2} {p:>7.2}");
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {}
