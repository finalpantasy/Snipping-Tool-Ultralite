//! Actual editor with authored reference pixels, never desktop screenshots.
use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    editor_ui::EditorUi,
    geometry::{Color, Point, Size},
    icons::IconCache,
    overlay::selection::Selection,
    preview::EditorView,
    preview_render::PreviewRenderer,
    state::OverlayState,
    theme::{self, ThemeMode},
    tools::{Annotation, ToolKind},
    ui::Canvas,
};
fn main() {
    let out = std::path::Path::new("target/editing-preview");
    std::fs::create_dir_all(out).unwrap();
    let mut doc = tiny_skia::Pixmap::new(500, 240).unwrap();
    doc.fill(tiny_skia::Color::WHITE);
    let mut icons = IconCache::new();
    let mut c = Canvas {
        pixmap: &mut doc,
        scale: 1.,
        icons: &mut icons,
    };
    c.label(30., 20., "Captured reference", 22., (35, 36, 43));
    c.label(
        30.,
        65.,
        "Words stay readable under a highlight.",
        16.,
        (35, 36, 43),
    );
    c.label(30., 104., "Private detail", 16., (35, 36, 43));
    c.label(
        30.,
        196.,
        "Synthetic image for UI verification.",
        12.,
        (110, 112, 126),
    );
    for (mode, name) in [(ThemeMode::Dark, "dark"), (ThemeMode::Light, "light")] {
        theme::set_mode(mode);
        for (tool, label) in [
            (ToolKind::Pencil, "pencil"),
            (ToolKind::Text, "text"),
            (ToolKind::Pixelate, "pixelate"),
            (ToolKind::Highlight, "highlight"),
        ] {
            let mut o = OverlayState::new(
                CapturedScreen {
                    pixels: doc.data().to_vec(),
                    width: 500,
                    height: 240,
                    x_offset: 0,
                    y_offset: 0,
                    scale_factor: 1.,
                },
                &Config::default(),
            );
            o.preview_mode = true;
            o.selection = Some(Selection {
                x: 0.,
                y: 0.,
                width: 500.,
                height: 240.,
            });
            o.active_tool = tool;
            let mut ui = EditorUi::new(&Config::default());
            ui.header.status = "Copied to clipboard".into();
            match tool {
                ToolKind::Pencil => {
                    o.set_thickness_all(6.);
                    let points = (0..=120)
                        .map(|n| {
                            let a = n as f32 * std::f32::consts::TAU / 120.;
                            Point::new(395. + a.cos() * 42., 136. + a.sin() * 42.)
                        })
                        .collect();
                    o.annotations.push(Annotation::Pencil {
                        points,
                        color: Color::red(),
                        thickness: 6.,
                    });
                }
                ToolKind::Text => {
                    o.set_tool_size(24., false);
                    o.text_input_active = true;
                    o.text_input_position = Point::new(30., 145.);
                    o.text_input_buffer = "UL: readable gyp jq É".into();
                }
                ToolKind::Pixelate => {
                    o.annotations.push(Annotation::Pixelate {
                        top_left: Point::new(27., 103.),
                        size: Size::new(100., 22.),
                        block_size: 10,
                    });
                    o.selected_index = Some(0);
                }
                ToolKind::Highlight => {
                    o.annotations.push(Annotation::Highlight {
                        top_left: Point::new(27., 66.),
                        size: Size::new(290., 20.),
                        color: Color::yellow(),
                    });
                    o.selected_index = Some(0);
                }
                _ => {}
            }
            for (w, h, dpi, suffix) in [
                (940, 612, 1., ""),
                (660, 390, 1., "-minimum"),
                (1410, 918, 1.5, "-dpi150"),
            ] {
                let mut frame = tiny_skia::Pixmap::new(w, h).unwrap();
                ui.draw_base(&mut frame, dpi as f32);
                let view = EditorView::default();
                PreviewRenderer::default().draw(&o, &mut frame, &view, dpi);
                ui.draw_controls(
                    &mut frame,
                    dpi as f32,
                    &mut o,
                    view.framed_layout(500, 240, w, h, dpi).scale,
                );
                let rgb = if mode == ThemeMode::Dark {
                    (24, 24, 30)
                } else {
                    (248, 249, 252)
                };
                for p in frame.data_mut().chunks_exact_mut(4) {
                    let inv = 255 - p[3] as u32;
                    for (i, b) in [rgb.0, rgb.1, rgb.2].iter().enumerate() {
                        p[i] = (p[i] as u32 + (*b as u32 * inv + 127) / 255) as u8;
                    }
                    p[3] = 255;
                }
                frame
                    .save_png(out.join(format!("{name}-{label}{suffix}.png")))
                    .unwrap();
            }
        }
    }
    theme::set_mode(ThemeMode::Dark);
    println!("Light/dark live editing rendered headlessly at default, minimum and 150% sizes.");
}
