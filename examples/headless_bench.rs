//! CPU-only synthetic rendering benchmark; no HWND, desktop capture, clipboard or upload.
use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    editor_ui::EditorUi,
    launcher::{Model, HEIGHT, WIDTH},
    overlay::selection::Selection,
    preview::EditorView,
    state::OverlayState,
};
use std::{hint::black_box, time::Instant};
fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}
fn main() {
    let cfg = Config::default();
    let model = Model::new(cfg.workflow.clone(), cfg.hotkey.capture.clone());
    let mut home = tiny_skia::Pixmap::new(WIDTH, HEIGHT).unwrap();
    let start = Instant::now();
    model.render(&mut home);
    let cold = start.elapsed().as_secs_f64() * 1000.;
    let mut samples = Vec::new();
    for _ in 0..30 {
        let start = Instant::now();
        model.render(&mut home);
        black_box(home.data());
        samples.push(start.elapsed().as_secs_f64() * 1000.);
    }
    println!(
        "Home CPU render: first={cold:.3} ms, warm median30={:.3} ms; pixel buffer={} bytes",
        median(&mut samples),
        home.data().len()
    );
    for scale in [1., 1.5, 2.] {
        let mut settings = snipping_tool_ul::settings_ui::SettingsUi::new(Config::default());
        settings.active_tab = 2;
        let mut frame = tiny_skia::Pixmap::new(
            (snipping_tool_ul::settings_ui::WIN_W as f32 * scale) as u32,
            (snipping_tool_ul::settings_ui::WIN_H as f32 * scale) as u32,
        )
        .unwrap();
        settings.draw(&mut frame);
        let mut samples = Vec::new();
        for _ in 0..30 {
            let start = Instant::now();
            settings.draw(&mut frame);
            black_box(frame.data());
            samples.push(start.elapsed().as_secs_f64() * 1000.);
        }
        println!(
            "Settings at {}% DPI: CPU render median30={:.3} ms; framebuffer={} bytes",
            (scale * 100.) as u32,
            median(&mut samples),
            frame.data().len()
        );
    }
    for (iw, ih) in [(960u32, 540u32), (1920, 1080), (3840, 2160)] {
        let screen = CapturedScreen {
            pixels: vec![255; (iw * ih * 4) as usize],
            width: iw,
            height: ih,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        };
        let mut state = OverlayState::new(screen, &cfg);
        state.preview_mode = true;
        state.selection = Some(Selection {
            x: 0.,
            y: 0.,
            width: iw as f32,
            height: ih as f32,
        });
        state.canvas_height = ih;
        let mut p = tiny_skia::Pixmap::new(iw, ih).unwrap();
        let mut dst = tiny_skia::Pixmap::new(1410, 918).unwrap();
        let ui = EditorUi::new(&cfg);
        let mut view = EditorView::default();
        view.zoom_framed(740., 310., 3., iw, ih, 1410, 918, 1.5);
        let mut samples = Vec::new();
        for _ in 0..21 {
            let start = Instant::now();
            snipping_tool_ul::renderer::render_overlay(&mut state, &mut p);
            ui.draw_base(&mut dst, 1.5);
            view.paint_framed(&p, &mut dst, 1.5);
            ui.draw_controls(
                &mut dst,
                1.5,
                &mut state,
                view.framed_layout(iw, ih, 1410, 918, 1.5).scale,
            );
            black_box(dst.data());
            samples.push(start.elapsed().as_secs_f64() * 1000.);
        }
        println!("Editor {iw}x{ih} at 1410x918 viewport (150% DPI): CPU frame median21={:.3} ms; retained image/render/output pixel buffers={} bytes",median(&mut samples),state.screenshot.pixels.len()+p.data().len()+dst.data().len());
        let mut renderer = snipping_tool_ul::preview_render::PreviewRenderer::default();
        renderer.draw(&state, &mut dst, &view, 1.5);
        let mut samples = Vec::new();
        for _ in 0..21 {
            let start = Instant::now();
            ui.draw_base(&mut dst, 1.5);
            renderer.draw(&state, &mut dst, &view, 1.5);
            ui.draw_controls(
                &mut dst,
                1.5,
                &mut state,
                view.framed_layout(iw, ih, 1410, 918, 1.5).scale,
            );
            black_box(dst.data());
            samples.push(start.elapsed().as_secs_f64() * 1000.);
        }
        println!("Viewport editor {iw}x{ih}: warm CPU median21={:.3} ms; retained capture/viewport/frame pixels={} bytes; image resamples={}",median(&mut samples),state.screenshot.pixels.len()+renderer.retained_bytes()+dst.data().len(),renderer.resamples);
    }
}
