//! Synthetic CPU-only edited zoom benchmark. No windows or native services.
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
    for case in ["blank", "marks", "pixelate12", "pixelate2"] {
        let mut pixels = vec![255; 3840 * 2160 * 4];
        for (i, p) in pixels.chunks_exact_mut(4).enumerate() {
            p[0] = (i % 251) as u8;
            p[1] = (i / 3840 % 251) as u8;
            p[2] = (i / 17 % 251) as u8;
        }
        let mut state = OverlayState::new(
            CapturedScreen {
                pixels,
                width: 3840,
                height: 2160,
                x_offset: 0,
                y_offset: 0,
                scale_factor: 1.,
            },
            &Config::default(),
        );
        state.preview_mode = true;
        if case == "marks" {
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
        }
        if case.starts_with("pixelate") {
            state.annotations.push(Annotation::Pixelate {
                top_left: Point::new(0., 0.),
                size: Size::new(3840., 2160.),
                block_size: if case == "pixelate12" { 12 } else { 2 },
            });
        }
        let mut renderer = PreviewRenderer::default();
        let mut view = EditorView::default();
        let mut frame = tiny_skia::Pixmap::new(1410, 918).unwrap();
        let mut ms = vec![];
        for i in 0..9 {
            view.zoom_framed(
                650.,
                350.,
                if i < 5 { 1. } else { -1. },
                3840,
                2160,
                1410,
                918,
                1.5,
            );
            let t = Instant::now();
            renderer.draw(&state, &mut frame, &view, 1.5);
            black_box(frame.data());
            ms.push(t.elapsed().as_secs_f64() * 1000.);
        }
        ms.sort_by(f64::total_cmp);
        println!(
            "{case}: median={:.3}ms max={:.3}ms retained={} bytes",
            ms[ms.len() / 2],
            ms.last().unwrap(),
            renderer.retained_bytes()
        );
    }
}
