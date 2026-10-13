//! Synthetic CPU timings, with the exact previous text/step implementation.
//! No window, screenshot, clipboard, upload or user configuration access.
#[path = "../tests/support/text_reference.rs"]
mod reference;

use snipping_tool_ul::geometry::{Color, Point};
use snipping_tool_ul::tools::{render_annotation, render_text_annotation, Annotation};
use std::{hint::black_box, time::Instant};

fn median(mut run: impl FnMut()) -> f64 {
    for _ in 0..3 {
        run();
    }
    let mut samples = Vec::new();
    for _ in 0..31 {
        let started = Instant::now();
        run();
        samples.push(started.elapsed().as_secs_f64() * 1000.);
    }
    samples.sort_by(f64::total_cmp);
    samples[15]
}

fn main() {
    let mut p = tiny_skia::Pixmap::new(660, 390).unwrap();
    let color = Color::new(0.3, 0.8, 0.1, 0.7);
    for (label, position, size, text) in [
        (
            "visible small text",
            Point::new(30., 30.),
            20.,
            "Short caption 123",
        ),
        (
            "clipped zoomed text",
            Point::new(600., -60.),
            256.,
            "Zoomed annotation text outside the viewport",
        ),
        (
            "offscreen zoomed text",
            Point::new(900., 30.),
            256.,
            "Zoomed annotation text outside the viewport",
        ),
    ] {
        let old = median(|| {
            reference::reference_text(&mut p, &position, text, &color, size);
            black_box(p.data());
        });
        let new = median(|| {
            render_text_annotation(&mut p, &position, text, &color, size);
            black_box(p.data());
        });
        println!(
            "{label}: previous={old:.4}ms optimized={new:.4}ms ratio={:.2}x",
            old / new
        );
    }
    let markers: Vec<_> = (10..34)
        .map(|number| Annotation::StepMarker {
            position: Point::new(
                40. + (number % 12) as f32 * 45.,
                90. + (number / 12) as f32 * 50.,
            ),
            number,
            color,
            size: 56.,
        })
        .collect();
    let old = median(|| {
        for a in &markers {
            reference::reference_marker(a, &mut p);
        }
        black_box(p.data());
    });
    let new = median(|| {
        for a in &markers {
            render_annotation(a, &mut p, None, None);
        }
        black_box(p.data());
    });
    println!(
        "24 numbered steps: previous={old:.4}ms optimized={new:.4}ms ratio={:.2}x",
        old / new
    );
}
