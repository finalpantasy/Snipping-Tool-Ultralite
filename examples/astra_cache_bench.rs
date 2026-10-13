//! Synthetic renderer comparison. Does not create windows or access user data.
#![allow(dead_code)]
#[path = "../tests/support/astra_renderer_before.rs"]
mod before;
pub use snipping_tool_ul::{capture, editor_ui, geometry, preview, state, theme, tools, ui};
use snipping_tool_ul::{config::Config, preview_render::PreviewRenderer};
use std::{hint::black_box, time::Instant};

fn scene(pixelate: bool) -> state::OverlayState {
    let (w, h) = (1280, 720);
    let mut pixels = vec![255; w * h * 4];
    for (i, p) in pixels.chunks_exact_mut(4).enumerate() {
        p[0] = (i % 251) as u8;
        p[1] = (i / w % 233) as u8;
    }
    let mut s = state::OverlayState::new(
        capture::CapturedScreen {
            pixels,
            width: w as u32,
            height: h as u32,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        },
        &Config::default(),
    );
    s.preview_mode = true;
    for i in 0..12 {
        let p = geometry::Point::new((i % 4) as f32 * 280. + 30., (i / 4) as f32 * 200. + 30.);
        s.annotations.push(if pixelate {
            tools::Annotation::Pixelate {
                top_left: p,
                size: geometry::Size::new(230., 140.),
                block_size: 8,
            }
        } else {
            tools::Annotation::Arrow {
                start: p,
                end: geometry::Point::new(p.x + 160., p.y + 90.),
                color: geometry::Color::red(),
                thickness: 5.,
            }
        });
    }
    s
}
fn measure(mut run: impl FnMut() -> (u64, u64, u64)) -> (f64, (u64, u64, u64)) {
    for _ in 0..2 {
        black_box(run());
    }
    let mut samples = Vec::new();
    let mut counts = (0, 0, 0);
    for _ in 0..9 {
        let start = Instant::now();
        counts = black_box(run());
        samples.push(start.elapsed().as_secs_f64() * 1000.);
    }
    samples.sort_by(f64::total_cmp);
    (samples[4], counts)
}
macro_rules! sequence {
    ($renderer:ty, $scene:expr, $pan:expr) => {{
        let mut r = <$renderer>::default();
        let mut view = preview::EditorView::default();
        let mut frame = tiny_skia::Pixmap::new(1920, 1080).unwrap();
        r.draw($scene, &mut frame, &view, 1.);
        if $pan {
            assert!(view.begin_pan(960., 500., 1920, 1080, 1.));
        }
        for step in 1..21 {
            if $pan {
                assert!(view.drag_pan(960. + step as f64, 500.));
            } else {
                view.zoom = 1. + step as f64 * 0.01;
            }
            r.draw($scene, &mut frame, &view, 1.);
            black_box(frame.data());
        }
        (r.resamples, r.compositions, r.pixelation_builds())
    }};
}
fn main() {
    for (name, pixelate, pan) in [
        ("integer_pan", false, true),
        ("twelve_pixelations_zoom", true, false),
    ] {
        let state = scene(pixelate);
        let old = measure(|| sequence!(before::PreviewRenderer, &state, pan));
        let new = measure(|| sequence!(PreviewRenderer, &state, pan));
        println!("{name}: 21 frames median old={:.3}ms new={:.3}ms ratio={:.2}x counts(resamples,compositions,grids) old={:?} new={:?}",old.0,new.0,old.0/new.0,old.1,new.1);
    }
}
