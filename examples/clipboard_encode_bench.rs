//! Clipboard image preparation timing: the former arboard pipeline against
//! `clipboard_image`. Pure in-memory work; never opens the clipboard or a window.
//!
//! Run with `cargo run --release --example clipboard_encode_bench`.
use image::ImageEncoder;
use snipping_tool_ul::clipboard_image::{dibv5_len, encode_png, write_dibv5, DIBV5_HEADER_LEN};
use std::{hint::black_box, time::Instant};

/// Staggered tiles of the committed app screenshots: real UI text, icons,
/// flat panels and gradients at full-screen sizes.
fn screenshot_frame(width: u32, height: u32) -> Vec<u8> {
    let root = env!("CARGO_MANIFEST_DIR");
    let tiles: Vec<_> = [
        "docs/images/editor-light.png",
        "docs/images/editor-dark.png",
        "docs/media/upload-light.png",
    ]
    .iter()
    .map(|path| image::open(format!("{root}/{path}")).unwrap().into_rgba8())
    .collect();
    let mut frame = image::RgbaImage::new(width, height);
    let (tw, th) = tiles[0].dimensions();
    for (row, y) in (0..height).step_by(th as usize).enumerate() {
        let shift = (row as u32 * tw / 3) % tw;
        for (col, x) in (0..width + tw).step_by(tw as usize).enumerate() {
            let tile = &tiles[(row + col) % tiles.len()];
            image::imageops::overlay(&mut frame, tile, i64::from(x) - i64::from(shift), y.into());
        }
    }
    frame.into_raw()
}

/// Smooth gradients with sensor-like noise, matching the earlier investigation.
fn photo_frame(width: u32, height: u32) -> Vec<u8> {
    let mut seed = 0x2545_f491_u32;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let n = seed & 15;
            pixels.extend_from_slice(&[
                ((x * 255 / width + n) & 255) as u8,
                ((y * 255 / height + n) & 255) as u8,
                (((x + y) * 128 / (width + height) + n * 2) & 255) as u8,
                255,
            ]);
        }
    }
    pixels
}

/// What arboard 3.6 did: default-compression PNG, an owned copy flipped in
/// place, a second copy into clipboard memory, then an in-place BGRA swizzle.
fn before(pixels: &[u8], width: u32, height: u32) -> (usize, f64, f64) {
    let start = Instant::now();
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(pixels, width, height, image::ExtendedColorType::Rgba8)
        .unwrap();
    let png_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let row = width as usize * 4;
    let mut flipped = pixels.to_vec();
    let mut tmp = vec![0; row];
    for a in 0..height as usize / 2 {
        let b = height as usize - a - 1;
        tmp.copy_from_slice(&flipped[a * row..][..row]);
        flipped.copy_within(b * row..(b + 1) * row, a * row);
        flipped[b * row..][..row].copy_from_slice(&tmp);
    }
    let mut global = vec![0u8; DIBV5_HEADER_LEN + flipped.len()];
    global[DIBV5_HEADER_LEN..].copy_from_slice(&flipped);
    for p in global[DIBV5_HEADER_LEN..].chunks_exact_mut(4) {
        p.swap(0, 2);
    }
    black_box((&png, &global));
    (png.len(), png_ms, start.elapsed().as_secs_f64() * 1000.)
}

fn after(pixels: &[u8], width: u32, height: u32) -> (usize, f64, f64) {
    let start = Instant::now();
    let png = encode_png(pixels, width, height).unwrap();
    let png_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let mut global = vec![0u8; dibv5_len(width, height).unwrap()];
    write_dibv5(&mut global, pixels, width, height);
    black_box((&png, &global));
    (png.len(), png_ms, start.elapsed().as_secs_f64() * 1000.)
}

fn median(run: impl Fn() -> (usize, f64, f64)) -> (usize, f64, f64) {
    black_box(run());
    let mut samples: Vec<_> = (0..7).map(|_| run()).collect();
    samples.sort_by(|a, b| (a.1 + a.2).total_cmp(&(b.1 + b.2)));
    samples[3]
}

fn main() {
    println!("content     size       path    png ms  dib ms  total ms  png KB");
    for (name, make) in [
        ("screenshot", screenshot_frame as fn(u32, u32) -> Vec<u8>),
        ("photo-like", photo_frame),
    ] {
        for (width, height) in [(1280, 720), (1920, 1080), (3840, 2160)] {
            let pixels = make(width, height);
            for (label, run) in [
                ("before", before as fn(&[u8], u32, u32) -> (usize, f64, f64)),
                ("after", after),
            ] {
                let (bytes, png, dib) = median(|| run(&pixels, width, height));
                println!(
                    "{name:<11} {:<10} {label:<7} {png:>6.1} {dib:>7.1} {:>9.1} {:>7}",
                    format!("{width}x{height}"),
                    png + dib,
                    bytes / 1024
                );
            }
        }
    }
}
