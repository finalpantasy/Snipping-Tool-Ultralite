//! Application client rendering only: synthetic content, no HWND or capture/clipboard access.
use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    editor_ui::EditorUi,
    geometry::{Color, Point},
    icons::IconCache,
    launcher::{Menu, Model, HEIGHT, WIDTH},
    overlay::selection::Selection,
    preview::EditorView,
    state::OverlayState,
    tools::{Annotation, ToolKind},
    ui::{Canvas, Rect},
};
use std::path::PathBuf;
use tiny_skia::Pixmap;
fn save(p: &Pixmap, path: PathBuf) {
    let mut out = p.clone();
    for (i, px) in out.data_mut().chunks_exact_mut(4).enumerate() {
        let y = i / p.width() as usize;
        let x = i % p.width() as usize;
        let t = (x as f32 / p.width() as f32 + y as f32 / p.height() as f32) / 2.;
        let bg = [
            (25. + 15. * t) as u8,
            (25. + 10. * t) as u8,
            (34. + 17. * t) as u8,
        ];
        let inv = 255 - px[3] as u32;
        for c in 0..3 {
            px[c] = (px[c] as u32 + (bg[c] as u32 * inv + 127) / 255) as u8;
        }
        px[3] = 255;
    }
    out.save_png(path).unwrap();
}
fn main() {
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| "target/frosted-preview".into());
    std::fs::create_dir_all(&out).unwrap();
    let cfg = Config::default();
    let mut home = Model::new(cfg.workflow.clone(), cfg.hotkey.capture.clone());
    for (name, scale) in [("home", 1.), ("home-dpi150", 1.5)] {
        let mut p = Pixmap::new(
            (WIDTH as f32 * scale) as u32,
            (HEIGHT as f32 * scale) as u32,
        )
        .unwrap();
        home.render(&mut p);
        save(&p, out.join(format!("{name}.png")));
    }
    home.menu = Menu::Options;
    let mut p = Pixmap::new(WIDTH, HEIGHT).unwrap();
    home.render(&mut p);
    save(&p, out.join("home-options.png"));
    let mut scaled = Pixmap::new(WIDTH * 2, HEIGHT * 2).unwrap();
    home.render(&mut scaled);
    save(&scaled, out.join("home-options-dpi200.png"));
    home.menu = Menu::None;
    home.connecting = true;
    home.draft = "YourClientID".into();
    home.render(&mut p);
    save(&p, out.join("connection.png"));
    // Authored document for an illustrative capture. None of this comes from a desktop.
    let mut doc = Pixmap::new(644, 354).unwrap();
    doc.fill(tiny_skia::Color::from_rgba8(238, 234, 227, 255));
    let mut icons = IconCache::new();
    let mut c = Canvas {
        pixmap: &mut doc,
        scale: 1.,
        icons: &mut icons,
    };
    c.label(38., 17., "Weekend route", 25., (37, 38, 42));
    c.label(
        38.,
        53.,
        "A few things to check before heading out.",
        14.,
        (115, 113, 111),
    );
    c.round(Rect::new(38., 92., 568., 1.), 0., (215, 210, 203), 1.);
    for (i, (time, label)) in [
        ("09:00", "Coffee & a slow start"),
        ("10:30", "Take the riverside trail"),
        ("12:00", "Lunch near the old bridge"),
        ("14:00", "Leave time for the lookout"),
    ]
    .iter()
    .enumerate()
    {
        let y = 109. + i as f32 * 43.;
        c.label(40., y, time, 14., (138, 133, 125));
        c.label(120., y, label, 17., (55, 56, 59));
    }
    c.label(38., 306., "Trail map + water bottle", 13., (133, 128, 122));
    let screen = CapturedScreen {
        pixels: doc.data().to_vec(),
        width: 644,
        height: 354,
        x_offset: 0,
        y_offset: 0,
        scale_factor: 1.,
    };
    let mut state = OverlayState::new(screen, &cfg);
    state.preview_mode = true;
    state.active_tool = ToolKind::Pencil;
    state.selection = Some(Selection {
        x: 0.,
        y: 0.,
        width: 644.,
        height: 354.,
    });
    state.annotations.push(Annotation::Arrow {
        start: Point::new(449., 178.),
        end: Point::new(368., 157.),
        color: Color::new(0.875, 0.385, 0.345, 1.),
        thickness: 3.2,
    });
    let mut source = Pixmap::new(644, 354).unwrap();
    snipping_tool_ul::renderer::render_overlay(&mut state, &mut source);
    let mut ui = EditorUi::new(&cfg);
    ui.header.status = "Copied to clipboard".into();
    for (name, scale, menu, steps) in [
        ("editor", 1., Menu::None, 0.),
        ("editor-dpi150", 1.5, Menu::None, 0.),
        ("editor-tools", 1., Menu::Tools, 0.),
        ("editor-colors", 1., Menu::Colors, 0.),
        ("editor-zoom", 1., Menu::None, 3.),
        ("editor-hint", 1., Menu::None, 0.),
        ("editor-minimum", 1., Menu::None, 0.),
        ("editor-minimum-tools", 1., Menu::Tools, 0.),
        ("editor-minimum-dpi200", 2., Menu::None, 0.),
    ] {
        let (lw, lh) = if name.starts_with("editor-minimum") {
            (660., 390.)
        } else {
            (940., 612.)
        };
        let (w, h) = ((lw * scale) as u32, (lh * scale) as u32);
        let mut frame = Pixmap::new(w, h).unwrap();
        ui.header.menu = menu;
        ui.header.hovered = if name == "editor-hint" {
            Some(snipping_tool_ul::launcher::Action::Button(6))
        } else {
            None
        };
        let mut view = EditorView::default();
        view.zoom_framed(530. * scale, 230. * scale, steps, 644, 354, w, h, scale);
        ui.draw_base(&mut frame, scale as f32);
        view.paint_framed(&source, &mut frame, scale);
        ui.draw_controls(
            &mut frame,
            scale as f32,
            &mut state,
            view.framed_layout(644, 354, w, h, scale).scale,
        );
        save(&frame, out.join(format!("{name}.png")));
    }
    println!("Application clients rendered headlessly; background is illustrative.");
}
