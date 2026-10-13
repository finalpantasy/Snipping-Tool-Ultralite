//! Synthetic CPU benchmark: no windows, capture, clipboard or uploads.
use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    geometry::{Color, Point, Size},
    preview::EditorView,
    preview_render::PreviewRenderer,
    state::OverlayState,
    tools::Annotation,
};
use std::{hint::black_box, time::Instant};
fn main() {
    let (w, h) = (3840, 2160);
    let mut pixels = vec![255; w as usize * h as usize * 4];
    for (i, p) in pixels.chunks_exact_mut(4).enumerate() {
        p[0] = (i % 251) as u8;
        p[1] = (i / 3840 % 251) as u8;
        p[2] = (i / 17 % 251) as u8;
    }
    let mut state = OverlayState::new(
        CapturedScreen {
            pixels,
            width: w,
            height: h,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        },
        &Config::default(),
    );
    state.preview_mode = true;
    for i in 0..24 {
        state.annotations.push(Annotation::Arrow {
            start: Point::new(50. + i as f32 * 90., 150.),
            end: Point::new(170. + i as f32 * 90., 1200.),
            color: Color::red(),
            thickness: 12.,
        });
    }
    state.annotations.push(Annotation::Pencil {
        points: (0..3000)
            .map(|i| Point::new(i as f32, 800. + (i as f32 * 0.03).sin() * 200.))
            .collect(),
        color: Color::blue(),
        thickness: 8.,
    });
    let mut renderer = PreviewRenderer::default();
    let view = EditorView::default();
    let mut frame = tiny_skia::Pixmap::new(1410, 918).unwrap();
    renderer.draw(&state, &mut frame, &view, 1.5);
    measure("unchanged edited viewport", 31, || {
        renderer.draw(&state, &mut frame, &view, 1.5);
        black_box(frame.data());
    });
    println!("renderer_retained_bytes={}", renderer.retained_bytes());
    for block in [12, 2] {
        let ann = Annotation::Pixelate {
            top_left: Point::new(0., 0.),
            size: Size::new(w as f32, h as f32),
            block_size: block,
        };
        measure(&format!("4K pixelation export block {block}"), 5, || {
            black_box(snipping_tool_ul::export::flatten_annotations(
                &state.screenshot.pixels,
                w,
                h,
                std::slice::from_ref(&ann),
            ));
        });
    }
}
fn measure(label: &str, n: usize, mut run: impl FnMut()) {
    let mut samples = Vec::new();
    for _ in 0..n {
        let start = Instant::now();
        run();
        samples.push(start.elapsed().as_secs_f64() * 1000.);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "{label}: median={:.3}ms max={:.3}ms",
        samples[n / 2],
        samples[n - 1]
    );
}
