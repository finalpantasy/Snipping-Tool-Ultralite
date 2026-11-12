//! CPU-only: building a pin's framed pixels on the UI thread. No window.
use std::{hint::black_box, time::Instant};
include!("../tests/support/pin_frame_before_20261111.rs");

fn main() {
    let (w, h) = (3840u32, 2160u32);
    let pixels: Vec<u8> = (0..w as usize * h as usize * 4)
        .map(|i| (i % 251) as u8)
        .collect();
    let accent = (12, 99, 201);
    assert_eq!(
        snipping_tool_ul::pin::framed_pixels(&pixels, w, h, accent),
        pin_frame_before(&pixels, w, h, accent)
    );
    let time = |name: &str, f: &dyn Fn() -> Vec<u8>| {
        for _ in 0..3 {
            black_box(f());
        }
        let mut samples: Vec<f64> = (0..15)
            .map(|_| {
                let t = Instant::now();
                black_box(f());
                t.elapsed().as_secs_f64() * 1000.
            })
            .collect();
        samples.sort_by(f64::total_cmp);
        println!("{name:<24} median {:>7.2} ms", samples[samples.len() / 2]);
    };
    time("4K pin frame, 0.3.27", &|| {
        pin_frame_before(black_box(&pixels), w, h, accent)
    });
    time("4K pin frame, row-wise", &|| {
        snipping_tool_ul::pin::framed_pixels(black_box(&pixels), w, h, accent)
    });
}
