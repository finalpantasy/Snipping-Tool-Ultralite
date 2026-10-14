//! Production renderer on authored pixels; no windows, capture or clipboard.
use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    editor_ui::EditorUi,
    geometry::{Color, Point, Size},
    overlay::selection::Selection,
    preview::EditorView,
    preview_render::PreviewRenderer,
    state::OverlayState,
    theme::{self, ThemeMode},
    tools::{self, Annotation, AnnotationTool, ResizeHandle, ToolKind},
};
fn main() {
    let out = std::path::Path::new("target/canvas-preview");
    std::fs::create_dir_all(out).unwrap();
    let mut image = tiny_skia::Pixmap::new(320, 180).unwrap();
    image.fill(tiny_skia::Color::from_rgba8(32, 79, 109, 255));
    tools::render_text_annotation(
        &mut image,
        &Point::new(24., 30.),
        "Synthetic reference",
        &Color::white(),
        22.,
    );
    tools::render_text_annotation(
        &mut image,
        &Point::new(24., 78.),
        "Original image: 320 x 180",
        &Color::white(),
        16.,
    );
    let mut pixels = image.take();
    for p in pixels.chunks_exact_mut(4) {
        p[3] = 255;
    }
    for (mode, name) in [(ThemeMode::Dark, "dark"), (ThemeMode::Light, "light")] {
        theme::set_mode(mode);
        for dpi in [1., 1.5] {
            let config = Config::default();
            let mut state = OverlayState::new(
                CapturedScreen {
                    pixels: pixels.clone(),
                    width: 320,
                    height: 180,
                    x_offset: 0,
                    y_offset: 0,
                    scale_factor: 1.,
                },
                &config,
            );
            state.preview_mode = true;
            state.selection = Some(Selection {
                x: 0.,
                y: 0.,
                width: 320.,
                height: 180.,
            });
            state.active_tool = ToolKind::Rectangle;
            let mut renderer = PreviewRenderer::default();
            let view = EditorView::default();
            let ui = EditorUi::new(&config);
            for step in 0..5 {
                match step {
                    1 => {
                        state.rectangle_tool.on_mouse_down(Point::new(80., 50.));
                        state.rectangle_tool.on_mouse_move(Point::new(430., 260.));
                    }
                    2 => {
                        state.rectangle_tool.on_mouse_up(Point::new(430., 260.));
                        state.annotations.push(Annotation::Rectangle {
                            top_left: Point::new(-60., -45.),
                            size: Size::new(490., 305.),
                            color: Color::red(),
                            thickness: 6.,
                        });
                        state.selected_index = Some(0);
                        state.active_tool = ToolKind::Select;
                    }
                    3 => tools::resize_annotation(
                        &mut state.annotations[0],
                        ResizeHandle::BottomRight,
                        Point::new(220., 120.),
                    ),
                    4 => {
                        state.delete_annotation(0);
                    }
                    _ => {}
                }
                let mut frame =
                    tiny_skia::Pixmap::new((900. * dpi) as u32, (650. * dpi) as u32).unwrap();
                ui.draw_base(&mut frame, dpi as f32);
                renderer.draw(&state, &mut frame, &view, dpi);
                ui.draw_controls(&mut frame, dpi as f32, &mut state, 1.);
                frame
                    .save_png(out.join(format!("{name}-{}-{step}.png", (dpi * 100.) as u32)))
                    .unwrap();
                if step == 2 && dpi == 1. && name == "dark" {
                    let (pixels, w, h) = state.flattened_selection().unwrap();
                    image::save_buffer(
                        out.join("expanded-export.png"),
                        &pixels,
                        w,
                        h,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                }
            }
        }
    }
    println!("20 headless editor renders and one full-resolution export generated.");
}
