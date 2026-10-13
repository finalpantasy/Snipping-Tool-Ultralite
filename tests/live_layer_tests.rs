//! Dragging a mark repaints only the live layer; pixels match a full render.
use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    geometry::{Color, Point, Size},
    preview::EditorView,
    preview_render::PreviewRenderer,
    state::OverlayState,
    tools::{Annotation, AnnotationTool, ToolKind},
};

fn state() -> OverlayState {
    let (w, h) = (240u32, 160u32);
    let mut pixels = vec![255; (w * h * 4) as usize];
    for (i, px) in pixels.chunks_exact_mut(4).enumerate() {
        px[0] = (i * 11 % 251) as u8;
        px[1] = (i / 240 * 7 % 251) as u8;
        px[2] = (i * 3 % 251) as u8;
    }
    let mut s = OverlayState::new(
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
    s.preview_mode = true;
    s.annotations.push(Annotation::Arrow {
        start: Point::new(20., 20.),
        end: Point::new(180., 130.),
        color: Color::red(),
        thickness: 6.,
    });
    s.annotations.push(Annotation::Spotlight {
        top_left: Point::new(100., 30.),
        size: Size::new(80., 60.),
    });
    s.selected_index = Some(0);
    s
}

fn fresh(s: &OverlayState) -> tiny_skia::Pixmap {
    let mut p = tiny_skia::Pixmap::new(660, 390).unwrap();
    PreviewRenderer::default().draw(s, &mut p, &EditorView::default(), 1.);
    p
}

#[test]
fn pencil_drag_reuses_committed_layer_and_matches_full_render() {
    let mut s = state();
    s.active_tool = ToolKind::Pencil;
    let mut renderer = PreviewRenderer::default();
    let mut frame = tiny_skia::Pixmap::new(660, 390).unwrap();
    renderer.draw(&s, &mut frame, &EditorView::default(), 1.);
    assert_eq!(renderer.compositions, 1);
    s.pencil_tool.on_mouse_down(Point::new(30., 140.));
    for step in 1..25 {
        s.pencil_tool
            .on_mouse_move(Point::new(30. + step as f32 * 7., 140. - step as f32 * 4.));
        frame.fill(tiny_skia::Color::TRANSPARENT);
        renderer.draw(&s, &mut frame, &EditorView::default(), 1.);
        assert_eq!(frame.data(), fresh(&s).data(), "pencil step {step}");
    }
    assert_eq!(renderer.compositions, 1, "committed marks were recomposed");
    let done = s.pencil_tool.on_mouse_up(Point::new(220., 40.)).unwrap();
    s.annotations.push(done);
    frame.fill(tiny_skia::Color::TRANSPARENT);
    renderer.draw(&s, &mut frame, &EditorView::default(), 1.);
    assert_eq!(frame.data(), fresh(&s).data(), "after commit");
    assert_eq!(renderer.compositions, 2);
}

#[test]
fn live_spotlight_and_typed_text_match_full_render() {
    let mut s = state();
    s.active_tool = ToolKind::Spotlight;
    let mut renderer = PreviewRenderer::default();
    let mut frame = tiny_skia::Pixmap::new(660, 390).unwrap();
    s.spotlight_tool.on_mouse_down(Point::new(10., 100.));
    for step in 1..6 {
        s.spotlight_tool
            .on_mouse_move(Point::new(10. + step as f32 * 15., 100. + step as f32 * 8.));
        frame.fill(tiny_skia::Color::TRANSPARENT);
        renderer.draw(&s, &mut frame, &EditorView::default(), 1.);
        assert_eq!(frame.data(), fresh(&s).data(), "spotlight step {step}");
    }
    let _ = s.spotlight_tool.on_mouse_up(Point::new(90., 140.));
    s.active_tool = ToolKind::Text;
    s.text_input_active = true;
    s.text_input_position = Point::new(40., 60.);
    for text in ["g", "gy", "gyp jq"] {
        s.text_input_buffer = text.into();
        frame.fill(tiny_skia::Color::TRANSPARENT);
        renderer.draw(&s, &mut frame, &EditorView::default(), 1.);
        assert_eq!(frame.data(), fresh(&s).data(), "typing {text}");
    }
}

#[test]
fn switching_to_direct_spotlight_releases_the_previous_live_layer() {
    let mut s = state();
    let mut renderer = PreviewRenderer::default();
    let mut frame = tiny_skia::Pixmap::new(660, 390).unwrap();
    s.active_tool = ToolKind::Pencil;
    s.pencil_tool.on_mouse_down(Point::new(10., 10.));
    s.pencil_tool.on_mouse_move(Point::new(70., 70.));
    renderer.draw(&s, &mut frame, &EditorView::default(), 1.);
    let _ = s.pencil_tool.on_mouse_up(Point::new(70., 70.));
    s.active_tool = ToolKind::Spotlight;
    s.spotlight_tool.on_mouse_down(Point::new(20., 20.));
    s.spotlight_tool.on_mouse_move(Point::new(90., 90.));
    frame.fill(tiny_skia::Color::TRANSPARENT);
    renderer.draw(&s, &mut frame, &EditorView::default(), 1.);
    let mut fresh_renderer = PreviewRenderer::default();
    let mut expected = tiny_skia::Pixmap::new(660, 390).unwrap();
    fresh_renderer.draw(&s, &mut expected, &EditorView::default(), 1.);
    assert_eq!(frame.data(), expected.data());
    assert_eq!(renderer.retained_bytes(), fresh_renderer.retained_bytes());
}

#[test]
fn panning_live_mark_offscreen_releases_its_buffer() {
    let mut s = state();
    s.active_tool = ToolKind::Pencil;
    let mut renderer = PreviewRenderer::default();
    let mut frame = tiny_skia::Pixmap::new(660, 390).unwrap();
    let mut view = EditorView::default();
    renderer.draw(&s, &mut frame, &view, 1.);
    let idle_bytes = renderer.retained_bytes();
    s.pencil_tool.on_mouse_down(Point::new(20., 20.));
    s.pencil_tool.on_mouse_move(Point::new(50., 50.));
    renderer.draw(&s, &mut frame, &view, 1.);
    assert!(renderer.retained_bytes() > idle_bytes);
    assert!(view.begin_pan(320., 220., 660, 390, 1.));
    assert!(view.drag_pan(5000., 5000.));
    let _ = s.pencil_tool.on_mouse_up(Point::new(50., 50.));
    renderer.draw(&s, &mut frame, &view, 1.);
    assert_eq!(renderer.retained_bytes(), idle_bytes);
}
