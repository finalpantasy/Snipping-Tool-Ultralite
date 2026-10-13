//! Renders synthetic UI evidence only. Never creates a window or captures a desktop.
use snipping_tool_ul::{
    config::Config,
    geometry::{Color, Point},
    launcher::{Model, HEIGHT, WIDTH},
    tools::render_text_annotation,
};
use std::path::PathBuf;
use tiny_skia::{Paint, PathBuilder, Pixmap, Stroke, Transform};
fn main() {
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/headless-preview"));
    std::fs::create_dir_all(&out).unwrap();
    let cfg = Config::default();
    let mut model = Model::new(cfg.workflow.clone(), cfg.hotkey.capture.clone());
    let mut canvas = Pixmap::new(WIDTH, HEIGHT).unwrap();
    model.render(&mut canvas);
    canvas.save_png(out.join("home.png")).unwrap();
    for (percent, w, h) in [(125, 900, 625), (150, 1080, 750), (200, 1440, 1000)] {
        let mut scaled = Pixmap::new(w, h).unwrap();
        model.render(&mut scaled);
        scaled
            .save_png(out.join(format!("home-dpi-{percent}.png")))
            .unwrap();
    }
    model.has_image = true;
    model.connected = true;
    model.status = "Copied to clipboard. Preview opens automatically.".into();
    model.render(&mut canvas);
    canvas.save_png(out.join("home-after-capture.png")).unwrap();
    model.connecting = true;
    model.status.clear();
    model.draft = "YourClientID".into();
    model.render(&mut canvas);
    canvas.save_png(out.join("connect-uploads.png")).unwrap();
    // Original brand mark, generated from capture-frame geometry and initials.
    let mut icon = Pixmap::new(256, 256).unwrap();
    icon.fill(tiny_skia::Color::from_rgba8(30, 36, 50, 255));
    let mut paint = Paint::default();
    paint.set_color_rgba8(157, 242, 216, 255);
    paint.anti_alias = true;
    for (x, y, dx, dy) in [
        (36., 36., 48., 0.),
        (36., 36., 0., 48.),
        (220., 36., -48., 0.),
        (220., 36., 0., 48.),
        (36., 220., 48., 0.),
        (36., 220., 0., -48.),
        (220., 220., -48., 0.),
        (220., 220., 0., -48.),
    ] {
        let mut path = PathBuilder::new();
        path.move_to(x, y);
        path.line_to(x + dx, y + dy);
        icon.stroke_path(
            &path.finish().unwrap(),
            &paint,
            &Stroke {
                width: 12.,
                line_cap: tiny_skia::LineCap::Round,
                ..Default::default()
            },
            Transform::identity(),
            None,
        );
    }
    render_text_annotation(&mut icon, &Point::new(54., 74.), "UL", &Color::white(), 87.);
    icon.save_png(out.join("icon.png")).unwrap();
    // A made-up image and real editor renderer demonstrate zoom without private screen data.
    let mut pixels = vec![0; 960 * 480 * 4];
    for y in 0..480 {
        for x in 0..960 {
            let i = (y * 960 + x) * 4;
            pixels[i..i + 4].copy_from_slice(&[
                38 + (x / 8 % 24) as u8,
                48 + (y / 8 % 32) as u8,
                76,
                255,
            ]);
        }
    }
    let screen = snipping_tool_ul::capture::CapturedScreen {
        pixels,
        width: 960,
        height: 480,
        x_offset: 0,
        y_offset: 0,
        scale_factor: 1.,
    };
    let mut state = snipping_tool_ul::state::OverlayState::new(screen, &cfg);
    state.preview_mode = true;
    state.selection = Some(snipping_tool_ul::overlay::selection::Selection {
        x: 0.,
        y: 0.,
        width: 960.,
        height: 480.,
    });
    let (cw, ch) = snipping_tool_ul::preview::canvas_size(960, 480, state.visible_buttons.len());
    state.canvas_height = ch;
    let mut p = Pixmap::new(cw, ch).unwrap();
    snipping_tool_ul::renderer::render_overlay(&mut state, &mut p);
    let mut view = snipping_tool_ul::preview::EditorView::default();
    let mut buffer = vec![0; 1100 * 700];
    for (name, steps) in [("editor-fit.png", 0.), ("editor-zoom.png", 3.)] {
        view.zoom_at(620., 270., steps, 960, 480, cw, 1100, 700, 1.);
        view.present(&p, &mut buffer, 1100, 700, 960, 480, cw, 1.);
        let mut output = Pixmap::new(1100, 700).unwrap();
        for (rgb, px) in buffer.iter().zip(output.data_mut().chunks_exact_mut(4)) {
            px.copy_from_slice(&[(*rgb >> 16) as u8, (*rgb >> 8) as u8, *rgb as u8, 255]);
        }
        output.save_png(out.join(name)).unwrap();
    }
    println!("Headless previews generated; no GUI or clipboard access.");
}
