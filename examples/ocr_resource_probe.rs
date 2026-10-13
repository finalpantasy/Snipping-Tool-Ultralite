//! Headless production OCR measurement worker. Generated pixels only.
//! No windows, desktop capture, clipboard, hotkeys, tray or network.
use std::io::{BufRead, Write};
use std::time::Instant;

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
    assert_eq!(args.len(), 4, "usage: ocr_resource_probe WIDTH HEIGHT MODE");
    let width: u32 = args[1].parse().unwrap();
    let height: u32 = args[2].parse().unwrap();
    assert!(width > 0 && height > 0 && width <= 3840 && height <= 2160);
    let mode = args[3].as_str();
    assert!(["text", "dense", "blank", "invalid", "cancel"].contains(&mode));
    let mut lines = std::io::stdin().lock().lines();
    // Parent assigns the outer accounting job before allowing any work.
    assert_eq!(lines.next().unwrap().unwrap(), "init");
    let pixels = fixture(width, height, mode);
    emit(
        serde_json::json!({"event":"ready", "width":width,"height":height,"mode":mode,"input_bytes":pixels.len()}),
    );
    assert_eq!(lines.next().unwrap().unwrap(), "go");
    let start = Instant::now();
    let (send, recv) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = snipping_tool_ul::ocr::extract_text(&pixels, width, height);
        // Keep input resident until parent collects the post-call baseline.
        send.send((result, pixels)).unwrap();
    });
    // Reading commands on the main thread permits cancellation only after
    // the observer actually sees the PowerShell child alive.
    // The input reader already borrows stdin; poll completion with a second
    // thread only in cancellation mode, where one parent command is expected.
    if mode == "cancel" {
        assert_eq!(lines.next().unwrap().unwrap(), "cancel");
        snipping_tool_ul::ocr::cancel_pending();
    }
    let (result, pixels) = recv.recv().unwrap();
    let wall_ms = start.elapsed().as_secs_f64() * 1000.0;
    let (ok, chars, tokens, error) = match result {
        Ok(text) => (
            true,
            text.chars().count(),
            text.contains("12345") && text.to_uppercase().contains("SNIPPING"),
            None,
        ),
        Err(error) => (false, 0, false, Some(error)),
    };
    let remaining_files = std::fs::read_dir(std::env::temp_dir()).unwrap().count();
    emit(
        serde_json::json!({"event":"done","ok":ok,"chars":chars,"expected_tokens":tokens,"error":error,"wall_ms":wall_ms,"remaining_files":remaining_files}),
    );
    assert_eq!(lines.next().unwrap().unwrap(), "quit");
    std::hint::black_box(pixels);
}
