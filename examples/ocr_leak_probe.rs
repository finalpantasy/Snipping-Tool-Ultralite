#![cfg_attr(windows, windows_subsystem = "windows")]
//! Production owned-input OCR resource diagnostic, using authored pixels only.
//! No windows, desktop capture, clipboard, tray, hotkeys or network.
use std::io::{BufRead, Write};
use std::time::Instant;
#[path = "support/leak_counters.rs"]
mod counters;
fn emit(value: serde_json::Value) {
    println!("{value}");
    std::io::stdout().flush().unwrap();
}
fn fixture(width: u32, height: u32, mode: &str) -> Vec<u8> {
    if mode == "invalid" {
        return Vec::new();
    }
    let mut pixels = vec![255; width as usize * height as usize * 4];
    if mode == "blank" {
        return pixels;
    }
    let font = fontdue::Font::from_bytes(
        include_bytes!("../assets/font.ttf") as &[u8],
        fontdue::FontSettings::default(),
    )
    .unwrap();
    let size = if width < 1000 { 28.0 } else { 36.0 };
    let text_lines = [
        "SNIPPING TOOL OCR TEST 12345",
        "The quick brown fox jumps over the lazy dog.",
        "Generated text only. No desktop capture.",
    ];
    let rows = if mode == "dense" {
        (height - 30) / 56
    } else {
        3
    };
    let columns = if mode == "dense" {
        (width / 1100).max(1)
    } else {
        1
    };
    for line in 0..rows {
        for column in 0..columns {
            let text = text_lines[line as usize % text_lines.len()];
            let mut x = 30.0_f32 + column as f32 * 1100.0;
            let baseline = 60 + line as i32 * 56;
            for ch in text.chars() {
                let (m, mask) = font.rasterize(ch, size);
                let left = x.round() as i32 + m.xmin;
                let top = baseline - m.height as i32 - m.ymin;
                for row in 0..m.height {
                    for col in 0..m.width {
                        let px = left + col as i32;
                        let py = top + row as i32;
                        if px >= 0 && py >= 0 && px < width as i32 && py < height as i32 {
                            let at = (py as usize * width as usize + px as usize) * 4;
                            let ink = 255 - mask[row * m.width + col];
                            pixels[at..at + 3].fill(ink);
                        }
                    }
                }
                x += m.advance_width;
            }
        }
    }
    pixels
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(
        (4..=5).contains(&args.len()),
        "usage: ocr_leak_probe WIDTH HEIGHT MODE [ITERATIONS]"
    );
    let width: u32 = args[1].parse().unwrap();
    let height: u32 = args[2].parse().unwrap();
    assert!(width > 0 && height > 0 && width <= 3840 && height <= 2160);
    let mode = args[3].as_str();
    assert!(["text", "dense", "blank", "invalid", "control", "copy"].contains(&mode));
    let iterations: usize = args.get(4).map(|s| s.parse().unwrap()).unwrap_or(1);
    assert!((1..=2000).contains(&iterations));
    let mut lines = std::io::stdin().lock().lines();
    assert_eq!(lines.next().unwrap().unwrap(), "init");
    let fixture_mode = if mode == "control" || mode == "copy" {
        "dense"
    } else {
        mode
    };
    let mut pixels = fixture(width, height, fixture_mode);
    // Repeated OCR must measure the engine rather than repeatedly parsing fonts.
    // The held authored source is constant; each request still owns fresh pixels.
    let source = if iterations > 1 && mode != "control" {
        Some(pixels.clone())
    } else {
        None
    };
    emit(
        serde_json::json!({"event":"ready","api":"owned","width":width,"height":height,"mode":mode,"input_bytes":pixels.len(),"iterations":iterations,"cached_source_bytes":source.as_ref().map_or(0,Vec::len),"ocr_executed":mode!="control" && mode!="copy"}),
    );
    assert_eq!(lines.next().unwrap().unwrap(), "go");
    for iteration in 0..iterations {
        let start = Instant::now();
        let result = if mode == "control" || mode == "copy" {
            drop(pixels);
            Ok("SNIPPING 12345: fixture-only control".to_owned())
        } else {
            snipping_tool_ul::ocr::extract_text_owned(pixels, width, height)
        };
        let wall_ms = start.elapsed().as_secs_f64() * 1000.;
        let (ok, chars, expected_tokens, error) = match result {
            Ok(text) => (
                true,
                text.chars().count(),
                text.contains("12345") && text.to_uppercase().contains("SNIPPING"),
                None,
            ),
            Err(error) => (false, 0, false, Some(error)),
        };
        let remaining_files = std::fs::read_dir(std::env::temp_dir()).unwrap().count();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let metrics = counters::metrics();
        emit(
            serde_json::json!({"metrics":metrics,"event":"done","iteration":iteration,"ok":ok,"chars":chars,"expected_tokens":expected_tokens,"error":error,"wall_ms":wall_ms,"remaining_files":remaining_files}),
        );
        loop {
            match lines.next().unwrap().unwrap().as_str() {
                "trim" => {
                    let before = counters::metrics();
                    let start = Instant::now();
                    counters::trim();
                    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.;
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    emit(
                        serde_json::json!({"event":"trim","before":before,"after":counters::metrics(),"elapsed_ms":elapsed_ms}),
                    );
                }
                "sample" => {
                    emit(serde_json::json!({"event":"sample","metrics":counters::metrics()}))
                }
                "quit" => {
                    assert_eq!(iteration + 1, iterations);
                    return;
                }
                "next" => {
                    assert!(iteration + 1 < iterations);
                    break;
                }
                other => panic!("unknown command {other}"),
            }
        }
        pixels = if let Some(source) = &source {
            source.clone()
        } else {
            fixture(width, height, fixture_mode)
        };
    }
}
