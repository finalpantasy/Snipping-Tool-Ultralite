//! Release timing of the system-font fallback that draws IME text.
//! Reads Windows fonts only; opens no window. `cargo run --release --example ime_font_bench`
use std::time::Instant;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.
}

fn main() {
    let latin = "Quarterly numbers, see row 42";
    for _ in 0..3 {
        snipping_tool_ul::tools::measure_text_width(latin, 24.);
    }
    let runs = 20_000;
    let t = Instant::now();
    for _ in 0..runs {
        std::hint::black_box(snipping_tool_ul::tools::measure_text_width(latin, 24.));
    }
    println!(
        "Latin width, per call        {:8.4} ms",
        ms(t) / runs as f64
    );

    let t = Instant::now();
    snipping_tool_ul::tools::measure_text_width("日", 24.);
    println!("First CJK character (load)   {:8.1} ms", ms(t));

    // Each character below sits in a new 128-code-point block.
    let mut worst: f64 = 0.;
    let mut total = 0.;
    let samples: Vec<char> = (0..40)
        .map(|i| char::from_u32(0x4E00 + i * 0x200).unwrap())
        .collect();
    let mut text = String::new();
    for ch in &samples {
        text.push(*ch);
        let t = Instant::now();
        snipping_tool_ul::tools::measure_text_width(&text, 24.);
        let took = ms(t);
        worst = worst.max(took);
        total += took;
    }
    println!(
        "New CJK block, mean / worst  {:8.1} / {:.1} ms",
        total / samples.len() as f64,
        worst
    );
    let t = Instant::now();
    for _ in 0..1000 {
        std::hint::black_box(snipping_tool_ul::tools::measure_text_width(&text, 24.));
    }
    println!("Loaded CJK text, per call    {:8.4} ms", ms(t) / 1000.);
}
